use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::{
    database::DatabaseManager,
    models::{AggregateRequest, AggregateResponse},
};

pub async fn aggregate_keys(
    db: web::Data<DatabaseManager>,
    req: web::Json<AggregateRequest>,
) -> Result<HttpResponse> {
    println!("Aggregating key shares for user: {}", req.user_id);
    
    // Fetch all key shares for the user from all databases
    let shares = match db.get_all_user_shares(&req.user_id).await {
        Ok(shares) => shares,
        Err(e) => {
            println!("Failed to fetch key shares for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to fetch key shares from databases"
            })));
        }
    };

    // Check if we have enough shares (need at least threshold)
    if shares.is_empty() {
        println!("No key shares found for user: {}", req.user_id);
        return Ok(HttpResponse::NotFound().json(json!({
            "error": "No key shares found for user"
        })));
    }

    // Verify all shares have the same public key and threshold
    let first_share = &shares[0];
    let expected_public_key = first_share.public_key.clone();
    let threshold = first_share.threshold;
    
    for share in &shares {
        if share.public_key != expected_public_key {
            println!("Mismatched public keys in shares for user: {}", req.user_id);
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "Inconsistent public keys across shares"
            })));
        }
    }

    if shares.len() < threshold as usize {
        println!("Insufficient shares for user {}: found {}, need {}", 
                 req.user_id, shares.len(), threshold);
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": format!("Insufficient shares: found {}, need {}", shares.len(), threshold)
        })));
    }

    // Sort shares by index to ensure correct reconstruction order
    let mut sorted_shares = shares;
    sorted_shares.sort_by_key(|s| s.share_index);

    // This is a simplified reconstruction - in a real implementation, 
    // you would use proper secret sharing algorithms like Shamir's Secret Sharing
    let mut reconstructed_private_key = String::new();
    let mut share_indices_used = Vec::new();

    // Take the required number of shares (threshold)
    for share in sorted_shares.iter().take(threshold as usize) {
        reconstructed_private_key.push_str(&share.encrypted_share);
        share_indices_used.push(share.share_index);
        
        println!("Using share {} for user {}: {}", 
                 share.share_index, req.user_id, share.encrypted_share);
    }

    println!("Successfully reconstructed private key for user: {}", req.user_id);
    println!("Reconstructed key: {}", reconstructed_private_key);
    println!("Used shares: {:?}", share_indices_used);

    let response = AggregateResponse {
        user_id: req.user_id.clone(),
        public_key: expected_public_key,
        private_key: reconstructed_private_key,
        shares_used: share_indices_used,
        success: true,
    };

    Ok(HttpResponse::Ok().json(response))
}
use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use std::collections::HashMap;

use crate::{
    models::{AggregateRequest, AggregateResponse},
    database::DatabaseManager,
    crypto::MPCCrypto,
};

pub async fn aggregate_keys(
    db: web::Data<DatabaseManager>,
    req: web::Json<AggregateRequest>,
) -> Result<HttpResponse> {
    log::info!("Aggregating keys for signature - user: {}", req.user_id);
    
    // Retrieve all shares for the user
    let shares = match db.get_all_user_shares(&req.user_id).await {
        Ok(shares) => shares,
        Err(e) => {
            log::error!("Failed to retrieve shares for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to retrieve key shares"
            })));
        }
    };
    
    if shares.len() < 2 {
        log::warn!("Insufficient shares for user {}: found {}", req.user_id, shares.len());
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Insufficient key shares for signing"
        })));
    }
    
    log::info!("Found {} shares for user {}", shares.len(), req.user_id);
    
    // Decrypt and prepare shares for signing
    let mut decrypted_shares: HashMap<u16, Vec<u8>> = HashMap::new();
    
    for share in &shares {
        let encrypted_data = match hex::decode(&share.encrypted_share) {
            Ok(data) => data,
            Err(e) => {
                log::error!("Failed to decode share for user {}: {}", req.user_id, e);
                return Ok(HttpResponse::InternalServerError().json(json!({
                    "error": "Failed to decode key share"
                })));
            }
        };
        
        // Decrypt the share
        let decrypted_share = MPCCrypto::simple_decrypt(&encrypted_data, share.share_index as u16);
        decrypted_shares.insert(share.share_index as u16, decrypted_share);
    }
    
    // Create message hash
    let message_hash = MPCCrypto::create_message_hash(&req.message);
    
    // Perform threshold signing
    let signature = match MPCCrypto::threshold_sign(&message_hash, &decrypted_shares, 2) {
        Ok(sig) => sig,
        Err(e) => {
            log::error!("Failed to create threshold signature for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to create signature"
            })));
        }
    };
    
    let signature_str = bs58::encode(signature.as_ref()).into_string();
    
    log::info!("Successfully created signature for user {}", req.user_id);
    
    let response = AggregateResponse {
        signature: signature_str,
        success: true,
    };
    
    Ok(HttpResponse::Ok().json(response))
}

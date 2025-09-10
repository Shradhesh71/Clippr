use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::{GenerateRequest, GenerateResponse, KeyShare},
    database::DatabaseManager,
};

pub async fn generate(
    db: web::Data<DatabaseManager>,
    req: web::Json<GenerateRequest>,
) -> Result<HttpResponse> {
    println!("Generating threshold keypair for user: {}", req.user_id);
    
    // Check if user already has shares
    match db.user_has_shares(&req.user_id).await {
        Ok(true) => {
            println!("User {} already has key shares", req.user_id);
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "User already has key shares generated"
            })));
        }
        Ok(false) => {} // Continue with generation
        Err(e) => {
            println!("Database error checking user shares: {}", e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            })));
        }
    }
    
    // Generate threshold keypair using MPC crypto (placeholder)
    // TODO: Re-enable once crypto module is fixed
    // let (public_key, shares) = match MPCCrypto::generate_threshold_keypair(2, 3) {
    //     Ok((pk, shares)) => (pk, shares),
    //     Err(e) => {
    //         return Ok(HttpResponse::InternalServerError().json(json!({
    //             "error": format!("Failed to generate keypair: {}", e)
    //         })));
    //     }
    // };

    // Placeholder implementation for now
    let public_key = format!("pk_placeholder_{}", req.user_id);
    let shares = vec![
        crate::models::KeyShare {
            id: Uuid::new_v4(),
            user_id: req.user_id.clone(),
            public_key: public_key.clone(),
            encrypted_share: "encrypted_share_1".to_string(),
            share_index: 1,
            threshold: 2,
            total_shares: 3,
            created_at: chrono::Utc::now(),
        },
        crate::models::KeyShare {
            id: Uuid::new_v4(),
            user_id: req.user_id.clone(),
            public_key: public_key.clone(),
            encrypted_share: "encrypted_share_2".to_string(),
            share_index: 2,
            threshold: 2,
            total_shares: 3,
            created_at: chrono::Utc::now(),
        },
        crate::models::KeyShare {
            id: Uuid::new_v4(),
            user_id: req.user_id.clone(),
            public_key: public_key.clone(),
            encrypted_share: "encrypted_share_3".to_string(),
            share_index: 3,
            threshold: 2,
            total_shares: 3,
            created_at: chrono::Utc::now(),
        },
    ];

    let public_key_str = public_key.clone();
    println!("Generated public key: {} for user: {}", public_key_str, req.user_id);

    // Store shares in different databases
    let mut storage_success = true;
    
    for (i, key_share) in shares.iter().enumerate() {        
        // Store in the corresponding database (share_index 1->db0, 2->db1, 3->db2)
        let db_index = (key_share.share_index - 1) as usize;
        
        if let Err(e) = db.store_key_share(&key_share, db_index).await {
            println!("Failed to store share {} for user {}: {}", 
                       key_share.share_index, req.user_id, e);
            storage_success = false;
            break;
        }
        
        println!("Stored share {} for user {} in database {}", 
                  key_share.share_index, req.user_id, db_index + 1);
    }
    
    if !storage_success {
        // Cleanup - delete any stored shares
        if let Err(e) = db.delete_user_shares(&req.user_id).await {
            println!("Failed to cleanup shares for user {}: {}", req.user_id, e);
        }
        
        return Ok(HttpResponse::InternalServerError().json(json!({
            "error": "Failed to store key shares"
        })));
    }
    
    let response = GenerateResponse {
        user_id: req.user_id.clone(),
        public_key: public_key_str,
        shares_created: true,
    };

    println!("Successfully generated and stored key shares for user: {}", req.user_id);
    Ok(HttpResponse::Ok().json(response))
}
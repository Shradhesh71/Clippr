
use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::{GenerateRequest, GenerateResponse, KeyShare},
    database::DatabaseManager,
    crypto::MPCCrypto,
};

pub async fn generate(
    db: web::Data<DatabaseManager>,
    req: web::Json<GenerateRequest>,
) -> Result<HttpResponse> {
    log::info!("Generating threshold keypair for user: {}", req.user_id);
    
    // Check if user already has shares
    match db.user_has_shares(&req.user_id).await {
        Ok(true) => {
            log::warn!("User {} already has key shares", req.user_id);
            return Ok(HttpResponse::BadRequest().json(json!({
                "error": "User already has key shares generated"
            })));
        }
        Ok(false) => {} // Continue with generation
        Err(e) => {
            log::error!("Database error checking user shares: {}", e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            })));
        }
    }
    
    // Generate threshold keypair (2-of-3 threshold)
    let (public_key, shares) = match MPCCrypto::generate_threshold_keypair(2, 3) {
        Ok(result) => result,
        Err(e) => {
            log::error!("Failed to generate threshold keypair: {}", e);
            return Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Failed to generate keypair"
            })));
        }
    };
    
    let public_key_str = public_key.to_string();
    log::info!("Generated public key: {} for user: {}", public_key_str, req.user_id);
    
    // Store shares in different databases
    let mut storage_success = true;
    
    for (share_index, encrypted_share) in shares {
        let key_share = KeyShare {
            id: Uuid::new_v4(),
            user_id: req.user_id.clone(),
            public_key: public_key_str.clone(),
            encrypted_share: hex::encode(&encrypted_share),
            share_index: share_index as i32,
            threshold: 2,
            total_shares: 3,
            created_at: chrono::Utc::now(),
        };
        
        // Store in the corresponding database (share_index 1->db0, 2->db1, 3->db2)
        let db_index = (share_index - 1) as usize;
        
        if let Err(e) = db.store_key_share(&key_share, db_index).await {
            log::error!("Failed to store share {} for user {}: {}", 
                       share_index, req.user_id, e);
            storage_success = false;
            break;
        }
        
        log::info!("Stored share {} for user {} in database {}", 
                  share_index, req.user_id, db_index + 1);
    }
    
    if !storage_success {
        // Cleanup - delete any stored shares
        if let Err(e) = db.delete_user_shares(&req.user_id).await {
            log::error!("Failed to cleanup shares for user {}: {}", req.user_id, e);
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
    
    log::info!("Successfully generated and stored key shares for user: {}", req.user_id);
    Ok(HttpResponse::Ok().json(response))
}
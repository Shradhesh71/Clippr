use actix_web::{web, HttpResponse, Result};
use serde_json::json;

use crate::{
    models::{KeyShareRequest, KeyShareResponse},
    database::DatabaseManager,
};

pub async fn send_single(
    db: web::Data<DatabaseManager>,
    req: web::Json<KeyShareRequest>,
) -> Result<HttpResponse> {
    log::info!("Checking single key share for user: {}, share_index: {}", 
              req.user_id, req.share_index);
    
    if req.share_index < 1 || req.share_index > 3 {
        return Ok(HttpResponse::BadRequest().json(json!({
            "error": "Invalid share index. Must be 1, 2, or 3"
        })));
    }
    
    let db_index = (req.share_index - 1) as usize;
    
    match db.get_key_share(&req.user_id, db_index).await {
        Ok(Some(share)) => {
            log::info!("Found share {} for user {}", req.share_index, req.user_id);
            let response = KeyShareResponse {
                share_exists: true,
                public_key: Some(share.public_key),
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => {
            log::info!("No share {} found for user {}", req.share_index, req.user_id);
            let response = KeyShareResponse {
                share_exists: false,
                public_key: None,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            log::error!("Database error checking share for user {}: {}", req.user_id, e);
            Ok(HttpResponse::InternalServerError().json(json!({
                "error": "Database error"
            })))
        }
    }
}
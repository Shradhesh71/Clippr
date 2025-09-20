use crate::models::{AddPublicKeyRequest, RemovePublicKeyRequest, PublicKeyResponse};
use crate::registry::{PublicKeyRegistry, PublicKeyRegistryStats};
use crate::subscriber::{YellowstoneSubscriber, YellowstoneStats};
use crate::database::Database;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub database: DatabaseHealth,
    pub registry: PublicKeyRegistryStats,
    pub subscriber: YellowstoneStats,
}

#[derive(Serialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub pool_size: u32,
}

// Error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ErrorResponse {
    pub fn new(error: &str, message: &str) -> Self {
        Self {
            error: error.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

// Success response
#[derive(Serialize)]
pub struct SuccessResponse<T> {
    pub success: bool,
    pub data: T,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

// Health check endpoint
pub async fn health_check(
    db: web::Data<Database>,
    registry: web::Data<Arc<PublicKeyRegistry>>,
    subscriber: web::Data<Arc<YellowstoneSubscriber>>,
) -> ActixResult<HttpResponse> {
    info!("Health check requested");

    // Check database health
    let db_health = match db.health_check().await {
        Ok(_) => DatabaseHealth {
            connected: true,
            pool_size: 10, // This should come from actual pool configuration
        },
        Err(e) => {
            error!("Database health check failed: {}", e);
            DatabaseHealth {
                connected: false,
                pool_size: 0,
            }
        }
    };

    // Get registry stats
    let registry_stats = match registry.get_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            error!("Failed to get registry stats: {}", e);
            PublicKeyRegistryStats {
                total_keys: 0,
                active_keys: 0,
                inactive_keys: 0,
                unique_users: 0,
            }
        }
    };

    // Get subscriber stats
    let subscriber_stats = subscriber.get_stats().await;

    let health = HealthResponse {
        status: if db_health.connected { "healthy" } else { "unhealthy" }.to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_health,
        registry: registry_stats,
        subscriber: subscriber_stats,
    };

    Ok(HttpResponse::Ok().json(health))
}

// Add public key endpoint
pub async fn add_public_key(
    registry: web::Data<Arc<PublicKeyRegistry>>,
    request: web::Json<AddPublicKeyRequest>,
) -> ActixResult<HttpResponse> {
    info!("Adding public key {} for user {}", request.public_key, request.user_id);

    match registry.add_public_key(request.into_inner()).await {
        Ok(subscribed_key) => {
            let response = PublicKeyResponse::from(subscribed_key);
            Ok(HttpResponse::Created().json(SuccessResponse::new(response)))
        }
        Err(e) => {
            error!("Failed to add public key: {}", e);
            Ok(HttpResponse::BadRequest().json(ErrorResponse::new(
                "AddPublicKeyError",
                &format!("Failed to add public key: {}", e),
            )))
        }
    }
}

// Remove public key endpoint
pub async fn remove_public_key(
    registry: web::Data<Arc<PublicKeyRegistry>>,
    request: web::Json<RemovePublicKeyRequest>,
) -> ActixResult<HttpResponse> {
    info!("Removing public key {} for user {}", request.public_key, request.user_id);

    match registry.remove_public_key(request.into_inner()).await {
        Ok(removed) => {
            if removed {
                Ok(HttpResponse::Ok().json(SuccessResponse::new(
                    serde_json::json!({ "removed": true })
                )))
            } else {
                Ok(HttpResponse::NotFound().json(ErrorResponse::new(
                    "PublicKeyNotFound",
                    "Public key not found or already inactive",
                )))
            }
        }
        Err(e) => {
            error!("Failed to remove public key: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "RemovePublicKeyError",
                &format!("Failed to remove public key: {}", e),
            )))
        }
    }
}

// Get user's public keys endpoint
pub async fn get_user_keys(
    registry: web::Data<Arc<PublicKeyRegistry>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let user_id = path.into_inner();
    info!("Getting public keys for user {}", user_id);

    match registry.get_user_keys(&user_id).await {
        Ok(keys) => {
            let responses: Vec<PublicKeyResponse> = keys
                .into_iter()
                .map(PublicKeyResponse::from)
                .collect();
            Ok(HttpResponse::Ok().json(SuccessResponse::new(responses)))
        }
        Err(e) => {
            error!("Failed to get user keys: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "GetUserKeysError",
                &format!("Failed to get user keys: {}", e),
            )))
        }
    }
}

// Get public key details endpoint
pub async fn get_public_key_details(
    registry: web::Data<Arc<PublicKeyRegistry>>,
    path: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let public_key = path.into_inner();
    info!("Getting details for public key {}", public_key);

    match registry.get_key_subscription(&public_key).await {
        Ok(Some(key)) => {
            let response = PublicKeyResponse::from(key);
            Ok(HttpResponse::Ok().json(SuccessResponse::new(response)))
        }
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(ErrorResponse::new(
                "PublicKeyNotFound",
                "Public key not found or not active",
            )))
        }
        Err(e) => {
            error!("Failed to get public key details: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "GetPublicKeyError",
                &format!("Failed to get public key details: {}", e),
            )))
        }
    }
}

// Get registry statistics endpoint
pub async fn get_registry_stats(
    registry: web::Data<Arc<PublicKeyRegistry>>,
) -> ActixResult<HttpResponse> {
    info!("Getting registry statistics");

    match registry.get_stats().await {
        Ok(stats) => {
            Ok(HttpResponse::Ok().json(SuccessResponse::new(stats)))
        }
        Err(e) => {
            error!("Failed to get registry stats: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "GetStatsError",
                &format!("Failed to get registry stats: {}", e),
            )))
        }
    }
}

// Refresh cache endpoint
pub async fn refresh_cache(
    registry: web::Data<Arc<PublicKeyRegistry>>,
) -> ActixResult<HttpResponse> {
    info!("Refreshing registry cache");

    match registry.refresh_cache().await {
        Ok(_) => {
            Ok(HttpResponse::Ok().json(SuccessResponse::new(
                serde_json::json!({ "refreshed": true })
            )))
        }
        Err(e) => {
            error!("Failed to refresh cache: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "RefreshCacheError",
                &format!("Failed to refresh cache: {}", e),
            )))
        }
    }
}

// Bulk operations request
#[derive(Deserialize)]
pub struct BulkAddKeysRequest {
    pub keys: Vec<AddPublicKeyRequest>,
}

// Bulk add public keys endpoint
pub async fn bulk_add_keys(
    registry: web::Data<Arc<PublicKeyRegistry>>,
    request: web::Json<BulkAddKeysRequest>,
) -> ActixResult<HttpResponse> {
    info!("Bulk adding {} public keys", request.keys.len());

    match registry.bulk_add_keys(request.keys.clone()).await {
        Ok(result) => {
            Ok(HttpResponse::Ok().json(SuccessResponse::new(result)))
        }
        Err(e) => {
            error!("Failed to bulk add keys: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse::new(
                "BulkAddKeysError",
                &format!("Failed to bulk add keys: {}", e),
            )))
        }
    }
}

// Configure routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health_check))
            .route("/keys", web::post().to(add_public_key))
            .route("/keys", web::delete().to(remove_public_key))
            .route("/keys/bulk", web::post().to(bulk_add_keys))
            .route("/users/{user_id}/keys", web::get().to(get_user_keys))
            .route("/keys/{public_key}", web::get().to(get_public_key_details))
            .route("/stats", web::get().to(get_registry_stats))
            .route("/cache/refresh", web::post().to(refresh_cache))
    );
}
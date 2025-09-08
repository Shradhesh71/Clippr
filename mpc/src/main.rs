use actix_web::{web, App, HttpResponse, HttpServer, middleware::Logger};

mod error;
pub mod serialization;
// pub mod tss; // Commented out until we fix the API issues
mod models;
mod database;
mod crypto;

mod routes;
use routes::*;

use database::DatabaseManager;

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    // Initialize logger
    env_logger::init();
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    log::info!("🚀 MPC Server starting on http://127.0.0.1:8081");
    
    // Initialize database connections
    let db_manager = match DatabaseManager::new().await {
        Ok(db) => {
            log::info!("✅ Successfully connected to all MPC databases");
            db
        }
        Err(e) => {
            log::error!("❌ Failed to connect to databases: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Database connection failed: {}", e),
            ));
        }
    };
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_manager.clone()))
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .route("/generate", web::post().to(generate))
                    .route("/send-single", web::post().to(send_single))
                    .route("/aggregate-keys", web::post().to(aggregate_keys))
                    .route("/agg-send-step1", web::post().to(agg_send_step1))
                    .route("/agg-send-step2", web::post().to(agg_send_step2))
                    .route("/aggregate-signatures-broadcast", web::post().to(aggregate_signatures_broadcast))
                    .route("/health", web::get().to(health_check))
            )
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}

async fn index() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "service": "MPC Server",
        "version": "1.0.0",
        "status": "running",
        "endpoints": [
            "POST /api/generate - Generate threshold keypair",
            "POST /api/send-single - Check single key share",
            "POST /api/aggregate-keys - Create threshold signature",
            "POST /api/agg-send-step1 - MPC Step 1",
            "POST /api/agg-send-step2 - MPC Step 2", 
            "POST /api/aggregate-signatures-broadcast - Aggregate signatures",
            "GET /api/health - Health check"
        ]
    }))
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now()
    }))
}

async fn agg_send_step1() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "MPC Step 1 - Placeholder implementation",
        "status": "not_implemented"
    }))
}

async fn agg_send_step2() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "MPC Step 2 - Placeholder implementation", 
        "status": "not_implemented"
    }))
}

async fn aggregate_signatures_broadcast() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Aggregate signatures broadcast - Placeholder implementation",
        "status": "not_implemented"
    }))
}
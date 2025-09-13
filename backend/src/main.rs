use actix_web::{web, App, HttpResponse, HttpServer, middleware::Logger};
use std::sync::Arc;
use tokio::sync::Mutex;

mod routes;
use routes::*;
use store::Store;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv::dotenv().ok();
	println!("🚀 Backend Server starting on http://127.0.0.1:8080");

	// Connect to database
	let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
	let store = match Store::connect(&database_url).await {
		Ok(s) => {
			println!("✅ Connected to database");
			Arc::new(Mutex::new(s))
		}
		Err(e) => {
			println!("❌ Failed to connect to database: {}", e);
			return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Database connection failed: {}", e)));
		}
	};

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(store.clone()))
			.wrap(Logger::default())
			.service(
				web::scope("/api")
					// User routes
					.service(sign_up)
					.service(sign_in)
					.service(get_user)
					// Solana routes
					.service(sol_balance)
					.service(token_balance)
					.service(send_sol)
					.service(add_sol_balance)
					// Jupiter routes
					.service(quote)
					.service(swap)
					// Asset routes
					.service(create_asset)
					.service(list_assets)
					.service(get_asset)
					.service(update_asset)
					.service(delete_asset)
					// Balance routes
					.service(create_balance)
					.service(get_user_balances)
					.service(get_balance)
					.service(update_balance)
					.service(transfer_balance)
					// Health check
					.route("/health", web::get().to(health_check))
			)
			.route("/", web::get().to(index))
	})
	.bind("127.0.0.1:8080")?
	.run()
	.await
}

async fn index() -> HttpResponse {
	HttpResponse::Ok().json(serde_json::json!({
		"service": "Clippr Backend Server",
		"version": "1.0.0",
		"status": "running",
		"endpoints": [
			"POST /api/signup - User signup",
			"POST /api/signin - User signin",
			"GET /api/user/{id} - Get user info",
			"GET /api/sol-balance/{pubkey} - Get SOL balance",
			"GET /api/token-balance/{pubkey}/{mint} - Get token balance",
			"POST /api/send-sol - Send SOL transaction",
			"POST /api/add-sol-balance - Add SOL balance",
			"POST /api/quote - Get Jupiter quote",
			"POST /api/swap - Jupiter swap",
			"POST /api/assets - Create asset",
			"GET /api/assets - List assets",
			"GET /api/assets/{asset_id} - Get asset",
			"PUT /api/assets/{asset_id} - Update asset",
			"DELETE /api/assets/{asset_id} - Delete asset",
			"POST /api/balances - Create balance",
			"GET /api/users/{user_id}/balances - Get user balances",
			"GET /api/users/{user_id}/balances/{asset_id} - Get balance",
			"PUT /api/users/{user_id}/balances/{asset_id} - Update balance",
			"POST /api/balances/transfer - Transfer balance",
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

use std::sync::Arc;
use actix_web::{web, App, HttpServer};
use store::Store;
use tokio::sync::Mutex;

mod routes;
use routes::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment");

    let store = Store::connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Wrap in Arc<Mutex<>> if you want to use this pattern
    let store = Arc::new(Mutex::new(store));

    println!("🚀 Server starting on http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(store.clone()))
            .service(sign_up)  
            .service(sign_in)
            .service(get_user)
            .service(quote)
            .service(swap)
            .service(sol_balance)
            .service(token_balance)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

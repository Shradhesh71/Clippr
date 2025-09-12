use std::sync::Arc;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use store::Store;
use tokio::sync::Mutex;

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    message: String,
}

#[derive(Serialize)]
pub struct TokenValidationResponse {
    valid: bool,
    user_id: Option<String>,
}

// #[actix_web::post("/validate-token")]
// pub async fn validate_token(
//     req: web::Json<serde_json::Value>,
//     store: web::Data<Store>,
// ) -> Result<HttpResponse> {
//     if let Some(token) = req.get("token").and_then(|t| t.as_str()) {
//         match store.validate_token(token) {
//             Ok(user_id) => {
//                 let response = TokenValidationResponse {
//                     valid: true,
//                     user_id: Some(user_id),
//                 };
//                 Ok(HttpResponse::Ok().json(response))
//             }
//             Err(_) => {
//                 let response = TokenValidationResponse {
//                     valid: false,
//                     user_id: None,
//                 };
//                 Ok(HttpResponse::Ok().json(response))
//             }
//         }
//     } else {
//         Ok(HttpResponse::BadRequest().json(serde_json::json!({
//             "error": "Token is required"
//         })))
//     }
// }

#[actix_web::post("/signup")]
pub async fn sign_up(
    req: web::Json<SignUpRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let user_request = store::user::CreateUserRequest {
        email: req.email.clone(),
        password: req.password.clone(),
    };

    let store_guard = store.lock().await;
    match store_guard.create_user(user_request).await {
        Ok(_user) => {
            let response = SignupResponse {
                message: "User created successfully".to_string(),
            };
            Ok(HttpResponse::Created().json(response))
        }
        Err(e) => {
            eprintln!("Error creating user: {}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

#[actix_web::post("/signin")]
pub async fn sign_in(
    req: web::Json<SignInRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let store_guard = store.lock().await;
    match store_guard.authenticate_user(&req.email, &req.password).await {
        Ok(token) => {
            let response = AuthResponse { token };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            eprintln!("Authentication failed: {}", e);
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Invalid credentials"
            })))
        }
    }
}

#[actix_web::get("/user/{id}")]
pub async fn get_user(
    path: web::Path<String>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    
    let store_guard = store.lock().await;
    match store_guard.get_user_by_id(&user_id).await {
        Ok(user) => {
            Ok(HttpResponse::Ok().json(user))
        }
        Err(e) => {
            eprintln!("Error fetching user: {}", e);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "User not found"
            })))
        }
    }
}

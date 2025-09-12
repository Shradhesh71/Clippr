use std::sync::Arc;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use store::Store;
use tokio::sync::Mutex;
use rust_decimal::Decimal;

#[derive(Deserialize)]
pub struct CreateBalanceRequest {
    pub user_id: String,
    pub asset_id: String,
    pub amount: Decimal,
}

#[derive(Deserialize)]
pub struct UpdateBalanceRequest {
    pub amount: Decimal,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub from_user_id: String,
    pub to_user_id: String,
    pub asset_id: String,
    pub amount: Decimal,
}

#[derive(Serialize)]
pub struct BalanceResponse {
    pub id: String,
    pub amount: Decimal,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_id: String,
    pub asset_id: String,
}

#[derive(Serialize)]
pub struct BalanceWithDetailsResponse {
    pub id: String,
    pub amount: Decimal,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub user_id: String,
    pub asset_id: String,
    pub asset_mint_address: String,
    pub asset_name: String,
    pub asset_symbol: String,
    pub asset_decimals: i32,
    pub asset_logo_url: Option<String>,
}

#[actix_web::post("/balances")]
pub async fn create_balance(
    req: web::Json<CreateBalanceRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let store_guard = store.lock().await;
    
    let create_request = store::balance::CreateBalanceRequest {
        user_id: req.user_id.clone(),
        asset_id: req.asset_id.clone(),
        amount: req.amount,
    };

    match store_guard.create_or_update_balance(create_request).await {
        Ok(balance) => {
            let response = BalanceResponse {
                id: balance.id,
                amount: balance.amount,
                created_at: balance.created_at,
                updated_at: balance.updated_at,
                user_id: balance.user_id,
                asset_id: balance.asset_id,
            };
            Ok(HttpResponse::Created().json(response))
        }
        Err(e) => {
            println!("Failed to create balance: {:?}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

#[actix_web::get("/users/{user_id}/balances")]
pub async fn get_user_balances(
    path: web::Path<String>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let store_guard = store.lock().await;

    match store_guard.get_user_balances(&user_id).await {
        Ok(balances) => {
            let response: Vec<BalanceWithDetailsResponse> = balances.into_iter().map(|balance| BalanceWithDetailsResponse {
                id: balance.id,
                amount: balance.amount,
                created_at: balance.created_at,
                updated_at: balance.updated_at,
                user_id: balance.user_id,
                asset_id: balance.asset_id,
                asset_mint_address: balance.asset_mint_address,
                asset_name: balance.asset_name,
                asset_symbol: balance.asset_symbol,
                asset_decimals: balance.asset_decimals,
                asset_logo_url: balance.asset_logo_url,
            }).collect();
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("Failed to get user balances: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve balances"
            })))
        }
    }
}

#[actix_web::get("/users/{user_id}/balances/{asset_id}")]
pub async fn get_balance(
    path: web::Path<(String, String)>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let (user_id, asset_id) = path.into_inner();
    let store_guard = store.lock().await;

    match store_guard.get_balance(&user_id, &asset_id).await {
        Ok(Some(balance)) => {
            let response = BalanceResponse {
                id: balance.id,
                amount: balance.amount,
                created_at: balance.created_at,
                updated_at: balance.updated_at,
                user_id: balance.user_id,
                asset_id: balance.asset_id,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Balance not found"
            })))
        }
        Err(e) => {
            println!("Failed to get balance: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve balance"
            })))
        }
    }
}

#[actix_web::put("/users/{user_id}/balances/{asset_id}")]
pub async fn update_balance(
    path: web::Path<(String, String)>,
    req: web::Json<UpdateBalanceRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let (user_id, asset_id) = path.into_inner();
    let store_guard = store.lock().await;

    let update_request = store::balance::UpdateBalanceRequest {
        user_id,
        asset_id,
        amount: req.amount,
    };

    match store_guard.update_balance(update_request).await {
        Ok(balance) => {
            let response = BalanceResponse {
                id: balance.id,
                amount: balance.amount,
                created_at: balance.created_at,
                updated_at: balance.updated_at,
                user_id: balance.user_id,
                asset_id: balance.asset_id,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("Failed to update balance: {:?}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

#[actix_web::post("/balances/transfer")]
pub async fn transfer_balance(
    req: web::Json<TransferRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let store_guard = store.lock().await;

    let transfer_request = store::balance::TransferRequest {
        from_user_id: req.from_user_id.clone(),
        to_user_id: req.to_user_id.clone(),
        asset_id: req.asset_id.clone(),
        amount: req.amount,
    };

    match store_guard.transfer_balance(transfer_request).await {
        Ok((sender_balance, receiver_balance)) => {
            let response = serde_json::json!({
                "sender_balance": {
                    "id": sender_balance.id,
                    "amount": sender_balance.amount,
                    "updated_at": sender_balance.updated_at,
                    "user_id": sender_balance.user_id,
                    "asset_id": sender_balance.asset_id,
                },
                "receiver_balance": {
                    "id": receiver_balance.id,
                    "amount": receiver_balance.amount,
                    "updated_at": receiver_balance.updated_at,
                    "user_id": receiver_balance.user_id,
                    "asset_id": receiver_balance.asset_id,
                }
            });
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("Failed to transfer balance: {:?}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}
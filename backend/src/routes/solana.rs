use std::sync::Arc;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use store::Store;
use tokio::sync::Mutex;
use rust_decimal::Decimal;

#[derive(Serialize)]
pub struct BalanceResponse {
}

#[derive(Serialize)]
pub struct TokenBalanceResponse {
}

#[derive(Deserialize)]
pub struct SendSolRequest {
    pub user_id: String,
    pub to: String,
    pub lamports: u64,
}

#[derive(Deserialize)]
pub struct AddBalanceRequest {
    pub user_id: String,
    pub lamports: u64,
}

#[derive(Serialize)]
pub struct SendSolResponse {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
}

#[actix_web::get("/sol-balance/{pubkey}")]
pub async fn sol_balance() -> Result<HttpResponse> {
    
    let response = BalanceResponse {
    };
    
    Ok(HttpResponse::Ok().json(response))
}

#[actix_web::get("/token-balance/{pubkey}/{mint}")]
pub async fn token_balance() -> Result<HttpResponse> {    
    
    let response = TokenBalanceResponse {
        
    };
    
    Ok(HttpResponse::Ok().json(response))
}

#[actix_web::post("/send-sol")]
pub async fn send_sol(
    req: web::Json<SendSolRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    println!("Processing SOL transfer request for user: {}", req.user_id);
    
    // SOL asset ID (native Solana)
    const SOL_ASSET_ID: &str = "sol-native";
    
    // Convert lamports to SOL (1 SOL = 1_000_000_000 lamports)
    let sol_amount = Decimal::from(req.lamports) / Decimal::from(1_000_000_000u64);
    
    // Check user's SOL balance and decrease it
    let store_guard = store.lock().await;
    
    // Get current balance
    let current_balance = match store_guard.get_balance(&req.user_id, SOL_ASSET_ID).await {
        Ok(Some(balance)) => balance,
        Ok(None) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "User has no SOL balance",
                "transaction_signature": null,
                "from_address": "unknown",
                "to_address": req.to,
                "amount_lamports": req.lamports
            })));
        }
        Err(e) => {
            println!("Failed to get user balance: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to check balance",
                "transaction_signature": null,
                "from_address": "unknown",
                "to_address": req.to,
                "amount_lamports": req.lamports
            })));
        }
    };
    
    // Check if user has sufficient balance
    if current_balance.amount < sol_amount {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": format!("Insufficient balance. Required: {} SOL, Available: {} SOL", 
                           sol_amount, current_balance.amount),
            "transaction_signature": null,
            "from_address": "unknown",
            "to_address": req.to,
            "amount_lamports": req.lamports
        })));
    }
    
    // Decrease the balance first (optimistic approach)
    let new_balance = current_balance.amount - sol_amount;
    let update_request = store::balance::UpdateBalanceRequest {
        user_id: req.user_id.clone(),
        asset_id: SOL_ASSET_ID.to_string(),
        amount: new_balance,
    };
    
    let updated_balance = match store_guard.update_balance(update_request).await {
        Ok(balance) => balance,
        Err(e) => {
            println!("Failed to update balance: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to update balance",
                "transaction_signature": null,
                "from_address": "unknown",
                "to_address": req.to,
                "amount_lamports": req.lamports
            })));
        }
    };
    
    println!("Updated user {} balance from {} to {} SOL", 
             req.user_id, current_balance.amount, updated_balance.amount);
    
    // Release the store lock before making external call
    drop(store_guard);
    
    // Forward the request to MPC service for secure key aggregation and transaction signing
    let mpc_service_url = std::env::var("MPC_SIMPLE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
    
    let client = reqwest::Client::new();
    
    // Prepare the request for MPC service
    let mpc_request = serde_json::json!({
        "user_id": req.user_id,
        "to_address": req.to,
        "amount_lamports": req.lamports
    });
    
    // Send request to MPC service
    let mpc_response = match client
        .post(format!("{}/api/send-sol", mpc_service_url))
        .json(&mpc_request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            println!("Failed to connect to MPC service: {}", e);
            
            // Rollback balance change
            let store_guard = store.lock().await;
            let rollback_request = store::balance::UpdateBalanceRequest {
                user_id: req.user_id.clone(),
                asset_id: SOL_ASSET_ID.to_string(),
                amount: current_balance.amount, // Restore original balance
            };
            
            if let Err(rollback_err) = store_guard.update_balance(rollback_request).await {
                println!("CRITICAL: Failed to rollback balance for user {}: {}", req.user_id, rollback_err);
            } else {
                println!("Rolled back balance for user {} due to MPC service failure", req.user_id);
            }
            
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to connect to MPC service",
                "transaction_signature": null,
                "from_address": "unknown",
                "to_address": req.to,
                "amount_lamports": req.lamports
            })));
        }
    };
    
    // Check if MPC service request was successful
    if !mpc_response.status().is_success() {
        let error_text = mpc_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        println!("MPC service returned error: {}", error_text);
        
        // Rollback balance change
        let store_guard = store.lock().await;
        let rollback_request = store::balance::UpdateBalanceRequest {
            user_id: req.user_id.clone(),
            asset_id: SOL_ASSET_ID.to_string(),
            amount: current_balance.amount, // Restore original balance
        };
        
        if let Err(rollback_err) = store_guard.update_balance(rollback_request).await {
            println!("CRITICAL: Failed to rollback balance for user {}: {}", req.user_id, rollback_err);
        } else {
            println!("Rolled back balance for user {} due to MPC service error", req.user_id);
        }
        
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("MPC service error: {}", error_text),
            "transaction_signature": null,
            "from_address": "unknown", 
            "to_address": req.to,
            "amount_lamports": req.lamports
        })));
    }
    
    // Parse and forward the MPC service response
    let mpc_result: serde_json::Value = match mpc_response.json().await {
        Ok(result) => result,
        Err(e) => {
            println!("Failed to parse MPC service response: {}", e);
            
            // Rollback balance change
            let store_guard = store.lock().await;
            let rollback_request = store::balance::UpdateBalanceRequest {
                user_id: req.user_id.clone(),
                asset_id: SOL_ASSET_ID.to_string(),
                amount: current_balance.amount, // Restore original balance
            };
            
            if let Err(rollback_err) = store_guard.update_balance(rollback_request).await {
                println!("CRITICAL: Failed to rollback balance for user {}: {}", req.user_id, rollback_err);
            } else {
                println!("Rolled back balance for user {} due to response parsing failure", req.user_id);
            }
            
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": "Failed to parse MPC service response",
                "transaction_signature": null,
                "from_address": "unknown",
                "to_address": req.to,
                "amount_lamports": req.lamports
            })));
        }
    };
    
    // Check if the actual transaction was successful
    let transaction_success = mpc_result
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    if !transaction_success {
        // Transaction failed, rollback the balance
        let store_guard = store.lock().await;
        let rollback_request = store::balance::UpdateBalanceRequest {
            user_id: req.user_id.clone(),
            asset_id: SOL_ASSET_ID.to_string(),
            amount: current_balance.amount, // Restore original balance
        };
        
        if let Err(rollback_err) = store_guard.update_balance(rollback_request).await {
            println!("CRITICAL: Failed to rollback balance for user {}: {}", req.user_id, rollback_err);
        } else {
            println!("Rolled back balance for user {} due to transaction failure", req.user_id);
        }
    } else {
        println!("SOL transfer completed successfully for user {}: {} lamports sent", 
                 req.user_id, req.lamports);
        println!("User {} balance updated: {} SOL remaining", req.user_id, new_balance);
    }
    
    Ok(HttpResponse::Ok().json(mpc_result))
}

#[actix_web::post("/add-sol-balance")]
pub async fn add_sol_balance(
    req: web::Json<AddBalanceRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    println!("Adding SOL balance for user: {}", req.user_id);
    
    // SOL asset ID (native Solana)
    const SOL_ASSET_ID: &str = "sol-native";
    
    // Convert lamports to SOL (1 SOL = 1_000_000_000 lamports)
    let sol_amount = Decimal::from(req.lamports) / Decimal::from(1_000_000_000u64);
    
    let store_guard = store.lock().await;
    
    // Create or update balance
    let create_request = store::balance::CreateBalanceRequest {
        user_id: req.user_id.clone(),
        asset_id: SOL_ASSET_ID.to_string(),
        amount: sol_amount,
    };
    
    match store_guard.create_or_update_balance(create_request).await {
        Ok(balance) => {
            println!("Successfully added {} lamports ({} SOL) to user {}", 
                     req.lamports, sol_amount, req.user_id);
            println!("User {} new balance: {} SOL", req.user_id, balance.amount);
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "user_id": req.user_id,
                "added_lamports": req.lamports,
                "added_sol": sol_amount,
                "new_balance_sol": balance.amount,
                "message": format!("Added {} SOL to user balance", sol_amount)
            })))
        }
        Err(e) => {
            println!("Failed to add balance for user {}: {}", req.user_id, e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error": format!("Failed to add balance: {}", e),
                "user_id": req.user_id,
                "requested_lamports": req.lamports
            })))
        }
    }
}
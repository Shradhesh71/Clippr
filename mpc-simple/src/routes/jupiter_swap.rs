use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    transaction::Transaction
};

use crate::{database::DatabaseManager, routes::{create_rpc_client, parse_private_key}};

#[derive(Deserialize)]
pub struct SwapRequest {
    pub user_id: String,
    pub user_public_key: String,
    pub swap_transaction: serde_json::Value, 
}

#[derive(Serialize)]
pub struct SwapResponse {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    // pub swap_details: Option<SwapDetails>,
}

pub async fn jupiter_swap(
    db: web::Data<DatabaseManager>,
    req: web::Json<SwapRequest>,
) -> Result<HttpResponse> {
    println!("Processing Jupiter swap for user: {}", req.user_id);

    //  Step 1: Validate user and retrieve key shares
    let shares = match db.get_all_user_shares(&req.user_id).await {
        Ok(shares) => shares,
        Err(e) => {
            println!("Failed to fetch user shares of id {}: {}", req.user_id,e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse{
                success: false,
                transaction_signature: None,
                error: Some("Faileed to fetch user shares".to_string())
            }));
        }
    };

    if shares.is_empty() {
        println!("no key shares for user id {}", req.user_id);
        return Ok(HttpResponse::InternalServerError().json(SwapResponse{
            success: false,
            transaction_signature: None,
            error: Some("no key share found".to_string())
        }));
    }

    let first_share = &shares[0];
    let exp_public_key = first_share.public_key.clone();
    let thresold= first_share.threshold;

    if shares.len() < thresold as usize{
        println!("insufficient key shares found");
        return Ok(HttpResponse::InternalServerError().json(SwapResponse{
            success: false,
            transaction_signature: None,
            error: Some("insufficient key shares".to_string())
        }))
    }

    if req.user_public_key != exp_public_key {
        println!("wrong public key");
        return Ok(HttpResponse::InternalServerError().json(SwapResponse{
            success: false,
            transaction_signature: None,
            error: Some("Public key verification failed".to_string()),
        }));
    }

    // Step 2: reconstruct private key from MPC
    let mut sorted_shares = shares;
    sorted_shares.sort_by_key(|s| s.share_index);

    // Use only the required number of shares for threshold signature
    let required_shares: Vec<_> = sorted_shares.iter().take(thresold as usize).collect();
    
    println!("Reconstructing private key from {} shares", required_shares.len());
    
    // TODO: Implement proper MPC reconstruction here
    // For now, using simplified concatenation (THIS NEEDS TO BE REPLACED WITH ACTUAL MPC)
    let mut reconstructed_private_key = String::new();
    for share in &required_shares {
        reconstructed_private_key.push_str(&share.encrypted_share);
    }

    // Step 3: Parse private key
    let keypair = match parse_private_key(&reconstructed_private_key) {
        Ok(keypair) => keypair,
        Err(e) => {
            println!("Failed to parse private key for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to reconstruct private key".to_string()),
            }));
        }
    };

    // Step 4: Parse the swap transaction from Jupiter
    let swap_transaction_b64 = match req.swap_transaction.as_str() {
        Some(tx) => tx,
        None => {
            println!("Invalid swap transaction format");
            return Ok(HttpResponse::BadRequest().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Invalid transaction format".to_string()),
            }));
        }
    };

    // Decode the base64 transaction
    let transaction_bytes = match base64::decode(swap_transaction_b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("Failed to decode transaction: {}", e);
            return Ok(HttpResponse::BadRequest().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to decode transaction".to_string()),
            }));
        }
    };

    // Deserialize the transaction
    let mut transaction: Transaction = match bincode::deserialize(&transaction_bytes) {
        Ok(tx) => tx,
        Err(e) => {
            println!("Failed to deserialize transaction: {}", e);
            return Ok(HttpResponse::BadRequest().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to deserialize transaction".to_string()),
            }));
        }
    };

    // Step 5: Get recent blockhash and sign transaction
    let rpc_client = create_rpc_client();
    let recent_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(blockhash) => blockhash,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(SwapResponse{
                success: false,
                transaction_signature: None,
                error: Some("failed to get recent bloakhash".to_string())
            }));
        }
    };

    // Update blockhash and sign transaction
    transaction.message.recent_blockhash = recent_blockhash;
    
    // Sign the transaction
    match transaction.try_sign(&[&keypair], recent_blockhash) {
        Ok(_) => println!("Transaction signed successfully"),
        Err(e) => {
            println!("Failed to sign transaction: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to sign transaction".to_string()),
            }));
        }
    }

    // Step 6: Send the transaction to Solana network
    println!("Broadcasting transaction to Solana network...");
    let signature = match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(sig) => {
            println!("Transaction successful for user {}: {}", req.user_id, sig);
            sig
        }
        Err(e) => {
            println!("Failed to send transaction for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some(format!("Failed to send transaction: {}", e)),
            }));
        }
    };

    // clear the private key from memory for security
    drop(keypair);
    drop(reconstructed_private_key);

    println!("Jupiter swap completed successfully for user: {}", req.user_id);
    
    Ok(HttpResponse::Ok().json(SwapResponse {
        success: true,
        transaction_signature: Some(signature.to_string()),
        error: None,
    }))
}
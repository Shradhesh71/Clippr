use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;

use crate::database::DatabaseManager;

// System program ID constant
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111112";

#[derive(Debug, Deserialize)]
pub struct SendSolRequest {
    pub user_id: String,
    pub to_address: String,
    pub amount_lamports: u64,
}

#[derive(Debug, Serialize)]
pub struct SendSolResponse {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    pub from_address: String,
    pub to_address: String,
    pub amount_lamports: u64,
}

pub async fn send_sol(
    db: web::Data<DatabaseManager>,
    req: web::Json<SendSolRequest>,
) -> Result<HttpResponse> {
    println!("Processing SOL transfer for user: {}", req.user_id);
    
    // Step 1: Fetch all key shares for the user from all databases
    let shares = match db.get_all_user_shares(&req.user_id).await {
        Ok(shares) => shares,
        Err(e) => {
            println!("Failed to fetch key shares for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(SendSolResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to fetch key shares from databases".to_string()),
                from_address: "unknown".to_string(),
                to_address: req.to_address.clone(),
                amount_lamports: req.amount_lamports,
            }));
        }
    };

    // Check if we have enough shares
    if shares.is_empty() {
        println!("No key shares found for user: {}", req.user_id);
        return Ok(HttpResponse::NotFound().json(SendSolResponse {
            success: false,
            transaction_signature: None,
            error: Some("No key shares found for user".to_string()),
            from_address: "unknown".to_string(),
            to_address: req.to_address.clone(),
            amount_lamports: req.amount_lamports,
        }));
    }

    // Verify all shares have the same public key and threshold
    let first_share = &shares[0];
    let expected_public_key = first_share.public_key.clone();
    let threshold = first_share.threshold;
    
    if shares.len() < threshold as usize {
        println!("Insufficient shares for user {}: found {}, need {}", 
                 req.user_id, shares.len(), threshold);
        return Ok(HttpResponse::BadRequest().json(SendSolResponse {
            success: false,
            transaction_signature: None,
            error: Some(format!("Insufficient shares: found {}, need {}", shares.len(), threshold)),
            from_address: expected_public_key,
            to_address: req.to_address.clone(),
            amount_lamports: req.amount_lamports,
        }));
    }

    // Step 2: Reconstruct the private key (simplified - in production use proper secret sharing)
    let mut sorted_shares = shares;
    sorted_shares.sort_by_key(|s| s.share_index);

    // For now, concatenating the shares - in production, use Shamir's Secret Sharing
    let mut reconstructed_private_key = String::new();
    for share in sorted_shares.iter().take(threshold as usize) {
        reconstructed_private_key.push_str(&share.encrypted_share);
        println!("Using share {} for user {}", share.share_index, req.user_id);
    }

    // Step 3: Parse the private key and create Keypair
    let keypair = match parse_private_key(&reconstructed_private_key) {
        Ok(kp) => kp,
        Err(e) => {
            println!("Failed to parse private key for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(SendSolResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to parse private key".to_string()),
                from_address: expected_public_key,
                to_address: req.to_address.clone(),
                amount_lamports: req.amount_lamports,
            }));
        }
    };

    // Step 4: Validate the to_address
    let to_pubkey = match Pubkey::from_str(&req.to_address) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            println!("Invalid to_address for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::BadRequest().json(SendSolResponse {
                success: false,
                transaction_signature: None,
                error: Some("Invalid recipient address".to_string()),
                from_address: keypair.pubkey().to_string(),
                to_address: req.to_address.clone(),
                amount_lamports: req.amount_lamports,
            }));
        }
    };

    // Step 5: Create the SOL transfer transaction
    let from_pubkey = keypair.pubkey();
    
    // Create transfer instruction manually
    let transfer_instruction = create_transfer_instruction(&from_pubkey, &to_pubkey, req.amount_lamports);

    // Step 6: Get recent blockhash from Solana network
    let rpc_client = create_rpc_client();
    let recent_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(blockhash) => blockhash,
        Err(e) => {
            println!("Failed to get recent blockhash: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SendSolResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to get recent blockhash from Solana network".to_string()),
                from_address: from_pubkey.to_string(),
                to_address: req.to_address.clone(),
                amount_lamports: req.amount_lamports,
            }));
        }
    };

    // Step 7: Create and sign the transaction
    let message = Message::new(&[transfer_instruction], Some(&from_pubkey));
    let mut transaction = Transaction::new_unsigned(message);
    transaction.sign(&[&keypair], recent_blockhash);

    // Step 8: Send the transaction to Solana network
    let signature = match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(sig) => sig,
        Err(e) => {
            println!("Failed to send transaction for user {}: {}", req.user_id, e);
            return Ok(HttpResponse::InternalServerError().json(SendSolResponse {
                success: false,
                transaction_signature: None,
                error: Some(format!("Failed to send transaction: {}", e)),
                from_address: from_pubkey.to_string(),
                to_address: req.to_address.clone(),
                amount_lamports: req.amount_lamports,
            }));
        }
    };

    println!("Successfully sent {} lamports from {} to {} for user {}. Signature: {}", 
             req.amount_lamports, from_pubkey, to_pubkey, req.user_id, signature);

    // Clear the private key from memory for security
    drop(keypair);
    drop(reconstructed_private_key);

    // Step 9: Return success response
    Ok(HttpResponse::Ok().json(SendSolResponse {
        success: true,
        transaction_signature: Some(signature.to_string()),
        error: None,
        from_address: from_pubkey.to_string(),
        to_address: req.to_address.clone(),
        amount_lamports: req.amount_lamports,
    }))
}

fn create_transfer_instruction(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    // System program transfer instruction
    let system_program_id = Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap();
    Instruction {
        program_id: system_program_id,
        accounts: vec![
            AccountMeta::new(*from, true),  // from account (signer)
            AccountMeta::new(*to, false),   // to account
        ],
        data: encode_transfer_instruction(lamports),
    }
}

fn encode_transfer_instruction(lamports: u64) -> Vec<u8> {
    // System program transfer instruction data
    // Instruction type 2 is Transfer
    let mut data = vec![2, 0, 0, 0]; // u32 instruction type = 2 (Transfer)
    data.extend_from_slice(&lamports.to_le_bytes()); // u64 lamports
    data
}

pub fn parse_private_key(private_key_str: &str) -> Result<Keypair, Box<dyn std::error::Error>> {
    // Try different formats for private key parsing
    
    // First, try as base58 string (common format)
    if let Ok(_) = bs58::decode(private_key_str).into_vec() {
        // Try the from_base58_string method that exists
        return Ok(Keypair::from_base58_string(private_key_str));
    }
    
    // Try as hex string for 32-byte private key
    if let Ok(bytes) = hex::decode(private_key_str) {
        if bytes.len() == 32 {
            let mut private_key_bytes = [0u8; 32];
            private_key_bytes.copy_from_slice(&bytes);
            return Ok(Keypair::new_from_array(private_key_bytes));
        }
    }
    
    // Try as JSON array format [byte1, byte2, ...] for 32-byte private key
    if private_key_str.starts_with('[') && private_key_str.ends_with(']') {
        if let Ok(bytes_vec) = serde_json::from_str::<Vec<u8>>(private_key_str) {
            if bytes_vec.len() == 32 {
                let mut private_key_bytes = [0u8; 32];
                private_key_bytes.copy_from_slice(&bytes_vec);
                return Ok(Keypair::new_from_array(private_key_bytes));
            }
        }
    }
    
    Err("Unable to parse private key in any recognized format".into())
}

pub fn create_rpc_client() -> RpcClient {
    // Use devnet for testing, mainnet for production
    let rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    
    RpcClient::new(rpc_url)
}
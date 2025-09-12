use chrono::Utc;
// use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{error::UserError};

pub fn generate_token(user_id: &str) -> Result<String, UserError> {
    // Generate a simple token with timestamp (in production, use JWT)
    let timestamp = Utc::now().timestamp();
    let token = format!("token-{}-{}", user_id, timestamp);
    Ok(token)
}

// pub fn generate_keypair() ->  Result<KeypairData, UserError> {
//     let keypair = Keypair::new();
//     let pubkey = keypair.pubkey().to_string();
//     let secret = bs58::encode(keypair.to_bytes()).into_string();

//     Ok(KeypairData {
//         pubkey,
//         secret,
//     })
// }

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct KeyShare {
    pub id: Uuid,
    pub user_id: String,
    pub public_key: String,
    pub encrypted_share: String, // encrypted private key share
    pub share_index: i32, // which share this is (1, 2, or 3)
    pub threshold: i32, // threshold for reconstruction
    pub total_shares: i32, // total number of shares
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub user_id: String,
    pub public_key: String,
    pub shares_created: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateRequest {
    pub user_id: String,
    pub message: String, // message to sign
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateResponse {
    pub signature: String,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyShareRequest {
    pub user_id: String,
    pub share_index: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyShareResponse {
    pub share_exists: bool,
    pub public_key: Option<String>,
}

// For communication between MPC servers
#[derive(Debug, Serialize, Deserialize)]
pub struct MPCMessage {
    pub message_type: String,
    pub user_id: String,
    pub data: serde_json::Value,
    pub from_server: String,
    pub to_server: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThresholdKeyGenRequest {
    pub user_id: String,
    pub threshold: u16,
    pub total_parties: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThresholdSignRequest {
    pub user_id: String,
    pub message_hash: String,
    pub participating_parties: Vec<u16>,
}

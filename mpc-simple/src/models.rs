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

// Session management for MPC protocols
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct MPCSession {
    pub id: Uuid,
    pub session_id: String,
    pub user_id: String,
    pub participants: Vec<String>, // JSON array of participant IDs
    pub current_step: i32, // 1 = commitment, 2 = signature shares, 3 = aggregation
    pub commitments: serde_json::Value, // JSON object of commitments
    pub signature_shares: serde_json::Value, // JSON object of signature shares
    pub final_signature: Option<String>,
    pub message_to_sign: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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

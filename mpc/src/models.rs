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

// MPC Step 1: Commitment Phase
#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStep1Request {
    pub user_id: String,
    pub session_id: String,
    pub participant_id: String,
    pub nonce: String, // Base64 encoded nonce
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStep1Response {
    pub session_id: String,
    pub participant_id: String,
    pub commitment: String, // Base64 encoded commitment
    pub success: bool,
    pub message: String,
}

// MPC Step 2: Signature Share Phase
#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStep2Request {
    pub user_id: String,
    pub session_id: String,
    pub participant_id: String,
    pub message_to_sign: String, // The actual message/transaction to sign
    pub commitments: Vec<CommitmentData>, // Commitments from other participants
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitmentData {
    pub participant_id: String,
    pub commitment: String, // Base64 encoded
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggSendStep2Response {
    pub session_id: String,
    pub participant_id: String,
    pub signature_share: String, // Base64 encoded signature share
    pub success: bool,
    pub message: String,
}

// Aggregate Signatures Broadcast
#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateSignaturesBroadcastRequest {
    pub user_id: String,
    pub session_id: String,
    pub message_to_sign: String,
    pub signature_shares: Vec<SignatureShareData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureShareData {
    pub participant_id: String,
    pub signature_share: String, // Base64 encoded
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AggregateSignaturesBroadcastResponse {
    pub session_id: String,
    pub final_signature: String, // Base64 encoded final aggregated signature
    pub public_key: String, // Public key for verification
    pub success: bool,
    pub message: String,
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

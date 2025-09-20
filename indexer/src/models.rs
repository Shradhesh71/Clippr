use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

/// Represents the current state of the indexer
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IndexerState {
    pub id: String,
    pub subscribed_keys: serde_json::Value, // JSON array of public keys
    pub last_processed_slot: i64,
    pub status: IndexerStatus,
    pub total_subscriptions: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "indexer_status", rename_all = "lowercase")]
pub enum IndexerStatus {
    Starting,
    Running,
    Stopped,
    Error,
}

/// Tracks which public keys are being monitored
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SubscribedKey {
    pub id: String,
    pub user_id: String,
    pub public_key: String,
    pub is_active: bool,
    pub subscription_type: SubscriptionType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "subscription_type", rename_all = "lowercase")]
pub enum SubscriptionType {
    Account,      // Monitor account balance changes
    Transaction,  // Monitor transactions involving this key
    Both,         // Monitor both account and transactions
}

/// Records balance update events from the blockchain
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BalanceUpdate {
    pub id: String,
    pub user_id: String,
    pub public_key: String,
    pub mint_address: String, // Native SOL = "11111111111111111111111111111112"
    pub old_balance: Decimal,
    pub new_balance: Decimal,
    pub change_amount: Decimal,
    pub change_type: BalanceChangeType,
    pub transaction_signature: Option<String>,
    pub slot: i64,
    pub block_time: Option<DateTime<Utc>>,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "balance_change_type", rename_all = "lowercase")]
pub enum BalanceChangeType {
    Increase,
    Decrease,
    SwapIn,
    SwapOut,
    Transfer,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_event_type", rename_all = "lowercase")]
pub enum TransactionEventType {
    Send,
    Receive,
    Swap,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_status", rename_all = "lowercase")]
pub enum TransactionStatus {
    Success,
    Failed,
    Pending,
}

/// Tracks transaction events for user accounts
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TransactionEvent {
    pub id: String,
    pub public_key: String,
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub event_type: TransactionEventType,
    pub amount: Option<i64>,
    pub mint: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub fee: Option<u64>,
    pub status: TransactionStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// Request/Response structures for API endpoints
#[derive(Debug, serde::Deserialize, Clone)]
pub struct AddPublicKeyRequest {
    pub user_id: String,
    pub public_key: String,
    pub subscription_type: SubscriptionType,
}

#[derive(Debug, serde::Deserialize)]
pub struct RemovePublicKeyRequest {
    pub user_id: String,
    pub public_key: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PublicKeyResponse {
    pub id: String,
    pub user_id: String,
    pub public_key: String,
    pub is_active: bool,
    pub subscription_type: SubscriptionType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<SubscribedKey> for PublicKeyResponse {
    fn from(key: SubscribedKey) -> Self {
        Self {
            id: key.id,
            user_id: key.user_id,
            public_key: key.public_key,
            is_active: key.is_active,
            subscription_type: key.subscription_type,
            created_at: key.created_at,
            updated_at: key.updated_at,
        }
    }
}#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "transaction_type", rename_all = "lowercase")]
pub enum TransactionType {
    Transfer,
    Swap,
    Stake,
    Vote,
    CreateAccount,
    CloseAccount,
    Other,
}

/// System statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IndexerStats {
    pub id: String,
    pub total_keys_monitored: i32,
    pub total_balance_updates: i64,
    pub total_transactions: i64,
    pub last_processed_slot: i64,
    pub avg_processing_time_ms: f64,
    pub errors_last_hour: i32,
    pub uptime_seconds: i64,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexerStatusResponse {
    pub status: IndexerStatus,
    pub subscribed_keys_count: i32,
    pub last_processed_slot: i64,
    pub uptime_seconds: i64,
    pub stats: Option<IndexerStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceUpdateResponse {
    pub updates: Vec<BalanceUpdate>,
    pub total_count: i64,
    pub page: i32,
    pub per_page: i32,
}

/// Helper methods
impl IndexerState {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            subscribed_keys: serde_json::json!([]),
            last_processed_slot: 0,
            status: IndexerStatus::Starting,
            total_subscriptions: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

impl SubscribedKey {
    pub fn new(user_id: String, public_key: String, subscription_type: SubscriptionType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            public_key,
            is_active: true,
            subscription_type,
            created_at: now,
            updated_at: now,
        }
    }
}

impl BalanceUpdate {
    pub fn new(
        user_id: String,
        public_key: String,
        mint_address: String,
        old_balance: Decimal,
        new_balance: Decimal,
        change_type: BalanceChangeType,
        transaction_signature: Option<String>,
        slot: i64,
    ) -> Self {
        let change_amount = new_balance - old_balance;
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            public_key,
            mint_address,
            old_balance,
            new_balance,
            change_amount,
            change_type,
            transaction_signature,
            slot,
            block_time: None,
            processed_at: Utc::now(),
        }
    }
}
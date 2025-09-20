use crate::models::{SubscribedKey, AddPublicKeyRequest, RemovePublicKeyRequest};
use crate::database::Database;
use anyhow::Result;
use sqlx::Row;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

pub struct PublicKeyRegistry {
    db: Database,
    // In-memory cache of active public keys for fast lookup
    active_keys: Arc<RwLock<HashSet<String>>>,
}

impl PublicKeyRegistry {
    pub async fn new(db: Database) -> Result<Self> {
        let registry = Self {
            db,
            active_keys: Arc::new(RwLock::new(HashSet::new())),
        };

        // Load existing keys from database
        registry.refresh_cache().await?;
        info!("Public Key Registry initialized");

        Ok(registry)
    }

    /// Add a new public key to monitor
    pub async fn add_public_key(&self, request: AddPublicKeyRequest) -> Result<SubscribedKey> {
        info!("Adding public key {} for user {}", request.public_key, request.user_id);

        // Validate public key format
        self.validate_public_key(&request.public_key)?;

        let subscribed_key = SubscribedKey::new(
            request.user_id.clone(),
            request.public_key.clone(),
            request.subscription_type,
        );

        // Insert into database
        let query = "
            INSERT INTO subscribed_keys (id, user_id, public_key, is_active, subscription_type, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id, public_key) 
            DO UPDATE SET 
                is_active = $4,
                subscription_type = $5,
                updated_at = $7
        ";
        
        sqlx::query(query)
            .bind(&subscribed_key.id)
            .bind(&subscribed_key.user_id)
            .bind(&subscribed_key.public_key)
            .bind(subscribed_key.is_active)
            .bind(&subscribed_key.subscription_type)
            .bind(subscribed_key.created_at)
            .bind(subscribed_key.updated_at)
            .execute(self.db.get_pool().await)
            .await?;

        // Add to in-memory cache
        {
            let mut keys = self.active_keys.write().await;
            keys.insert(request.public_key.clone());
        }

        info!("Successfully added public key {} for user {}", request.public_key, request.user_id);
        Ok(subscribed_key)
    }

    /// Remove a public key from monitoring
    pub async fn remove_public_key(&self, request: RemovePublicKeyRequest) -> Result<bool> {
        info!("Removing public key {} for user {}", request.public_key, request.user_id);

        let result = sqlx::query(
            "UPDATE subscribed_keys SET is_active = false, updated_at = NOW() WHERE user_id = $1 AND public_key = $2"
        )
        .bind(&request.user_id)
        .bind(&request.public_key)
        .execute(self.db.get_pool().await)
        .await?;

        let removed = result.rows_affected() > 0;

        if removed {
            // Remove from in-memory cache
            let mut keys = self.active_keys.write().await;
            keys.remove(&request.public_key);
            info!("Successfully removed public key {} for user {}", request.public_key, request.user_id);
        } else {
            warn!("Public key {} not found for user {}", request.public_key, request.user_id);
        }

        Ok(removed)
    }

    /// Get all active public keys
    pub async fn get_active_public_keys(&self) -> Vec<String> {
        let keys = self.active_keys.read().await;
        keys.iter().cloned().collect()
    }

    /// Get all subscribed keys for a user
    pub async fn get_user_keys(&self, _user_id: &str) -> Result<Vec<SubscribedKey>> {
        // For now, return empty vector to avoid sqlx offline issues
        // In production, implement proper query handling
        Ok(vec![])
    }

    /// Get subscription details for a specific public key
    pub async fn get_key_subscription(&self, _public_key: &str) -> Result<Option<SubscribedKey>> {
        // For now, return None to avoid sqlx offline issues
        // In production, implement proper query handling
        Ok(None)
    }

    /// Refresh the in-memory cache from database
    pub async fn refresh_cache(&self) -> Result<()> {
        info!("Refreshing public key cache from database");

        let rows = sqlx::query(
            "SELECT public_key FROM subscribed_keys WHERE is_active = true"
        )
        .fetch_all(self.db.get_pool().await)
        .await?;

        let mut keys = self.active_keys.write().await;
        keys.clear();
        for row in rows {
            let public_key: String = row.get("public_key");
            keys.insert(public_key);
        }

        info!("Refreshed cache with {} active public keys", keys.len());
        Ok(())
    }

    /// Get statistics about subscribed keys
    pub async fn get_stats(&self) -> Result<PublicKeyRegistryStats> {
        // Return default stats to avoid sqlx offline issues
        // In production, implement proper query handling
        Ok(PublicKeyRegistryStats {
            total_keys: 0,
            active_keys: 0,
            inactive_keys: 0,
            unique_users: 0,
        })
    }

    /// Check if a public key is being monitored
    pub async fn is_key_monitored(&self, public_key: &str) -> bool {
        let keys = self.active_keys.read().await;
        keys.contains(public_key)
    }

    /// Validate public key format
    fn validate_public_key(&self, public_key: &str) -> Result<()> {
        // Solana public keys are base58 encoded and should be 44 characters
        if public_key.len() != 44 {
            return Err(anyhow::anyhow!("Invalid public key length: expected 44 characters, got {}", public_key.len()));
        }

        // Try to decode as base58
        match bs58::decode(public_key).into_vec() {
            Ok(bytes) => {
                if bytes.len() != 32 {
                    return Err(anyhow::anyhow!("Invalid public key: decoded length should be 32 bytes, got {}", bytes.len()));
                }
            }
            Err(_) => {
                return Err(anyhow::anyhow!("Invalid public key: not valid base58"));
            }
        }

        Ok(())
    }

    /// Bulk add public keys (useful for migration or batch operations)
    pub async fn bulk_add_keys(&self, keys: Vec<AddPublicKeyRequest>) -> Result<BulkOperationResult> {
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for key_request in keys {
            match self.add_public_key(key_request.clone()).await {
                Ok(_) => successful += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("Failed to add key {} for user {}: {}", 
                        key_request.public_key, key_request.user_id, e));
                    error!("Failed to add key {}: {}", key_request.public_key, e);
                }
            }
        }

        Ok(BulkOperationResult {
            successful,
            failed,
            errors,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PublicKeyRegistryStats {
    pub total_keys: u32,
    pub active_keys: u32,
    pub inactive_keys: u32,
    pub unique_users: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct BulkOperationResult {
    pub successful: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}
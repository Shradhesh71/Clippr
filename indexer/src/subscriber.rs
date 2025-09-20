use crate::models::{BalanceUpdate, TransactionEvent, BalanceChangeType};
use crate::registry::PublicKeyRegistry;
use crate::database::Database;
use crate::config::Config;
use crate::yellowstone::GeyserGrpcClient;
use anyhow::Result;
use futures::StreamExt;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error, debug};
use yellowstone_grpc_proto::prelude::*;

#[derive(Clone)]
pub struct YellowstoneSubscriber {
    registry: Arc<PublicKeyRegistry>,
    database: Database,
    config: Config,
    // Channel for balance updates
    balance_tx: mpsc::UnboundedSender<BalanceUpdate>,
    // Channel for transaction events
    transaction_tx: mpsc::UnboundedSender<TransactionEvent>,
}

impl YellowstoneSubscriber {
    pub fn new(
        registry: Arc<PublicKeyRegistry>,
        database: Database,
        config: Config,
    ) -> (Self, mpsc::UnboundedReceiver<BalanceUpdate>, mpsc::UnboundedReceiver<TransactionEvent>) {
        let (balance_tx, balance_rx) = mpsc::unbounded_channel();
        let (transaction_tx, transaction_rx) = mpsc::unbounded_channel();

        let subscriber = Self {
            registry,
            database,
            config,
            balance_tx,
            transaction_tx,
        };

        (subscriber, balance_rx, transaction_rx)
    }

    /// Start the Yellowstone subscriber
    pub async fn start(&self) -> Result<()> {
        info!("Starting Yellowstone subscriber for endpoint: {}", self.config.yellowstone_endpoint);

        let mut reconnect_attempts = 0;
        let max_reconnect_attempts = 10;

        loop {
            match self.connect_and_subscribe().await {
                Ok(_) => {
                    info!("Yellowstone subscription ended normally");
                    reconnect_attempts = 0; // Reset on successful connection
                }
                Err(e) => {
                    error!("Yellowstone subscription error: {}", e);
                    reconnect_attempts += 1;

                    if reconnect_attempts >= max_reconnect_attempts {
                        error!("Max reconnection attempts reached, giving up");
                        return Err(e);
                    }

                    let backoff_duration = Duration::from_secs(2_u64.pow(reconnect_attempts.min(6)));
                    warn!("Reconnecting in {:?} (attempt {}/{})", backoff_duration, reconnect_attempts, max_reconnect_attempts);
                    sleep(backoff_duration).await;
                }
            }
        }
    }

    async fn connect_and_subscribe(&self) -> Result<()> {
        // Create gRPC client using the existing yellowstone client
        let mut client = GeyserGrpcClient::build_from_shared(self.config.yellowstone_endpoint.clone())?
            .x_token(Some(self.config.yellowstone_x_token.clone()))?
            .connect()
            .await?;

        info!("Connected to Yellowstone Geyser");

        // Get current active public keys
        let public_keys = self.registry.get_active_public_keys().await;
        if public_keys.is_empty() {
            warn!("No public keys to monitor, waiting for subscriptions...");
            sleep(Duration::from_secs(30)).await;
            return Ok(());
        }

        info!("Monitoring {} public keys", public_keys.len());

        // Create subscription request
        let mut accounts = HashMap::new();
        let mut transactions = HashMap::new();

        // Subscribe to account updates for balance monitoring
        for (i, public_key) in public_keys.iter().enumerate() {
            accounts.insert(
                format!("account_{}", i),
                SubscribeRequestFilterAccounts {
                    account: vec![public_key.clone()],
                    owner: vec![],
                    filters: vec![],
                    nonempty_txn_signature: None,
                },
            );
        }

        // Subscribe to transactions involving our monitored accounts
        transactions.insert(
            "transactions".to_string(),
            SubscribeRequestFilterTransactions {
                vote: Some(false),
                failed: Some(false),
                signature: None,
                account_include: public_keys.clone(),
                account_exclude: vec![],
                account_required: vec![],
            },
        );

        let subscribe_request = SubscribeRequest {
            accounts,
            slots: HashMap::new(),
            transactions,
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(CommitmentLevel::Confirmed as i32),
            accounts_data_slice: vec![],
            from_slot: None,
            ping: None,
            transactions_status: HashMap::new(),
        };

        // Start subscription
        let mut stream = client.subscribe_once(subscribe_request).await?;

        info!("Yellowstone subscription active");

        // Process stream messages
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    if let Err(e) = self.process_message(msg).await {
                        error!("Error processing message: {}", e);
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    return Err(e.into());
                }
            }

            // Periodically refresh public keys
            if rand::random::<f64>() < 0.001 { // ~0.1% chance per message
                if let Err(e) = self.registry.refresh_cache().await {
                    warn!("Failed to refresh registry cache: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn process_message(&self, message: SubscribeUpdate) -> Result<()> {
        match message.update_oneof {
            Some(subscribe_update_oneof) => match subscribe_update_oneof {
                yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Account(account_update) => {
                    self.process_account_update(account_update).await?;
                }
                yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Transaction(transaction_update) => {
                    self.process_transaction_update(transaction_update).await?;
                }
                yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof::Ping(_) => {
                    debug!("Received ping from Yellowstone");
                }
                _ => {
                    debug!("Received other message type from Yellowstone");
                }
            },
            None => {
                debug!("Received empty message from Yellowstone");
            }
        }

        Ok(())
    }

    async fn process_account_update(&self, update: SubscribeUpdateAccount) -> Result<()> {
        let account = match update.account {
            Some(account) => account,
            None => return Ok(()),
        };

        // Convert pubkey bytes to base58 string
        let pubkey = bs58::encode(&account.pubkey).into_string();
        let lamports = account.lamports;
        let slot = update.slot;

        debug!("Account update: {} lamports: {} slot: {}", pubkey, lamports, slot);

        // Check if this is a monitored key
        if !self.registry.is_key_monitored(&pubkey).await {
            return Ok(());
        }

        // Get subscription details
        let subscription = match self.registry.get_key_subscription(&pubkey).await? {
            Some(sub) => sub,
            None => return Ok(()),
        };

        // Create balance update with proper parameters
        let balance_update = BalanceUpdate::new(
            subscription.user_id,
            pubkey.clone(),
            "11111111111111111111111111111112".to_string(), // Native SOL mint
            Decimal::from(0), // We don't have old balance here, would need to track it
            Decimal::from(lamports),
            BalanceChangeType::Transfer, // Use existing enum value
            None, // No transaction signature for account updates
            slot as i64,
        );

        // Send to balance processor
        if let Err(e) = self.balance_tx.send(balance_update.clone()) {
            error!("Failed to send balance update: {}", e);
        }

        // Store in database
        self.store_balance_update(&balance_update).await?;

        info!("Processed balance update for {}: {} lamports", pubkey, lamports);

        Ok(())
    }

    async fn process_transaction_update(&self, update: SubscribeUpdateTransaction) -> Result<()> {
        let transaction = match update.transaction {
            Some(tx) => tx,
            None => return Ok(()),
        };

        // Convert signature bytes to base58 string
        let signature = bs58::encode(&transaction.signature).into_string();
        let slot = update.slot;

        debug!("Transaction update: {} slot: {}", signature, slot);

        // Parse transaction and extract relevant information
        if let Some(_meta) = transaction.meta {
            // For now, just log transaction info since transaction parsing is complex
            debug!("Processing transaction meta for {}", signature);
        }

        Ok(())
    }

    async fn store_balance_update(&self, update: &BalanceUpdate) -> Result<()> {
        // Use simple execute instead of macro to avoid sqlx offline issues
        let query = "
            INSERT INTO balance_updates (id, user_id, public_key, mint_address, old_balance, new_balance, change_amount, change_type, transaction_signature, slot, block_time, processed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ";
        
        sqlx::query(query)
            .bind(&update.id)
            .bind(&update.user_id)
            .bind(&update.public_key)
            .bind(&update.mint_address)
            .bind(update.old_balance)
            .bind(update.new_balance)
            .bind(update.change_amount)
            .bind(&update.change_type)
            .bind(&update.transaction_signature)
            .bind(update.slot)
            .bind(update.block_time)
            .bind(update.processed_at)
            .execute(self.database.get_pool().await)
            .await?;

        Ok(())
    }

    async fn store_transaction_event(&self, event: &TransactionEvent) -> Result<()> {
        // Use simple execute instead of macro to avoid sqlx offline issues
        let query = "
            INSERT INTO transaction_events (id, public_key, signature, slot, block_time, event_type, 
                                          amount, mint, from_address, to_address, fee, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ";
        
        sqlx::query(query)
            .bind(&event.id)
            .bind(&event.public_key)
            .bind(&event.signature)
            .bind(event.slot as i64)
            .bind(event.block_time)
            .bind(format!("{:?}", event.event_type).to_lowercase())
            .bind(event.amount)
            .bind(&event.mint)
            .bind(&event.from_address)
            .bind(&event.to_address)
            .bind(event.fee.map(|f| f as i64))
            .bind(format!("{:?}", event.status).to_lowercase())
            .bind(event.created_at)
            .execute(self.database.get_pool().await)
            .await?;

        Ok(())
    }

    /// Get subscription statistics
    pub async fn get_stats(&self) -> YellowstoneStats {
        let active_keys = self.registry.get_active_public_keys().await;
        
        YellowstoneStats {
            monitored_keys: active_keys.len() as u32,
            connection_status: "connected".to_string(), // Simplified
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct YellowstoneStats {
    pub monitored_keys: u32,
    pub connection_status: String,
}
use crate::{error::UserError, Store};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Row;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub id: String,
    pub amount: Decimal,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub user_id: String,
    pub asset_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceWithDetails {
    pub id: String,
    pub amount: Decimal,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub user_id: String,
    pub asset_id: String,
    pub asset_mint_address: String,
    pub asset_name: String,
    pub asset_symbol: String,
    pub asset_decimals: i32,
    pub asset_logo_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBalanceRequest {
    pub user_id: String,
    pub asset_id: String,
    pub amount: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBalanceRequest {
    pub user_id: String,
    pub asset_id: String,
    pub amount: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequest {
    pub from_user_id: String,
    pub to_user_id: String,
    pub asset_id: String,
    pub amount: Decimal,
}

impl Store {
    pub async fn create_or_update_balance(&self, request: CreateBalanceRequest) -> Result<Balance, UserError> {
        let now = Utc::now();

        // Check if balance already exists for this user and asset
        let existing = sqlx::query(
            "SELECT id, amount FROM balances WHERE user_id = $1 AND asset_id = $2"
        )
        .bind(&request.user_id)
        .bind(&request.asset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = existing {
            // Update existing balance
            let existing_id: String = row.try_get("id").unwrap_or_default();
            let existing_amount: Decimal = row.try_get("amount").unwrap_or(Decimal::ZERO);
            let new_amount = existing_amount + request.amount;

            sqlx::query(
                "UPDATE balances SET amount = $1, updated_at = $2 WHERE id = $3"
            )
            .bind(new_amount)
            .bind(now)
            .bind(&existing_id)
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

            Ok(Balance {
                id: existing_id,
                amount: new_amount,
                created_at: now, // Will be overwritten by actual created_at from DB if needed
                updated_at: now,
                user_id: request.user_id,
                asset_id: request.asset_id,
            })
        } else {
            // Create new balance
            let balance_id = Uuid::new_v4().to_string();

            sqlx::query(
                r#"
                INSERT INTO balances (id, amount, created_at, updated_at, user_id, asset_id)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(&balance_id)
            .bind(request.amount)
            .bind(now)
            .bind(now)
            .bind(&request.user_id)
            .bind(&request.asset_id)
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

            Ok(Balance {
                id: balance_id,
                amount: request.amount,
                created_at: now,
                updated_at: now,
                user_id: request.user_id,
                asset_id: request.asset_id,
            })
        }
    }

    pub async fn get_user_balances(&self, user_id: &str) -> Result<Vec<BalanceWithDetails>, UserError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                b.id, b.amount, b.created_at, b.updated_at, b.user_id, b.asset_id,
                a.mint_address as asset_mint_address, a.name as asset_name, 
                a.symbol as asset_symbol, a.decimals as asset_decimals, a.logo_url as asset_logo_url
            FROM balances b
            JOIN assets a ON b.asset_id = a.id
            WHERE b.user_id = $1
            ORDER BY b.updated_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        let balances = rows.into_iter().map(|row| {
            BalanceWithDetails {
                id: row.try_get("id").unwrap_or_default(),
                amount: row.try_get("amount").unwrap_or(Decimal::ZERO),
                created_at: row.try_get("created_at").unwrap_or_default(),
                updated_at: row.try_get("updated_at").unwrap_or_default(),
                user_id: row.try_get("user_id").unwrap_or_default(),
                asset_id: row.try_get("asset_id").unwrap_or_default(),
                asset_mint_address: row.try_get("asset_mint_address").unwrap_or_default(),
                asset_name: row.try_get("asset_name").unwrap_or_default(),
                asset_symbol: row.try_get("asset_symbol").unwrap_or_default(),
                asset_decimals: row.try_get("asset_decimals").unwrap_or(0),
                asset_logo_url: row.try_get("asset_logo_url").unwrap_or(None),
            }
        }).collect();

        Ok(balances)
    }

    pub async fn get_balance(&self, user_id: &str, asset_id: &str) -> Result<Option<Balance>, UserError> {
        let row = sqlx::query(
            r#"
            SELECT id, amount, created_at, updated_at, user_id, asset_id
            FROM balances 
            WHERE user_id = $1 AND asset_id = $2
            "#
        )
        .bind(user_id)
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            let balance = Balance {
                id: row.try_get("id").unwrap_or_default(),
                amount: row.try_get("amount").unwrap_or(Decimal::ZERO),
                created_at: row.try_get("created_at").unwrap_or_default(),
                updated_at: row.try_get("updated_at").unwrap_or_default(),
                user_id: row.try_get("user_id").unwrap_or_default(),
                asset_id: row.try_get("asset_id").unwrap_or_default(),
            };
            Ok(Some(balance))
        } else {
            Ok(None)
        }
    }

    pub async fn update_balance(&self, request: UpdateBalanceRequest) -> Result<Balance, UserError> {
        let now = Utc::now();

        // Check if balance exists
        let existing = self.get_balance(&request.user_id, &request.asset_id).await?;
        
        if let Some(balance) = existing {
            sqlx::query(
                "UPDATE balances SET amount = $1, updated_at = $2 WHERE id = $3"
            )
            .bind(request.amount)
            .bind(now)
            .bind(&balance.id)
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

            Ok(Balance {
                id: balance.id,
                amount: request.amount,
                created_at: balance.created_at,
                updated_at: now,
                user_id: request.user_id,
                asset_id: request.asset_id,
            })
        } else {
            // Create new balance if it doesn't exist
            self.create_or_update_balance(CreateBalanceRequest {
                user_id: request.user_id,
                asset_id: request.asset_id,
                amount: request.amount,
            }).await
        }
    }

    pub async fn transfer_balance(&self, request: TransferRequest) -> Result<(Balance, Balance), UserError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Clone the values we'll need later
        let from_user_id = request.from_user_id.clone();
        let to_user_id = request.to_user_id.clone();
        let asset_id = request.asset_id.clone();
        let amount = request.amount;

        // Get sender balance
        let sender_balance = self.get_balance(&request.from_user_id, &request.asset_id).await?
            .ok_or(UserError::InsufficientBalance)?;

        if sender_balance.amount < request.amount {
            return Err(UserError::InsufficientBalance);
        }

        let now = Utc::now();
        let new_sender_amount = sender_balance.amount - request.amount;

        // Update sender balance
        sqlx::query(
            "UPDATE balances SET amount = $1, updated_at = $2 WHERE id = $3"
        )
        .bind(new_sender_amount)
        .bind(now)
        .bind(&sender_balance.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Get or create receiver balance
        let receiver_balance = self.get_balance(&request.to_user_id, &request.asset_id).await?;
        
        let updated_receiver = if let Some(balance) = receiver_balance {
            let new_receiver_amount = balance.amount + request.amount;
            
            sqlx::query(
                "UPDATE balances SET amount = $1, updated_at = $2 WHERE id = $3"
            )
            .bind(new_receiver_amount)
            .bind(now)
            .bind(&balance.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

            Balance {
                id: balance.id,
                amount: new_receiver_amount,
                created_at: balance.created_at,
                updated_at: now,
                user_id: to_user_id.clone(),
                asset_id: asset_id.clone(),
            }
        } else {
            // Create new balance for receiver
            let receiver_id = Uuid::new_v4().to_string();
            
            sqlx::query(
                r#"
                INSERT INTO balances (id, amount, created_at, updated_at, user_id, asset_id)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#
            )
            .bind(&receiver_id)
            .bind(amount)
            .bind(now)
            .bind(now)
            .bind(&to_user_id)
            .bind(&asset_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

            Balance {
                id: receiver_id,
                amount,
                created_at: now,
                updated_at: now,
                user_id: to_user_id,
                asset_id: asset_id.clone(),
            }
        };

        tx.commit().await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        let updated_sender = Balance {
            id: sender_balance.id,
            amount: new_sender_amount,
            created_at: sender_balance.created_at,
            updated_at: now,
            user_id: from_user_id,
            asset_id,
        };

        Ok((updated_sender, updated_receiver))
    }
}
use crate::{error::UserError, Store};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAssetRequest {
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAssetRequest {
    pub id: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub logo_url: Option<String>,
}

impl Store {
    pub async fn create_asset(&self, request: CreateAssetRequest) -> Result<Asset, UserError> {
        let asset_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Check if asset with this mint address already exists
        let existing = sqlx::query("SELECT id FROM assets WHERE mint_address = $1")
            .bind(&request.mint_address)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if existing.is_some() {
            return Err(UserError::AssetAlreadyExists);
        }

        sqlx::query(
            r#"
            INSERT INTO assets (id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(&asset_id)
        .bind(&request.mint_address)
        .bind(request.decimals)
        .bind(&request.name)
        .bind(&request.symbol)
        .bind(&request.logo_url)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        let asset = Asset {
            id: asset_id,
            mint_address: request.mint_address,
            decimals: request.decimals,
            name: request.name,
            symbol: request.symbol,
            logo_url: request.logo_url,
            created_at: now,
            updated_at: now,
        };

        Ok(asset)
    }

    pub async fn get_asset_by_id(&self, asset_id: &str) -> Result<Option<Asset>, UserError> {
        let row = sqlx::query(
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets 
            WHERE id = $1
            "#
        )
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            let asset = Asset {
                id: row.try_get("id").unwrap_or_default(),
                mint_address: row.try_get("mint_address").unwrap_or_default(),
                decimals: row.try_get("decimals").unwrap_or(0),
                name: row.try_get("name").unwrap_or_default(),
                symbol: row.try_get("symbol").unwrap_or_default(),
                logo_url: row.try_get("logo_url").unwrap_or(None),
                created_at: row.try_get("created_at").unwrap_or_default(),
                updated_at: row.try_get("updated_at").unwrap_or_default(),
            };
            Ok(Some(asset))
        } else {
            Ok(None)
        }
    }

    pub async fn get_asset_by_mint(&self, mint_address: &str) -> Result<Option<Asset>, UserError> {
        let row = sqlx::query(
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets 
            WHERE mint_address = $1
            "#
        )
        .bind(mint_address)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            let asset = Asset {
                id: row.try_get("id").unwrap_or_default(),
                mint_address: row.try_get("mint_address").unwrap_or_default(),
                decimals: row.try_get("decimals").unwrap_or(0),
                name: row.try_get("name").unwrap_or_default(),
                symbol: row.try_get("symbol").unwrap_or_default(),
                logo_url: row.try_get("logo_url").unwrap_or(None),
                created_at: row.try_get("created_at").unwrap_or_default(),
                updated_at: row.try_get("updated_at").unwrap_or_default(),
            };
            Ok(Some(asset))
        } else {
            Ok(None)
        }
    }

    pub async fn list_assets(&self) -> Result<Vec<Asset>, UserError> {
        let rows = sqlx::query(
            r#"
            SELECT id, mint_address, decimals, name, symbol, logo_url, created_at, updated_at
            FROM assets 
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        let assets = rows.into_iter().map(|row| {
            Asset {
                id: row.try_get("id").unwrap_or_default(),
                mint_address: row.try_get("mint_address").unwrap_or_default(),
                decimals: row.try_get("decimals").unwrap_or(0),
                name: row.try_get("name").unwrap_or_default(),
                symbol: row.try_get("symbol").unwrap_or_default(),
                logo_url: row.try_get("logo_url").unwrap_or(None),
                created_at: row.try_get("created_at").unwrap_or_default(),
                updated_at: row.try_get("updated_at").unwrap_or_default(),
            }
        }).collect();

        Ok(assets)
    }

    pub async fn update_asset(&self, request: UpdateAssetRequest) -> Result<Asset, UserError> {
        let now = Utc::now();
        
        // Get the current asset
        let current_asset = self.get_asset_by_id(&request.id).await?
            .ok_or(UserError::AssetNotFound)?;

        sqlx::query(
            r#"
            UPDATE assets 
            SET name = COALESCE($2, name),
                symbol = COALESCE($3, symbol),
                logo_url = COALESCE($4, logo_url),
                updated_at = $5
            WHERE id = $1
            "#
        )
        .bind(&request.id)
        .bind(&request.name)
        .bind(&request.symbol)
        .bind(&request.logo_url)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Return updated asset
        let updated_asset = Asset {
            id: current_asset.id,
            mint_address: current_asset.mint_address,
            decimals: current_asset.decimals,
            name: request.name.unwrap_or(current_asset.name),
            symbol: request.symbol.unwrap_or(current_asset.symbol),
            logo_url: request.logo_url.or(current_asset.logo_url),
            created_at: current_asset.created_at,
            updated_at: now,
        };

        Ok(updated_asset)
    }

    pub async fn delete_asset(&self, asset_id: &str) -> Result<(), UserError> {
        let result = sqlx::query("DELETE FROM assets WHERE id = $1")
            .bind(asset_id)
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(UserError::AssetNotFound);
        }

        Ok(())
    }
}
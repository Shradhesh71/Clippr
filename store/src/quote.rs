use crate::{error::UserError, Store};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteData {
    pub id: String,
    pub user_id: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: i32,
    pub platform_fee: Option<serde_json::Value>,
    pub price_impact_pct: String,
    pub route_plan: serde_json::Value,
    pub context_slot: Option<i64>,
    pub time_taken: Option<f64>,
    pub created_at: chrono::DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveQuoteRequest {
    pub user_id: String,
    pub quote_response: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetQuoteRequest {
    pub user_id: String,
    pub quote_id: Option<String>,
}

impl Store {
    pub async fn save_quote(&self, request: SaveQuoteRequest) -> Result<QuoteData, UserError> {
        // Parse the quote response
        let quote = &request.quote_response;
        
        let quote_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        // Deactivate all previous quotes for this user
        sqlx::query("UPDATE quotes SET is_active = false WHERE user_id = $1")
            .bind(&request.user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Insert new quote
        sqlx::query(
            r#"
            INSERT INTO quotes (
                id, user_id, input_mint, output_mint, in_amount, out_amount,
                other_amount_threshold, swap_mode, slippage_bps, platform_fee,
                price_impact_pct, route_plan, context_slot, time_taken, created_at, is_active
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#
        )
        .bind(&quote_id)
        .bind(&request.user_id)
        .bind(quote.get("inputMint").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(quote.get("outputMint").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(quote.get("inAmount").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(quote.get("outAmount").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(quote.get("otherAmountThreshold").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(quote.get("swapMode").and_then(|v| v.as_str()).unwrap_or("ExactIn"))
        .bind(quote.get("slippageBps").and_then(|v| v.as_i64()).unwrap_or(50) as i32)
        .bind(quote.get("platformFee"))
        .bind(quote.get("priceImpactPct").and_then(|v| v.as_str()).unwrap_or("0"))
        .bind(quote.get("routePlan").unwrap_or(&serde_json::json!([])))
        .bind(quote.get("contextSlot").and_then(|v| v.as_i64()))
        .bind(quote.get("timeTaken").and_then(|v| v.as_f64()))
        .bind(&created_at)
        .bind(true) // is_active
        .execute(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Return the saved quote data
        let saved_quote = QuoteData {
            id: quote_id,
            user_id: request.user_id,
            input_mint: quote.get("inputMint").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            output_mint: quote.get("outputMint").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            in_amount: quote.get("inAmount").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            out_amount: quote.get("outAmount").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            other_amount_threshold: quote.get("otherAmountThreshold").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            swap_mode: quote.get("swapMode").and_then(|v| v.as_str()).unwrap_or("ExactIn").to_string(),
            slippage_bps: quote.get("slippageBps").and_then(|v| v.as_i64()).unwrap_or(50) as i32,
            platform_fee: quote.get("platformFee").cloned(),
            price_impact_pct: quote.get("priceImpactPct").and_then(|v| v.as_str()).unwrap_or("0").to_string(),
            route_plan: quote.get("routePlan").cloned().unwrap_or(serde_json::json!([])),
            context_slot: quote.get("contextSlot").and_then(|v| v.as_i64()),
            time_taken: quote.get("timeTaken").and_then(|v| v.as_f64()),
            created_at,
            is_active: true,
        };

        Ok(saved_quote)
    }

    pub async fn get_active_quote(&self, user_id: &str) -> Result<Option<serde_json::Value>, UserError> {
        let row = sqlx::query(
            r#"
            SELECT input_mint, output_mint, in_amount, out_amount, other_amount_threshold,
                   swap_mode, slippage_bps, platform_fee, price_impact_pct, route_plan,
                   context_slot, time_taken
            FROM quotes 
            WHERE user_id = $1 AND is_active = true 
            ORDER BY created_at DESC 
            LIMIT 1
            "#
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            let quote_response = serde_json::json!({
                "inputMint": row.try_get::<String, _>("input_mint").unwrap_or_default(),
                "inAmount": row.try_get::<String, _>("in_amount").unwrap_or_default(),
                "outputMint": row.try_get::<String, _>("output_mint").unwrap_or_default(),
                "outAmount": row.try_get::<String, _>("out_amount").unwrap_or_default(),
                "otherAmountThreshold": row.try_get::<String, _>("other_amount_threshold").unwrap_or_default(),
                "swapMode": row.try_get::<String, _>("swap_mode").unwrap_or_default(),
                "slippageBps": row.try_get::<i32, _>("slippage_bps").unwrap_or(50),
                "platformFee": row.try_get::<Option<serde_json::Value>, _>("platform_fee").unwrap_or(None),
                "priceImpactPct": row.try_get::<String, _>("price_impact_pct").unwrap_or_default(),
                "routePlan": row.try_get::<serde_json::Value, _>("route_plan").unwrap_or(serde_json::json!([])),
                "contextSlot": row.try_get::<Option<i64>, _>("context_slot").unwrap_or(None),
                "timeTaken": row.try_get::<Option<f64>, _>("time_taken").unwrap_or(None)
            });

            Ok(Some(quote_response))
        } else {
            Ok(None)
        }
    }

    pub async fn get_quote_by_id(&self, quote_id: &str, user_id: &str) -> Result<Option<serde_json::Value>, UserError> {
        let row = sqlx::query(
            r#"
            SELECT input_mint, output_mint, in_amount, out_amount, other_amount_threshold,
                   swap_mode, slippage_bps, platform_fee, price_impact_pct, route_plan,
                   context_slot, time_taken
            FROM quotes 
            WHERE id = $1 AND user_id = $2
            "#
        )
        .bind(quote_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            let quote_response = serde_json::json!({
                "inputMint": row.try_get::<String, _>("input_mint").unwrap_or_default(),
                "inAmount": row.try_get::<String, _>("in_amount").unwrap_or_default(),
                "outputMint": row.try_get::<String, _>("output_mint").unwrap_or_default(),
                "outAmount": row.try_get::<String, _>("out_amount").unwrap_or_default(),
                "otherAmountThreshold": row.try_get::<String, _>("other_amount_threshold").unwrap_or_default(),
                "swapMode": row.try_get::<String, _>("swap_mode").unwrap_or_default(),
                "slippageBps": row.try_get::<i32, _>("slippage_bps").unwrap_or(50),
                "platformFee": row.try_get::<Option<serde_json::Value>, _>("platform_fee").unwrap_or(None),
                "priceImpactPct": row.try_get::<String, _>("price_impact_pct").unwrap_or_default(),
                "routePlan": row.try_get::<serde_json::Value, _>("route_plan").unwrap_or(serde_json::json!([])),
                "contextSlot": row.try_get::<Option<i64>, _>("context_slot").unwrap_or(None),
                "timeTaken": row.try_get::<Option<f64>, _>("time_taken").unwrap_or(None)
            });

            Ok(Some(quote_response))
        } else {
            Ok(None)
        }
    }
}
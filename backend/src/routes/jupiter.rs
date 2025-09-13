use std::sync::Arc;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use store::Store;
use tokio::sync::Mutex;


#[derive(Deserialize)]
pub struct QuoteRequest {
    pub user_id: String,
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u16,
}

#[derive(Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

#[derive(Serialize)]
pub struct QuoteResponse {
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub price_impact_pct: String,
    pub slippage_bps: u16,
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Deserialize)]
pub struct SwapRequest {
    pub user_id: String,
    pub user_public_key: String,
}

#[derive(Serialize)]
pub struct SwapResponse {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    pub swap_details: Option<SwapDetails>,
    pub balance_updates: Option<BalanceUpdates>,
}

#[derive(Serialize)]
pub struct SwapDetails {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: String,
    pub output_amount: String,
    pub price_impact_pct: String,
}

#[derive(Serialize)]
pub struct BalanceUpdates {
    pub input_token_balance: String,
    pub output_token_balance: String,
    pub input_token_symbol: String,
    pub output_token_symbol: String,
}

#[actix_web::post("/quote")]
pub async fn quote(req: web::Json<QuoteRequest>, store: web::Data<Arc<Mutex<Store>>>) -> Result<HttpResponse> {
    // let response = QuoteResponse {};
    
    // let quote = reqwest::Client::new();

    // let response = quote
    //     .post(format!("https://lite-api.jup.ag/swap/v1/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&restrictIntermediateTokens=true", req.input_mint, req.output_mint, req.amount, req.slippage_bps))
    //     .send()
    //     .await
    //     .map_err(|e| {
    //         actix_web::error::ErrorInternalServerError("Failed to call Jup API")
    //     })?;

    let client = reqwest::Client::builder().build()
        .map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to build HTTP client"))?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/json".parse()?);

    let url = format!(
        "https://lite-api.jup.ag/swap/v1/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}&restrictIntermediateTokens=true",
        req.input_mint, 
        req.output_mint, 
        req.amount, 
        req.slippage_bps
    );

    let request = client.request(reqwest::Method::GET, url)
        .headers(headers);

    let response = request.send().await.map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to call Jup API"))?;
    let body = response.text().await.map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to read response body"))?;

    println!("Jupiter Quote Response: {}", body);

    // Parse the response as JSON to save to database
    let quote_response: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to parse Jupiter response"))?;

    // Save the quote response to database
    let save_request = store::quote::SaveQuoteRequest {
        user_id: req.user_id.clone(),
        quote_response: quote_response.clone(),
    };

    let store_guard = store.lock().await;
    match store_guard.save_quote(save_request).await {
        Ok(saved_quote) => {
            println!("Quote saved successfully for user: {}", saved_quote.user_id);
        }
        Err(e) => {
            println!("Failed to save quote: {:?}", e);
            // Continue anyway - don't fail the request if quote saving fails
        }
    }
    drop(store_guard);

    // Extract necessary information for user response
    let user_quote_response = QuoteResponse {
        input_mint: quote_response.get("inputMint")
            .and_then(|v| v.as_str())
            .unwrap_or(&req.input_mint)
            .to_string(),
        output_mint: quote_response.get("outputMint")
            .and_then(|v| v.as_str())
            .unwrap_or(&req.output_mint)
            .to_string(),
        in_amount: quote_response.get("inAmount")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string(),
        out_amount: quote_response.get("outAmount")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string(),
        price_impact_pct: quote_response.get("priceImpactPct")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string(),
        slippage_bps: req.slippage_bps,
        route_plan: quote_response.get("routePlan")
            .and_then(|v| v.as_array())
            .map(|routes| {
                routes.iter().filter_map(|route| {
                    let swap_info = route.get("swapInfo")?;
                    Some(RoutePlan {
                        swap_info: SwapInfo {
                            amm_key: swap_info.get("ammKey")?.as_str()?.to_string(),
                            label: swap_info.get("label")?.as_str()?.to_string(),
                            input_mint: swap_info.get("inputMint")?.as_str()?.to_string(),
                            output_mint: swap_info.get("outputMint")?.as_str()?.to_string(),
                            in_amount: swap_info.get("inAmount")?.as_str()?.to_string(),
                            out_amount: swap_info.get("outAmount")?.as_str()?.to_string(),
                            fee_amount: swap_info.get("feeAmount")?.as_str()?.to_string(),
                            fee_mint: swap_info.get("feeMint")?.as_str()?.to_string(),
                        },
                        percent: route.get("percent")?.as_u64().unwrap_or(100) as u8,
                    })
                }).collect()
            })
            .unwrap_or_default(),
    };

    Ok(HttpResponse::Ok().json(user_quote_response))
}

#[actix_web::post("/swap")]
pub async fn swap(req: web::Json<SwapRequest>, store: web::Data<Arc<Mutex<Store>>>) -> Result<HttpResponse> {
    println!("Processing swap request for user: {}", req.user_id);

    // Step 1: Get the saved quote from database
    let store_guard = store.lock().await;
    let quote_response = match store_guard.get_active_quote(&req.user_id).await {
        Ok(Some(quote_data)) => {
            println!("Retrieved active quote for user: {}", req.user_id);
            quote_data
        }
        Ok(None) => {
            println!("No active quote found for user: {}", req.user_id);
            return Ok(HttpResponse::BadRequest().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("No active quote found for user. Please get a quote first.".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
        Err(e) => {
            println!("Failed to get quote from database: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to retrieve quote from database".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };
    drop(store_guard);

    // Extract swap information from quote
    let input_mint = quote_response.get("inputMint")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let output_mint = quote_response.get("outputMint")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let input_amount_str = quote_response.get("inAmount")
        .and_then(|v| v.as_str())
        .unwrap_or("0")
        .to_string();
    let output_amount_str = quote_response.get("outAmount")
        .and_then(|v| v.as_str())
        .unwrap_or("0")
        .to_string();

    // Parse amounts for balance calculations
    let input_amount: u64 = input_amount_str.parse().unwrap_or(0);
    let output_amount: u64 = output_amount_str.parse().unwrap_or(0);

    // Step 2: Ensure assets exist in our database
    let store_guard = store.lock().await;
    
    // Check/create input asset
    let input_asset = match store_guard.get_asset_by_mint(&input_mint).await {
        Ok(Some(asset)) => asset,
        Ok(None) => {
            // Try to create asset with default values (you might want to fetch from token registry)
            let create_request = store::asset::CreateAssetRequest {
                mint_address: input_mint.clone(),
                decimals: 9, // Default, should be fetched from chain/registry
                name: format!("Token {}", &input_mint[..8]),
                symbol: format!("TK{}", &input_mint[..4]),
                logo_url: None,
            };
            
            match store_guard.create_asset(create_request).await {
                Ok(asset) => {
                    println!("Created input asset: {}", asset.symbol);
                    asset
                }
                Err(e) => {
                    println!("Failed to create input asset: {:?}", e);
                    return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                        success: false,
                        transaction_signature: None,
                        error: Some("Failed to create input asset".to_string()),
                        swap_details: None,
                        balance_updates: None,
                    }));
                }
            }
        }
        Err(e) => {
            println!("Failed to get input asset: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to get input asset".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    // Check/create output asset
    let output_asset = match store_guard.get_asset_by_mint(&output_mint).await {
        Ok(Some(asset)) => asset,
        Ok(None) => {
            let create_request = store::asset::CreateAssetRequest {
                mint_address: output_mint.clone(),
                decimals: 9, // Default, should be fetched from chain/registry
                name: format!("Token {}", &output_mint[..8]),
                symbol: format!("TK{}", &output_mint[..4]),
                logo_url: None,
            };
            
            match store_guard.create_asset(create_request).await {
                Ok(asset) => {
                    println!("Created output asset: {}", asset.symbol);
                    asset
                }
                Err(e) => {
                    println!("Failed to create output asset: {:?}", e);
                    return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                        success: false,
                        transaction_signature: None,
                        error: Some("Failed to create output asset".to_string()),
                        swap_details: None,
                        balance_updates: None,
                    }));
                }
            }
        }
        Err(e) => {
            println!("Failed to get output asset: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to get output asset".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    // Step 3: Check user has sufficient input balance
    let input_balance = match store_guard.get_balance(&req.user_id, &input_asset.id).await {
        Ok(Some(balance)) => balance,
        Ok(None) => {
            return Ok(HttpResponse::BadRequest().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some(format!("No {} balance found for user", input_asset.symbol)),
                swap_details: None,
                balance_updates: None,
            }));
        }
        Err(e) => {
            println!("Failed to get input balance: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to check input balance".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    // Convert input amount to decimal (considering token decimals)
    let input_amount_decimal = rust_decimal::Decimal::from(input_amount) / 
        rust_decimal::Decimal::from(10u64.pow(input_asset.decimals as u32));
    
    if input_balance.amount < input_amount_decimal {
        return Ok(HttpResponse::BadRequest().json(SwapResponse {
            success: false,
            transaction_signature: None,
            error: Some(format!(
                "Insufficient {} balance. Required: {}, Available: {}", 
                input_asset.symbol, input_amount_decimal, input_balance.amount
            )),
            swap_details: None,
            balance_updates: None,
        }));
    }

    drop(store_guard);

    // Step 4: Build swap transaction using Jupiter API
    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to create header"))?);
    headers.insert("Accept", "application/json".parse()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Failed to create header"))?);

    let swap_build_request = serde_json::json!({
        "userPublicKey": req.user_public_key,
        "quoteResponse": quote_response,
        "prioritizationFeeLamports": {
            "priorityLevelWithMaxLamports": {
                "maxLamports": 10000000,
                "priorityLevel": "veryHigh"
            }
        },
        "dynamicComputeUnitLimit": true
    });

    println!("Building swap transaction with Jupiter API...");

    let jupiter_response = match client
        .post("https://lite-api.jup.ag/swap/v1/swap")
        .headers(headers)
        .json(&swap_build_request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            println!("Failed to call Jupiter swap API: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to build swap transaction".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    if !jupiter_response.status().is_success() {
        let error_text = jupiter_response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        println!("Jupiter API returned error: {}", error_text);
        return Ok(HttpResponse::BadRequest().json(SwapResponse {
            success: false,
            transaction_signature: None,
            error: Some(format!("Jupiter API error: {}", error_text)),
            swap_details: None,
            balance_updates: None,
        }));
    }

    let jupiter_swap_response: serde_json::Value = match jupiter_response.json().await {
        Ok(response) => {
            println!("Successfully built swap transaction");
            response
        }
        Err(e) => {
            println!("Failed to parse Jupiter response: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to parse Jupiter response".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    // Step 5: Forward to MPC service for secure signing and broadcasting
    let mpc_service_url = std::env::var("MPC_SIMPLE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());

    println!("Forwarding transaction to MPC service for signing...");

    let mpc_request = serde_json::json!({
        "user_id": req.user_id,
        "user_public_key": req.user_public_key,
        "swap_transaction": jupiter_swap_response.get("swapTransaction"),
        "operation": "jupiter_swap"
    });

    let mpc_response = match client
        .post(format!("{}/api/jupiter-swap", mpc_service_url))
        .json(&mpc_request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            println!("Failed to connect to MPC service: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to connect to MPC service".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    let mpc_result: serde_json::Value = match mpc_response.json().await {
        Ok(result) => result,
        Err(e) => {
            println!("Failed to parse MPC service response: {}", e);
            return Ok(HttpResponse::InternalServerError().json(SwapResponse {
                success: false,
                transaction_signature: None,
                error: Some("Failed to parse MPC service response".to_string()),
                swap_details: None,
                balance_updates: None,
            }));
        }
    };

    let swap_success = mpc_result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    
    // Step 6: Update balances if swap was successful
    let balance_updates = if swap_success {
        println!("Swap successful, updating user balances...");
        
        let store_guard = store.lock().await;
        
        // Decrease input token balance
        let new_input_balance = input_balance.amount - input_amount_decimal;
        let input_update_request = store::balance::UpdateBalanceRequest {
            user_id: req.user_id.clone(),
            asset_id: input_asset.id.clone(),
            amount: new_input_balance,
        };
        
        match store_guard.update_balance(input_update_request).await {
            Ok(_) => {
                println!("Updated {} balance: -{}", input_asset.symbol, input_amount_decimal);
            }
            Err(e) => {
                println!("Failed to update input balance: {:?}", e);
                // Continue - don't fail the whole operation if balance update fails
            }
        }
        
        // Increase output token balance
        let output_amount_decimal = rust_decimal::Decimal::from(output_amount) / 
            rust_decimal::Decimal::from(10u64.pow(output_asset.decimals as u32));
        
        let output_balance_request = store::balance::CreateBalanceRequest {
            user_id: req.user_id.clone(),
            asset_id: output_asset.id.clone(),
            amount: output_amount_decimal,
        };
        
        let final_output_balance = match store_guard.create_or_update_balance(output_balance_request).await {
            Ok(balance) => {
                println!("Updated {} balance: +{}", output_asset.symbol, output_amount_decimal);
                balance.amount
            }
            Err(e) => {
                println!("Failed to update output balance: {:?}", e);
                output_amount_decimal // Fallback
            }
        };
        
        drop(store_guard);
        
        Some(BalanceUpdates {
            input_token_balance: new_input_balance.to_string(),
            output_token_balance: final_output_balance.to_string(),
            input_token_symbol: input_asset.symbol.clone(),
            output_token_symbol: output_asset.symbol.clone(),
        })
    } else {
        None
    };

    let swap_details = SwapDetails {
        input_mint,
        output_mint,
        input_amount: input_amount_str,
        output_amount: output_amount_str,
        price_impact_pct: quote_response.get("priceImpactPct")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string(),
    };

    let final_response = SwapResponse {
        success: swap_success,
        transaction_signature: mpc_result.get("transaction_signature")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        error: mpc_result.get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        swap_details: Some(swap_details),
        balance_updates,
    };

    if final_response.success {
        println!("Swap completed successfully for user: {}", req.user_id);
        if let Some(ref sig) = final_response.transaction_signature {
            println!("Transaction signature: {}", sig);
        }
    } else {
        println!("Swap failed for user: {}", req.user_id);
        if let Some(ref error) = final_response.error {
            println!("Error: {}", error);
        }
    }

    Ok(HttpResponse::Ok().json(final_response))
}
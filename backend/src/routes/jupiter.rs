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
pub struct QuoteResponse {
    
}

#[derive(Deserialize)]
pub struct SwapRequest {
    pub user_id: String,
    pub user_public_key: String,
}

#[derive(Serialize)]
pub struct SwapResponse {
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

    Ok(HttpResponse::Ok().json(quote_response))
}

#[actix_web::post("/swap")]
pub async fn swap(req: web::Json<SwapRequest>, store: web::Data<Arc<Mutex<Store>>>) -> Result<HttpResponse> {
    
    // let response = SwapResponse {};

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse()?);
    headers.insert("Accept", "application/json".parse()?);

    let client = reqwest::Client::builder().build()
        .map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to build HTTP client"))?;

    // Get the saved quote from database
    let store_guard = store.lock().await;
    let quote_response = match store_guard.get_active_quote(&req.user_id).await {
        Ok(Some(quote_data)) => quote_data,
        Ok(None) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "No active quote found for user. Please get a quote first."
            })));
        }
        Err(e) => {
            println!("Failed to get quote from database: {:?}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve quote from database"
            })));
        }
    };

    // Use the actual request data with the saved quote
    let swap_request = serde_json::json!({
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

    let request = client.request(reqwest::Method::POST, "https://lite-api.jup.ag/swap/v1/swap")
        .headers(headers)
        .json(&swap_request);

    let response = request.send().await.map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to call Jup API"))?;
    let body = response.text().await.map_err(|_e| actix_web::error::ErrorInternalServerError("Failed to read response body"))?;

    println!("Jupiter Swap Response: {}", body);

    Ok(HttpResponse::Ok().json(body))
}
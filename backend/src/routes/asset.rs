use std::sync::Arc;
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use store::Store;
use tokio::sync::Mutex;

#[derive(Deserialize)]
pub struct CreateAssetRequest {
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateAssetRequest {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub logo_url: Option<String>,
}

#[derive(Serialize)]
pub struct AssetResponse {
    pub id: String,
    pub mint_address: String,
    pub decimals: i32,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[actix_web::post("/assets")]
pub async fn create_asset(
    req: web::Json<CreateAssetRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let store_guard = store.lock().await;
    
    let create_request = store::asset::CreateAssetRequest {
        mint_address: req.mint_address.clone(),
        decimals: req.decimals,
        name: req.name.clone(),
        symbol: req.symbol.clone(),
        logo_url: req.logo_url.clone(),
    };

    match store_guard.create_asset(create_request).await {
        Ok(asset) => {
            let response = AssetResponse {
                id: asset.id,
                mint_address: asset.mint_address,
                decimals: asset.decimals,
                name: asset.name,
                symbol: asset.symbol,
                logo_url: asset.logo_url,
                created_at: asset.created_at,
                updated_at: asset.updated_at,
            };
            Ok(HttpResponse::Created().json(response))
        }
        Err(e) => {
            println!("Failed to create asset: {:?}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

#[actix_web::get("/assets")]
pub async fn list_assets(
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let store_guard = store.lock().await;

    match store_guard.list_assets().await {
        Ok(assets) => {
            let response: Vec<AssetResponse> = assets.into_iter().map(|asset| AssetResponse {
                id: asset.id,
                mint_address: asset.mint_address,
                decimals: asset.decimals,
                name: asset.name,
                symbol: asset.symbol,
                logo_url: asset.logo_url,
                created_at: asset.created_at,
                updated_at: asset.updated_at,
            }).collect();
            
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("Failed to list assets: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve assets"
            })))
        }
    }
}

#[actix_web::get("/assets/{asset_id}")]
pub async fn get_asset(
    path: web::Path<String>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let asset_id = path.into_inner();
    let store_guard = store.lock().await;

    match store_guard.get_asset_by_id(&asset_id).await {
        Ok(Some(asset)) => {
            let response = AssetResponse {
                id: asset.id,
                mint_address: asset.mint_address,
                decimals: asset.decimals,
                name: asset.name,
                symbol: asset.symbol,
                logo_url: asset.logo_url,
                created_at: asset.created_at,
                updated_at: asset.updated_at,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => {
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Asset not found"
            })))
        }
        Err(e) => {
            println!("Failed to get asset: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve asset"
            })))
        }
    }
}

#[actix_web::put("/assets/{asset_id}")]
pub async fn update_asset(
    path: web::Path<String>,
    req: web::Json<UpdateAssetRequest>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let asset_id = path.into_inner();
    let store_guard = store.lock().await;

    let update_request = store::asset::UpdateAssetRequest {
        id: asset_id,
        name: req.name.clone(),
        symbol: req.symbol.clone(),
        logo_url: req.logo_url.clone(),
    };

    match store_guard.update_asset(update_request).await {
        Ok(asset) => {
            let response = AssetResponse {
                id: asset.id,
                mint_address: asset.mint_address,
                decimals: asset.decimals,
                name: asset.name,
                symbol: asset.symbol,
                logo_url: asset.logo_url,
                created_at: asset.created_at,
                updated_at: asset.updated_at,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            println!("Failed to update asset: {:?}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}

#[actix_web::delete("/assets/{asset_id}")]
pub async fn delete_asset(
    path: web::Path<String>,
    store: web::Data<Arc<Mutex<Store>>>,
) -> Result<HttpResponse> {
    let asset_id = path.into_inner();
    let store_guard = store.lock().await;

    match store_guard.delete_asset(&asset_id).await {
        Ok(()) => {
            Ok(HttpResponse::NoContent().finish())
        }
        Err(e) => {
            println!("Failed to delete asset: {:?}", e);
            Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": e.to_string()
            })))
        }
    }
}
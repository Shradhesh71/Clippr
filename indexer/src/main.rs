mod config;
mod database;
mod models;
mod registry;
mod subscriber;
mod yellowstone;
mod routes;

use actix_web::{web, App, HttpServer, middleware::Logger};
use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use database::Database;
use registry::PublicKeyRegistry;
use subscriber::YellowstoneSubscriber;

#[actix_web::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "indexer=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Clippr Indexer Service...");

    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");

    // Initialize database
    let database = Database::new(&config.database_url).await?;
    info!("Database connection established");

    // Run migrations
    database.migrate().await?;
    info!("Database migrations completed");

    // Initialize public key registry
    let registry = Arc::new(PublicKeyRegistry::new(database.clone()).await?);
    info!("Public key registry initialized");

    // Initialize Yellowstone subscriber
    let (subscriber, balance_rx, transaction_rx) = YellowstoneSubscriber::new(
        registry.clone(),
        database.clone(),
        config.clone(),
    );
    let subscriber = Arc::new(subscriber);
    
    info!("Yellowstone subscriber initialized");

    // Start balance processor
    let balance_processor_registry = registry.clone();
    let balance_processor_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = start_balance_processor(balance_rx, balance_processor_registry, balance_processor_config).await {
            error!("Balance processor error: {}", e);
        }
    });

    // Start transaction processor
    let transaction_processor_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = start_transaction_processor(transaction_rx, transaction_processor_config).await {
            error!("Transaction processor error: {}", e);
        }
    });

    // Start Yellowstone subscriber in background
    let yellowstone_subscriber = subscriber.clone();
    tokio::spawn(async move {
        if let Err(e) = yellowstone_subscriber.start().await {
            error!("Yellowstone subscriber error: {}", e);
        }
    });

    // Start HTTP server
    info!("Starting HTTP server on {}:{}", config.server_host, config.server_port);
    
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(database.clone()))
            .app_data(web::Data::new(registry.clone()))
            .app_data(web::Data::new(subscriber.clone()))
            .wrap(Logger::default())
            .configure(routes::configure_routes)
    })
    .bind((config.server_host.clone(), config.server_port))?
    .run();

    info!("Indexer service is now running");

    // Wait for shutdown signal
    tokio::select! {
        _ = server => {
            info!("HTTP server stopped");
        }
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("Shutting down indexer service...");
    Ok(())
}

async fn start_balance_processor(
    mut balance_rx: tokio::sync::mpsc::UnboundedReceiver<models::BalanceUpdate>,
    _registry: Arc<PublicKeyRegistry>,
    config: Config,
) -> Result<()> {
    info!("Starting balance processor");

    while let Some(balance_update) = balance_rx.recv().await {
        if let Err(e) = process_balance_update(&balance_update, &config).await {
            error!("Failed to process balance update: {}", e);
        }
    }

    Ok(())
}

async fn start_transaction_processor(
    mut transaction_rx: tokio::sync::mpsc::UnboundedReceiver<models::TransactionEvent>,
    config: Config,
) -> Result<()> {
    info!("Starting transaction processor");

    while let Some(transaction_event) = transaction_rx.recv().await {
        if let Err(e) = process_transaction_event(&transaction_event, &config).await {
            error!("Failed to process transaction event: {}", e);
        }
    }

    Ok(())
}

async fn process_balance_update(
    balance_update: &models::BalanceUpdate,
    config: &Config,
) -> Result<()> {
    // Send balance update to main backend service
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/balance/update", config.backend_url))
        .json(balance_update)
        .send()
        .await?;

    if response.status().is_success() {
        info!("Successfully sent balance update for user {} to backend", balance_update.user_id);
    } else {
        error!("Failed to send balance update to backend: status {}", response.status());
    }

    Ok(())
}

async fn process_transaction_event(
    transaction_event: &models::TransactionEvent,
    config: &Config,
) -> Result<()> {
    // Send transaction event to main backend service
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/api/transactions/event", config.backend_url))
        .json(transaction_event)
        .send()
        .await?;

    if response.status().is_success() {
        info!("Successfully sent transaction event {} to backend", transaction_event.signature);
    } else {
        error!("Failed to send transaction event to backend: status {}", response.status());
    }

    Ok(())
}

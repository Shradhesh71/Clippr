use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub yellowstone_endpoint: String,
    pub yellowstone_x_token: String,
    pub backend_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // Load .env file if present

        let config = Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:password@localhost/clippr_indexer".to_string()),
            
            server_host: env::var("SERVER_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .context("Invalid SERVER_PORT")?,
            
            yellowstone_endpoint: env::var("YELLOWSTONE_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:10000".to_string()),
            
            yellowstone_x_token: env::var("YELLOWSTONE_X_TOKEN")
                .unwrap_or_else(|_| "your-token-here".to_string()),
            
            backend_url: env::var("BACKEND_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.database_url.is_empty() {
            return Err(anyhow::anyhow!("DATABASE_URL cannot be empty"));
        }

        if self.yellowstone_endpoint.is_empty() {
            return Err(anyhow::anyhow!("YELLOWSTONE_ENDPOINT cannot be empty"));
        }

        if self.backend_url.is_empty() {
            return Err(anyhow::anyhow!("BACKEND_URL cannot be empty"));
        }

        Ok(())
    }
}
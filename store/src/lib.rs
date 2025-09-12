pub mod user;
pub mod helper;
pub mod error;
pub mod quote;
pub mod asset;
pub mod balance;

use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(Clone)]
pub struct Store {
    pub pool: PgPool,
}

impl Store {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self::new(pool))
    }

    pub async fn connect_with_options(
        database_url: &str,
        max_connections: u32,
    ) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;

        Ok(Self::new(pool))
    }
}
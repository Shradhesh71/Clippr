use sqlx::PgPool;
use anyhow::Result;
use std::env;
use crate::models::KeyShare;

#[derive(Clone)]
pub struct DatabaseManager {
    pub mpc1_pool: PgPool,
    pub mpc2_pool: PgPool, 
    pub mpc3_pool: PgPool,
}

impl DatabaseManager {
    pub async fn new() -> Result<Self> {
        let mpc1_url = env::var("MPC1_DATABASE_URL")
            .expect("MPC1_DATABASE_URL must be set");
        let mpc2_url = env::var("MPC2_DATABASE_URL")
            .expect("MPC2_DATABASE_URL must be set");
        let mpc3_url = env::var("MPC3_DATABASE_URL")
            .expect("MPC3_DATABASE_URL must be set");

        let mpc1_pool = PgPool::connect(&mpc1_url).await?;
        let mpc2_pool = PgPool::connect(&mpc2_url).await?;
        let mpc3_pool = PgPool::connect(&mpc3_url).await?;

        // Initialize tables for all databases
        Self::initialize_tables(&mpc1_pool).await?;
        Self::initialize_tables(&mpc2_pool).await?;
        Self::initialize_tables(&mpc3_pool).await?;

        Ok(Self {
            mpc1_pool,
            mpc2_pool,
            mpc3_pool,
        })
    }

    async fn initialize_tables(pool: &PgPool) -> Result<()> {
        let query = r#"
            CREATE TABLE IF NOT EXISTS key_shares (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id TEXT NOT NULL,
                public_key TEXT NOT NULL,
                encrypted_share TEXT NOT NULL,
                share_index INTEGER NOT NULL,
                threshold INTEGER NOT NULL,
                total_shares INTEGER NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                UNIQUE(user_id, share_index)
            );

            CREATE INDEX IF NOT EXISTS idx_key_shares_user_id ON key_shares(user_id);
            CREATE INDEX IF NOT EXISTS idx_key_shares_share_index ON key_shares(share_index);
        "#;

        sqlx::query(query).execute(pool).await?;
        Ok(())
    }

    pub fn get_pool_by_index(&self, index: usize) -> &PgPool {
        match index {
            0 => &self.mpc1_pool,
            1 => &self.mpc2_pool,
            2 => &self.mpc3_pool,
            _ => panic!("Invalid pool index"),
        }
    }

    pub async fn store_key_share(
        &self,
        share: &KeyShare,
        database_index: usize,
    ) -> Result<()> {
        let pool = self.get_pool_by_index(database_index);
        
        let query = r#"
            INSERT INTO key_shares (id, user_id, public_key, encrypted_share, share_index, threshold, total_shares, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (user_id, share_index) 
            DO UPDATE SET 
                public_key = EXCLUDED.public_key,
                encrypted_share = EXCLUDED.encrypted_share,
                threshold = EXCLUDED.threshold,
                total_shares = EXCLUDED.total_shares,
                created_at = EXCLUDED.created_at
        "#;

        sqlx::query(query)
            .bind(share.id)
            .bind(&share.user_id)
            .bind(&share.public_key)
            .bind(&share.encrypted_share)
            .bind(share.share_index)
            .bind(share.threshold)
            .bind(share.total_shares)
            .bind(share.created_at)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_key_share(
        &self,
        user_id: &str,
        database_index: usize,
    ) -> Result<Option<KeyShare>> {
        let pool = self.get_pool_by_index(database_index);
        
        let query = r#"
            SELECT id, user_id, public_key, encrypted_share, share_index, threshold, total_shares, created_at
            FROM key_shares 
            WHERE user_id = $1 AND share_index = $2
        "#;

        let result = sqlx::query_as::<_, KeyShare>(query)
            .bind(user_id)
            .bind((database_index + 1) as i32) // share_index is 1-based
            .fetch_optional(pool)
            .await?;

        Ok(result)
    }

    pub async fn get_all_user_shares(&self, user_id: &str) -> Result<Vec<KeyShare>> {
        let mut all_shares = Vec::new();

        for i in 0..3 {
            if let Some(share) = self.get_key_share(user_id, i).await? {
                all_shares.push(share);
            }
        }

        Ok(all_shares)
    }

    pub async fn delete_user_shares(&self, user_id: &str) -> Result<()> {
        for i in 0..3 {
            let pool = self.get_pool_by_index(i);
            let query = "DELETE FROM key_shares WHERE user_id = $1";
            sqlx::query(query).bind(user_id).execute(pool).await?;
        }
        Ok(())
    }

    pub async fn user_has_shares(&self, user_id: &str) -> Result<bool> {
        let shares = self.get_all_user_shares(user_id).await?;
        Ok(shares.len() == 3) // Should have shares in all 3 databases
    }
}

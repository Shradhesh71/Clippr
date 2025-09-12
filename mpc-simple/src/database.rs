use sqlx::{PgPool, Row};
use anyhow::Result;
use std::env;
use crate::models::{KeyShare, MPCSession};

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
        // Create key_shares table
        let key_shares_query = r#"
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
            )
        "#;

        sqlx::query(key_shares_query).execute(pool).await?;

        // Create indexes for key_shares
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_key_shares_user_id ON key_shares(user_id)")
            .execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_key_shares_share_index ON key_shares(share_index)")
            .execute(pool).await?;

        // Create mpc_sessions table
        let mpc_sessions_query = r#"
            CREATE TABLE IF NOT EXISTS mpc_sessions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                session_id TEXT UNIQUE NOT NULL,
                user_id TEXT NOT NULL,
                participants TEXT[] NOT NULL,
                current_step INTEGER DEFAULT 1,
                commitments JSONB DEFAULT '{}',
                signature_shares JSONB DEFAULT '{}',
                final_signature TEXT,
                message_to_sign TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
        "#;

        sqlx::query(mpc_sessions_query).execute(pool).await?;

        // Create indexes for mpc_sessions
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mpc_sessions_session_id ON mpc_sessions(session_id)")
            .execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mpc_sessions_user_id ON mpc_sessions(user_id)")
            .execute(pool).await?;

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

    // MPC Session management methods
    pub async fn create_mpc_session(&self, session: &MPCSession) -> Result<()> {
        let pool = &self.mpc1_pool; // Use MPC1 for session coordination
        
        let query = r#"
            INSERT INTO mpc_sessions (session_id, user_id, participants, current_step, 
                                    commitments, signature_shares, final_signature, message_to_sign)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;

        sqlx::query(query)
            .bind(&session.session_id)
            .bind(&session.user_id)
            .bind(&session.participants)
            .bind(session.current_step)
            .bind(serde_json::to_value(&session.commitments).unwrap())
            .bind(serde_json::to_value(&session.signature_shares).unwrap())
            .bind(&session.final_signature)
            .bind(&session.message_to_sign)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_mpc_session(&self, session_id: &str) -> Result<Option<MPCSession>> {
        let pool = &self.mpc1_pool;
        
        let query = r#"
            SELECT id, session_id, user_id, participants, current_step, 
                   commitments, signature_shares, final_signature, message_to_sign,
                   created_at, updated_at
            FROM mpc_sessions 
            WHERE session_id = $1
        "#;

        let result = sqlx::query(query)
            .bind(session_id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = result {
            Ok(Some(MPCSession {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                user_id: row.try_get("user_id")?,
                participants: row.try_get("participants")?,
                current_step: row.try_get("current_step")?,
                commitments: serde_json::from_value(row.try_get("commitments")?).unwrap_or_default(),
                signature_shares: serde_json::from_value(row.try_get("signature_shares")?).unwrap_or_default(),
                final_signature: row.try_get("final_signature")?,
                message_to_sign: row.try_get("message_to_sign")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_mpc_session(&self, session: &MPCSession) -> Result<()> {
        let pool = &self.mpc1_pool;
        
        let query = r#"
            UPDATE mpc_sessions 
            SET current_step = $1, commitments = $2, signature_shares = $3, 
                final_signature = $4, updated_at = NOW()
            WHERE session_id = $5
        "#;

        sqlx::query(query)
            .bind(session.current_step)
            .bind(serde_json::to_value(&session.commitments).unwrap())
            .bind(serde_json::to_value(&session.signature_shares).unwrap())
            .bind(&session.final_signature)
            .bind(&session.session_id)
            .execute(pool)
            .await?;

        Ok(())
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

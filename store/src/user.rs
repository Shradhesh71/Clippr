use crate::{error::UserError, helper::{generate_token, generate_keypair}, Store};
use uuid::Uuid;
use chrono::Utc;
use sqlx::Row;
// use solana_sdk::{
//     signature::{Keypair, Signer},
//     pubkey::Pubkey,
// };

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// #[derive(Serialize)]
pub struct KeypairData {
    pub pubkey: String,
    pub secret: String,
}

impl Store {
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserError> {
        // Validate email format
        if !request.email.contains('@') {
            return Err(UserError::InvalidInput("Invalid email format".to_string()));
        }

        // Validate password length
        if request.password.len() < 6 {
            return Err(UserError::InvalidInput("Password must be at least 6 characters".to_string()));
        }

        // Check if user already exists
        let existing_user = sqlx::query("SELECT id FROM users WHERE email = $1")
            .bind(&request.email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if existing_user.is_some() {
            return Err(UserError::UserExists);
        }

        // Hash the password
        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)
            .map_err(|e| UserError::DatabaseError(format!("Password hashing failed: {}", e)))?;

        // Generate user ID and timestamp
        let user_id = Uuid::new_v4().to_string();
        let created_at = Utc::now();

        // let user_keypair = 

        // Insert user into database
        sqlx::query("INSERT INTO users (id, email, password_hash, created_at, update_at, publicKey) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&user_id)
            .bind(&request.email)
            .bind(&password_hash)
            .bind(&created_at)
            .bind(&created_at)
            .bind(&user_keypair.pubkey)
            .execute(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        // Return the created user
        let user = User {
            id: user_id,
            email: request.email,
            created_at: created_at.to_rfc3339(),
        };

        Ok(user)
    }

    pub async fn authenticate_user(&self, email: &str, password: &str) -> Result<String, UserError> {
        // Validate input
        if email.is_empty() || password.is_empty() {
            return Err(UserError::InvalidInput("Email and password cannot be empty".to_string()));
        }

        // Fetch user by email
        let user = sqlx::query("SELECT id, password_hash FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = user {
            let user_id: String = row.try_get("id").map_err(|e| UserError::DatabaseError(e.to_string()))?;
            let password_hash: String = row.try_get("password_hash").map_err(|e| UserError::DatabaseError(e.to_string()))?;

            // Verify password
            let is_valid = bcrypt::verify(password, &password_hash)
                .map_err(|e| UserError::DatabaseError(format!("Password verification failed: {}", e)))?;

            if is_valid {
                // Generate token
                let token = generate_token(&user_id)?;
                Ok(token)
            } else {
                Err(UserError::InvalidCredentials)
            }
        } else {
            Err(UserError::UserNotFound)
        }
    }

    // pub fn validate_token(&self, token: &str) -> Result<String, UserError> {
    //     // Simple token validation (in production, use proper JWT validation)
    //     if token.starts_with("token-") {
    //         let parts: Vec<&str> = token.split('-').collect();
    //         if parts.len() >= 2 {
    //             // Extract user_id from token
    //             let user_id = parts[1].to_string();
    //             Ok(user_id)
    //         } else {
    //             Err(UserError::InvalidInput("Invalid token format".to_string()))
    //         }
    //     } else {
    //         Err(UserError::InvalidInput("Invalid token".to_string()))
    //     }
    // }

    pub async fn get_user_by_id(&self, user_id: &str) -> Result<User, UserError> {
        let user = sqlx::query("SELECT id, email, created_at FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| UserError::DatabaseError(e.to_string()))?;

        if let Some(row) = user {
            let id: String = row.try_get("id").map_err(|e| UserError::DatabaseError(e.to_string()))?;
            let email: String = row.try_get("email").map_err(|e| UserError::DatabaseError(e.to_string()))?;
            let created_at: chrono::DateTime<Utc> = row.try_get("created_at").map_err(|e| UserError::DatabaseError(e.to_string()))?;

            Ok(User {
                id,
                email,
                created_at: created_at.to_rfc3339(),
            })
        } else {
            Err(UserError::UserNotFound)
        }
    }

    // pub async fn get_user_by_email(&self, email: &str) -> Result<User, UserError> {
    //     let user = sqlx::query("SELECT id, email, created_at FROM users WHERE email = $1")
    //         .bind(email)
    //         .fetch_optional(&self.pool)
    //         .await
    //         .map_err(|e| UserError::DatabaseError(e.to_string()))?;

    //     if let Some(row) = user {
    //         let id: String = row.try_get("id").map_err(|e| UserError::DatabaseError(e.to_string()))?;
    //         let email: String = row.try_get("email").map_err(|e| UserError::DatabaseError(e.to_string()))?;
    //         let created_at: chrono::DateTime<Utc> = row.try_get("created_at").map_err(|e| UserError::DatabaseError(e.to_string()))?;

    //         Ok(User {
    //             id,
    //             email,
    //             created_at: created_at.to_rfc3339(),
    //         })
    //     } else {
    //         Err(UserError::UserNotFound)
    //     }
    // }

}

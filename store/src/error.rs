
#[derive(Debug)]
pub enum UserError {
    UserExists,
    UserNotFound,
    InvalidCredentials,
    InvalidInput(String),
    DatabaseError(String),
    // Asset-related errors
    AssetNotFound,
    AssetAlreadyExists,
    // Balance-related errors
    InsufficientBalance,
    BalanceNotFound,
    // Quote-related errors
    QuoteNotFound,
    InvalidQuote,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::UserExists => write!(f, "User already exists"),
            UserError::UserNotFound => write!(f, "User not found"),
            UserError::InvalidCredentials => write!(f, "Invalid credentials"),
            UserError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            UserError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            UserError::AssetNotFound => write!(f, "Asset not found"),
            UserError::AssetAlreadyExists => write!(f, "Asset already exists"),
            UserError::InsufficientBalance => write!(f, "Insufficient balance"),
            UserError::BalanceNotFound => write!(f, "Balance not found"),
            UserError::QuoteNotFound => write!(f, "Quote not found"),
            UserError::InvalidQuote => write!(f, "Invalid quote data"),
        }
    }
}

impl std::error::Error for UserError {}
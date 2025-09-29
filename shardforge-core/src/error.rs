//! Core error types for ShardForge

use thiserror::Error;

/// Core error type for ShardForge operations
#[derive(Debug, Error)]
pub enum ShardForgeError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Storage error: {source}")]
    Storage {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Network error: {source}")]
    Network {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Consensus error: {source}")]
    Consensus {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Query error: {message}")]
    Query { message: String },

    #[error("Transaction error: {message}")]
    Transaction { message: String },

    #[error("Security error: {message}")]
    Security { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl ShardForgeError {
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config { message: message.into() }
    }

    pub fn storage<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Storage { source: Box::new(error) }
    }

    pub fn network<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Network { source: Box::new(error) }
    }

    pub fn consensus<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self::Consensus { source: Box::new(error) }
    }

    pub fn query<S: Into<String>>(message: S) -> Self {
        Self::Query { message: message.into() }
    }

    pub fn transaction<S: Into<String>>(message: S) -> Self {
        Self::Transaction { message: message.into() }
    }

    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::Security { message: message.into() }
    }

    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal { message: message.into() }
    }
}

/// Result type alias for ShardForge operations
pub type Result<T> = std::result::Result<T, ShardForgeError>;

impl From<uuid::Error> for ShardForgeError {
    fn from(error: uuid::Error) -> Self {
        ShardForgeError::internal(format!("UUID parsing error: {}", error))
    }
}

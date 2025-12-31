use thiserror::Error;

/// Storage layer errors
#[derive(Error, Debug)]
pub enum StorageError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// Database query error
    #[error("Database query error: {0}")]
    QueryError(String),

    /// Database pool error
    #[error("Database pool error: {0}")]
    PoolError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Data not found
    #[error("Data not found: {0}")]
    NotFound(String),

    /// Schema error (migrations, DDL)
    #[error("Schema error: {0}")]
    SchemaError(String),

    /// Retention policy error
    #[error("Retention policy error: {0}")]
    RetentionError(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<tokio_postgres::Error> for StorageError {
    fn from(err: tokio_postgres::Error) -> Self {
        StorageError::QueryError(err.to_string())
    }
}

impl From<deadpool_postgres::PoolError> for StorageError {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        StorageError::PoolError(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

impl From<serde_yaml::Error> for StorageError {
    fn from(err: serde_yaml::Error) -> Self {
        StorageError::ConfigError(err.to_string())
    }
}

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;

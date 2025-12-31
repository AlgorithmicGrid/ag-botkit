//! Error types for the strategy engine

use thiserror::Error;

/// Main error type for strategy operations
#[derive(Error, Debug)]
pub enum StrategyError {
    /// Risk engine rejected the action
    #[error("Risk rejected: {policies:?}")]
    RiskRejected {
        policies: Vec<String>,
    },

    /// Execution engine error
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Order not found
    #[error("Order not found: {0}")]
    OrderNotFound(String),

    /// Market not found
    #[error("Market not found: {0}")]
    MarketNotFound(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Strategy initialization failed
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// Strategy is not initialized
    #[error("Strategy not initialized")]
    NotInitialized,

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// YAML parsing error
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Insufficient data for calculation
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    /// Signal generation error
    #[error("Signal generation error: {0}")]
    SignalError(String),

    /// Backtesting error
    #[error("Backtest error: {0}")]
    BacktestError(String),

    /// Generic error
    #[error("Strategy error: {0}")]
    Other(String),
}

/// Result type for strategy operations
pub type StrategyResult<T> = Result<T, StrategyError>;

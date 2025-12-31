//! Error types for the execution gateway

use thiserror::Error;

use crate::order::OrderId;

/// Result type for execution operations
pub type ExecResult<T> = Result<T, ExecError>;

/// Execution gateway error types
#[derive(Debug, Error)]
pub enum ExecError {
    /// Order validation failed
    #[error("Order validation failed: {0}")]
    ValidationError(String),

    /// Risk check rejected the order
    #[error("Risk check rejected order: {policies:?}")]
    RiskRejected {
        /// List of violated risk policies
        policies: Vec<String>,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded for venue {venue}: {message}")]
    RateLimitExceeded {
        /// Venue identifier
        venue: String,
        /// Error message
        message: String,
    },

    /// Venue error
    #[error("Venue error from {venue}: {message}")]
    VenueError {
        /// Venue identifier
        venue: String,
        /// Error message
        message: String,
        /// Optional error code from venue
        code: Option<String>,
    },

    /// Order not found
    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    /// Venue not supported
    #[error("Venue not supported: {0}")]
    VenueNotSupported(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// API authentication error
    #[error("API authentication failed: {0}")]
    AuthenticationError(String),

    /// Invalid API response
    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    /// Order is in invalid state for operation
    #[error("Order {order_id} in invalid state {current_state} for {operation}")]
    InvalidOrderState {
        /// Order ID
        order_id: OrderId,
        /// Current state
        current_state: String,
        /// Operation attempted
        operation: String,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl ExecError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ExecError::NetworkError(_)
                | ExecError::Timeout(_)
                | ExecError::RateLimitExceeded { .. }
                | ExecError::HttpError(_)
        )
    }

    /// Check if error is due to rate limiting
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, ExecError::RateLimitExceeded { .. })
    }

    /// Check if error is due to risk rejection
    pub fn is_risk_rejection(&self) -> bool {
        matches!(self, ExecError::RiskRejected { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        let network_err = ExecError::NetworkError("connection lost".to_string());
        assert!(network_err.is_retryable());

        let validation_err = ExecError::ValidationError("invalid price".to_string());
        assert!(!validation_err.is_retryable());

        let rate_limit_err = ExecError::RateLimitExceeded {
            venue: "polymarket".to_string(),
            message: "too many requests".to_string(),
        };
        assert!(rate_limit_err.is_retryable());
        assert!(rate_limit_err.is_rate_limit());
    }

    #[test]
    fn test_risk_rejection() {
        let risk_err = ExecError::RiskRejected {
            policies: vec!["PositionLimit".to_string()],
        };
        assert!(risk_err.is_risk_rejection());
        assert!(!risk_err.is_retryable());
    }
}

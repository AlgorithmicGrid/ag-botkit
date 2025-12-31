//! Error types for advanced risk models

use thiserror::Error;

/// Errors that can occur in advanced risk calculations
#[derive(Error, Debug)]
pub enum AdvancedRiskError {
    #[error("Insufficient data: {0}")]
    InsufficientData(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Calculation error: {0}")]
    CalculationError(String),

    #[error("Numerical instability: {0}")]
    NumericalInstability(String),

    #[error("Matrix operation failed: {0}")]
    MatrixError(String),

    #[error("Invalid confidence level: {0} (must be between 0 and 1)")]
    InvalidConfidenceLevel(f64),

    #[error("Invalid time horizon: {0} (must be positive)")]
    InvalidTimeHorizon(u32),

    #[error("Division by zero in calculation: {0}")]
    DivisionByZero(String),

    #[error("Negative volatility not allowed")]
    NegativeVolatility,

    #[error("Negative price not allowed")]
    NegativePrice,

    #[error("Option pricing error: {0}")]
    OptionPricingError(String),
}

pub type Result<T> = std::result::Result<T, AdvancedRiskError>;

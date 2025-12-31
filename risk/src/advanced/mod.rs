//! # Advanced Risk Models
//!
//! This module provides quantitative risk models extending the base policy-based
//! risk engine. Includes VaR calculation, Greeks analytics, portfolio risk metrics,
//! stress testing, and performance evaluation.
//!
//! ## Modules
//!
//! - `var`: Value at Risk (Historical, Parametric, Monte Carlo, CVaR)
//! - `greeks`: Options Greeks calculation (Delta, Gamma, Vega, Theta, Rho)
//! - `portfolio`: Portfolio analytics and risk decomposition
//! - `stress`: Stress testing and scenario analysis
//! - `metrics`: Performance metrics (Sharpe, Sortino, drawdown)
//! - `error`: Advanced risk error types

mod error;
mod var;
mod greeks;
mod portfolio;
mod stress;
mod metrics;

pub use error::AdvancedRiskError;
pub use var::{VarEngine, VarConfig, VarResult, VarMethod, VarBacktestResult};
pub use greeks::{GreeksEngine, GreeksConfig, Greeks, PortfolioGreeks, HedgeRecommendation};
pub use portfolio::{PortfolioAnalyzer, PortfolioConfig, RiskContribution, MarginalVarResult};
pub use stress::{StressTestEngine, StressScenario, StressTestResult, StressTestReport};
pub use metrics::PerformanceMetrics;

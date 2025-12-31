//! # ag-risk: Policy-based Risk Engine for Polymarket
//!
//! This library provides risk management infrastructure for trading systems,
//! with specific support for Polymarket YES/NO binary outcome markets.
//!
//! ## Core Components
//!
//! - **RiskEngine**: Policy evaluation engine for trading decisions
//! - **PolymarketSimulator**: Position and PnL tracking for binary markets
//! - **Policy System**: Flexible YAML/JSON-based risk policies
//!
//! ## Example Usage
//!
//! ```rust
//! use ag_risk::{RiskEngine, RiskContext, PolymarketSimulator};
//!
//! // Load risk policies from YAML
//! let yaml = r#"
//! policies:
//!   - type: PositionLimit
//!     market_id: "0x123abc"
//!     max_size: 1000.0
//!   - type: InventoryLimit
//!     max_value_usd: 10000.0
//! "#;
//!
//! let engine = RiskEngine::from_yaml(yaml).unwrap();
//!
//! // Evaluate a trade
//! let ctx = RiskContext {
//!     market_id: "0x123abc".to_string(),
//!     current_position: 500.0,
//!     proposed_size: 600.0,
//!     inventory_value_usd: 5000.0,
//! };
//!
//! let decision = engine.evaluate(&ctx);
//! assert!(!decision.allowed); // Would exceed position limit
//! ```

mod policy;
mod engine;
mod simulator;

pub use policy::{PolicyRule, RiskPolicyConfig};
pub use engine::RiskEngine;
pub use simulator::PolymarketSimulator;

use serde::{Deserialize, Serialize};

/// Context information for risk evaluation
///
/// This structure contains all necessary information to evaluate
/// whether a proposed trading action should be allowed.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskContext {
    /// Market identifier (e.g., "0x123abc")
    pub market_id: String,

    /// Current position size (positive = long, negative = short)
    pub current_position: f64,

    /// Proposed additional size (signed: +long, -short)
    pub proposed_size: f64,

    /// Total inventory value in USD
    pub inventory_value_usd: f64,
}

/// Result of risk evaluation
///
/// Contains the decision (allowed/rejected) and details about
/// which policies were violated, if any.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskDecision {
    /// Whether the action is allowed
    pub allowed: bool,

    /// List of policy names that were violated
    pub violated_policies: Vec<String>,
}

impl RiskDecision {
    /// Create a decision that allows the action
    pub fn allow() -> Self {
        Self {
            allowed: true,
            violated_policies: Vec::new(),
        }
    }

    /// Create a decision that rejects the action
    pub fn reject(violated_policies: Vec<String>) -> Self {
        Self {
            allowed: false,
            violated_policies,
        }
    }
}

/// Risk action types for different violation severities
///
/// This enum represents the action to take based on risk evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskAction {
    /// Allow the action to proceed
    Allow,

    /// Reject this specific action
    Reject,

    /// Emergency stop - halt all trading
    KillSwitch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_context_creation() {
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 100.0,
            proposed_size: 50.0,
            inventory_value_usd: 1000.0,
        };

        assert_eq!(ctx.market_id, "0x123");
        assert_eq!(ctx.current_position, 100.0);
    }

    #[test]
    fn test_risk_decision_allow() {
        let decision = RiskDecision::allow();
        assert!(decision.allowed);
        assert!(decision.violated_policies.is_empty());
    }

    #[test]
    fn test_risk_decision_reject() {
        let decision = RiskDecision::reject(vec!["PositionLimit".to_string()]);
        assert!(!decision.allowed);
        assert_eq!(decision.violated_policies.len(), 1);
        assert_eq!(decision.violated_policies[0], "PositionLimit");
    }
}

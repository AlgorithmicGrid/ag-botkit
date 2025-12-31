//! Risk evaluation engine
//!
//! This module implements the core risk evaluation logic that checks
//! trading decisions against loaded policies.

use crate::policy::{PolicyRule, RiskPolicyConfig};
use crate::{RiskContext, RiskDecision};
use std::sync::RwLock;

/// Risk evaluation engine
///
/// The RiskEngine loads policies and evaluates trading decisions
/// against them. It maintains state for the kill-switch.
pub struct RiskEngine {
    config: RiskPolicyConfig,
    kill_switch_active: RwLock<bool>,
}

impl RiskEngine {
    /// Create a new RiskEngine from a configuration
    pub fn new(config: RiskPolicyConfig) -> Self {
        Self {
            config,
            kill_switch_active: RwLock::new(false),
        }
    }

    /// Load policies from YAML string
    ///
    /// # Example
    ///
    /// ```
    /// use ag_risk::RiskEngine;
    ///
    /// let yaml = r#"
    /// policies:
    ///   - type: PositionLimit
    ///     max_size: 1000.0
    /// "#;
    ///
    /// let engine = RiskEngine::from_yaml(yaml).unwrap();
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        let config: RiskPolicyConfig =
            serde_yaml::from_str(yaml).map_err(|e| format!("Failed to parse YAML: {}", e))?;
        Ok(Self::new(config))
    }

    /// Load policies from JSON string
    ///
    /// # Example
    ///
    /// ```
    /// use ag_risk::RiskEngine;
    ///
    /// let json = r#"{
    ///   "policies": [
    ///     {
    ///       "type": "InventoryLimit",
    ///       "max_value_usd": 10000.0
    ///     }
    ///   ]
    /// }"#;
    ///
    /// let engine = RiskEngine::from_json(json).unwrap();
    /// ```
    pub fn from_json(json: &str) -> Result<Self, String> {
        let config: RiskPolicyConfig =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))?;
        Ok(Self::new(config))
    }

    /// Evaluate if an action is allowed based on current policies
    ///
    /// This is the core evaluation function that checks the provided context
    /// against all loaded policies.
    ///
    /// # Example
    ///
    /// ```
    /// use ag_risk::{RiskEngine, RiskContext};
    ///
    /// let yaml = r#"
    /// policies:
    ///   - type: PositionLimit
    ///     max_size: 1000.0
    /// "#;
    ///
    /// let engine = RiskEngine::from_yaml(yaml).unwrap();
    /// let ctx = RiskContext {
    ///     market_id: "0x123".to_string(),
    ///     current_position: 500.0,
    ///     proposed_size: 600.0,
    ///     inventory_value_usd: 5000.0,
    /// };
    ///
    /// let decision = engine.evaluate(&ctx);
    /// assert!(!decision.allowed); // Would result in 1100.0, exceeds limit of 1000.0
    /// ```
    pub fn evaluate(&self, ctx: &RiskContext) -> RiskDecision {
        let mut violated_policies = Vec::new();

        // Check if kill-switch is active
        if *self.kill_switch_active.read().unwrap() {
            violated_policies.push("KillSwitch (active)".to_string());
            return RiskDecision::reject(violated_policies);
        }

        // Evaluate each policy
        for policy in &self.config.policies {
            // Skip policies that don't apply to this market
            if !policy.applies_to_market(&ctx.market_id) {
                continue;
            }

            // Evaluate policy
            if let Some(violation) = self.evaluate_policy(policy, ctx) {
                violated_policies.push(violation);
            }
        }

        // Return decision
        if violated_policies.is_empty() {
            RiskDecision::allow()
        } else {
            RiskDecision::reject(violated_policies)
        }
    }

    /// Trigger the kill-switch, blocking all future trades
    pub fn trigger_kill_switch(&self) {
        *self.kill_switch_active.write().unwrap() = true;
    }

    /// Reset the kill-switch, allowing trades again
    pub fn reset_kill_switch(&self) {
        *self.kill_switch_active.write().unwrap() = false;
    }

    /// Check if kill-switch is currently active
    pub fn is_kill_switch_active(&self) -> bool {
        *self.kill_switch_active.read().unwrap()
    }

    /// Evaluate a single policy against the context
    ///
    /// Returns Some(violation_message) if policy is violated, None otherwise
    fn evaluate_policy(&self, policy: &PolicyRule, ctx: &RiskContext) -> Option<String> {
        match policy {
            PolicyRule::PositionLimit { market_id, max_size } => {
                let new_position = ctx.current_position + ctx.proposed_size;
                if new_position.abs() > *max_size {
                    let market_str = market_id
                        .as_ref()
                        .map(|m| format!(" (market: {})", m))
                        .unwrap_or_default();
                    Some(format!(
                        "PositionLimit{}: new position {:.2} exceeds max {:.2}",
                        market_str, new_position.abs(), max_size
                    ))
                } else {
                    None
                }
            }
            PolicyRule::InventoryLimit { max_value_usd } => {
                if ctx.inventory_value_usd > *max_value_usd {
                    Some(format!(
                        "InventoryLimit: inventory {:.2} USD exceeds max {:.2} USD",
                        ctx.inventory_value_usd, max_value_usd
                    ))
                } else {
                    None
                }
            }
            PolicyRule::KillSwitch { enabled } => {
                if *enabled {
                    Some("KillSwitch: enabled in policy".to_string())
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_limit_global() {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        // Should allow within limit
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 500.0,
            proposed_size: 400.0,
            inventory_value_usd: 5000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(decision.allowed);

        // Should reject when exceeding limit
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 800.0,
            proposed_size: 300.0,
            inventory_value_usd: 5000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed);
        assert_eq!(decision.violated_policies.len(), 1);
        assert!(decision.violated_policies[0].contains("PositionLimit"));
    }

    #[test]
    fn test_position_limit_per_market() {
        let yaml = r#"
policies:
  - type: PositionLimit
    market_id: "0x123"
    max_size: 500.0
  - type: PositionLimit
    market_id: "0x456"
    max_size: 1000.0
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        // Should apply 500 limit to 0x123
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 400.0,
            proposed_size: 200.0,
            inventory_value_usd: 5000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed);

        // Should apply 1000 limit to 0x456
        let ctx = RiskContext {
            market_id: "0x456".to_string(),
            current_position: 400.0,
            proposed_size: 200.0,
            inventory_value_usd: 5000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(decision.allowed);
    }

    #[test]
    fn test_inventory_limit() {
        let yaml = r#"
policies:
  - type: InventoryLimit
    max_value_usd: 10000.0
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        // Should allow within limit
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 500.0,
            proposed_size: 100.0,
            inventory_value_usd: 8000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(decision.allowed);

        // Should reject when exceeding limit
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 500.0,
            proposed_size: 100.0,
            inventory_value_usd: 12000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed);
        assert!(decision.violated_policies[0].contains("InventoryLimit"));
    }

    #[test]
    fn test_kill_switch_in_policy() {
        let yaml = r#"
policies:
  - type: KillSwitch
    enabled: true
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 100.0,
            proposed_size: 50.0,
            inventory_value_usd: 1000.0,
        };

        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed);
        assert!(decision.violated_policies[0].contains("KillSwitch"));
    }

    #[test]
    fn test_kill_switch_trigger() {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 10000.0
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 100.0,
            proposed_size: 50.0,
            inventory_value_usd: 1000.0,
        };

        // Should allow initially
        assert!(engine.evaluate(&ctx).allowed);

        // Trigger kill-switch
        engine.trigger_kill_switch();
        assert!(engine.is_kill_switch_active());

        // Should now reject
        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed);
        assert!(decision.violated_policies[0].contains("KillSwitch (active)"));

        // Reset kill-switch
        engine.reset_kill_switch();
        assert!(!engine.is_kill_switch_active());

        // Should allow again
        assert!(engine.evaluate(&ctx).allowed);
    }

    #[test]
    fn test_multiple_violations() {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 100.0
  - type: InventoryLimit
    max_value_usd: 500.0
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 80.0,
            proposed_size: 50.0,
            inventory_value_usd: 1000.0,
        };

        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed);
        assert_eq!(decision.violated_policies.len(), 2);
    }

    #[test]
    fn test_negative_positions() {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#;
        let engine = RiskEngine::from_yaml(yaml).unwrap();

        // Short position
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: -800.0,
            proposed_size: -300.0,
            inventory_value_usd: 5000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(!decision.allowed); // |-1100| > 1000

        // Reducing short position
        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: -800.0,
            proposed_size: 200.0,
            inventory_value_usd: 5000.0,
        };
        let decision = engine.evaluate(&ctx);
        assert!(decision.allowed); // |-600| < 1000
    }

    #[test]
    fn test_from_json() {
        let json = r#"{
  "policies": [
    {
      "type": "PositionLimit",
      "max_size": 2000.0
    }
  ]
}"#;
        let engine = RiskEngine::from_json(json).unwrap();

        let ctx = RiskContext {
            market_id: "0x123".to_string(),
            current_position: 1000.0,
            proposed_size: 500.0,
            inventory_value_usd: 5000.0,
        };

        let decision = engine.evaluate(&ctx);
        assert!(decision.allowed);
    }

    #[test]
    fn test_invalid_yaml() {
        let yaml = "invalid: {yaml: [structure";
        let result = RiskEngine::from_yaml(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json() {
        let json = "{invalid json}";
        let result = RiskEngine::from_json(json);
        assert!(result.is_err());
    }
}

//! Risk policy definitions and configuration
//!
//! This module defines the policy types and configuration structures
//! for the risk management system.

use serde::{Deserialize, Serialize};

/// Complete risk policy configuration
///
/// This structure represents a full risk policy document,
/// typically loaded from YAML or JSON files.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RiskPolicyConfig {
    /// List of policy rules to evaluate
    pub policies: Vec<PolicyRule>,
}

/// Individual policy rule types
///
/// Each variant represents a different type of risk check
/// with specific parameters.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum PolicyRule {
    /// Limit maximum position size
    ///
    /// Can be applied globally (market_id = None) or per-market.
    /// Checks that |current_position + proposed_size| <= max_size
    PositionLimit {
        /// Optional market ID filter (None = apply to all markets)
        #[serde(skip_serializing_if = "Option::is_none")]
        market_id: Option<String>,

        /// Maximum absolute position size
        max_size: f64,
    },

    /// Limit total inventory value
    ///
    /// Checks that total inventory value in USD does not exceed limit.
    InventoryLimit {
        /// Maximum inventory value in USD
        max_value_usd: f64,
    },

    /// Emergency kill switch
    ///
    /// When enabled, blocks all trading activity.
    KillSwitch {
        /// Whether kill switch is enabled
        enabled: bool,
    },
}

impl PolicyRule {
    /// Get a human-readable name for this policy type
    pub fn name(&self) -> &'static str {
        match self {
            PolicyRule::PositionLimit { .. } => "PositionLimit",
            PolicyRule::InventoryLimit { .. } => "InventoryLimit",
            PolicyRule::KillSwitch { .. } => "KillSwitch",
        }
    }

    /// Check if this policy applies to the given market ID
    pub fn applies_to_market(&self, market_id: &str) -> bool {
        match self {
            PolicyRule::PositionLimit {
                market_id: Some(policy_market_id),
                ..
            } => policy_market_id == market_id,
            PolicyRule::PositionLimit { market_id: None, .. } => true,
            PolicyRule::InventoryLimit { .. } => true,
            PolicyRule::KillSwitch { .. } => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_deserialization_yaml() {
        let yaml = r#"
policies:
  - type: PositionLimit
    market_id: "0x123abc"
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 10000.0
  - type: KillSwitch
    enabled: false
"#;

        let config: RiskPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.policies.len(), 3);

        match &config.policies[0] {
            PolicyRule::PositionLimit { market_id, max_size } => {
                assert_eq!(market_id.as_ref().unwrap(), "0x123abc");
                assert_eq!(*max_size, 1000.0);
            }
            _ => panic!("Expected PositionLimit"),
        }

        match &config.policies[1] {
            PolicyRule::InventoryLimit { max_value_usd } => {
                assert_eq!(*max_value_usd, 10000.0);
            }
            _ => panic!("Expected InventoryLimit"),
        }

        match &config.policies[2] {
            PolicyRule::KillSwitch { enabled } => {
                assert!(!enabled);
            }
            _ => panic!("Expected KillSwitch"),
        }
    }

    #[test]
    fn test_policy_deserialization_json() {
        let json = r#"{
  "policies": [
    {
      "type": "PositionLimit",
      "max_size": 500.0
    },
    {
      "type": "InventoryLimit",
      "max_value_usd": 5000.0
    }
  ]
}"#;

        let config: RiskPolicyConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.policies.len(), 2);
    }

    #[test]
    fn test_policy_serialization_yaml() {
        let config = RiskPolicyConfig {
            policies: vec![
                PolicyRule::PositionLimit {
                    market_id: Some("0x456".to_string()),
                    max_size: 2000.0,
                },
                PolicyRule::KillSwitch { enabled: true },
            ],
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("PositionLimit"));
        assert!(yaml.contains("KillSwitch"));
    }

    #[test]
    fn test_policy_name() {
        let pos_limit = PolicyRule::PositionLimit {
            market_id: None,
            max_size: 100.0,
        };
        assert_eq!(pos_limit.name(), "PositionLimit");

        let inv_limit = PolicyRule::InventoryLimit {
            max_value_usd: 1000.0,
        };
        assert_eq!(inv_limit.name(), "InventoryLimit");

        let kill_switch = PolicyRule::KillSwitch { enabled: false };
        assert_eq!(kill_switch.name(), "KillSwitch");
    }

    #[test]
    fn test_applies_to_market() {
        let global_limit = PolicyRule::PositionLimit {
            market_id: None,
            max_size: 100.0,
        };
        assert!(global_limit.applies_to_market("any_market"));

        let market_limit = PolicyRule::PositionLimit {
            market_id: Some("0x123".to_string()),
            max_size: 100.0,
        };
        assert!(market_limit.applies_to_market("0x123"));
        assert!(!market_limit.applies_to_market("0x456"));

        let inv_limit = PolicyRule::InventoryLimit {
            max_value_usd: 1000.0,
        };
        assert!(inv_limit.applies_to_market("any_market"));
    }

    #[test]
    fn test_global_position_limit_no_market_id() {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 500.0
"#;

        let config: RiskPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        match &config.policies[0] {
            PolicyRule::PositionLimit { market_id, max_size } => {
                assert!(market_id.is_none());
                assert_eq!(*max_size, 500.0);
            }
            _ => panic!("Expected PositionLimit"),
        }
    }
}

//! Integration tests for the risk management system
//!
//! These tests verify end-to-end functionality including policy loading,
//! risk evaluation, and simulator integration.

use ag_risk::{PolymarketSimulator, RiskContext, RiskEngine};
use std::fs;

#[test]
fn test_load_example_policy() {
    let policy_path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/example_policy.yaml");
    let yaml = fs::read_to_string(policy_path).expect("Failed to read example policy");

    let engine = RiskEngine::from_yaml(&yaml).expect("Failed to parse policy");

    let ctx = RiskContext {
        market_id: "0x123abc".to_string(),
        current_position: 400.0,
        proposed_size: 50.0,
        inventory_value_usd: 5000.0,
    };

    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);
}

#[test]
fn test_conservative_policy() {
    let policy_path = concat!(env!("CARGO_MANIFEST_DIR"), "/policies/conservative.yaml");
    let yaml = fs::read_to_string(policy_path).expect("Failed to read conservative policy");

    let engine = RiskEngine::from_yaml(&yaml).unwrap();

    // Should reject large position
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 400.0,
        proposed_size: 200.0,
        inventory_value_usd: 3000.0,
    };

    let decision = engine.evaluate(&ctx);
    assert!(!decision.allowed);
    assert!(decision.violated_policies[0].contains("PositionLimit"));
}

#[test]
fn test_aggressive_policy() {
    let policy_path = concat!(env!("CARGO_MANIFEST_DIR"), "/policies/aggressive.yaml");
    let yaml = fs::read_to_string(policy_path).expect("Failed to read aggressive policy");

    let engine = RiskEngine::from_yaml(&yaml).unwrap();

    // Should allow large position
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 2000.0,
        proposed_size: 2000.0,
        inventory_value_usd: 20000.0,
    };

    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);
}

#[test]
fn test_multi_market_policy() {
    let policy_path = concat!(env!("CARGO_MANIFEST_DIR"), "/policies/multi_market.yaml");
    let yaml = fs::read_to_string(policy_path).expect("Failed to read multi-market policy");

    let engine = RiskEngine::from_yaml(&yaml).unwrap();

    // High-confidence market: should allow larger position
    let ctx = RiskContext {
        market_id: "0xhigh_confidence".to_string(),
        current_position: 1500.0,
        proposed_size: 400.0,
        inventory_value_usd: 10000.0,
    };
    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);

    // Low-confidence market: should reject large position
    let ctx = RiskContext {
        market_id: "0xlow_confidence".to_string(),
        current_position: 150.0,
        proposed_size: 100.0,
        inventory_value_usd: 10000.0,
    };
    let decision = engine.evaluate(&ctx);
    assert!(!decision.allowed);
}

#[test]
fn test_simulator_with_risk_engine() {
    let mut sim = PolymarketSimulator::new();
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 5000.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    // Simulate a trading sequence
    sim.update_position("0x123", 500.0, 0.55);

    // Check if we can add more
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: sim.get_position("0x123"),
        proposed_size: 300.0,
        inventory_value_usd: sim.get_inventory_value_usd(),
    };

    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);

    // Apply the trade
    sim.update_position("0x123", 300.0, 0.60);

    // Try to add even more (should violate)
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: sim.get_position("0x123"),
        proposed_size: 300.0,
        inventory_value_usd: sim.get_inventory_value_usd(),
    };

    let decision = engine.evaluate(&ctx);
    assert!(!decision.allowed);
}

#[test]
fn test_kill_switch_workflow() {
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 10000.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 500.0,
        proposed_size: 300.0,
        inventory_value_usd: 5000.0,
    };

    // Should allow initially
    assert!(engine.evaluate(&ctx).allowed);

    // Trigger kill-switch
    engine.trigger_kill_switch();

    // Should now block
    let decision = engine.evaluate(&ctx);
    assert!(!decision.allowed);
    assert!(decision.violated_policies[0].contains("KillSwitch"));

    // Reset
    engine.reset_kill_switch();

    // Should allow again
    assert!(engine.evaluate(&ctx).allowed);
}

#[test]
fn test_inventory_limit_across_markets() {
    let mut sim = PolymarketSimulator::new();
    let yaml = r#"
policies:
  - type: InventoryLimit
    max_value_usd: 1000.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    // Build up inventory across multiple markets
    sim.update_position("0x123", 500.0, 0.60); // 300 USD
    sim.update_position("0x456", 1000.0, 0.50); // 500 USD
    // Total: 800 USD

    let ctx = RiskContext {
        market_id: "0x789".to_string(),
        current_position: 0.0,
        proposed_size: 500.0, // Would add 250-500 USD depending on price
        inventory_value_usd: sim.get_inventory_value_usd(),
    };

    // Should still allow (under 1000)
    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);

    // Add more inventory
    sim.update_position("0x789", 500.0, 0.80); // 400 USD
    // Total: 1200 USD

    let ctx = RiskContext {
        market_id: "0xabc".to_string(),
        current_position: 0.0,
        proposed_size: 100.0,
        inventory_value_usd: sim.get_inventory_value_usd(),
    };

    // Should reject (over 1000)
    let decision = engine.evaluate(&ctx);
    assert!(!decision.allowed);
}

#[test]
fn test_position_reduction_allowed() {
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 500.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    // Current position is at max
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 500.0,
        proposed_size: -200.0, // Reducing position
        inventory_value_usd: 2500.0,
    };

    // Should allow reduction
    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);
}

#[test]
fn test_boundary_conditions() {
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    // Exactly at limit
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 999.0,
        proposed_size: 1.0,
        inventory_value_usd: 5000.0,
    };
    assert!(engine.evaluate(&ctx).allowed);

    // Just over limit
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 999.0,
        proposed_size: 1.1,
        inventory_value_usd: 5000.0,
    };
    assert!(!engine.evaluate(&ctx).allowed);
}

#[test]
fn test_simulator_pnl_tracking() {
    let mut sim = PolymarketSimulator::new();

    // Buy position
    sim.update_position("0x123", 1000.0, 0.50);
    assert_eq!(sim.get_avg_price("0x123"), 0.50);

    // Price moves favorably
    sim.update_position("0x123", 0.0, 0.60);
    let pnl = sim.get_unrealized_pnl("0x123");

    // PnL = (1000 * 0.60) - (1000 * 0.50) = 100
    assert_eq!(pnl, 100.0);

    // Partial exit at profit
    sim.update_position("0x123", -500.0, 0.65);
    assert_eq!(sim.get_position("0x123"), 500.0);

    // Remaining PnL should still be positive
    let remaining_pnl = sim.get_unrealized_pnl("0x123");
    assert!(remaining_pnl > 0.0);
}

#[test]
fn test_zero_size_trades() {
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    // Zero-size trade (price update only)
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 500.0,
        proposed_size: 0.0,
        inventory_value_usd: 2500.0,
    };

    let decision = engine.evaluate(&ctx);
    assert!(decision.allowed);
}

#[test]
fn test_policy_priority() {
    // If multiple policies fail, all should be reported
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 100.0
  - type: InventoryLimit
    max_value_usd: 50.0
"#;

    let engine = RiskEngine::from_yaml(yaml).unwrap();

    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: 90.0,
        proposed_size: 50.0,
        inventory_value_usd: 100.0,
    };

    let decision = engine.evaluate(&ctx);
    assert!(!decision.allowed);
    // Both policies should be violated
    assert!(decision.violated_policies.len() >= 2);
}

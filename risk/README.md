# ag-risk: Policy-Based Risk Engine for Polymarket

A comprehensive Rust library providing policy-based risk management and market simulation for Polymarket binary outcome markets.

## Features

- **Policy-Based Risk Evaluation**: Define risk limits using YAML/JSON configuration files
- **Polymarket Simulator**: Track positions, PnL, and inventory across multiple markets
- **Multiple Policy Types**: Position limits, inventory limits, and emergency kill-switch
- **Flexible Configuration**: Global and per-market policy rules
- **Zero Allocation**: Efficient evaluation suitable for high-frequency trading
- **Thread-Safe**: Kill-switch state protected by RwLock

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ag-risk = { path = "../risk" }
```

## Quick Start

### Loading Policies

```rust
use ag_risk::{RiskEngine, RiskContext};

// Load from YAML
let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 10000.0
"#;

let engine = RiskEngine::from_yaml(yaml).unwrap();

// Or load from JSON
let json = r#"{
  "policies": [
    {
      "type": "PositionLimit",
      "max_size": 1000.0
    }
  ]
}"#;

let engine = RiskEngine::from_json(json).unwrap();
```

### Evaluating Risk

```rust
use ag_risk::RiskContext;

let ctx = RiskContext {
    market_id: "0x123abc".to_string(),
    current_position: 500.0,
    proposed_size: 600.0,
    inventory_value_usd: 5000.0,
};

let decision = engine.evaluate(&ctx);

if decision.allowed {
    println!("Trade approved");
} else {
    println!("Trade rejected: {:?}", decision.violated_policies);
}
```

### Using the Simulator

```rust
use ag_risk::PolymarketSimulator;

let mut sim = PolymarketSimulator::new();

// Apply trades
sim.update_position("0x123", 100.0, 0.55);  // Buy 100 @ 0.55
sim.update_position("0x456", 200.0, 0.40);  // Buy 200 @ 0.40

// Check positions
assert_eq!(sim.get_position("0x123"), 100.0);
assert_eq!(sim.get_avg_price("0x123"), 0.55);

// Calculate PnL
sim.update_position("0x123", 0.0, 0.60);  // Price update
let pnl = sim.get_unrealized_pnl("0x123");
println!("Unrealized PnL: ${:.2}", pnl);

// Get total inventory
let total_inventory = sim.get_inventory_value_usd();
println!("Total inventory: ${:.2}", total_inventory);
```

### Kill-Switch

```rust
// Trigger emergency stop
engine.trigger_kill_switch();

// All trades will be rejected while active
let decision = engine.evaluate(&ctx);
assert!(!decision.allowed);

// Reset when safe
engine.reset_kill_switch();
```

## Policy Types

### PositionLimit

Limits the maximum absolute position size for a market.

**Global Limit:**
```yaml
policies:
  - type: PositionLimit
    max_size: 1000.0
```

**Per-Market Limit:**
```yaml
policies:
  - type: PositionLimit
    market_id: "0x123abc"
    max_size: 500.0
```

**Evaluation Logic:**
- Checks: `|current_position + proposed_size| <= max_size`
- Applies to both long and short positions
- Per-market limits override global limits when both are present

### InventoryLimit

Limits total inventory value across all markets.

```yaml
policies:
  - type: InventoryLimit
    max_value_usd: 10000.0
```

**Evaluation Logic:**
- Checks: `sum(|position_i * price_i|) <= max_value_usd`
- Aggregates across all markets
- Uses absolute values (both longs and shorts count toward limit)

### KillSwitch

Emergency stop that blocks all trading when enabled.

```yaml
policies:
  - type: KillSwitch
    enabled: false
```

**Programmatic Control:**
```rust
engine.trigger_kill_switch();  // Block all trades
engine.reset_kill_switch();    // Resume trading
```

**Evaluation Logic:**
- When enabled: rejects all trades immediately
- Can be triggered via policy config or programmatically
- Highest priority (evaluated first)

## API Reference

### RiskEngine

#### Constructors

- `RiskEngine::from_yaml(yaml: &str) -> Result<Self, String>`
  - Load policies from YAML string
  - Returns error if YAML is malformed

- `RiskEngine::from_json(json: &str) -> Result<Self, String>`
  - Load policies from JSON string
  - Returns error if JSON is malformed

#### Methods

- `evaluate(&self, ctx: &RiskContext) -> RiskDecision`
  - Evaluate whether a trade should be allowed
  - Returns decision with violation details

- `trigger_kill_switch(&self)`
  - Activate emergency stop

- `reset_kill_switch(&self)`
  - Deactivate emergency stop

- `is_kill_switch_active(&self) -> bool`
  - Check kill-switch state

### RiskContext

```rust
pub struct RiskContext {
    pub market_id: String,           // Market identifier
    pub current_position: f64,       // Current position (signed)
    pub proposed_size: f64,          // Proposed trade size (signed)
    pub inventory_value_usd: f64,    // Total inventory value
}
```

### RiskDecision

```rust
pub struct RiskDecision {
    pub allowed: bool,                    // Whether trade is allowed
    pub violated_policies: Vec<String>,   // List of violations
}
```

### PolymarketSimulator

#### Constructors

- `PolymarketSimulator::new() -> Self`
  - Create empty simulator

#### Methods

- `update_position(&mut self, market_id: &str, size: f64, price: f64)`
  - Apply a fill (positive = buy, negative = sell)
  - Updates position, average price, and invested capital

- `get_position(&self, market_id: &str) -> f64`
  - Get current position for market (0.0 if none)

- `get_avg_price(&self, market_id: &str) -> f64`
  - Get average entry price

- `get_unrealized_pnl(&self, market_id: &str) -> f64`
  - Calculate unrealized PnL: `(size * current_price) - invested_capital`

- `get_inventory_value_usd(&self) -> f64`
  - Sum of absolute position values: `sum(|size_i * price_i|)`

- `get_total_pnl(&self) -> f64`
  - Sum of unrealized PnL across all markets

- `reset(&mut self)`
  - Clear all positions

- `get_active_markets(&self) -> Vec<String>`
  - List of markets with non-zero positions

- `get_position_details(&self, market_id: &str) -> Option<PositionDetails>`
  - Detailed position information

## Example Workflows

### Conservative Trading

```rust
use ag_risk::{RiskEngine, RiskContext, PolymarketSimulator};

// Load conservative policy
let yaml = std::fs::read_to_string("policies/conservative.yaml").unwrap();
let engine = RiskEngine::from_yaml(&yaml).unwrap();

let mut sim = PolymarketSimulator::new();

// Trading loop
loop {
    // Receive market data...
    let proposed_trade = 100.0;

    // Check risk
    let ctx = RiskContext {
        market_id: "0x123".to_string(),
        current_position: sim.get_position("0x123"),
        proposed_size: proposed_trade,
        inventory_value_usd: sim.get_inventory_value_usd(),
    };

    let decision = engine.evaluate(&ctx);

    if decision.allowed {
        // Execute trade
        sim.update_position("0x123", proposed_trade, 0.55);
        println!("Trade executed");
    } else {
        println!("Trade blocked: {:?}", decision.violated_policies);
    }
}
```

### Multi-Market Portfolio

```rust
use ag_risk::PolymarketSimulator;

let mut sim = PolymarketSimulator::new();

// Build diversified portfolio
sim.update_position("market_A", 500.0, 0.60);
sim.update_position("market_B", 300.0, 0.45);
sim.update_position("market_C", 200.0, 0.70);

// Monitor portfolio
let active_markets = sim.get_active_markets();
for market_id in active_markets {
    let details = sim.get_position_details(&market_id).unwrap();
    println!(
        "{}: size={:.0}, pnl=${:.2}",
        market_id, details.size, details.unrealized_pnl
    );
}

println!("Total PnL: ${:.2}", sim.get_total_pnl());
println!("Total inventory: ${:.2}", sim.get_inventory_value_usd());
```

### Dynamic Risk Adjustment

```rust
let mut engine = RiskEngine::from_yaml(&yaml).unwrap();

// Normal operation
let decision = engine.evaluate(&ctx);

// Detect adverse market conditions
if market_volatility > threshold {
    println!("High volatility detected - triggering kill-switch");
    engine.trigger_kill_switch();
}

// All trades blocked until conditions improve
// ...

// Market stabilizes
if market_volatility < recovery_threshold {
    println!("Conditions normalized - resuming trading");
    engine.reset_kill_switch();
}
```

## Policy File Examples

See the `policies/` directory for complete examples:

- `conservative.yaml` - Tight limits for low-risk strategies
- `aggressive.yaml` - Higher limits for active trading
- `multi_market.yaml` - Per-market limits with different thresholds

## Testing

Run the test suite:

```bash
cargo test
```

Run with output:

```bash
cargo test -- --nocapture
```

Run integration tests only:

```bash
cargo test --test integration_tests
```

## Performance Characteristics

- **Policy Evaluation**: O(n) where n = number of policies
- **Position Update**: O(1) hash map lookup
- **Memory**: Minimal allocations after initialization
- **Thread Safety**: RwLock for kill-switch state (lock-free reads)

## Architecture

```
risk/
├── src/
│   ├── lib.rs          # Public API and core types
│   ├── policy.rs       # Policy definitions and parsing
│   ├── engine.rs       # Risk evaluation logic
│   └── simulator.rs    # Position tracking and PnL
├── policies/           # Example policy configurations
│   ├── conservative.yaml
│   ├── aggressive.yaml
│   └── multi_market.yaml
├── examples/
│   └── example_policy.yaml
├── tests/
│   └── integration_tests.rs
└── README.md
```

## Error Handling

All policy loading functions return `Result<T, String>`:

```rust
match RiskEngine::from_yaml(yaml) {
    Ok(engine) => { /* use engine */ }
    Err(e) => {
        eprintln!("Failed to load policy: {}", e);
        // Handle error
    }
}
```

Common errors:
- Invalid YAML/JSON syntax
- Missing required fields
- Invalid policy types

## Integration with Polymarket

This library is designed to integrate with Polymarket's CLOB (Central Limit Order Book):

```rust
// RTDS message received
let market_price = rtds_msg.payload.asks[0][0];

// Update simulator with latest price
sim.update_position(market_id, 0.0, market_price);

// Before placing order
let ctx = RiskContext {
    market_id: market_id.to_string(),
    current_position: sim.get_position(market_id),
    proposed_size: order_size,
    inventory_value_usd: sim.get_inventory_value_usd(),
};

if engine.evaluate(&ctx).allowed {
    // Place order via CLOB API
}
```

## Best Practices

1. **Always check risk before execution**: Never skip risk evaluation
2. **Update simulator on fills**: Keep position tracking accurate
3. **Use per-market limits**: Different markets have different risk profiles
4. **Monitor inventory**: Aggregate exposure matters
5. **Test policies**: Validate with historical data before live trading
6. **Have a kill-switch plan**: Know when and how to trigger emergency stop

## License

MIT

## Contributing

This is part of the ag-botkit project. See the main repository for contribution guidelines.

## Support

For issues and questions, see the main ag-botkit repository.

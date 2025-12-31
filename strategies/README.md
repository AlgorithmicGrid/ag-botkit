# ag-strategies: Multi-Market Trading Strategy Framework

A comprehensive framework for building, testing, and deploying trading strategies across multiple markets and venues.

## Features

- **Strategy Trait**: Base trait with lifecycle hooks (initialize, on_tick, on_fill, on_cancel, shutdown)
- **Multi-Market Coordination**: Orchestrate multiple strategies across different markets
- **Risk Integration**: Pre-trade risk checks using ag-risk module
- **Signal Framework**: Technical indicators, microstructure signals, and composite signals
- **Backtesting Engine**: Event-driven backtesting with realistic fill simulation
- **Metrics System**: Comprehensive strategy metrics for monitoring
- **Built-in Strategies**: Market making and cross-market arbitrage implementations

## Architecture

```
strategies/
├── src/                    # Core library
│   ├── lib.rs             # Strategy trait and main exports
│   ├── types.rs           # Core types (Order, Fill, Position, Signal, etc.)
│   ├── context.rs         # Strategy execution context
│   ├── coordinator.rs     # Multi-market coordinator
│   ├── metrics.rs         # Strategy metrics
│   └── error.rs           # Error types
├── signals/               # Signal generation framework
│   ├── technical.rs       # Technical indicators (SMA, EMA, RSI, etc.)
│   ├── microstructure.rs  # Market microstructure signals
│   └── composite.rs       # Composite signal generation
├── impl/                  # Strategy implementations
│   ├── market_maker.rs    # Market making with inventory skewing
│   └── cross_market_arb.rs # Cross-market arbitrage
├── backtest/              # Backtesting framework
│   ├── engine.rs          # Backtesting engine
│   └── fill_simulator.rs  # Fill simulation
├── tests/                 # Integration tests
└── examples/              # Example usage
```

## Quick Start

### Implementing a Strategy

```rust
use ag_strategies::{Strategy, StrategyContext, StrategyMetadata, StrategyResult};
use async_trait::async_trait;

struct MyStrategy {
    market_id: String,
}

#[async_trait]
impl Strategy for MyStrategy {
    async fn initialize(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()> {
        // Initialize strategy
        println!("Strategy initialized");
        Ok(())
    }

    async fn on_market_tick(
        &mut self,
        market_id: &str,
        tick: &ag_strategies::MarketTick,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Process market data
        let mid = tick.mid_price();

        // Generate and submit orders based on your logic
        // ctx.submit_order(order).await?;

        Ok(())
    }

    async fn on_fill(
        &mut self,
        fill: &ag_strategies::Fill,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Handle order fills
        println!("Filled: {} @ {}", fill.size, fill.price);
        Ok(())
    }

    async fn on_cancel(
        &mut self,
        order_id: &ag_strategies::OrderId,
        _ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        println!("Order cancelled: {}", order_id);
        Ok(())
    }

    async fn on_timer(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()> {
        // Periodic housekeeping
        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()> {
        // Cleanup
        Ok(())
    }

    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            name: "MyStrategy".to_string(),
            version: "1.0.0".to_string(),
            description: "My custom trading strategy".to_string(),
            markets: vec![self.market_id.clone()],
            required_params: vec![],
        }
    }
}
```

### Using the Market Maker Strategy

```rust
use ag_strategies::impl::{MarketMakerStrategy, MarketMakerConfig};
use ag_strategies::{StrategyContext, StrategyParams};
use ag_risk::RiskEngine;

// Configure market maker
let config = MarketMakerConfig {
    target_spread_bps: 20.0,
    quote_size: 100.0,
    max_position: 1000.0,
    inventory_target: 0.0,
    skew_factor: 0.5,
    min_quote_interval_ms: 100,
};

let mut strategy = MarketMakerStrategy::new("polymarket:0x123abc".to_string(), config);

// Create strategy context
let risk_engine = RiskEngine::from_yaml(r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#).unwrap();

let mut ctx = StrategyContext::new(
    "mm_strategy_1".to_string(),
    Arc::new(Mutex::new(risk_engine)),
    StrategyParams::new(),
);

// Initialize and run
strategy.initialize(&mut ctx).await.unwrap();
```

### Multi-Market Coordination

```rust
use ag_strategies::MultiMarketCoordinator;

let mut coordinator = MultiMarketCoordinator::new();

// Register strategies with their markets
coordinator.register_strategy(
    "mm_strategy_1".to_string(),
    Box::new(market_maker_strategy),
    mm_context,
    vec!["polymarket:0x123abc".to_string()],
).await?;

coordinator.register_strategy(
    "arb_strategy_1".to_string(),
    Box::new(arb_strategy),
    arb_context,
    vec!["polymarket:0x123abc".to_string(), "polymarket:0x456def".to_string()],
).await?;

// Route market data
coordinator.route_market_tick("polymarket:0x123abc", &tick).await?;

// Get cross-market exposure
let exposure = coordinator.calculate_total_exposure();
println!("Total exposure: ${}", exposure.total_value);
```

### Signal Generation

```rust
use ag_strategies::signals::{SimpleMovingAverage, SignalGenerator};

let mut sma = SimpleMovingAverage::new(20);

// Generate signal from market data
let signal = sma.generate_signal(&market_data);

match signal.signal_type {
    SignalType::Long => println!("Buy signal: strength={}", signal.strength),
    SignalType::Short => println!("Sell signal: strength={}", signal.strength),
    SignalType::Neutral => println!("No signal"),
    _ => {}
}
```

### Backtesting

```rust
use ag_strategies::backtest::{BacktestEngine, BacktestConfig};

let config = BacktestConfig {
    initial_capital: 10000.0,
    ..Default::default()
};

let mut engine = BacktestEngine::new(config)?;

// Run backtest
let result = engine.run_backtest(
    Box::new(my_strategy),
    historical_ticks,
    params,
).await?;

println!("Total return: ${}", result.total_return);
println!("Sharpe ratio: {:.2}", result.sharpe_ratio);
println!("Max drawdown: {:.2}%", result.max_drawdown_pct);
println!("Win rate: {:.2}%", result.win_rate);
```

## Available Signals

### Technical Indicators

- **SimpleMovingAverage**: SMA(n)
- **ExponentialMovingAverage**: EMA(n)
- **RelativeStrengthIndex**: RSI(n)
- **BollingerBands**: BB(n, σ)
- **MACD**: Moving Average Convergence Divergence

### Microstructure Signals

- **OrderImbalance**: Bid/ask volume imbalance
- **SpreadAnalyzer**: Bid-ask spread dynamics

### Composite Signals

- **CompositeSignal**: Weighted combination of multiple signals
- **SignalAggregator**: Consensus and strongest signal selection

## Strategy Metrics

Strategies automatically emit metrics for monitoring:

- `strategy.pnl_usd`: PnL in USD
- `strategy.position_size`: Current position size
- `strategy.signals_generated`: Number of signals
- `strategy.orders_placed`: Number of orders placed
- `strategy.orders_filled`: Number of fills
- `strategy.sharpe_ratio`: Strategy Sharpe ratio
- `strategy.max_drawdown`: Maximum drawdown

## Risk Integration

All strategies integrate with the ag-risk module for pre-trade risk checks:

```rust
// Orders are automatically checked against risk policies
let order = Order { /* ... */ };

match ctx.submit_order(order).await {
    Ok(order_id) => println!("Order submitted: {}", order_id),
    Err(StrategyError::RiskRejected { policies }) => {
        println!("Order rejected by policies: {:?}", policies);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

Risk policies are defined in YAML:

```yaml
policies:
  - type: PositionLimit
    market_id: "polymarket:0x123abc"
    max_size: 1000.0

  - type: InventoryLimit
    max_value_usd: 10000.0

  - type: KillSwitch
    enabled: false
```

## Building and Testing

```bash
# Build the library
cargo build --release

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_market_maker

# Generate documentation
cargo doc --no-deps --open

# Run clippy
cargo clippy
```

## Examples

See the `examples/` directory for complete examples:

- `run_strategy.rs`: Running a strategy in production
- `backtest.rs`: Backtesting a strategy

Run examples with:

```bash
cargo run --example run_strategy
cargo run --example backtest
```

## Integration with ag-botkit

The strategy module integrates with other ag-botkit components:

- **ag-risk**: Pre-trade risk checks and position limits
- **exec** (future): Order execution via execution gateway
- **monitor**: Strategy metrics emission
- **storage** (future): Historical data for backtesting

## Performance Targets

- Strategy tick processing: <1ms per market update
- Order submission latency: <5ms
- Multi-market coordination: <10ms
- Backtest throughput: >10k ticks/sec

## License

MIT

## Contributing

See the main ag-botkit repository for contribution guidelines.

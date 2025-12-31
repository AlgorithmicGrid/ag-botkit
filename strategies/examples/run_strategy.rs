//! Example: Running a market making strategy
//!
//! This example demonstrates how to set up and run a market making strategy
//! with the ag-strategies framework.

use ag_strategies::{
    Strategy, StrategyContext, StrategyParams,
    MultiMarketCoordinator,
    impl::{MarketMakerStrategy, MarketMakerConfig},
    types::MarketTick,
};
use ag_risk::RiskEngine;
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Market Making Strategy Example ===\n");

    // 1. Configure the market maker
    let config = MarketMakerConfig {
        target_spread_bps: 20.0,    // 0.2% spread
        quote_size: 100.0,           // Quote 100 units
        max_position: 1000.0,        // Max position 1000 units
        inventory_target: 0.0,       // Target neutral inventory
        skew_factor: 0.5,            // 50% inventory skew adjustment
        min_quote_interval_ms: 100,  // Requote every 100ms minimum
    };

    println!("Market Maker Config:");
    println!("  Target spread: {} bps", config.target_spread_bps);
    println!("  Quote size: {}", config.quote_size);
    println!("  Max position: {}", config.max_position);
    println!();

    // 2. Create risk engine
    let risk_yaml = r#"
policies:
  - type: PositionLimit
    market_id: "polymarket:0x123abc"
    max_size: 1000.0

  - type: InventoryLimit
    max_value_usd: 10000.0

  - type: KillSwitch
    enabled: false
"#;

    let risk_engine = RiskEngine::from_yaml(risk_yaml)?;
    let risk_engine = Arc::new(Mutex::new(risk_engine));

    println!("Risk policies loaded:");
    println!("  Position limit: 1000 units");
    println!("  Inventory limit: $10,000");
    println!();

    // 3. Create strategy and context
    let market_id = "polymarket:0x123abc".to_string();
    let strategy = Box::new(MarketMakerStrategy::new(
        market_id.clone(),
        config,
    ));

    let mut context = StrategyContext::new(
        "mm_strategy_1".to_string(),
        risk_engine.clone(),
        StrategyParams::new(),
    );

    // 4. Register with coordinator
    let mut coordinator = MultiMarketCoordinator::new();

    coordinator.register_strategy(
        "mm_strategy_1".to_string(),
        strategy,
        context,
        vec![market_id.clone()],
    ).await?;

    println!("Strategy registered with coordinator");
    println!("Strategy count: {}", coordinator.strategy_count());
    println!();

    // 5. Simulate market data
    println!("Simulating market ticks...\n");

    for i in 0..10 {
        let base_price = 100.0 + (i as f64) * 0.1;

        let tick = MarketTick {
            market: market_id.clone(),
            timestamp: Utc::now(),
            bid: Some(base_price),
            ask: Some(base_price + 0.2),
            bid_size: Some(100.0),
            ask_size: Some(100.0),
            last: Some(base_price + 0.1),
            volume_24h: Some(10000.0),
        };

        println!("Tick {}: mid={:.2}, spread={:.3}", i+1, tick.mid_price(), tick.spread().unwrap_or(0.0));

        // Route tick to strategy
        coordinator.route_market_tick(&market_id, &tick).await?;

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    // 6. Get exposure summary
    println!("\n=== Final Exposure Summary ===");
    let exposure = coordinator.calculate_total_exposure();
    println!("Total inventory value: ${:.2}", exposure.total_value);
    println!("Total unrealized PnL: ${:.2}", exposure.total_unrealized_pnl);
    println!("Total realized PnL: ${:.2}", exposure.total_realized_pnl);

    println!("\nPositions by market:");
    for (market, size) in exposure.positions_by_market {
        println!("  {}: {:.2} units", market, size);
    }

    // 7. Shutdown
    coordinator.unregister_strategy("mm_strategy_1").await?;

    println!("\nStrategy shutdown complete");

    Ok(())
}

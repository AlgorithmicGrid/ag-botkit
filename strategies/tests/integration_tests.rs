//! Integration tests for the strategy framework

use ag_strategies::{
    Strategy, StrategyContext, StrategyParams, StrategyMetadata, SignalGenerator,
    MultiMarketCoordinator,
    types::{MarketTick, Side},
    backtest::{BacktestEngine, BacktestConfig},
    signals::SimpleMovingAverage,
};
use ag_risk::RiskEngine;
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::Utc;

// Import strategy implementations with r#impl syntax
use ag_strategies::r#impl::{
    MarketMakerStrategy, MarketMakerConfig,
    CrossMarketArbStrategy, CrossMarketArbConfig,
};

// Helper function to create test context
fn create_test_context(strategy_id: &str) -> StrategyContext {
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 10000.0
"#;
    let risk_engine = RiskEngine::from_yaml(yaml).unwrap();
    StrategyContext::new(
        strategy_id.to_string(),
        Arc::new(Mutex::new(risk_engine)),
        StrategyParams::new(),
    )
}

// Helper function to create test tick
fn create_test_tick(market_id: &str, mid: f64) -> MarketTick {
    MarketTick {
        market: market_id.to_string(),
        timestamp: Utc::now(),
        bid: Some(mid - 0.05),
        ask: Some(mid + 0.05),
        bid_size: Some(100.0),
        ask_size: Some(100.0),
        last: Some(mid),
        volume_24h: Some(1000.0),
    }
}

#[tokio::test]
async fn test_market_maker_initialization() {
    let config = MarketMakerConfig::default();
    let mut strategy = MarketMakerStrategy::new("market1".to_string(), config);

    let mut ctx = create_test_context("mm_test");

    let result = strategy.initialize(&mut ctx).await;
    assert!(result.is_ok());

    let metadata = strategy.metadata();
    assert_eq!(metadata.name, "MarketMaker");
    assert_eq!(metadata.version, "1.0.0");
}

#[tokio::test]
async fn test_market_maker_quoting() {
    let config = MarketMakerConfig {
        target_spread_bps: 20.0,
        quote_size: 100.0,
        max_position: 1000.0,
        inventory_target: 0.0,
        skew_factor: 0.5,
        min_quote_interval_ms: 0, // No rate limiting for test
    };

    let mut strategy = MarketMakerStrategy::new("market1".to_string(), config);
    let mut ctx = create_test_context("mm_test");

    strategy.initialize(&mut ctx).await.unwrap();

    // Send market tick
    let tick = create_test_tick("market1", 100.0);
    let result = strategy.on_market_tick("market1", &tick, &mut ctx).await;
    assert!(result.is_ok());

    // Should have submitted orders (though they won't actually execute in test)
    // In a real integration test with exec module, we'd verify order submission
}

#[tokio::test]
async fn test_cross_market_arbitrage() {
    let config = CrossMarketArbConfig {
        min_spread_bps: 50.0, // 0.5% minimum spread
        size: 50.0,
        max_position: 500.0,
    };

    let mut strategy = CrossMarketArbStrategy::new(
        "market_a".to_string(),
        "market_b".to_string(),
        config,
    );

    let mut ctx = create_test_context("arb_test");

    strategy.initialize(&mut ctx).await.unwrap();

    // Send ticks for both markets with significant spread
    let tick_a = create_test_tick("market_a", 100.0);
    let tick_b = create_test_tick("market_b", 101.0);

    // First tick
    strategy.on_market_tick("market_a", &tick_a, &mut ctx).await.unwrap();

    // Second tick should trigger arbitrage
    let result = strategy.on_market_tick("market_b", &tick_b, &mut ctx).await;
    assert!(result.is_ok());

    // In a real test with exec module, we'd verify orders were submitted
}

#[tokio::test]
async fn test_multi_market_coordinator() {
    let mut coordinator = MultiMarketCoordinator::new();

    // Create and register first strategy
    let mm_config = MarketMakerConfig::default();
    let mm_strategy = Box::new(MarketMakerStrategy::new("market1".to_string(), mm_config));
    let mm_ctx = create_test_context("mm_1");

    coordinator.register_strategy(
        "mm_1".to_string(),
        mm_strategy,
        mm_ctx,
        vec!["market1".to_string()],
    ).await.unwrap();

    // Create and register second strategy
    let arb_config = CrossMarketArbConfig::default();
    let arb_strategy = Box::new(CrossMarketArbStrategy::new(
        "market1".to_string(),
        "market2".to_string(),
        arb_config,
    ));
    let arb_ctx = create_test_context("arb_1");

    coordinator.register_strategy(
        "arb_1".to_string(),
        arb_strategy,
        arb_ctx,
        vec!["market1".to_string(), "market2".to_string()],
    ).await.unwrap();

    // Verify registration
    assert_eq!(coordinator.strategy_count(), 2);

    // Route tick to market1 (should reach both strategies)
    let tick = create_test_tick("market1", 100.0);
    let result = coordinator.route_market_tick("market1", &tick).await;
    assert!(result.is_ok());

    // Get exposure
    let exposure = coordinator.calculate_total_exposure();
    assert_eq!(exposure.total_value, 0.0); // No positions yet

    // Unregister strategy
    coordinator.unregister_strategy("mm_1").await.unwrap();
    assert_eq!(coordinator.strategy_count(), 1);
}

#[tokio::test]
async fn test_signal_generation() {
    let mut sma = SimpleMovingAverage::new(5);

    // Create mock market data
    let mut market_data = ag_strategies::types::MarketData {
        market: "test".to_string(),
        ticks: Vec::new(),
        bars: Vec::new(),
    };

    // Add ticks
    for i in 0..10 {
        let tick = create_test_tick("test", 100.0 + i as f64);
        market_data.ticks.push(tick);
    }

    // Generate signal
    let signal = sma.generate_signal(&market_data);

    assert_eq!(signal.market_id, "test");
    assert!(signal.confidence >= 0.0 && signal.confidence <= 1.0);
    assert!(signal.strength >= 0.0 && signal.strength <= 1.0);
}

#[tokio::test]
async fn test_backtest_engine() {
    // Generate historical data
    let mut historical_ticks = Vec::new();
    for i in 0..100 {
        let tick = create_test_tick("test_market", 100.0 + (i as f64 * 0.1));
        historical_ticks.push(tick);
    }

    // Create strategy
    let config = MarketMakerConfig {
        target_spread_bps: 20.0,
        quote_size: 50.0,
        max_position: 500.0,
        inventory_target: 0.0,
        skew_factor: 0.5,
        min_quote_interval_ms: 0,
    };

    let strategy = Box::new(MarketMakerStrategy::new(
        "test_market".to_string(),
        config,
    ));

    // Configure backtest
    let backtest_config = BacktestConfig::default();

    // Run backtest
    let mut engine = BacktestEngine::new(backtest_config).unwrap();

    let result = engine.run_backtest(
        strategy,
        historical_ticks,
        StrategyParams::new(),
    ).await;

    assert!(result.is_ok());

    let result = result.unwrap();
    // Strategy may execute trades and make/lose money in backtest
    assert!(result.final_capital > 0.0); // Should have positive capital
    assert!(result.sharpe_ratio >= 0.0);
}

#[tokio::test]
async fn test_position_tracking() {
    let mut ctx = create_test_context("position_test");

    // Initial position should be flat
    assert!(ctx.get_position("market1").is_none());

    // Update position with buy
    ctx.update_position("market1", 100.0, 100.0);

    let pos = ctx.get_position("market1").unwrap();
    assert_eq!(pos.size, 100.0);
    assert_eq!(pos.entry_price, 100.0);
    assert!(pos.is_long());
    assert!(!pos.is_short());
    assert!(!pos.is_flat());

    // Update with another buy at different price
    ctx.update_position("market1", 50.0, 102.0);

    let pos = ctx.get_position("market1").unwrap();
    assert_eq!(pos.size, 150.0);
    // Entry price should be weighted average: (100*100 + 50*102) / 150 = 100.67
    assert!((pos.entry_price - 100.666).abs() < 0.01);

    // Sell to reduce position
    ctx.update_position("market1", -50.0, 105.0);

    let pos = ctx.get_position("market1").unwrap();
    assert_eq!(pos.size, 100.0);
}

#[tokio::test]
async fn test_risk_rejection() {
    // Create context with strict position limits
    let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 100.0
"#;
    let risk_engine = RiskEngine::from_yaml(yaml).unwrap();
    let mut ctx = StrategyContext::new(
        "risk_test".to_string(),
        Arc::new(Mutex::new(risk_engine)),
        StrategyParams::new(),
    );

    // Set current position to 90
    ctx.update_position("market1", 90.0, 100.0);

    // Try to buy 20 more (would exceed limit of 100)
    let order = ag_strategies::types::Order {
        venue: "test".to_string(),
        market: "market1".to_string(),
        side: Side::Buy,
        order_type: ag_strategies::types::OrderType::Limit,
        price: Some(100.0),
        size: 20.0,
        time_in_force: ag_strategies::types::TimeInForce::GTC,
        ..Default::default()
    };

    let result = ctx.submit_order(order).await;

    // Should be rejected by risk
    match result {
        Err(ag_strategies::StrategyError::RiskRejected { policies }) => {
            assert!(!policies.is_empty());
        }
        _ => panic!("Expected risk rejection"),
    }
}

#[tokio::test]
async fn test_metrics_emission() {
    let mut ctx = create_test_context("metrics_test");

    let metric = ag_strategies::metrics::StrategyMetric::gauge(
        "test_strategy".to_string(),
        "test.metric".to_string(),
        42.0,
        std::collections::HashMap::new(),
    );

    let result = ctx.emit_metric(metric).await;
    assert!(result.is_ok());

    // Verify metric was buffered
    let buffered = ctx.get_metrics_buffer();
    assert_eq!(buffered.len(), 1);
    assert_eq!(buffered[0].value, 42.0);
}

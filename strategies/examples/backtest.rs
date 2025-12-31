//! Example: Backtesting a market making strategy
//!
//! This example demonstrates how to backtest a strategy using historical data.

use ag_strategies::{
    StrategyParams,
    impl::{MarketMakerStrategy, MarketMakerConfig},
    types::MarketTick,
    backtest::{BacktestEngine, BacktestConfig, FillSimulatorConfig},
};
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Market Making Strategy Backtest ===\n");

    // 1. Generate synthetic historical data
    println!("Generating historical market data...");

    let start_time = Utc::now() - Duration::days(1);
    let mut historical_ticks = Vec::new();

    // Generate 1000 ticks over 1 day (~ 1 tick per minute)
    for i in 0..1000 {
        // Simulate a random walk with drift
        let base_price = 100.0 + (i as f64 * 0.01) + ((i % 10) as f64 - 5.0) * 0.1;

        let tick = MarketTick {
            market: "test_market".to_string(),
            timestamp: start_time + Duration::seconds(i * 86),
            bid: Some(base_price - 0.05),
            ask: Some(base_price + 0.05),
            bid_size: Some(100.0 + (i % 50) as f64),
            ask_size: Some(100.0 + ((i + 25) % 50) as f64),
            last: Some(base_price),
            volume_24h: Some(10000.0),
        };

        historical_ticks.push(tick);
    }

    println!("Generated {} ticks", historical_ticks.len());
    println!("Time range: {} to {}\n",
        historical_ticks.first().unwrap().timestamp,
        historical_ticks.last().unwrap().timestamp
    );

    // 2. Configure strategy
    let strategy_config = MarketMakerConfig {
        target_spread_bps: 15.0,
        quote_size: 50.0,
        max_position: 500.0,
        inventory_target: 0.0,
        skew_factor: 0.5,
        min_quote_interval_ms: 5000, // Every 5 seconds
    };

    let strategy = Box::new(MarketMakerStrategy::new(
        "test_market".to_string(),
        strategy_config,
    ));

    println!("Strategy Configuration:");
    println!("  Name: Market Maker");
    println!("  Target spread: 15 bps");
    println!("  Quote size: 50 units");
    println!("  Max position: 500 units");
    println!();

    // 3. Configure backtest
    let backtest_config = BacktestConfig {
        initial_capital: 10000.0,
        fill_simulator: FillSimulatorConfig {
            slippage_bps: 5.0,
            fill_probability: 0.7,
            taker_fee_bps: 10.0,
            maker_fee_bps: -5.0,
        },
        risk_policy_yaml: r#"
policies:
  - type: PositionLimit
    max_size: 500.0
  - type: InventoryLimit
    max_value_usd: 10000.0
"#.to_string(),
    };

    println!("Backtest Configuration:");
    println!("  Initial capital: ${}", backtest_config.initial_capital);
    println!("  Slippage: 5 bps");
    println!("  Taker fee: 10 bps");
    println!("  Maker rebate: 5 bps");
    println!();

    // 4. Run backtest
    println!("Running backtest...\n");

    let mut engine = BacktestEngine::new(backtest_config)?;

    let result = engine.run_backtest(
        strategy,
        historical_ticks,
        StrategyParams::new(),
    ).await?;

    // 5. Display results
    println!("=== Backtest Results ===\n");

    println!("Performance:");
    println!("  Total return: ${:.2} ({:.2}%)",
        result.total_return,
        result.total_return_pct
    );
    println!("  Final capital: ${:.2}", result.final_capital);
    println!();

    println!("Risk Metrics:");
    println!("  Sharpe ratio: {:.2}", result.sharpe_ratio);
    println!("  Max drawdown: ${:.2} ({:.2}%)",
        result.max_drawdown,
        result.max_drawdown_pct
    );
    println!();

    println!("Trading Activity:");
    println!("  Total trades: {}", result.num_trades);
    println!("  Win rate: {:.2}%", result.win_rate);
    println!("  Avg trade PnL: ${:.2}", result.avg_trade_pnl);
    println!();

    // 6. Show first few trades
    if !result.trades.is_empty() {
        println!("Sample Trades (first 5):");
        for (i, trade) in result.trades.iter().take(5).enumerate() {
            println!("  {}: {:?} {} @ {:.2} (fee: ${:.4})",
                i + 1,
                trade.side,
                trade.size,
                trade.price,
                trade.fee
            );
        }
        println!();
    }

    // 7. Performance summary
    println!("=== Summary ===");

    if result.total_return > 0.0 {
        println!("✓ Profitable strategy");
    } else {
        println!("✗ Losing strategy");
    }

    if result.sharpe_ratio > 1.0 {
        println!("✓ Good risk-adjusted returns (Sharpe > 1.0)");
    } else {
        println!("✗ Poor risk-adjusted returns (Sharpe < 1.0)");
    }

    if result.max_drawdown_pct < 10.0 {
        println!("✓ Low drawdown (< 10%)");
    } else {
        println!("⚠ High drawdown (> 10%)");
    }

    println!();
    println!("Backtest complete!");

    Ok(())
}

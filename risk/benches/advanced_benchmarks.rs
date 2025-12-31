//! Benchmarks for advanced risk models
//!
//! Run with: cargo bench

use ag_risk::advanced::*;
use std::collections::HashMap;

fn main() {
    println!("=== Advanced Risk Models Performance Benchmarks ===\n");

    benchmark_var_calculations();
    benchmark_greeks_calculations();
    benchmark_portfolio_analytics();
    benchmark_stress_testing();
    benchmark_performance_metrics();
}

fn benchmark_var_calculations() {
    println!("## VaR Calculations");

    // Historical VaR
    let returns: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.001).sin() * 0.02).collect();
    let config = VarConfig::default();
    let engine = VarEngine::with_historical_returns(config.clone(), returns.clone());

    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = engine.calculate_historical_var(100000.0, 0.95, 1);
    }
    let elapsed = start.elapsed();
    println!("  Historical VaR (100 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 100);

    // Parametric VaR
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = engine.calculate_parametric_var(100000.0, 0.02, 0.95, 1);
    }
    let elapsed = start.elapsed();
    println!("  Parametric VaR (1000 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 1000);

    // Monte Carlo VaR
    let start = std::time::Instant::now();
    let _ = engine.calculate_monte_carlo_var(100000.0, 0.0, 0.02, 0.95, 1, 10000);
    let elapsed = start.elapsed();
    println!("  Monte Carlo VaR (10,000 simulations): {:?}", elapsed);

    println!();
}

fn benchmark_greeks_calculations() {
    println!("## Greeks Calculations");

    let config = GreeksConfig::default();
    let engine = GreeksEngine::new(config);

    let option = greeks::Option {
        option_type: greeks::OptionType::Call,
        strike: 100.0,
        time_to_expiry: 1.0,
        contract_size: 100.0,
    };

    // Single option Greeks
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        let _ = engine.calculate_greeks(&option, 100.0, 0.20, 0.05);
    }
    let elapsed = start.elapsed();
    println!("  Single option Greeks (10,000 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 10000);

    // Portfolio Greeks (100 positions)
    let positions: Vec<greeks::OptionPosition> = (0..100)
        .map(|i| greeks::OptionPosition {
            option: greeks::Option {
                option_type: if i % 2 == 0 {
                    greeks::OptionType::Call
                } else {
                    greeks::OptionType::Put
                },
                strike: 100.0,
                time_to_expiry: 1.0,
                contract_size: 100.0,
            },
            quantity: 10.0,
            underlying: "SPY".to_string(),
        })
        .collect();

    let mut market_data = greeks::MarketData {
        underlying_prices: HashMap::new(),
        volatilities: HashMap::new(),
        risk_free_rate: 0.05,
    };
    market_data.underlying_prices.insert("SPY".to_string(), 100.0);
    market_data.volatilities.insert("SPY".to_string(), 0.20);

    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = engine.calculate_portfolio_greeks(&positions, &market_data);
    }
    let elapsed = start.elapsed();
    println!("  Portfolio Greeks (100 positions, 100 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 100);

    println!();
}

fn benchmark_portfolio_analytics() {
    println!("## Portfolio Analytics");

    let config = PortfolioConfig::default();
    let analyzer = PortfolioAnalyzer::new(config);

    // Create sample returns data
    let mut returns = HashMap::new();
    for i in 0..100 {
        let asset_returns: Vec<f64> = (0..252)
            .map(|j| ((i + j) as f64 * 0.01).sin() * 0.02)
            .collect();
        returns.insert(format!("Asset{}", i), asset_returns);
    }

    // Correlation matrix calculation
    let start = std::time::Instant::now();
    let _ = analyzer.calculate_correlation_matrix(&returns);
    let elapsed = start.elapsed();
    println!("  Correlation matrix (100 assets, 252 obs): {:?}", elapsed);

    // Create positions
    let positions: Vec<portfolio::Position> = (0..100)
        .map(|i| portfolio::Position {
            asset_id: format!("Asset{}", i),
            value_usd: 1000.0,
            weight: 0.01,
        })
        .collect();

    // Risk contribution (requires covariance matrix)
    let cov_matrix = nalgebra::DMatrix::identity(100, 100) * 0.04;

    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = analyzer.calculate_risk_contribution(&positions, &cov_matrix);
    }
    let elapsed = start.elapsed();
    println!("  Risk contribution (100 positions, 100 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 100);

    println!();
}

fn benchmark_stress_testing() {
    println!("## Stress Testing");

    let engine = StressTestEngine::with_historical_scenarios();

    let mut positions = HashMap::new();
    positions.insert(
        "SPY".to_string(),
        stress::Position {
            asset_id: "SPY".to_string(),
            quantity: 100.0,
            current_price: 450.0,
            value_usd: 45000.0,
        },
    );
    positions.insert(
        "QQQ".to_string(),
        stress::Position {
            asset_id: "QQQ".to_string(),
            quantity: 50.0,
            current_price: 380.0,
            value_usd: 19000.0,
        },
    );

    let portfolio = stress::Portfolio {
        positions,
        total_value_usd: 64000.0,
    };

    // Single scenario
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = engine.run_all_scenarios(&portfolio);
    }
    let elapsed = start.elapsed();
    println!("  All scenarios (5 scenarios, 1000 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 1000);

    println!();
}

fn benchmark_performance_metrics() {
    println!("## Performance Metrics");

    let returns: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin() * 0.02).collect();
    let market_returns: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.008).sin() * 0.015).collect();

    let metrics = PerformanceMetrics::new(returns.clone(), 0.02);

    // Sharpe ratio
    let start = std::time::Instant::now();
    for _ in 0..10000 {
        let _ = metrics.sharpe_ratio();
    }
    let elapsed = start.elapsed();
    println!("  Sharpe ratio (10,000 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 10000);

    // Max drawdown
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = metrics.max_drawdown();
    }
    let elapsed = start.elapsed();
    println!("  Max drawdown (1000 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 1000);

    // Beta
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = metrics.beta(&market_returns);
    }
    let elapsed = start.elapsed();
    println!("  Beta (1000 iterations): {:?}", elapsed);
    println!("  Average: {:?}", elapsed / 1000);

    println!();
    println!("=== Benchmarks Complete ===");
}

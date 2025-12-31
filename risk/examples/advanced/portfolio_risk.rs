//! Portfolio risk analytics example
//!
//! Demonstrates portfolio volatility, correlation, and risk contribution analysis.
//!
//! Run with: cargo run --example portfolio_risk

use ag_risk::advanced::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Portfolio Risk Analytics Example ===\n");

    // 1. Define portfolio positions
    let positions = vec![
        portfolio::Position {
            asset_id: "SPY".to_string(),
            value_usd: 50_000.0,
            weight: 0.50,
        },
        portfolio::Position {
            asset_id: "QQQ".to_string(),
            value_usd: 30_000.0,
            weight: 0.30,
        },
        portfolio::Position {
            asset_id: "TLT".to_string(),
            value_usd: 20_000.0,
            weight: 0.20,
        },
    ];

    let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
    println!("Portfolio Value: ${:,.0}", total_value);
    println!("\nPositions:");
    for pos in &positions {
        println!("  {}: ${:>10,.0} ({:.0}%)", pos.asset_id, pos.value_usd, pos.weight * 100.0);
    }
    println!();

    // 2. Create historical returns data (simulated)
    let mut returns = HashMap::new();

    // SPY returns (moderate volatility)
    let spy_returns: Vec<f64> = (0..252)
        .map(|i| (i as f64 * 0.05).sin() * 0.015 + ((i * 7) % 100) as f64 / 10000.0)
        .collect();
    returns.insert("SPY".to_string(), spy_returns);

    // QQQ returns (higher volatility, correlated with SPY)
    let qqq_returns: Vec<f64> = (0..252)
        .map(|i| (i as f64 * 0.05).sin() * 0.020 + ((i * 11) % 100) as f64 / 8000.0)
        .collect();
    returns.insert("QQQ".to_string(), qqq_returns);

    // TLT returns (lower volatility, negative correlation)
    let tlt_returns: Vec<f64> = (0..252)
        .map(|i| -(i as f64 * 0.05).sin() * 0.008 + ((i * 13) % 100) as f64 / 15000.0)
        .collect();
    returns.insert("TLT".to_string(), tlt_returns);

    // 3. Create portfolio analyzer
    let config = PortfolioConfig {
        min_observations: 30,
        regularization: 1e-6,
    };
    let analyzer = PortfolioAnalyzer::new(config);

    // 4. Calculate correlation matrix
    println!("--- Correlation Matrix ---");
    let corr_matrix = analyzer.calculate_correlation_matrix(&returns)?;

    println!("         SPY      QQQ      TLT");
    let assets = ["SPY", "QQQ", "TLT"];
    for (i, asset_i) in assets.iter().enumerate() {
        print!("{:<5}", asset_i);
        for j in 0..assets.len() {
            print!("  {:>6.3}", corr_matrix[(i, j)]);
        }
        println!();
    }
    println!();

    // 5. Calculate portfolio volatility
    println!("--- Portfolio Volatility ---");
    let portfolio_vol = analyzer.calculate_volatility(&positions, &corr_matrix)?;
    println!("Portfolio Volatility: {:.4}", portfolio_vol);

    // Calculate individual asset volatilities
    let mut asset_vols = HashMap::new();
    for (asset, rets) in &returns {
        let mean = rets.iter().sum::<f64>() / rets.len() as f64;
        let variance = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (rets.len() - 1) as f64;
        let vol = variance.sqrt();
        asset_vols.insert(asset.clone(), vol);
        println!("  {} Volatility: {:.4}", asset, vol);
    }
    println!();

    // 6. Calculate diversification ratio
    println!("--- Diversification Analysis ---");
    let div_ratio = analyzer.calculate_diversification_ratio(
        &positions,
        &asset_vols,
        portfolio_vol,
    )?;

    println!("Diversification Ratio: {:.2}", div_ratio);
    println!("Interpretation: Portfolio volatility is {:.1}% of weighted average individual volatility",
        (1.0 / div_ratio) * 100.0);
    println!("Benefit from diversification: {:.1}%",
        (1.0 - 1.0 / div_ratio) * 100.0);
    println!();

    // 7. Calculate concentration (HHI)
    println!("--- Concentration Risk ---");
    let hhi = analyzer.calculate_concentration_hhi(&positions)?;
    println!("Herfindahl-Hirschman Index: {:.3}", hhi);
    println!("Interpretation:");
    if hhi > 0.5 {
        println!("  HIGH concentration - portfolio dominated by few positions");
    } else if hhi > 0.25 {
        println!("  MODERATE concentration");
    } else {
        println!("  LOW concentration - well diversified");
    }
    println!("  Effective number of positions: {:.1}", 1.0 / hhi);
    println!();

    // 8. Calculate risk contribution
    println!("--- Risk Contribution Analysis ---");
    let cov_matrix = analyzer.calculate_covariance_matrix(&returns)?;
    let risk_contributions = analyzer.calculate_risk_contribution(&positions, &cov_matrix)?;

    println!("Position     Volatility Contrib    Risk %");
    println!("{:-<48}", "");
    for contrib in &risk_contributions {
        println!("{:<10}   {:>15.4}      {:>6.1}%",
            contrib.asset_id,
            contrib.volatility_contribution,
            contrib.risk_pct
        );
    }
    println!();

    // 9. Calculate marginal VaR
    println!("--- Marginal VaR Analysis ---");

    // First calculate portfolio VaR
    let portfolio_var = 10000.0; // Assume we calculated this

    let marginal_vars = analyzer.calculate_marginal_var(
        portfolio_var,
        &positions,
        &cov_matrix,
    )?;

    println!("Portfolio VaR: ${:,.2}", portfolio_var);
    println!("\nPosition     Marginal VaR    Component VaR");
    println!("{:-<48}", "");
    for mvar in &marginal_vars {
        println!("{:<10}   ${:>10,.2}     ${:>10,.2}",
            mvar.asset_id,
            mvar.marginal_var,
            mvar.component_var
        );
    }
    println!();

    // 10. Risk optimization recommendations
    println!("--- Risk Optimization Recommendations ---");

    // Find most risky positions
    let mut sorted_risk = risk_contributions.clone();
    sorted_risk.sort_by(|a, b| b.risk_pct.partial_cmp(&a.risk_pct).unwrap());

    println!("Highest risk contributors:");
    for (i, contrib) in sorted_risk.iter().take(2).enumerate() {
        println!("  {}. {} ({}% of portfolio risk)", i + 1, contrib.asset_id, contrib.risk_pct as i32);
        if contrib.risk_pct > 50.0 {
            println!("     → Recommendation: Consider reducing position size");
        }
    }
    println!();

    // Check if diversification can be improved
    if div_ratio < 1.5 {
        println!("Low diversification benefit detected.");
        println!("  → Recommendation: Add uncorrelated assets");
        println!("  → Consider: International stocks, commodities, real estate");
    }
    println!();

    // Check concentration
    if hhi > 0.4 {
        println!("High concentration risk detected.");
        println!("  → Recommendation: Spread capital across more positions");
        println!("  → Target HHI: < 0.25 (equivalent to 4+ equal positions)");
    }

    println!("\n=== Example Complete ===");

    Ok(())
}

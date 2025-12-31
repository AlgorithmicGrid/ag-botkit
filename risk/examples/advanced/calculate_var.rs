//! VaR calculation example
//!
//! Demonstrates how to calculate Value at Risk using multiple methodologies.
//!
//! Run with: cargo run --example calculate_var

use ag_risk::advanced::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Value at Risk (VaR) Calculation Example ===\n");

    // 1. Create sample historical returns (simulating 100 days of trading)
    let returns: Vec<f64> = (0..100)
        .map(|i| {
            // Simulate returns with some volatility
            let base_return = (i as f64 * 0.1).sin() * 0.01;
            let noise = ((i * 17) % 100) as f64 / 100.0 * 0.005;
            base_return + noise - 0.0025 // Add some negative bias
        })
        .collect();

    println!("Sample returns statistics:");
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
    let std_dev = variance.sqrt();
    println!("  Mean return: {:.4}%", mean * 100.0);
    println!("  Std deviation: {:.4}%", std_dev * 100.0);
    println!("  Number of observations: {}", returns.len());
    println!();

    // 2. Create VaR engine with configuration
    let config = VarConfig {
        default_simulations: 10_000,
        min_observations: 30,
        random_seed: Some(42), // For reproducible results
    };

    let engine = VarEngine::with_historical_returns(config, returns);

    // Portfolio parameters
    let portfolio_value = 1_000_000.0; // $1M portfolio
    let confidence_level = 0.95; // 95% confidence
    let time_horizon = 1; // 1 day

    println!("Portfolio: ${:,.0}", portfolio_value);
    println!("Confidence Level: {}%", confidence_level * 100.0);
    println!("Time Horizon: {} day(s)", time_horizon);
    println!();

    // 3. Calculate Historical VaR
    println!("--- Historical VaR ---");
    let historical_var = engine.calculate_historical_var(
        portfolio_value,
        confidence_level,
        time_horizon,
    )?;

    println!("VaR: ${:,.2}", historical_var.var_amount);
    println!("Interpretation: With {}% confidence, we will not lose more than ${:,.2} over {} day(s)",
        confidence_level * 100.0,
        historical_var.var_amount,
        time_horizon
    );
    println!("Or: There's a {}% chance of losing more than ${:,.2}",
        (1.0 - confidence_level) * 100.0,
        historical_var.var_amount
    );
    println!();

    // 4. Calculate Parametric VaR
    println!("--- Parametric VaR (Normal Distribution) ---");
    let parametric_var = engine.calculate_parametric_var(
        portfolio_value,
        std_dev, // Use calculated std dev
        confidence_level,
        time_horizon,
    )?;

    println!("VaR: ${:,.2}", parametric_var.var_amount);
    println!();

    // 5. Calculate Monte Carlo VaR
    println!("--- Monte Carlo VaR ---");
    let start = std::time::Instant::now();
    let monte_carlo_var = engine.calculate_monte_carlo_var(
        portfolio_value,
        mean,
        std_dev,
        confidence_level,
        time_horizon,
        10_000, // 10,000 simulations
    )?;
    let elapsed = start.elapsed();

    println!("VaR: ${:,.2}", monte_carlo_var.var_amount);
    println!("Simulations: 10,000");
    println!("Calculation time: {:?}", elapsed);
    println!();

    // 6. Calculate CVaR (Expected Shortfall)
    println!("--- Conditional VaR (CVaR / Expected Shortfall) ---");
    let cvar = engine.calculate_cvar(
        portfolio_value,
        confidence_level,
        time_horizon,
    )?;

    println!("CVaR: ${:,.2}", cvar);
    println!("Interpretation: If losses exceed VaR, the expected loss is ${:,.2}", cvar);
    println!();

    // 7. Compare all methods
    println!("--- Comparison of VaR Methods ---");
    println!("{:<20} {:>15}", "Method", "VaR Amount");
    println!("{:-<35}", "");
    println!("{:<20} ${:>14,.2}", "Historical", historical_var.var_amount);
    println!("{:<20} ${:>14,.2}", "Parametric", parametric_var.var_amount);
    println!("{:<20} ${:>14,.2}", "Monte Carlo", monte_carlo_var.var_amount);
    println!("{:<20} ${:>14,.2}", "CVaR", cvar);
    println!();

    // 8. Multiple confidence levels
    println!("--- VaR at Different Confidence Levels ---");
    for &conf_level in &[0.90, 0.95, 0.99, 0.999] {
        let var = engine.calculate_parametric_var(
            portfolio_value,
            std_dev,
            conf_level,
            time_horizon,
        )?;

        println!("{}% VaR: ${:>10,.2} ({}% chance of exceeding)",
            conf_level * 100.0,
            var.var_amount,
            (1.0 - conf_level) * 100.0
        );
    }
    println!();

    // 9. Multiple time horizons
    println!("--- VaR at Different Time Horizons ---");
    for &days in &[1, 5, 10, 21] {
        let var = engine.calculate_parametric_var(
            portfolio_value,
            std_dev,
            confidence_level,
            days,
        )?;

        println!("{:>2}-day VaR: ${:>10,.2}",
            days,
            var.var_amount
        );
    }
    println!();

    // 10. VaR Backtesting simulation
    println!("--- VaR Backtesting Example ---");

    // Simulate predictions and actual losses
    let mut predictions = Vec::new();
    let mut actual_losses = Vec::new();

    for _ in 0..100 {
        // Use historical VaR as prediction
        let pred = engine.calculate_historical_var(
            portfolio_value,
            0.95,
            1,
        )?;
        predictions.push(pred);

        // Simulate actual loss (random from our return distribution)
        let random_idx = (predictions.len() * 17) % returns.len();
        let actual_loss = -returns[random_idx] * portfolio_value;
        actual_losses.push(actual_loss);
    }

    let backtest = engine.backtest_var(predictions, actual_losses)?;

    println!("Predictions: {}", backtest.num_predictions);
    println!("Violations: {}", backtest.num_violations);
    println!("Violation Rate: {:.1}%", backtest.violation_rate * 100.0);
    println!("Expected Rate: {:.1}%", backtest.expected_violation_rate * 100.0);
    println!("Validated: {}", if backtest.validated { "PASS ✓" } else { "FAIL ✗" });

    println!("\n=== Example Complete ===");

    Ok(())
}

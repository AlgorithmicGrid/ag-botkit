//! Value at Risk (VaR) calculation engines
//!
//! Implements multiple VaR methodologies:
//! - Historical VaR: Empirical percentile from historical returns
//! - Parametric VaR: Assumes normal distribution (VaR = -μ + σ * Z_α * √T)
//! - Monte Carlo VaR: Simulation-based approach
//! - Conditional VaR (CVaR/Expected Shortfall): Average loss beyond VaR

use crate::advanced::error::{AdvancedRiskError, Result};
use chrono::{DateTime, Utc};
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal as StatrsNormal};

/// VaR calculation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VarMethod {
    Historical,
    Parametric,
    MonteCarlo,
}

/// VaR engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarConfig {
    /// Default number of Monte Carlo simulations
    pub default_simulations: usize,

    /// Minimum number of historical observations required
    pub min_observations: usize,

    /// Random seed for reproducible Monte Carlo (None = random)
    pub random_seed: Option<u64>,
}

impl Default for VarConfig {
    fn default() -> Self {
        Self {
            default_simulations: 10_000,
            min_observations: 30,
            random_seed: None,
        }
    }
}

/// VaR calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarResult {
    /// VaR amount (positive value represents potential loss)
    pub var_amount: f64,

    /// Confidence level (e.g., 0.95, 0.99)
    pub confidence_level: f64,

    /// Time horizon in days
    pub time_horizon_days: u32,

    /// Calculation method used
    pub method: VarMethod,

    /// Timestamp of calculation
    pub timestamp: DateTime<Utc>,
}

/// VaR backtesting result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarBacktestResult {
    /// Number of VaR predictions
    pub num_predictions: usize,

    /// Number of violations (actual loss exceeded VaR)
    pub num_violations: usize,

    /// Violation rate (num_violations / num_predictions)
    pub violation_rate: f64,

    /// Expected violation rate based on confidence level
    pub expected_violation_rate: f64,

    /// Whether the model is validated (violation rate within acceptable range)
    pub validated: bool,
}

/// VaR calculation engine
pub struct VarEngine {
    config: VarConfig,
    historical_returns: Vec<f64>,
}

impl VarEngine {
    /// Create a new VaR engine with configuration
    pub fn new(config: VarConfig) -> Self {
        Self {
            config,
            historical_returns: Vec::new(),
        }
    }

    /// Create a VaR engine with historical returns data
    pub fn with_historical_returns(config: VarConfig, returns: Vec<f64>) -> Self {
        Self {
            config,
            historical_returns: returns,
        }
    }

    /// Update historical returns (e.g., rolling window)
    pub fn update_historical_returns(&mut self, returns: Vec<f64>) {
        self.historical_returns = returns;
    }

    /// Add a new return observation
    pub fn add_return(&mut self, return_value: f64) {
        self.historical_returns.push(return_value);
    }

    /// Calculate Historical VaR using empirical percentile
    ///
    /// Formula: VaR_α = -percentile(returns, α) * portfolio_value * √T
    pub fn calculate_historical_var(
        &self,
        portfolio_value: f64,
        confidence_level: f64,
        time_horizon_days: u32,
    ) -> Result<VarResult> {
        self.validate_inputs(portfolio_value, confidence_level, time_horizon_days)?;

        if self.historical_returns.len() < self.config.min_observations {
            return Err(AdvancedRiskError::InsufficientData(format!(
                "Need at least {} observations, got {}",
                self.config.min_observations,
                self.historical_returns.len()
            )));
        }

        // Sort returns for percentile calculation
        let mut sorted_returns = self.historical_returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Calculate percentile index (lower tail for losses)
        let percentile = 1.0 - confidence_level;
        let index = (percentile * (sorted_returns.len() - 1) as f64).ceil() as usize;
        let percentile_return = sorted_returns[index.min(sorted_returns.len() - 1)];

        // Scale by portfolio value and time horizon
        let time_scaling = (time_horizon_days as f64).sqrt();
        let var_amount = -percentile_return * portfolio_value * time_scaling;

        Ok(VarResult {
            var_amount,
            confidence_level,
            time_horizon_days,
            method: VarMethod::Historical,
            timestamp: Utc::now(),
        })
    }

    /// Calculate Parametric VaR assuming normal distribution
    ///
    /// Formula: VaR = -μ + σ * Z_α * √T
    /// where Z_α is the inverse CDF of standard normal at confidence level α
    pub fn calculate_parametric_var(
        &self,
        portfolio_value: f64,
        volatility: f64,
        confidence_level: f64,
        time_horizon_days: u32,
    ) -> Result<VarResult> {
        self.validate_inputs(portfolio_value, confidence_level, time_horizon_days)?;

        if volatility < 0.0 {
            return Err(AdvancedRiskError::NegativeVolatility);
        }

        // Get Z-score for confidence level using inverse CDF
        let normal = StatrsNormal::new(0.0, 1.0)
            .map_err(|e| AdvancedRiskError::CalculationError(e.to_string()))?;

        let z_score = normal.inverse_cdf(1.0 - confidence_level);

        // Calculate VaR with time scaling
        let time_scaling = (time_horizon_days as f64).sqrt();
        let var_amount = -z_score * volatility * portfolio_value * time_scaling;

        Ok(VarResult {
            var_amount,
            confidence_level,
            time_horizon_days,
            method: VarMethod::Parametric,
            timestamp: Utc::now(),
        })
    }

    /// Calculate Monte Carlo VaR using simulated price paths
    ///
    /// Simulates portfolio returns assuming geometric Brownian motion
    pub fn calculate_monte_carlo_var(
        &self,
        portfolio_value: f64,
        mean_return: f64,
        volatility: f64,
        confidence_level: f64,
        time_horizon_days: u32,
        num_simulations: usize,
    ) -> Result<VarResult> {
        self.validate_inputs(portfolio_value, confidence_level, time_horizon_days)?;

        if volatility < 0.0 {
            return Err(AdvancedRiskError::NegativeVolatility);
        }

        if num_simulations == 0 {
            return Err(AdvancedRiskError::InvalidParameter(
                "Number of simulations must be positive".to_string()
            ));
        }

        // Create RNG
        let mut rng = match self.config.random_seed {
            Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
            None => rand::rngs::StdRng::from_entropy(),
        };

        let normal = Normal::new(0.0, 1.0)
            .map_err(|e| AdvancedRiskError::CalculationError(e.to_string()))?;

        // Time scaling
        let dt = time_horizon_days as f64 / 252.0; // Assuming 252 trading days
        let drift = (mean_return - 0.5 * volatility * volatility) * dt;
        let diffusion = volatility * dt.sqrt();

        // Run simulations
        let mut simulated_returns = Vec::with_capacity(num_simulations);
        for _ in 0..num_simulations {
            let z = normal.sample(&mut rng);
            let log_return = drift + diffusion * z;
            let portfolio_return = log_return.exp() - 1.0;
            simulated_returns.push(portfolio_return);
        }

        // Sort returns and extract percentile
        simulated_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let percentile = 1.0 - confidence_level;
        let index = (percentile * (simulated_returns.len() - 1) as f64).ceil() as usize;
        let percentile_return = simulated_returns[index.min(simulated_returns.len() - 1)];

        let var_amount = -percentile_return * portfolio_value;

        Ok(VarResult {
            var_amount,
            confidence_level,
            time_horizon_days,
            method: VarMethod::MonteCarlo,
            timestamp: Utc::now(),
        })
    }

    /// Calculate Conditional VaR (CVaR / Expected Shortfall)
    ///
    /// CVaR is the expected loss given that the loss exceeds VaR
    pub fn calculate_cvar(
        &self,
        portfolio_value: f64,
        confidence_level: f64,
        time_horizon_days: u32,
    ) -> Result<f64> {
        self.validate_inputs(portfolio_value, confidence_level, time_horizon_days)?;

        if self.historical_returns.len() < self.config.min_observations {
            return Err(AdvancedRiskError::InsufficientData(format!(
                "Need at least {} observations, got {}",
                self.config.min_observations,
                self.historical_returns.len()
            )));
        }

        // Sort returns
        let mut sorted_returns = self.historical_returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Find VaR threshold
        let percentile = 1.0 - confidence_level;
        let var_index = (percentile * (sorted_returns.len() - 1) as f64).ceil() as usize;

        // Calculate average of losses beyond VaR
        let tail_losses: Vec<f64> = sorted_returns.iter()
            .take(var_index + 1)
            .copied()
            .collect();

        if tail_losses.is_empty() {
            return Err(AdvancedRiskError::CalculationError(
                "No tail losses found for CVaR calculation".to_string()
            ));
        }

        let average_tail_loss = tail_losses.iter().sum::<f64>() / tail_losses.len() as f64;

        // Scale by portfolio value and time horizon
        let time_scaling = (time_horizon_days as f64).sqrt();
        let cvar = -average_tail_loss * portfolio_value * time_scaling;

        Ok(cvar)
    }

    /// Backtest VaR model by comparing predictions to actual losses
    pub fn backtest_var(
        &self,
        predictions: Vec<VarResult>,
        actual_losses: Vec<f64>,
    ) -> Result<VarBacktestResult> {
        if predictions.len() != actual_losses.len() {
            return Err(AdvancedRiskError::InvalidParameter(
                "Predictions and actual losses must have same length".to_string()
            ));
        }

        if predictions.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No predictions to backtest".to_string()
            ));
        }

        let num_predictions = predictions.len();
        let mut num_violations = 0;

        // Count violations (actual loss > predicted VaR)
        for (pred, actual_loss) in predictions.iter().zip(actual_losses.iter()) {
            if *actual_loss > pred.var_amount {
                num_violations += 1;
            }
        }

        let violation_rate = num_violations as f64 / num_predictions as f64;

        // Expected violation rate is (1 - confidence_level)
        let expected_violation_rate = 1.0 - predictions[0].confidence_level;

        // Simple validation: check if violation rate is within reasonable bounds
        // Using a 2-sigma confidence interval for binomial distribution
        let std_error = (expected_violation_rate * (1.0 - expected_violation_rate)
                        / num_predictions as f64).sqrt();
        let lower_bound = (expected_violation_rate - 2.0 * std_error).max(0.0);
        let upper_bound = (expected_violation_rate + 2.0 * std_error).min(1.0);

        let validated = violation_rate >= lower_bound && violation_rate <= upper_bound;

        Ok(VarBacktestResult {
            num_predictions,
            num_violations,
            violation_rate,
            expected_violation_rate,
            validated,
        })
    }

    /// Validate common input parameters
    fn validate_inputs(
        &self,
        portfolio_value: f64,
        confidence_level: f64,
        time_horizon_days: u32,
    ) -> Result<()> {
        if portfolio_value < 0.0 {
            return Err(AdvancedRiskError::NegativePrice);
        }

        if confidence_level <= 0.0 || confidence_level >= 1.0 {
            return Err(AdvancedRiskError::InvalidConfidenceLevel(confidence_level));
        }

        if time_horizon_days == 0 {
            return Err(AdvancedRiskError::InvalidTimeHorizon(time_horizon_days));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_returns() -> Vec<f64> {
        vec![
            -0.05, -0.03, -0.02, -0.01, 0.00,
            0.01, 0.02, 0.03, 0.04, 0.05,
            -0.04, 0.01, 0.02, -0.01, 0.03,
            0.00, -0.02, 0.01, 0.02, -0.01,
        ]
    }

    #[test]
    fn test_historical_var() {
        let returns = create_test_returns();
        let engine = VarEngine::with_historical_returns(VarConfig::default(), returns);

        let result = engine.calculate_historical_var(10000.0, 0.95, 1).unwrap();

        assert!(result.var_amount > 0.0);
        assert_eq!(result.confidence_level, 0.95);
        assert_eq!(result.time_horizon_days, 1);
        assert_eq!(result.method, VarMethod::Historical);
    }

    #[test]
    fn test_parametric_var() {
        let engine = VarEngine::new(VarConfig::default());

        let result = engine.calculate_parametric_var(
            10000.0,  // portfolio value
            0.02,     // 2% daily volatility
            0.95,     // 95% confidence
            1,        // 1 day
        ).unwrap();

        assert!(result.var_amount > 0.0);
        assert_eq!(result.method, VarMethod::Parametric);
    }

    #[test]
    fn test_monte_carlo_var() {
        let config = VarConfig {
            random_seed: Some(42),
            ..Default::default()
        };
        let engine = VarEngine::new(config);

        let result = engine.calculate_monte_carlo_var(
            10000.0,  // portfolio value
            0.0001,   // mean daily return
            0.02,     // 2% daily volatility
            0.95,     // 95% confidence
            1,        // 1 day
            1000,     // simulations
        ).unwrap();

        assert!(result.var_amount > 0.0);
        assert_eq!(result.method, VarMethod::MonteCarlo);
    }

    #[test]
    fn test_cvar() {
        let returns = create_test_returns();
        let engine = VarEngine::with_historical_returns(VarConfig::default(), returns);

        let cvar = engine.calculate_cvar(10000.0, 0.95, 1).unwrap();

        assert!(cvar > 0.0);
    }

    #[test]
    fn test_var_backtest() {
        let engine = VarEngine::new(VarConfig::default());

        let predictions = vec![
            VarResult {
                var_amount: 100.0,
                confidence_level: 0.95,
                time_horizon_days: 1,
                method: VarMethod::Parametric,
                timestamp: Utc::now(),
            };
            100
        ];

        // Simulate ~5% violations (expected for 95% confidence)
        let mut actual_losses = vec![50.0; 100];
        actual_losses[0] = 150.0;  // 1 violation
        actual_losses[1] = 120.0;  // 2 violations
        actual_losses[2] = 110.0;  // 3 violations
        actual_losses[3] = 105.0;  // 4 violations
        actual_losses[4] = 101.0;  // 5 violations

        let backtest = engine.backtest_var(predictions, actual_losses).unwrap();

        assert_eq!(backtest.num_predictions, 100);
        assert_eq!(backtest.num_violations, 5);
        assert_eq!(backtest.violation_rate, 0.05);
        assert!((backtest.expected_violation_rate - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_invalid_confidence_level() {
        let engine = VarEngine::new(VarConfig::default());

        let result = engine.calculate_parametric_var(10000.0, 0.02, 1.5, 1);
        assert!(result.is_err());

        let result = engine.calculate_parametric_var(10000.0, 0.02, -0.1, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_insufficient_data() {
        let engine = VarEngine::with_historical_returns(
            VarConfig::default(),
            vec![0.01, 0.02],  // Only 2 observations, need 30
        );

        let result = engine.calculate_historical_var(10000.0, 0.95, 1);
        assert!(result.is_err());
    }
}

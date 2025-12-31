//! Options Greeks calculation engine
//!
//! Implements Black-Scholes Greeks calculation for European options:
//! - Delta (∂V/∂S): Sensitivity to underlying price
//! - Gamma (∂²V/∂S²): Rate of change of Delta
//! - Vega (∂V/∂σ): Sensitivity to volatility
//! - Theta (∂V/∂t): Time decay
//! - Rho (∂V/∂r): Sensitivity to interest rate

use crate::advanced::error::{AdvancedRiskError, Result};
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Normal};
use std::collections::HashMap;
use std::f64::consts::PI;

/// Option type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionType {
    Call,
    Put,
}

/// Option contract details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Option {
    /// Option type (Call/Put)
    pub option_type: OptionType,

    /// Strike price
    pub strike: f64,

    /// Time to expiration in years
    pub time_to_expiry: f64,

    /// Contract size/multiplier
    pub contract_size: f64,
}

/// Position in an option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionPosition {
    /// Option contract
    pub option: Option,

    /// Position size (positive = long, negative = short)
    pub quantity: f64,

    /// Underlying asset identifier
    pub underlying: String,
}

/// Market data for Greeks calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Underlying prices by asset identifier
    pub underlying_prices: HashMap<String, f64>,

    /// Implied volatilities by asset identifier
    pub volatilities: HashMap<String, f64>,

    /// Risk-free interest rate
    pub risk_free_rate: f64,
}

/// Greeks for a single option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Greeks {
    /// Delta: ∂V/∂S
    pub delta: f64,

    /// Gamma: ∂²V/∂S²
    pub gamma: f64,

    /// Vega: ∂V/∂σ (per 1% change in volatility)
    pub vega: f64,

    /// Theta: ∂V/∂t (per day)
    pub theta: f64,

    /// Rho: ∂V/∂r (per 1% change in interest rate)
    pub rho: f64,
}

/// Portfolio-level Greeks aggregated across all positions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioGreeks {
    /// Total portfolio Delta
    pub total_delta: f64,

    /// Total portfolio Gamma
    pub total_gamma: f64,

    /// Total portfolio Vega
    pub total_vega: f64,

    /// Total portfolio Theta
    pub total_theta: f64,

    /// Total portfolio Rho
    pub total_rho: f64,

    /// Greeks broken down by underlying asset
    pub by_underlying: HashMap<String, Greeks>,
}

/// Hedge recommendation to neutralize Greeks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgeRecommendation {
    /// Instrument to use for hedging
    pub instrument: String,

    /// Quantity to trade (positive = buy, negative = sell)
    pub quantity: f64,

    /// Target Greek this hedge addresses
    pub target_greek: String,

    /// Expected impact on portfolio Greeks
    pub expected_impact: f64,
}

/// Greeks engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreeksConfig {
    /// Finite difference step size for numerical Greeks (default: 0.01)
    pub fd_step: f64,

    /// Minimum time to expiry to calculate Greeks (default: 1 hour)
    pub min_time_to_expiry_hours: f64,
}

impl Default for GreeksConfig {
    fn default() -> Self {
        Self {
            fd_step: 0.01,
            min_time_to_expiry_hours: 1.0 / 24.0,
        }
    }
}

/// Greeks calculation engine
pub struct GreeksEngine {
    config: GreeksConfig,
}

impl GreeksEngine {
    /// Create a new Greeks engine with configuration
    pub fn new(config: GreeksConfig) -> Self {
        Self { config }
    }

    /// Calculate all Greeks for a single option using Black-Scholes
    pub fn calculate_greeks(
        &self,
        option: &Option,
        underlying_price: f64,
        volatility: f64,
        risk_free_rate: f64,
    ) -> Result<Greeks> {
        self.validate_inputs(option, underlying_price, volatility)?;

        let s = underlying_price;
        let k = option.strike;
        let t = option.time_to_expiry;
        let sigma = volatility;
        let r = risk_free_rate;

        // Calculate d1 and d2 for Black-Scholes
        let d1 = self.calculate_d1(s, k, t, sigma, r)?;
        let d2 = d1 - sigma * t.sqrt();

        let normal = Normal::new(0.0, 1.0)
            .map_err(|e| AdvancedRiskError::CalculationError(e.to_string()))?;

        let n_d1 = normal.cdf(d1);
        let n_d2 = normal.cdf(d2);
        let n_prime_d1 = self.normal_pdf(d1);

        // Calculate Greeks based on option type
        let (delta, rho) = match option.option_type {
            OptionType::Call => {
                let delta = n_d1;
                let rho = k * t * (-r * t).exp() * n_d2 / 100.0;
                (delta, rho)
            }
            OptionType::Put => {
                let delta = n_d1 - 1.0;
                let rho = -k * t * (-r * t).exp() * normal.cdf(-d2) / 100.0;
                (delta, rho)
            }
        };

        // Gamma is the same for calls and puts
        let gamma = n_prime_d1 / (s * sigma * t.sqrt());

        // Vega is the same for calls and puts (per 1% volatility change)
        let vega = s * n_prime_d1 * t.sqrt() / 100.0;

        // Theta (per day)
        let theta = match option.option_type {
            OptionType::Call => {
                (-s * n_prime_d1 * sigma / (2.0 * t.sqrt())
                    - r * k * (-r * t).exp() * n_d2) / 365.0
            }
            OptionType::Put => {
                (-s * n_prime_d1 * sigma / (2.0 * t.sqrt())
                    + r * k * (-r * t).exp() * normal.cdf(-d2)) / 365.0
            }
        };

        Ok(Greeks {
            delta,
            gamma,
            vega,
            theta,
            rho,
        })
    }

    /// Calculate portfolio Greeks aggregated across all positions
    pub fn calculate_portfolio_greeks(
        &self,
        positions: &[OptionPosition],
        market_data: &MarketData,
    ) -> Result<PortfolioGreeks> {
        let mut total_delta = 0.0;
        let mut total_gamma = 0.0;
        let mut total_vega = 0.0;
        let mut total_theta = 0.0;
        let mut total_rho = 0.0;

        let mut by_underlying: HashMap<String, Greeks> = HashMap::new();

        for position in positions {
            let underlying_price = market_data.underlying_prices
                .get(&position.underlying)
                .ok_or_else(|| {
                    AdvancedRiskError::InvalidParameter(format!(
                        "Missing price for underlying: {}",
                        position.underlying
                    ))
                })?;

            let volatility = market_data.volatilities
                .get(&position.underlying)
                .ok_or_else(|| {
                    AdvancedRiskError::InvalidParameter(format!(
                        "Missing volatility for underlying: {}",
                        position.underlying
                    ))
                })?;

            let greeks = self.calculate_greeks(
                &position.option,
                *underlying_price,
                *volatility,
                market_data.risk_free_rate,
            )?;

            // Scale by position size
            let scaled_delta = greeks.delta * position.quantity * position.option.contract_size;
            let scaled_gamma = greeks.gamma * position.quantity * position.option.contract_size;
            let scaled_vega = greeks.vega * position.quantity * position.option.contract_size;
            let scaled_theta = greeks.theta * position.quantity * position.option.contract_size;
            let scaled_rho = greeks.rho * position.quantity * position.option.contract_size;

            total_delta += scaled_delta;
            total_gamma += scaled_gamma;
            total_vega += scaled_vega;
            total_theta += scaled_theta;
            total_rho += scaled_rho;

            // Aggregate by underlying
            by_underlying
                .entry(position.underlying.clone())
                .and_modify(|g| {
                    g.delta += scaled_delta;
                    g.gamma += scaled_gamma;
                    g.vega += scaled_vega;
                    g.theta += scaled_theta;
                    g.rho += scaled_rho;
                })
                .or_insert(Greeks {
                    delta: scaled_delta,
                    gamma: scaled_gamma,
                    vega: scaled_vega,
                    theta: scaled_theta,
                    rho: scaled_rho,
                });
        }

        Ok(PortfolioGreeks {
            total_delta,
            total_gamma,
            total_vega,
            total_theta,
            total_rho,
            by_underlying,
        })
    }

    /// Suggest hedge to move current Greeks toward target Greeks
    pub fn suggest_hedge(
        &self,
        current_greeks: &PortfolioGreeks,
        target_greeks: &PortfolioGreeks,
        available_instruments: &[String],
    ) -> Result<Vec<HedgeRecommendation>> {
        let mut recommendations = Vec::new();

        // Delta hedge recommendation
        let delta_diff = target_greeks.total_delta - current_greeks.total_delta;
        if delta_diff.abs() > 0.01 {
            // Simple delta hedge using underlying
            if let Some(underlying) = available_instruments.first() {
                recommendations.push(HedgeRecommendation {
                    instrument: underlying.clone(),
                    quantity: delta_diff,
                    target_greek: "Delta".to_string(),
                    expected_impact: delta_diff,
                });
            }
        }

        // Vega hedge recommendation
        let vega_diff = target_greeks.total_vega - current_greeks.total_vega;
        if vega_diff.abs() > 0.01 {
            recommendations.push(HedgeRecommendation {
                instrument: "ATM_Option".to_string(),
                quantity: vega_diff,
                target_greek: "Vega".to_string(),
                expected_impact: vega_diff,
            });
        }

        Ok(recommendations)
    }

    /// Calculate d1 parameter for Black-Scholes
    fn calculate_d1(
        &self,
        s: f64,
        k: f64,
        t: f64,
        sigma: f64,
        r: f64,
    ) -> Result<f64> {
        if sigma <= 0.0 || t <= 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Volatility and time must be positive".to_string()
            ));
        }

        let numerator = (s / k).ln() + (r + 0.5 * sigma * sigma) * t;
        let denominator = sigma * t.sqrt();

        Ok(numerator / denominator)
    }

    /// Calculate standard normal PDF
    fn normal_pdf(&self, x: f64) -> f64 {
        (1.0 / (2.0 * PI).sqrt()) * (-0.5 * x * x).exp()
    }

    /// Validate inputs for Greeks calculation
    fn validate_inputs(
        &self,
        option: &Option,
        underlying_price: f64,
        volatility: f64,
    ) -> Result<()> {
        if underlying_price <= 0.0 {
            return Err(AdvancedRiskError::NegativePrice);
        }

        if option.strike <= 0.0 {
            return Err(AdvancedRiskError::InvalidParameter(
                "Strike price must be positive".to_string()
            ));
        }

        if volatility <= 0.0 {
            return Err(AdvancedRiskError::NegativeVolatility);
        }

        if option.time_to_expiry <= 0.0 {
            return Err(AdvancedRiskError::InvalidParameter(
                "Time to expiry must be positive".to_string()
            ));
        }

        let min_time = self.config.min_time_to_expiry_hours / (365.0 * 24.0);
        if option.time_to_expiry < min_time {
            return Err(AdvancedRiskError::InvalidParameter(
                format!("Time to expiry too small: {} years", option.time_to_expiry)
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_option_greeks() {
        let engine = GreeksEngine::new(GreeksConfig::default());

        let option = Option {
            option_type: OptionType::Call,
            strike: 100.0,
            time_to_expiry: 1.0, // 1 year
            contract_size: 100.0,
        };

        let greeks = engine.calculate_greeks(
            &option,
            100.0,  // underlying price (at-the-money)
            0.20,   // 20% volatility
            0.05,   // 5% risk-free rate
        ).unwrap();

        // Delta for ATM call should be around 0.5
        assert!(greeks.delta > 0.4 && greeks.delta < 0.6);

        // Gamma should be positive
        assert!(greeks.gamma > 0.0);

        // Vega should be positive
        assert!(greeks.vega > 0.0);

        // Theta should be negative for long call
        assert!(greeks.theta < 0.0);

        // Rho should be positive for call
        assert!(greeks.rho > 0.0);
    }

    #[test]
    fn test_put_option_greeks() {
        let engine = GreeksEngine::new(GreeksConfig::default());

        let option = Option {
            option_type: OptionType::Put,
            strike: 100.0,
            time_to_expiry: 1.0,
            contract_size: 100.0,
        };

        let greeks = engine.calculate_greeks(
            &option,
            100.0,
            0.20,
            0.05,
        ).unwrap();

        // Delta for ATM put should be around -0.5
        assert!(greeks.delta < -0.4 && greeks.delta > -0.6);

        // Gamma should be positive
        assert!(greeks.gamma > 0.0);

        // Vega should be positive
        assert!(greeks.vega > 0.0);

        // Theta should be negative for long put
        assert!(greeks.theta < 0.0);

        // Rho should be negative for put
        assert!(greeks.rho < 0.0);
    }

    #[test]
    fn test_portfolio_greeks() {
        let engine = GreeksEngine::new(GreeksConfig::default());

        let positions = vec![
            OptionPosition {
                option: Option {
                    option_type: OptionType::Call,
                    strike: 100.0,
                    time_to_expiry: 1.0,
                    contract_size: 100.0,
                },
                quantity: 10.0,
                underlying: "SPY".to_string(),
            },
            OptionPosition {
                option: Option {
                    option_type: OptionType::Put,
                    strike: 100.0,
                    time_to_expiry: 1.0,
                    contract_size: 100.0,
                },
                quantity: -5.0,  // Short 5 puts
                underlying: "SPY".to_string(),
            },
        ];

        let mut market_data = MarketData {
            underlying_prices: HashMap::new(),
            volatilities: HashMap::new(),
            risk_free_rate: 0.05,
        };
        market_data.underlying_prices.insert("SPY".to_string(), 100.0);
        market_data.volatilities.insert("SPY".to_string(), 0.20);

        let portfolio_greeks = engine.calculate_portfolio_greeks(&positions, &market_data).unwrap();

        // Portfolio should have net positive delta (10 long calls, 5 short puts)
        assert!(portfolio_greeks.total_delta > 0.0);

        // Check that by_underlying aggregation works
        assert!(portfolio_greeks.by_underlying.contains_key("SPY"));
    }

    #[test]
    fn test_invalid_inputs() {
        let engine = GreeksEngine::new(GreeksConfig::default());

        let option = Option {
            option_type: OptionType::Call,
            strike: 100.0,
            time_to_expiry: 1.0,
            contract_size: 100.0,
        };

        // Negative underlying price
        let result = engine.calculate_greeks(&option, -100.0, 0.20, 0.05);
        assert!(result.is_err());

        // Negative volatility
        let result = engine.calculate_greeks(&option, 100.0, -0.20, 0.05);
        assert!(result.is_err());
    }
}

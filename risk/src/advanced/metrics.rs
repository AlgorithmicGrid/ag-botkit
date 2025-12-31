//! Performance metrics calculation
//!
//! Implements risk-adjusted performance metrics:
//! - Sharpe Ratio: (Return - Risk-Free Rate) / Volatility
//! - Sortino Ratio: Uses downside deviation instead of total volatility
//! - Calmar Ratio: Return / Maximum Drawdown
//! - Maximum Drawdown: Largest peak-to-trough decline
//! - Beta: Sensitivity to market movements
//! - Alpha: Excess return over market

use crate::advanced::error::{AdvancedRiskError, Result};
use serde::{Deserialize, Serialize};

/// Performance metrics calculator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Portfolio returns time series
    returns: Vec<f64>,

    /// Risk-free rate (annualized)
    risk_free_rate: f64,
}

impl PerformanceMetrics {
    /// Create a new performance metrics calculator
    pub fn new(returns: Vec<f64>, risk_free_rate: f64) -> Self {
        Self {
            returns,
            risk_free_rate,
        }
    }

    /// Calculate Sharpe Ratio
    ///
    /// Sharpe = (Mean Return - Risk-Free Rate) / Standard Deviation
    /// Annualized by multiplying by √252 (trading days)
    pub fn sharpe_ratio(&self) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let mean_return = self.mean_return();
        let std_dev = self.standard_deviation()?;

        if std_dev == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Standard deviation is zero".to_string()
            ));
        }

        // Convert annual risk-free rate to daily
        let daily_rf = self.risk_free_rate / 252.0;

        // Calculate daily Sharpe and annualize
        let daily_sharpe = (mean_return - daily_rf) / std_dev;
        let annualized_sharpe = daily_sharpe * (252.0_f64).sqrt();

        Ok(annualized_sharpe)
    }

    /// Calculate Sortino Ratio
    ///
    /// Sortino = (Mean Return - MAR) / Downside Deviation
    /// Only considers downside volatility (returns below minimum acceptable return)
    pub fn sortino_ratio(&self, minimum_acceptable_return: f64) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let mean_return = self.mean_return();
        let downside_dev = self.downside_deviation(minimum_acceptable_return)?;

        if downside_dev == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Downside deviation is zero".to_string()
            ));
        }

        // Convert annual MAR to daily
        let daily_mar = minimum_acceptable_return / 252.0;

        // Calculate daily Sortino and annualize
        let daily_sortino = (mean_return - daily_mar) / downside_dev;
        let annualized_sortino = daily_sortino * (252.0_f64).sqrt();

        Ok(annualized_sortino)
    }

    /// Calculate Maximum Drawdown
    ///
    /// Max DD = (Trough Value - Peak Value) / Peak Value
    /// Returns the largest peak-to-trough decline in percentage
    pub fn max_drawdown(&self) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        // Convert returns to cumulative wealth
        let mut cumulative_wealth = vec![1.0];
        for ret in &self.returns {
            let new_wealth = cumulative_wealth.last().unwrap() * (1.0 + ret);
            cumulative_wealth.push(new_wealth);
        }

        let mut max_dd = 0.0;
        let mut peak = cumulative_wealth[0];

        for &wealth in &cumulative_wealth {
            if wealth > peak {
                peak = wealth;
            }

            let drawdown = (wealth - peak) / peak;
            if drawdown < max_dd {
                max_dd = drawdown;
            }
        }

        Ok(max_dd)
    }

    /// Calculate Calmar Ratio
    ///
    /// Calmar = Annualized Return / Absolute Maximum Drawdown
    pub fn calmar_ratio(&self) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let annualized_return = self.annualized_return();
        let max_dd = self.max_drawdown()?.abs();

        if max_dd == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Maximum drawdown is zero".to_string()
            ));
        }

        Ok(annualized_return / max_dd)
    }

    /// Calculate Beta relative to market
    ///
    /// Beta = Covariance(Portfolio, Market) / Variance(Market)
    pub fn beta(&self, market_returns: &[f64]) -> Result<f64> {
        if self.returns.len() != market_returns.len() {
            return Err(AdvancedRiskError::InvalidParameter(
                "Portfolio and market returns must have same length".to_string()
            ));
        }

        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let port_mean = self.mean_return();
        let market_mean = Self::calculate_mean(market_returns);

        // Calculate covariance
        let covariance: f64 = self.returns
            .iter()
            .zip(market_returns.iter())
            .map(|(p, m)| (p - port_mean) * (m - market_mean))
            .sum::<f64>()
            / (self.returns.len() - 1) as f64;

        // Calculate market variance
        let market_variance: f64 = market_returns
            .iter()
            .map(|m| (m - market_mean).powi(2))
            .sum::<f64>()
            / (market_returns.len() - 1) as f64;

        if market_variance == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Market variance is zero".to_string()
            ));
        }

        Ok(covariance / market_variance)
    }

    /// Calculate Alpha relative to market
    ///
    /// Alpha = Portfolio Return - (Risk-Free Rate + Beta * (Market Return - Risk-Free Rate))
    pub fn alpha(&self, market_returns: &[f64]) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let portfolio_return = self.annualized_return();
        let market_return = Self::annualize_return(
            Self::calculate_mean(market_returns),
            self.returns.len(),
        );
        let beta = self.beta(market_returns)?;

        // CAPM expected return
        let expected_return = self.risk_free_rate + beta * (market_return - self.risk_free_rate);

        // Alpha is actual return minus expected return
        Ok(portfolio_return - expected_return)
    }

    /// Calculate win rate (percentage of positive returns)
    pub fn win_rate(&self) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let wins = self.returns.iter().filter(|&&r| r > 0.0).count();
        Ok(wins as f64 / self.returns.len() as f64)
    }

    /// Calculate profit factor (sum of gains / sum of losses)
    pub fn profit_factor(&self) -> Result<f64> {
        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        let gains: f64 = self.returns.iter().filter(|&&r| r > 0.0).sum();
        let losses: f64 = self.returns.iter().filter(|&&r| r < 0.0).sum::<f64>().abs();

        if losses == 0.0 {
            return Ok(f64::INFINITY);
        }

        Ok(gains / losses)
    }

    /// Calculate tracking error relative to market
    pub fn tracking_error(&self, market_returns: &[f64]) -> Result<f64> {
        if self.returns.len() != market_returns.len() {
            return Err(AdvancedRiskError::InvalidParameter(
                "Portfolio and market returns must have same length".to_string()
            ));
        }

        if self.returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No returns data".to_string()
            ));
        }

        // Calculate excess returns
        let excess_returns: Vec<f64> = self.returns
            .iter()
            .zip(market_returns.iter())
            .map(|(p, m)| p - m)
            .collect();

        // Tracking error is std dev of excess returns, annualized
        let mean_excess = Self::calculate_mean(&excess_returns);
        let variance: f64 = excess_returns
            .iter()
            .map(|e| (e - mean_excess).powi(2))
            .sum::<f64>()
            / (excess_returns.len() - 1) as f64;

        let daily_te = variance.sqrt();
        let annualized_te = daily_te * (252.0_f64).sqrt();

        Ok(annualized_te)
    }

    /// Calculate mean return
    fn mean_return(&self) -> f64 {
        Self::calculate_mean(&self.returns)
    }

    /// Calculate standard deviation of returns
    fn standard_deviation(&self) -> Result<f64> {
        if self.returns.len() < 2 {
            return Err(AdvancedRiskError::InsufficientData(
                "Need at least 2 returns for standard deviation".to_string()
            ));
        }

        let mean = self.mean_return();
        let variance: f64 = self.returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (self.returns.len() - 1) as f64;

        Ok(variance.sqrt())
    }

    /// Calculate downside deviation (only negative returns)
    fn downside_deviation(&self, target: f64) -> Result<f64> {
        let downside_returns: Vec<f64> = self.returns
            .iter()
            .filter(|&&r| r < target)
            .map(|&r| r - target)
            .collect();

        if downside_returns.is_empty() {
            return Ok(0.0);
        }

        let variance: f64 = downside_returns
            .iter()
            .map(|r| r.powi(2))
            .sum::<f64>()
            / downside_returns.len() as f64;

        Ok(variance.sqrt())
    }

    /// Calculate annualized return from daily returns
    fn annualized_return(&self) -> f64 {
        let mean_daily = self.mean_return();
        Self::annualize_return(mean_daily, self.returns.len())
    }

    /// Helper to annualize a daily return
    fn annualize_return(daily_return: f64, _num_days: usize) -> f64 {
        // Compound return: (1 + r)^252 - 1
        (1.0 + daily_return).powf(252.0) - 1.0
    }

    /// Helper to calculate mean of a slice
    fn calculate_mean(data: &[f64]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        data.iter().sum::<f64>() / data.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_returns() -> Vec<f64> {
        vec![
            0.01, 0.02, -0.01, 0.015, -0.005,
            0.03, -0.02, 0.01, 0.005, -0.01,
            0.02, 0.01, -0.015, 0.025, 0.01,
            -0.005, 0.015, 0.02, -0.01, 0.005,
        ]
    }

    fn create_market_returns() -> Vec<f64> {
        vec![
            0.008, 0.015, -0.012, 0.01, -0.008,
            0.025, -0.018, 0.012, 0.003, -0.015,
            0.018, 0.009, -0.02, 0.022, 0.012,
            -0.007, 0.013, 0.017, -0.012, 0.004,
        ]
    }

    #[test]
    fn test_sharpe_ratio() {
        let returns = create_test_returns();
        let metrics = PerformanceMetrics::new(returns, 0.02); // 2% risk-free rate

        let sharpe = metrics.sharpe_ratio().unwrap();

        // Sharpe should be positive for positive average returns
        assert!(sharpe > 0.0);
    }

    #[test]
    fn test_sortino_ratio() {
        let returns = create_test_returns();
        let metrics = PerformanceMetrics::new(returns, 0.02);

        let sortino = metrics.sortino_ratio(0.0).unwrap();

        // Sortino should be positive
        assert!(sortino > 0.0);

        // Sortino should generally be higher than Sharpe (only penalizes downside)
        let sharpe = metrics.sharpe_ratio().unwrap();
        assert!(sortino >= sharpe);
    }

    #[test]
    fn test_max_drawdown() {
        let returns = vec![0.10, 0.05, -0.20, -0.10, 0.15, 0.05];
        let metrics = PerformanceMetrics::new(returns, 0.02);

        let max_dd = metrics.max_drawdown().unwrap();

        // Max drawdown should be negative
        assert!(max_dd < 0.0);

        // Should be approximately -0.26 (drawdown from peak after 0.05 to trough after -0.10)
        assert!(max_dd < -0.20);
    }

    #[test]
    fn test_calmar_ratio() {
        let returns = create_test_returns();
        let metrics = PerformanceMetrics::new(returns, 0.02);

        let calmar = metrics.calmar_ratio().unwrap();

        // Calmar should be positive for positive returns
        assert!(calmar > 0.0);
    }

    #[test]
    fn test_beta() {
        let portfolio_returns = create_test_returns();
        let market_returns = create_market_returns();
        let metrics = PerformanceMetrics::new(portfolio_returns, 0.02);

        let beta = metrics.beta(&market_returns).unwrap();

        // Beta should be positive for correlated returns
        assert!(beta > 0.0);

        // Beta close to 1.0 indicates similar volatility to market
        assert!(beta > 0.5 && beta < 2.0);
    }

    #[test]
    fn test_alpha() {
        let portfolio_returns = create_test_returns();
        let market_returns = create_market_returns();
        let metrics = PerformanceMetrics::new(portfolio_returns, 0.02);

        let alpha = metrics.alpha(&market_returns).unwrap();

        // Alpha can be positive or negative
        // Just verify it calculates without error
        assert!(alpha.is_finite());
    }

    #[test]
    fn test_win_rate() {
        let returns = vec![0.01, 0.02, -0.01, 0.01, -0.02];
        let metrics = PerformanceMetrics::new(returns, 0.02);

        let win_rate = metrics.win_rate().unwrap();

        // 3 positive out of 5 = 60%
        assert!((win_rate - 0.6).abs() < 1e-6);
    }

    #[test]
    fn test_profit_factor() {
        let returns = vec![0.10, 0.05, -0.03, 0.08, -0.02];
        let metrics = PerformanceMetrics::new(returns, 0.02);

        let pf = metrics.profit_factor().unwrap();

        // Gains = 0.23, Losses = 0.05, PF = 4.6
        assert!((pf - 4.6).abs() < 0.1);
    }

    #[test]
    fn test_tracking_error() {
        let portfolio_returns = create_test_returns();
        let market_returns = create_market_returns();
        let metrics = PerformanceMetrics::new(portfolio_returns, 0.02);

        let te = metrics.tracking_error(&market_returns).unwrap();

        // Tracking error should be positive
        assert!(te > 0.0);

        // Annualized tracking error should be reasonable
        assert!(te < 1.0); // Less than 100%
    }

    #[test]
    fn test_insufficient_data() {
        let metrics = PerformanceMetrics::new(vec![], 0.02);

        assert!(metrics.sharpe_ratio().is_err());
        assert!(metrics.max_drawdown().is_err());
        assert!(metrics.win_rate().is_err());
    }

    #[test]
    fn test_zero_volatility() {
        let returns = vec![0.01; 20]; // Constant returns
        let metrics = PerformanceMetrics::new(returns, 0.02);

        // Sharpe should error with zero volatility
        assert!(metrics.sharpe_ratio().is_err());
    }
}

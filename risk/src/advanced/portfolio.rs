//! Portfolio risk analytics
//!
//! Provides portfolio-level risk metrics including:
//! - Portfolio volatility calculation using covariance matrix
//! - Correlation matrix computation from historical returns
//! - Risk contribution analysis (component VaR, marginal VaR)
//! - Diversification metrics
//! - Concentration risk measures

use crate::advanced::error::{AdvancedRiskError, Result};
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Position in a portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Asset identifier
    pub asset_id: String,

    /// Position value in USD
    pub value_usd: f64,

    /// Position weight in portfolio (0.0 to 1.0)
    pub weight: f64,
}

/// Risk contribution for a single position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskContribution {
    /// Asset identifier
    pub asset_id: String,

    /// Contribution to total portfolio variance
    pub variance_contribution: f64,

    /// Contribution to total portfolio volatility
    pub volatility_contribution: f64,

    /// Percentage of total portfolio risk
    pub risk_pct: f64,
}

/// Marginal VaR result for a position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginalVarResult {
    /// Asset identifier
    pub asset_id: String,

    /// Marginal VaR: change in portfolio VaR per unit change in position
    pub marginal_var: f64,

    /// Component VaR: marginal VaR * position size
    pub component_var: f64,
}

/// Portfolio analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioConfig {
    /// Minimum number of observations for correlation calculation
    pub min_observations: usize,

    /// Regularization parameter for covariance matrix (default: 1e-6)
    pub regularization: f64,
}

impl Default for PortfolioConfig {
    fn default() -> Self {
        Self {
            min_observations: 30,
            regularization: 1e-6,
        }
    }
}

/// Portfolio risk analyzer
pub struct PortfolioAnalyzer {
    config: PortfolioConfig,
}

impl PortfolioAnalyzer {
    /// Create a new portfolio analyzer
    pub fn new(config: PortfolioConfig) -> Self {
        Self { config }
    }

    /// Calculate portfolio volatility using σ_p = √(w^T Σ w)
    ///
    /// where w is the weight vector and Σ is the covariance matrix
    pub fn calculate_volatility(
        &self,
        positions: &[Position],
        correlation_matrix: &DMatrix<f64>,
    ) -> Result<f64> {
        if positions.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No positions provided".to_string()
            ));
        }

        // Extract weights and create weight vector
        let weights: Vec<f64> = positions.iter().map(|p| p.weight).collect();
        let w = DVector::from_vec(weights.clone());

        // Verify correlation matrix dimensions
        if correlation_matrix.nrows() != positions.len()
            || correlation_matrix.ncols() != positions.len()
        {
            return Err(AdvancedRiskError::MatrixError(
                "Correlation matrix dimensions don't match positions".to_string()
            ));
        }

        // Calculate portfolio variance: w^T Σ w
        let variance = (&w.transpose() * correlation_matrix * &w)[(0, 0)];

        if variance < 0.0 {
            return Err(AdvancedRiskError::NumericalInstability(
                format!("Negative portfolio variance: {}", variance)
            ));
        }

        Ok(variance.sqrt())
    }

    /// Calculate correlation matrix from historical returns
    ///
    /// Returns an n×n correlation matrix where n is the number of assets
    pub fn calculate_correlation_matrix(
        &self,
        returns: &HashMap<String, Vec<f64>>,
    ) -> Result<DMatrix<f64>> {
        if returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No return data provided".to_string()
            ));
        }

        // Validate all return series have same length
        let num_obs = returns.values().next().unwrap().len();
        if num_obs < self.config.min_observations {
            return Err(AdvancedRiskError::InsufficientData(format!(
                "Need at least {} observations, got {}",
                self.config.min_observations, num_obs
            )));
        }

        for (asset, data) in returns {
            if data.len() != num_obs {
                return Err(AdvancedRiskError::InvalidParameter(format!(
                    "Asset {} has {} observations, expected {}",
                    asset,
                    data.len(),
                    num_obs
                )));
            }
        }

        // Sort assets for consistent ordering
        let mut assets: Vec<_> = returns.keys().collect();
        assets.sort();
        let n = assets.len();

        // Calculate means
        let mut means = HashMap::new();
        for asset in &assets {
            let data = &returns[*asset];
            let mean = data.iter().sum::<f64>() / data.len() as f64;
            means.insert(*asset, mean);
        }

        // Calculate covariance matrix
        let mut cov_matrix = DMatrix::zeros(n, n);

        for (i, asset_i) in assets.iter().enumerate() {
            for (j, asset_j) in assets.iter().enumerate() {
                let returns_i = &returns[*asset_i];
                let returns_j = &returns[*asset_j];
                let mean_i = means[asset_i];
                let mean_j = means[asset_j];

                let covariance: f64 = returns_i
                    .iter()
                    .zip(returns_j.iter())
                    .map(|(r_i, r_j)| (r_i - mean_i) * (r_j - mean_j))
                    .sum::<f64>()
                    / (num_obs - 1) as f64;

                cov_matrix[(i, j)] = covariance;
            }
        }

        // Add regularization to diagonal for numerical stability
        for i in 0..n {
            cov_matrix[(i, i)] += self.config.regularization;
        }

        // Convert covariance to correlation
        let mut corr_matrix = DMatrix::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                let std_i = cov_matrix[(i, i)].sqrt();
                let std_j = cov_matrix[(j, j)].sqrt();

                if std_i == 0.0 || std_j == 0.0 {
                    return Err(AdvancedRiskError::DivisionByZero(
                        format!("Zero standard deviation for asset index {}", i)
                    ));
                }

                corr_matrix[(i, j)] = cov_matrix[(i, j)] / (std_i * std_j);
            }
        }

        Ok(corr_matrix)
    }

    /// Calculate covariance matrix from historical returns
    pub fn calculate_covariance_matrix(
        &self,
        returns: &HashMap<String, Vec<f64>>,
    ) -> Result<DMatrix<f64>> {
        if returns.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No return data provided".to_string()
            ));
        }

        let num_obs = returns.values().next().unwrap().len();
        if num_obs < self.config.min_observations {
            return Err(AdvancedRiskError::InsufficientData(format!(
                "Need at least {} observations, got {}",
                self.config.min_observations, num_obs
            )));
        }

        let mut assets: Vec<_> = returns.keys().collect();
        assets.sort();
        let n = assets.len();

        // Calculate means
        let mut means = HashMap::new();
        for asset in &assets {
            let data = &returns[*asset];
            let mean = data.iter().sum::<f64>() / data.len() as f64;
            means.insert(*asset, mean);
        }

        // Calculate covariance matrix
        let mut cov_matrix = DMatrix::zeros(n, n);

        for (i, asset_i) in assets.iter().enumerate() {
            for (j, asset_j) in assets.iter().enumerate() {
                let returns_i = &returns[*asset_i];
                let returns_j = &returns[*asset_j];
                let mean_i = means[asset_i];
                let mean_j = means[asset_j];

                let covariance: f64 = returns_i
                    .iter()
                    .zip(returns_j.iter())
                    .map(|(r_i, r_j)| (r_i - mean_i) * (r_j - mean_j))
                    .sum::<f64>()
                    / (num_obs - 1) as f64;

                cov_matrix[(i, j)] = covariance;
            }
        }

        // Add regularization to diagonal
        for i in 0..n {
            cov_matrix[(i, i)] += self.config.regularization;
        }

        Ok(cov_matrix)
    }

    /// Calculate risk contribution for each position
    ///
    /// Risk contribution = weight * (Σ * w) / σ_p
    pub fn calculate_risk_contribution(
        &self,
        positions: &[Position],
        covariance_matrix: &DMatrix<f64>,
    ) -> Result<Vec<RiskContribution>> {
        if positions.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No positions provided".to_string()
            ));
        }

        let weights: Vec<f64> = positions.iter().map(|p| p.weight).collect();
        let w = DVector::from_vec(weights);

        // Calculate portfolio variance
        let portfolio_variance = (&w.transpose() * covariance_matrix * &w)[(0, 0)];
        let portfolio_volatility = portfolio_variance.sqrt();

        if portfolio_volatility == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Portfolio volatility is zero".to_string()
            ));
        }

        // Calculate marginal contribution to risk: Σ * w
        let marginal_risk = covariance_matrix * &w;

        // Calculate risk contributions
        let mut contributions = Vec::new();
        let mut total_contribution = 0.0;

        for (i, position) in positions.iter().enumerate() {
            let variance_contrib = w[i] * marginal_risk[i];
            let volatility_contrib = variance_contrib / portfolio_volatility;
            total_contribution += volatility_contrib;

            contributions.push(RiskContribution {
                asset_id: position.asset_id.clone(),
                variance_contribution: variance_contrib,
                volatility_contribution: volatility_contrib,
                risk_pct: 0.0, // Will be filled in next step
            });
        }

        // Calculate percentage contributions
        for contrib in &mut contributions {
            contrib.risk_pct = if total_contribution != 0.0 {
                (contrib.volatility_contribution / total_contribution) * 100.0
            } else {
                0.0
            };
        }

        Ok(contributions)
    }

    /// Calculate marginal VaR for each position
    ///
    /// Marginal VaR = (∂VaR/∂w_i) ≈ (Σ * w)_i * Z_α / σ_p
    pub fn calculate_marginal_var(
        &self,
        portfolio_var: f64,
        positions: &[Position],
        covariance_matrix: &DMatrix<f64>,
    ) -> Result<Vec<MarginalVarResult>> {
        if positions.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No positions provided".to_string()
            ));
        }

        let weights: Vec<f64> = positions.iter().map(|p| p.weight).collect();
        let w = DVector::from_vec(weights);

        // Calculate portfolio variance
        let portfolio_variance = (&w.transpose() * covariance_matrix * &w)[(0, 0)];
        let portfolio_volatility = portfolio_variance.sqrt();

        if portfolio_volatility == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Portfolio volatility is zero".to_string()
            ));
        }

        // Marginal risk: Σ * w
        let marginal_risk = covariance_matrix * &w;

        // Calculate marginal VaR for each position
        let mut results = Vec::new();

        for (i, position) in positions.iter().enumerate() {
            let marginal_var = (marginal_risk[i] / portfolio_volatility) * portfolio_var;
            let component_var = marginal_var * position.weight;

            results.push(MarginalVarResult {
                asset_id: position.asset_id.clone(),
                marginal_var,
                component_var,
            });
        }

        Ok(results)
    }

    /// Calculate diversification ratio
    ///
    /// Diversification Ratio = (Σ w_i * σ_i) / σ_p
    /// where σ_i is the volatility of asset i and σ_p is portfolio volatility
    pub fn calculate_diversification_ratio(
        &self,
        positions: &[Position],
        asset_volatilities: &HashMap<String, f64>,
        portfolio_volatility: f64,
    ) -> Result<f64> {
        if portfolio_volatility == 0.0 {
            return Err(AdvancedRiskError::DivisionByZero(
                "Portfolio volatility is zero".to_string()
            ));
        }

        // Calculate weighted average volatility
        let mut weighted_vol_sum = 0.0;

        for position in positions {
            let asset_vol = asset_volatilities
                .get(&position.asset_id)
                .ok_or_else(|| {
                    AdvancedRiskError::InvalidParameter(format!(
                        "Missing volatility for asset: {}",
                        position.asset_id
                    ))
                })?;

            weighted_vol_sum += position.weight * asset_vol;
        }

        Ok(weighted_vol_sum / portfolio_volatility)
    }

    /// Calculate concentration risk using Herfindahl-Hirschman Index (HHI)
    ///
    /// HHI = Σ w_i^2
    /// Higher values indicate more concentration (1.0 = all in one asset)
    pub fn calculate_concentration_hhi(
        &self,
        positions: &[Position],
    ) -> Result<f64> {
        if positions.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No positions provided".to_string()
            ));
        }

        let hhi: f64 = positions.iter().map(|p| p.weight * p.weight).sum();

        Ok(hhi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_volatility() {
        let analyzer = PortfolioAnalyzer::new(PortfolioConfig::default());

        let positions = vec![
            Position {
                asset_id: "A".to_string(),
                value_usd: 5000.0,
                weight: 0.5,
            },
            Position {
                asset_id: "B".to_string(),
                value_usd: 5000.0,
                weight: 0.5,
            },
        ];

        // Perfect correlation
        let corr_matrix = DMatrix::from_vec(2, 2, vec![1.0, 1.0, 1.0, 1.0]);

        let volatility = analyzer.calculate_volatility(&positions, &corr_matrix).unwrap();

        // With perfect correlation and equal weights, portfolio vol should equal 1.0
        assert!((volatility - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_correlation_matrix() {
        let analyzer = PortfolioAnalyzer::new(PortfolioConfig {
            min_observations: 5,
            ..Default::default()
        });

        let mut returns = HashMap::new();
        returns.insert("A".to_string(), vec![0.01, 0.02, -0.01, 0.03, -0.02]);
        returns.insert("B".to_string(), vec![0.02, 0.01, -0.02, 0.02, -0.01]);

        let corr_matrix = analyzer.calculate_correlation_matrix(&returns).unwrap();

        // Matrix should be 2x2
        assert_eq!(corr_matrix.nrows(), 2);
        assert_eq!(corr_matrix.ncols(), 2);

        // Diagonal should be 1.0 (asset correlated with itself)
        assert!((corr_matrix[(0, 0)] - 1.0).abs() < 1e-6);
        assert!((corr_matrix[(1, 1)] - 1.0).abs() < 1e-6);

        // Off-diagonal should be symmetric
        assert!((corr_matrix[(0, 1)] - corr_matrix[(1, 0)]).abs() < 1e-6);
    }

    #[test]
    fn test_risk_contribution() {
        let analyzer = PortfolioAnalyzer::new(PortfolioConfig::default());

        let positions = vec![
            Position {
                asset_id: "A".to_string(),
                value_usd: 6000.0,
                weight: 0.6,
            },
            Position {
                asset_id: "B".to_string(),
                value_usd: 4000.0,
                weight: 0.4,
            },
        ];

        // Sample covariance matrix
        let cov_matrix = DMatrix::from_vec(
            2,
            2,
            vec![0.04, 0.02, 0.02, 0.09], // Asset A: 20% vol, Asset B: 30% vol, corr: 0.33
        );

        let contributions = analyzer.calculate_risk_contribution(&positions, &cov_matrix).unwrap();

        assert_eq!(contributions.len(), 2);

        // Risk percentages should sum to approximately 100%
        let total_pct: f64 = contributions.iter().map(|c| c.risk_pct).sum();
        assert!((total_pct - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_diversification_ratio() {
        let analyzer = PortfolioAnalyzer::new(PortfolioConfig::default());

        let positions = vec![
            Position {
                asset_id: "A".to_string(),
                value_usd: 5000.0,
                weight: 0.5,
            },
            Position {
                asset_id: "B".to_string(),
                value_usd: 5000.0,
                weight: 0.5,
            },
        ];

        let mut asset_vols = HashMap::new();
        asset_vols.insert("A".to_string(), 0.20);
        asset_vols.insert("B".to_string(), 0.30);

        let portfolio_vol = 0.20; // Lower than weighted average due to diversification

        let div_ratio = analyzer
            .calculate_diversification_ratio(&positions, &asset_vols, portfolio_vol)
            .unwrap();

        // Diversification ratio should be > 1 (benefits from diversification)
        assert!(div_ratio > 1.0);
    }

    #[test]
    fn test_concentration_hhi() {
        let analyzer = PortfolioAnalyzer::new(PortfolioConfig::default());

        // Concentrated portfolio (all in one asset)
        let concentrated = vec![Position {
            asset_id: "A".to_string(),
            value_usd: 10000.0,
            weight: 1.0,
        }];

        let hhi_concentrated = analyzer.calculate_concentration_hhi(&concentrated).unwrap();
        assert!((hhi_concentrated - 1.0).abs() < 1e-6);

        // Diversified portfolio (equal weights)
        let diversified = vec![
            Position {
                asset_id: "A".to_string(),
                value_usd: 2500.0,
                weight: 0.25,
            },
            Position {
                asset_id: "B".to_string(),
                value_usd: 2500.0,
                weight: 0.25,
            },
            Position {
                asset_id: "C".to_string(),
                value_usd: 2500.0,
                weight: 0.25,
            },
            Position {
                asset_id: "D".to_string(),
                value_usd: 2500.0,
                weight: 0.25,
            },
        ];

        let hhi_diversified = analyzer.calculate_concentration_hhi(&diversified).unwrap();
        assert!(hhi_diversified < hhi_concentrated);
        assert!((hhi_diversified - 0.25).abs() < 1e-6); // 4 * 0.25^2 = 0.25
    }
}

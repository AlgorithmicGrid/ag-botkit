//! Stress testing and scenario analysis
//!
//! Implements stress testing framework for portfolio risk assessment:
//! - Historical stress scenarios (e.g., 2008 crisis, COVID crash)
//! - Hypothetical stress scenarios
//! - Multi-factor stress tests
//! - Reverse stress testing

use crate::advanced::error::{AdvancedRiskError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Portfolio for stress testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    /// Positions by asset identifier
    pub positions: HashMap<String, Position>,

    /// Total portfolio value in USD
    pub total_value_usd: f64,
}

/// Position in portfolio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Asset identifier
    pub asset_id: String,

    /// Quantity held
    pub quantity: f64,

    /// Current price
    pub current_price: f64,

    /// Position value
    pub value_usd: f64,
}

/// Stress test scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScenario {
    /// Scenario name
    pub name: String,

    /// Scenario description
    pub description: String,

    /// Market shocks by asset (percentage change, e.g., -0.20 for -20%)
    pub market_shocks: HashMap<String, f64>,

    /// Volatility shocks by asset (percentage change in volatility)
    pub volatility_shocks: HashMap<String, f64>,

    /// Correlation shocks (optional)
    pub correlation_shock: Option<f64>,
}

/// Result of a stress test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    /// Scenario name
    pub scenario_name: String,

    /// Portfolio impact in USD
    pub portfolio_impact: f64,

    /// Portfolio impact as percentage
    pub portfolio_impact_pct: f64,

    /// Worst affected position
    pub worst_position: String,

    /// Impact on worst position
    pub worst_position_impact: f64,

    /// Position-level impacts
    pub position_impacts: HashMap<String, f64>,

    /// Timestamp of stress test
    pub timestamp: DateTime<Utc>,
}

/// Comprehensive stress test report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestReport {
    /// All scenario results
    pub results: Vec<StressTestResult>,

    /// Worst-case scenario
    pub worst_scenario: String,

    /// Maximum portfolio loss
    pub max_loss: f64,

    /// Best-case scenario
    pub best_scenario: String,

    /// Maximum portfolio gain
    pub max_gain: f64,

    /// Average impact across scenarios
    pub average_impact: f64,

    /// Report generation timestamp
    pub timestamp: DateTime<Utc>,
}

/// Stress testing engine
pub struct StressTestEngine {
    scenarios: Vec<StressScenario>,
}

impl StressTestEngine {
    /// Create a new stress test engine with predefined scenarios
    pub fn new(scenarios: Vec<StressScenario>) -> Self {
        Self { scenarios }
    }

    /// Create engine with common historical scenarios
    pub fn with_historical_scenarios() -> Self {
        let scenarios = vec![
            Self::create_2008_financial_crisis(),
            Self::create_2020_covid_crash(),
            Self::create_2022_inflation_shock(),
            Self::create_flash_crash(),
            Self::create_mild_correction(),
        ];

        Self { scenarios }
    }

    /// Run stress test on portfolio for a specific scenario
    pub fn run_stress_test(
        &self,
        portfolio: &Portfolio,
        scenario: &StressScenario,
    ) -> Result<StressTestResult> {
        let mut position_impacts = HashMap::new();
        let mut total_impact = 0.0;
        let mut worst_position = String::new();
        let mut worst_position_impact = 0.0;

        for (asset_id, position) in &portfolio.positions {
            // Get market shock for this asset (default to 0 if not specified)
            let shock = scenario.market_shocks.get(asset_id).copied().unwrap_or(0.0);

            // Calculate position impact
            let impact = position.value_usd * shock;
            position_impacts.insert(asset_id.clone(), impact);

            total_impact += impact;

            // Track worst position
            if impact < worst_position_impact {
                worst_position = asset_id.clone();
                worst_position_impact = impact;
            }
        }

        let portfolio_impact_pct = if portfolio.total_value_usd > 0.0 {
            total_impact / portfolio.total_value_usd
        } else {
            0.0
        };

        Ok(StressTestResult {
            scenario_name: scenario.name.clone(),
            portfolio_impact: total_impact,
            portfolio_impact_pct,
            worst_position,
            worst_position_impact,
            position_impacts,
            timestamp: Utc::now(),
        })
    }

    /// Run multiple stress test scenarios
    pub fn run_scenarios(
        &self,
        portfolio: &Portfolio,
        scenarios: &[StressScenario],
    ) -> Result<Vec<StressTestResult>> {
        scenarios
            .iter()
            .map(|scenario| self.run_stress_test(portfolio, scenario))
            .collect()
    }

    /// Run all predefined scenarios
    pub fn run_all_scenarios(
        &self,
        portfolio: &Portfolio,
    ) -> Result<Vec<StressTestResult>> {
        self.run_scenarios(portfolio, &self.scenarios)
    }

    /// Generate comprehensive stress test report
    pub fn generate_report(
        &self,
        results: &[StressTestResult],
    ) -> Result<StressTestReport> {
        if results.is_empty() {
            return Err(AdvancedRiskError::InsufficientData(
                "No stress test results provided".to_string()
            ));
        }

        // Initialize with first result to handle all-negative or all-positive scenarios
        let first_result = &results[0];
        let mut worst_scenario = first_result.scenario_name.clone();
        let mut max_loss = first_result.portfolio_impact;
        let mut best_scenario = first_result.scenario_name.clone();
        let mut max_gain = first_result.portfolio_impact;
        let mut total_impact = 0.0;

        for result in results {
            total_impact += result.portfolio_impact;

            if result.portfolio_impact < max_loss {
                max_loss = result.portfolio_impact;
                worst_scenario = result.scenario_name.clone();
            }

            if result.portfolio_impact > max_gain {
                max_gain = result.portfolio_impact;
                best_scenario = result.scenario_name.clone();
            }
        }

        let average_impact = total_impact / results.len() as f64;

        Ok(StressTestReport {
            results: results.to_vec(),
            worst_scenario,
            max_loss,
            best_scenario,
            max_gain,
            average_impact,
            timestamp: Utc::now(),
        })
    }

    /// Create 2008 Financial Crisis scenario
    fn create_2008_financial_crisis() -> StressScenario {
        let mut market_shocks = HashMap::new();
        market_shocks.insert("SPY".to_string(), -0.38);  // -38% S&P 500
        market_shocks.insert("QQQ".to_string(), -0.42);  // -42% NASDAQ
        market_shocks.insert("IWM".to_string(), -0.34);  // -34% Russell 2000
        market_shocks.insert("TLT".to_string(), 0.14);   // +14% Treasuries

        let mut volatility_shocks = HashMap::new();
        volatility_shocks.insert("SPY".to_string(), 2.5);  // VIX spike

        StressScenario {
            name: "2008 Financial Crisis".to_string(),
            description: "Lehman Brothers collapse and credit crisis".to_string(),
            market_shocks,
            volatility_shocks,
            correlation_shock: Some(0.95),  // Correlations spike to 0.95
        }
    }

    /// Create 2020 COVID Crash scenario
    fn create_2020_covid_crash() -> StressScenario {
        let mut market_shocks = HashMap::new();
        market_shocks.insert("SPY".to_string(), -0.34);  // -34% March 2020
        market_shocks.insert("QQQ".to_string(), -0.27);  // Tech more resilient
        market_shocks.insert("IWM".to_string(), -0.41);  // Small caps hit harder

        let mut volatility_shocks = HashMap::new();
        volatility_shocks.insert("SPY".to_string(), 3.0);

        StressScenario {
            name: "2020 COVID Crash".to_string(),
            description: "Pandemic-induced market crash".to_string(),
            market_shocks,
            volatility_shocks,
            correlation_shock: Some(0.90),
        }
    }

    /// Create 2022 Inflation Shock scenario
    fn create_2022_inflation_shock() -> StressScenario {
        let mut market_shocks = HashMap::new();
        market_shocks.insert("SPY".to_string(), -0.19);  // -19% YTD 2022
        market_shocks.insert("QQQ".to_string(), -0.33);  // Growth stocks hit harder
        market_shocks.insert("TLT".to_string(), -0.25);  // Bonds also down

        let volatility_shocks = HashMap::new();

        StressScenario {
            name: "2022 Inflation Shock".to_string(),
            description: "Fed rate hikes and inflation concerns".to_string(),
            market_shocks,
            volatility_shocks,
            correlation_shock: None,
        }
    }

    /// Create Flash Crash scenario
    fn create_flash_crash() -> StressScenario {
        let mut market_shocks = HashMap::new();
        market_shocks.insert("SPY".to_string(), -0.10);  // -10% intraday
        market_shocks.insert("QQQ".to_string(), -0.12);
        market_shocks.insert("IWM".to_string(), -0.15);

        let mut volatility_shocks = HashMap::new();
        volatility_shocks.insert("SPY".to_string(), 4.0);  // Extreme vol spike

        StressScenario {
            name: "Flash Crash".to_string(),
            description: "Rapid intraday market crash".to_string(),
            market_shocks,
            volatility_shocks,
            correlation_shock: Some(0.98),
        }
    }

    /// Create Mild Correction scenario
    fn create_mild_correction() -> StressScenario {
        let mut market_shocks = HashMap::new();
        market_shocks.insert("SPY".to_string(), -0.05);  // -5% pullback
        market_shocks.insert("QQQ".to_string(), -0.06);
        market_shocks.insert("IWM".to_string(), -0.07);

        let volatility_shocks = HashMap::new();

        StressScenario {
            name: "Mild Correction".to_string(),
            description: "Normal market pullback".to_string(),
            market_shocks,
            volatility_shocks,
            correlation_shock: None,
        }
    }

    /// Add custom scenario
    pub fn add_scenario(&mut self, scenario: StressScenario) {
        self.scenarios.push(scenario);
    }

    /// Get all scenarios
    pub fn scenarios(&self) -> &[StressScenario] {
        &self.scenarios
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_portfolio() -> Portfolio {
        let mut positions = HashMap::new();

        positions.insert(
            "SPY".to_string(),
            Position {
                asset_id: "SPY".to_string(),
                quantity: 100.0,
                current_price: 450.0,
                value_usd: 45000.0,
            },
        );

        positions.insert(
            "QQQ".to_string(),
            Position {
                asset_id: "QQQ".to_string(),
                quantity: 50.0,
                current_price: 380.0,
                value_usd: 19000.0,
            },
        );

        positions.insert(
            "TLT".to_string(),
            Position {
                asset_id: "TLT".to_string(),
                quantity: 200.0,
                current_price: 95.0,
                value_usd: 19000.0,
            },
        );

        Portfolio {
            positions,
            total_value_usd: 83000.0,
        }
    }

    #[test]
    fn test_stress_test_2008_crisis() {
        let engine = StressTestEngine::with_historical_scenarios();
        let portfolio = create_test_portfolio();

        let crisis_scenario = &engine.scenarios()[0]; // 2008 crisis
        let result = engine.run_stress_test(&portfolio, crisis_scenario).unwrap();

        assert_eq!(result.scenario_name, "2008 Financial Crisis");

        // Portfolio should have significant negative impact
        assert!(result.portfolio_impact < 0.0);

        // Impact percentage should be negative
        assert!(result.portfolio_impact_pct < 0.0);

        // SPY should be heavily impacted (position_impacts should contain SPY)
        assert!(result.position_impacts.contains_key("SPY"));
    }

    #[test]
    fn test_multiple_scenarios() {
        let engine = StressTestEngine::with_historical_scenarios();
        let portfolio = create_test_portfolio();

        let results = engine.run_all_scenarios(&portfolio).unwrap();

        assert_eq!(results.len(), 5); // 5 historical scenarios

        // All scenarios should have results
        for result in &results {
            assert!(!result.scenario_name.is_empty());
            assert!(result.position_impacts.len() > 0);
        }
    }

    #[test]
    fn test_stress_test_report() {
        let engine = StressTestEngine::with_historical_scenarios();
        let portfolio = create_test_portfolio();

        let results = engine.run_all_scenarios(&portfolio).unwrap();
        let report = engine.generate_report(&results).unwrap();

        assert_eq!(report.results.len(), 5);

        // Worst scenario should have negative impact
        assert!(report.max_loss < 0.0);

        // Report should identify worst and best scenarios
        assert!(!report.worst_scenario.is_empty());
        assert!(!report.best_scenario.is_empty());

        // Average impact should be calculated
        assert!(report.average_impact != 0.0);
    }

    #[test]
    fn test_custom_scenario() {
        let mut engine = StressTestEngine::new(vec![]);

        let mut market_shocks = HashMap::new();
        market_shocks.insert("SPY".to_string(), -0.15);

        let custom_scenario = StressScenario {
            name: "Custom Test".to_string(),
            description: "Test scenario".to_string(),
            market_shocks,
            volatility_shocks: HashMap::new(),
            correlation_shock: None,
        };

        engine.add_scenario(custom_scenario.clone());

        let portfolio = create_test_portfolio();
        let result = engine.run_stress_test(&portfolio, &custom_scenario).unwrap();

        assert_eq!(result.scenario_name, "Custom Test");

        // SPY position should lose 15%
        let spy_impact = result.position_impacts.get("SPY").unwrap();
        let expected_impact = 45000.0 * -0.15;
        assert!((spy_impact - expected_impact).abs() < 1.0);
    }

    #[test]
    fn test_flash_crash_scenario() {
        let engine = StressTestEngine::with_historical_scenarios();
        let portfolio = create_test_portfolio();

        let flash_crash = engine.scenarios()
            .iter()
            .find(|s| s.name == "Flash Crash")
            .unwrap();

        let result = engine.run_stress_test(&portfolio, flash_crash).unwrap();

        // Should have negative impact
        assert!(result.portfolio_impact < 0.0);

        // Should have high volatility shock
        assert!(flash_crash.volatility_shocks.len() > 0);
    }
}

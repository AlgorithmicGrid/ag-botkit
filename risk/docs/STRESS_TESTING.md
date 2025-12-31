# Stress Testing Guide

## Overview

Stress testing evaluates portfolio performance under extreme but plausible scenarios. This guide covers the stress testing framework implemented in ag-risk.

## Purpose of Stress Testing

Stress testing answers critical questions:
- How would the portfolio perform during a market crash?
- What are the worst-case losses?
- Which positions are most vulnerable?
- Are current risk limits adequate?

**Regulatory Context:** Required by Basel III, Dodd-Frank, and other frameworks.

---

## Stress Testing Methodologies

### 1. Historical Scenario Analysis

**Approach:** Apply historical crisis shocks to current portfolio.

**Advantages:**
- Based on actual events
- Credible and easily understood
- Regulatory acceptance

**Limitations:**
- Past events may not repeat
- Limited to observed scenarios
- May miss new types of risk

### 2. Hypothetical Scenarios

**Approach:** Create "what-if" scenarios based on potential future events.

**Examples:**
- Geopolitical crisis
- Central bank policy shock
- Technology disruption
- Climate-related events

### 3. Reverse Stress Testing

**Approach:** Identify scenarios that would cause portfolio failure.

**Question:** "What would have to happen for us to lose X%?"

---

## Implemented Historical Scenarios

### 1. 2008 Financial Crisis

**Event:** Lehman Brothers collapse, credit market freeze

**Market Impact:**
```
SPY:  -38% (S&P 500)
QQQ:  -42% (NASDAQ)
IWM:  -34% (Russell 2000)
TLT:  +14% (Treasuries rally)

Volatility: VIX spiked to 80+
Correlations: Increased to 0.95 (everything moved together)
```

**Key Characteristics:**
- Credit crisis
- Liquidity evaporation
- Flight to quality
- Correlation spike

**Usage:**
```rust
let engine = StressTestEngine::with_historical_scenarios();
let result = engine.run_stress_test(&portfolio, &scenarios[0])?;

// 2008 Financial Crisis scenario
println!("Impact: ${:.2}", result.portfolio_impact);
```

---

### 2. 2020 COVID Crash

**Event:** Pandemic-induced economic shutdown

**Market Impact:**
```
SPY:  -34% (March 2020 crash)
QQQ:  -27% (Tech more resilient)
IWM:  -41% (Small caps hit harder)

Volatility: VIX spiked to 82
Correlations: 0.90
```

**Key Characteristics:**
- Black swan event
- Sudden economic stop
- Unprecedented policy response
- V-shaped recovery

**Lessons:**
- Modern portfolio theory broke down
- Diversification failed temporarily
- Cash was king
- Government intervention critical

---

### 3. 2022 Inflation Shock

**Event:** Federal Reserve aggressive rate hikes

**Market Impact:**
```
SPY:  -19% (YTD 2022)
QQQ:  -33% (Growth stocks hammered)
TLT:  -25% (Bonds also down - no diversification benefit)

Volatility: Elevated but not extreme
Correlations: Normal
```

**Key Characteristics:**
- Stocks AND bonds down (rare)
- Growth vs. value rotation
- Rising rates
- No safe haven

**Implication:** Traditional 60/40 portfolio failed.

---

### 4. Flash Crash

**Event:** Rapid intraday market collapse

**Market Impact:**
```
SPY:  -10% (intraday)
QQQ:  -12%
IWM:  -15%

Volatility: Extreme spike (4x normal)
Correlations: 0.98 (near perfect)
Duration: Minutes to hours
```

**Key Characteristics:**
- Algorithmic trading failure
- Liquidity vacuum
- Stop-loss cascades
- Quick recovery (often)

**Risk Management Focus:**
- Circuit breakers
- Position limits
- Volatility controls

---

### 5. Mild Correction

**Event:** Normal market pullback

**Market Impact:**
```
SPY:  -5%
QQQ:  -6%
IWM:  -7%

Volatility: Moderate increase
Correlations: Normal
```

**Purpose:**
- Baseline stress test
- Frequent occurrence
- Tests normal risk limits

---

## Custom Scenario Creation

### Define Your Own Scenario

```rust
use std::collections::HashMap;

let mut market_shocks = HashMap::new();
market_shocks.insert("SPY".to_string(), -0.20);  // -20%
market_shocks.insert("TLT".to_string(), -0.10);  // -10%
market_shocks.insert("GLD".to_string(), 0.15);   // +15% (gold rally)

let mut volatility_shocks = HashMap::new();
volatility_shocks.insert("SPY".to_string(), 2.0);  // Vol doubles

let scenario = StressScenario {
    name: "Stagflation Scenario".to_string(),
    description: "High inflation + recession".to_string(),
    market_shocks,
    volatility_shocks,
    correlation_shock: Some(0.80),
};

let result = engine.run_stress_test(&portfolio, &scenario)?;
```

---

## Interpreting Results

### Stress Test Result Components

```rust
pub struct StressTestResult {
    scenario_name: String,
    portfolio_impact: f64,         // Dollar impact
    portfolio_impact_pct: f64,     // Percentage impact
    worst_position: String,        // Most affected position
    worst_position_impact: f64,    // Dollar impact on worst position
    position_impacts: HashMap,     // All position impacts
    timestamp: DateTime<Utc>,
}
```

### Example Output

```
Scenario: 2008 Financial Crisis
Portfolio Impact: -$25,420
Portfolio Impact %: -30.6%
Worst Position: SPY (-$17,100)
```

**Analysis:**
- Total loss: $25,420 (30.6% drawdown)
- SPY contributes 67% of losses
- Diversification provided some benefit
- Consider reducing SPY concentration

---

## Comprehensive Stress Test Report

### Generate Multi-Scenario Report

```rust
let results = engine.run_all_scenarios(&portfolio)?;
let report = engine.generate_report(&results)?;

println!("=== Stress Test Report ===");
println!("Worst Scenario: {}", report.worst_scenario);
println!("Max Loss: ${:.2}", report.max_loss);
println!("Best Scenario: {}", report.best_scenario);
println!("Max Gain: ${:.2}", report.max_gain);
println!("Average Impact: ${:.2}", report.average_impact);
```

### Sample Report

```
=== Stress Test Report ===

Scenarios Tested: 5

Worst Scenario: 2008 Financial Crisis
  Max Loss: -$25,420 (-30.6%)

Best Scenario: Mild Correction
  Max Loss: -$4,150 (-5.0%)

Average Impact: -$12,384 (-14.9%)

Position Breakdown:
  SPY: -$17,100 (67% of worst-case)
  QQQ: -$6,320 (25% of worst-case)
  TLT: +$2,000 (hedged worst-case)
```

---

## Risk Management Actions

### Traffic Light System

Based on stress test results, implement thresholds:

| Impact | Action | Example |
|--------|--------|---------|
| < 10% | Green | Normal operations |
| 10-20% | Yellow | Monitor closely, reduce new positions |
| 20-30% | Orange | Reduce risk, hedge exposures |
| > 30% | Red | Emergency hedging, position liquidation |

### Example Implementation

```rust
let max_loss_pct = report.max_loss.abs() / portfolio.total_value_usd;

let action = if max_loss_pct < 0.10 {
    "GREEN: Normal operations"
} else if max_loss_pct < 0.20 {
    "YELLOW: Monitor closely"
} else if max_loss_pct < 0.30 {
    "ORANGE: Reduce risk"
} else {
    "RED: Emergency action required"
};

println!("Risk Level: {}", action);
```

---

## Advanced Techniques

### Multi-Factor Stress Tests

Combine multiple risk factors:

```rust
let scenario = StressScenario {
    name: "Perfect Storm".to_string(),
    description: "Simultaneous equity crash + credit crisis + vol spike".to_string(),
    market_shocks: hashmap!{
        "SPY" => -0.30,
        "HYG" => -0.25,  // High-yield bonds
        "EMB" => -0.35,  // Emerging markets
    },
    volatility_shocks: hashmap!{
        "SPY" => 3.0,    // Triple volatility
    },
    correlation_shock: Some(0.98),
};
```

### Conditional Scenarios

Model cascading effects:

```
IF SPY drops > 20%
  THEN credit spreads widen by 500bp
  AND volatility spikes 3x
  AND liquidity dries up (widen bid-ask by 2x)
```

### Sensitivity Analysis

Test range of shock magnitudes:

```rust
for shock_magnitude in [-0.05, -0.10, -0.15, -0.20, -0.25, -0.30] {
    let mut shocks = HashMap::new();
    shocks.insert("SPY".to_string(), shock_magnitude);

    let scenario = StressScenario {
        name: format!("SPY {}%", shock_magnitude * 100.0),
        market_shocks: shocks,
        ..Default::default()
    };

    let result = engine.run_stress_test(&portfolio, &scenario)?;
    println!("{}% shock → ${:.0} loss",
        shock_magnitude * 100.0,
        result.portfolio_impact
    );
}
```

---

## Stress Testing Best Practices

### 1. Regular Frequency

- Daily: For trading desks
- Weekly: For active portfolios
- Monthly: For strategic portfolios
- Ad-hoc: When market conditions change

### 2. Scenario Selection

**Include:**
- Historical crashes (2008, 2020, etc.)
- Sector-specific events
- Interest rate shocks
- Liquidity crises
- Tail events

**Avoid:**
- Only benign scenarios
- Overly specific scenarios
- Outdated scenarios

### 3. Documentation

Document for each scenario:
- Event description
- Shock magnitudes
- Assumptions
- Limitations
- Results history

### 4. Action Plans

Pre-define responses:

```
IF worst-case loss > $X:
  1. Hedge with SPY puts
  2. Reduce position sizes by Y%
  3. Raise cash to Z%
  4. Notify risk committee
```

### 5. Backtesting

Validate scenarios using historical data:

```rust
// Test if 2020 scenario matches actual March 2020 results
let predicted_loss = stress_test_result.portfolio_impact;
let actual_loss = historical_pnl.get("2020-03")?;

let error = (predicted_loss - actual_loss).abs() / actual_loss;
println!("Prediction error: {:.1}%", error * 100.0);
```

---

## Integration with VaR

### VaR vs. Stress Testing

| Aspect | VaR | Stress Testing |
|--------|-----|----------------|
| Question | "How much could we lose normally?" | "What happens in a crisis?" |
| Probability | High (5%, 1%) | Low (undefined) |
| Method | Statistical | Scenario-based |
| Horizon | Short (1-10 days) | Event-driven |
| Use Case | Day-to-day risk | Tail risk, planning |

### Combined Framework

```rust
// Regular risk: VaR
let var_95 = var_engine.calculate_historical_var(
    portfolio_value, 0.95, 1
)?;
println!("95% VaR (1-day): ${:.2}", var_95.var_amount);

// Extreme risk: Stress tests
let stress_results = stress_engine.run_all_scenarios(&portfolio)?;
let report = stress_engine.generate_report(&stress_results)?;
println!("Worst-case stress loss: ${:.2}", report.max_loss);

// Compare
if report.max_loss.abs() > var_95.var_amount * 3.0 {
    println!("WARNING: Stress loss >> VaR. Tail risk present.");
}
```

---

## Regulatory Requirements

### Basel III

**Requirements:**
- Minimum 3 scenarios (baseline, adverse, severely adverse)
- 9-quarter horizon
- Includes recession, unemployment, house prices
- Annual exercise (CCAR)

### FRTB (Fundamental Review of Trading Book)

**Requirements:**
- Standardized stress scenarios
- Historical scenarios (2008, sovereign crisis)
- Hypothetical scenarios
- Reverse stress tests

### Internal Models

Even if using internal models, must demonstrate:
1. Scenario selection methodology
2. Expert judgment process
3. Governance and validation
4. Results used in decision-making

---

## Case Study: Portfolio Stress Test

### Initial Portfolio

```
Total Value: $100,000

Positions:
- SPY (S&P 500 ETF): $50,000 (50%)
- QQQ (NASDAQ ETF): $30,000 (30%)
- TLT (Treasury ETF): $20,000 (20%)
```

### Run Stress Tests

```rust
let engine = StressTestEngine::with_historical_scenarios();
let results = engine.run_all_scenarios(&portfolio)?;
```

### Results

```
2008 Financial Crisis:
  SPY: -$19,000 (-38%)
  QQQ: -$12,600 (-42%)
  TLT: +$2,800 (+14%)
  Total: -$28,800 (-28.8%)

2020 COVID Crash:
  SPY: -$17,000 (-34%)
  QQQ: -$8,100 (-27%)
  TLT: +$0 (flat)
  Total: -$25,100 (-25.1%)

Mild Correction:
  SPY: -$2,500 (-5%)
  QQQ: -$1,800 (-6%)
  TLT: +$0
  Total: -$4,300 (-4.3%)
```

### Risk Assessment

**Observations:**
1. Worst-case loss: ~29% (2008 scenario)
2. TLT provides hedge in severe crises
3. Concentration in equities (80%)

**Actions:**
1. Reduce equity allocation to 65%
2. Increase TLT to 25%
3. Add gold (5%) for diversification
4. Set stop-loss at -15%

---

## Tools and Automation

### Automated Stress Testing

```rust
// Daily automated stress testing
pub fn daily_stress_check(portfolio: &Portfolio) -> Result<StressAlert> {
    let engine = StressTestEngine::with_historical_scenarios();
    let results = engine.run_all_scenarios(portfolio)?;
    let report = engine.generate_report(&results)?;

    let alert = if report.max_loss.abs() > 50000.0 {
        StressAlert::Critical
    } else if report.max_loss.abs() > 30000.0 {
        StressAlert::Warning
    } else {
        StressAlert::Normal
    };

    Ok(alert)
}
```

### Integration with Monitoring

```rust
// Emit stress test metrics to monitoring dashboard
for result in &results {
    emit_metric(Metric {
        timestamp: Utc::now(),
        metric_name: "stress_test.portfolio_impact".to_string(),
        value: result.portfolio_impact,
        labels: hashmap!{
            "scenario" => &result.scenario_name,
        },
    });
}
```

---

## Conclusion

Stress testing is essential for:
- Understanding tail risk
- Validating risk limits
- Regulatory compliance
- Strategic planning
- Crisis preparedness

**Key Takeaways:**
1. Use multiple scenarios (historical + hypothetical)
2. Test regularly, especially before major events
3. Document methodology and results
4. Link results to concrete actions
5. Combine with VaR for comprehensive risk view

---

## References

1. Basel Committee on Banking Supervision (2009). "Principles for sound stress testing practices and supervision"
2. Federal Reserve (2021). "Dodd-Frank Act Stress Test (DFAST) Methodology"
3. Rebonato, R. (2010). "Coherent Stress Testing: A Bayesian Approach to the Analysis of Financial Stress"
4. Breuer, T., & Csiszár, I. (2013). "Systematic stress tests with entropic plausibility constraints"

---

## Appendix: Additional Scenarios

### Tech Bubble Burst
```rust
SPY:  -20%
QQQ:  -40%  // Tech-heavy
IWM:  -15%
```

### Oil Price Shock
```rust
Energy stocks: -30%
Airlines: -20%
Consumers: +5% (benefit from lower gas prices)
```

### Currency Crisis
```rust
USD strength: +15%
Emerging markets: -25%
Exporters: -10%
Importers: +5%
```

### Cyber Attack
```rust
Financial stocks: -15%
Tech stocks: -20%
Cyber security: +30%
```

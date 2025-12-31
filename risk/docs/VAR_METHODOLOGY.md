# Value at Risk (VaR) Methodology

## Overview

Value at Risk (VaR) is a statistical measure that quantifies the potential loss in portfolio value over a specified time horizon at a given confidence level. This document describes the VaR methodologies implemented in the ag-risk library.

## VaR Definition

**VaR(α, T)** answers the question: "What is the maximum loss over time period T with confidence level α?"

Formally: P(Loss ≤ VaR) = α

For example, a 1-day 95% VaR of $100,000 means:
- There is a 95% probability that portfolio losses will not exceed $100,000 over the next day
- There is a 5% probability of losses exceeding $100,000

## Implemented Methodologies

### 1. Historical VaR

**Approach:** Uses empirical distribution of historical returns to estimate VaR.

**Formula:**
```
VaR_α = -percentile(historical_returns, 1-α) × portfolio_value × √T
```

**Advantages:**
- No distributional assumptions
- Captures actual market behavior including fat tails
- Simple to understand and implement

**Disadvantages:**
- Requires substantial historical data
- Assumes future will resemble past
- Does not adapt to changing market conditions

**Implementation:**
```rust
let engine = VarEngine::with_historical_returns(config, returns);
let var_result = engine.calculate_historical_var(
    100000.0,  // portfolio value
    0.95,      // 95% confidence
    1,         // 1 day horizon
)?;
```

**Minimum Data Requirements:** 30 observations (configurable)

---

### 2. Parametric VaR (Variance-Covariance)

**Approach:** Assumes returns follow a normal distribution.

**Formula:**
```
VaR = -μ + σ × Z_α × √T

where:
- μ = expected return
- σ = portfolio volatility
- Z_α = inverse CDF of standard normal at confidence α
- T = time horizon in days
```

**Z-scores for common confidence levels:**
- 90%: 1.282
- 95%: 1.645
- 99%: 2.326
- 99.9%: 3.090

**Advantages:**
- Fast computation
- Only requires mean and volatility
- Easy to scale across portfolios

**Disadvantages:**
- Assumes normal distribution (underestimates tail risk)
- May not capture extreme events
- Sensitive to volatility estimation

**Implementation:**
```rust
let var_result = engine.calculate_parametric_var(
    100000.0,  // portfolio value
    0.02,      // 2% daily volatility
    0.95,      // 95% confidence
    1,         // 1 day horizon
)?;
```

---

### 3. Monte Carlo VaR

**Approach:** Simulates thousands of potential price paths using stochastic models.

**Formula (Geometric Brownian Motion):**
```
S_t = S_0 × exp((μ - σ²/2)×t + σ×√t×Z)

where:
- S_t = simulated price at time t
- S_0 = current price
- μ = drift (expected return)
- σ = volatility
- Z ~ N(0,1) (standard normal)
```

**Advantages:**
- Handles non-normal distributions
- Can incorporate complex portfolio structures
- Flexible for various asset types

**Disadvantages:**
- Computationally intensive
- Requires model assumptions
- Results depend on quality of input parameters

**Implementation:**
```rust
let var_result = engine.calculate_monte_carlo_var(
    100000.0,   // portfolio value
    0.0001,     // mean daily return
    0.02,       // volatility
    0.95,       // confidence
    1,          // time horizon
    10000,      // number of simulations
)?;
```

**Performance Target:** <1 second for 10,000 simulations

---

### 4. Conditional VaR (CVaR / Expected Shortfall)

**Approach:** Estimates the expected loss given that loss exceeds VaR.

**Formula:**
```
CVaR_α = E[Loss | Loss > VaR_α]
```

**Advantages:**
- Captures tail risk better than VaR
- Coherent risk measure (satisfies subadditivity)
- Preferred by regulators

**Disadvantages:**
- Requires more data than VaR
- More difficult to estimate
- Less intuitive interpretation

**Implementation:**
```rust
let cvar = engine.calculate_cvar(
    100000.0,  // portfolio value
    0.95,      // confidence level
    1,         // time horizon
)?;
```

**Interpretation:** If VaR is $10,000 and CVaR is $15,000, the average loss when losses exceed VaR is $15,000.

---

## Time Scaling

VaR scales with the square root of time:

```
VaR(T days) = VaR(1 day) × √T
```

**Examples:**
- 1-day VaR: $10,000
- 10-day VaR: $10,000 × √10 ≈ $31,623
- 252-day (1 year) VaR: $10,000 × √252 ≈ $158,745

**Assumption:** Returns are independent and identically distributed (i.i.d.)

---

## VaR Backtesting

Backtesting validates VaR model accuracy by comparing predicted VaR to actual losses.

**Methodology:**
1. Calculate VaR predictions for historical dates
2. Count violations (actual loss > predicted VaR)
3. Calculate violation rate = violations / total predictions
4. Compare to expected rate (1 - confidence level)

**Statistical Test:**
For n predictions at confidence α, violations should follow binomial distribution:
```
Violations ~ Binomial(n, 1-α)

Expected violations = n × (1-α)
Standard error = √(n × (1-α) × α)
```

**Validation Criteria:**
Model is validated if violation rate is within 2 standard errors:
```
|violation_rate - (1-α)| ≤ 2 × √((1-α) × α / n)
```

**Implementation:**
```rust
let backtest = engine.backtest_var(predictions, actual_losses)?;

if backtest.validated {
    println!("Model passed backtest");
} else {
    println!("Model failed: {}% violations vs {}% expected",
        backtest.violation_rate * 100.0,
        backtest.expected_violation_rate * 100.0
    );
}
```

**Example:**
- 100 predictions at 95% confidence
- Expected violations: 5
- Acceptable range: 1-9 violations (2σ interval)
- Observed: 6 violations → PASS

---

## Best Practices

### Choosing a Method

| Scenario | Recommended Method |
|----------|-------------------|
| Sufficient historical data available | Historical VaR |
| Need quick calculation | Parametric VaR |
| Complex portfolio with options | Monte Carlo VaR |
| Regulatory reporting | CVaR |
| Normal market conditions | Parametric VaR |
| Extreme events / tail risk | Historical or Monte Carlo VaR |

### Data Requirements

**Historical VaR:**
- Minimum: 30 observations (1 month of daily data)
- Recommended: 252 observations (1 year)
- Optimal: 500-1000 observations (2-4 years)

**Parametric VaR:**
- Minimum: Mean and volatility estimates
- Recommended: 60+ observations for volatility
- Consider using EWMA or GARCH for volatility

**Monte Carlo VaR:**
- Minimum: 1,000 simulations
- Recommended: 10,000 simulations
- High accuracy: 100,000+ simulations

### Confidence Levels

| Confidence | Use Case |
|------------|----------|
| 90% | Internal risk monitoring |
| 95% | Standard market risk reporting |
| 99% | Regulatory capital requirements |
| 99.9% | Stress testing, tail risk |

### Time Horizons

| Horizon | Use Case |
|---------|----------|
| 1 day | Trading desks, market risk |
| 10 days | Basel II/III regulatory capital |
| 1 month | Strategic risk management |
| 1 year | Long-term capital planning |

---

## Limitations and Assumptions

### All Methods
- Historical patterns may not repeat
- Assumes no portfolio rebalancing during horizon
- Does not capture model risk
- Liquidity risk not explicitly modeled

### Historical VaR
- Limited by available data
- Ghost effects (old events influence results)
- Step function (jumps when old data drops off)

### Parametric VaR
- Normal distribution assumption often violated
- Underestimates tail risk (fat tails)
- Ignores skewness and kurtosis
- Poor performance in crisis periods

### Monte Carlo VaR
- Model risk (wrong distribution assumption)
- Computationally expensive
- Random seed dependency (use fixed seed for reproducibility)
- Parameter estimation error

---

## References

1. J.P. Morgan (1996). RiskMetrics Technical Document, 4th Edition
2. Basel Committee on Banking Supervision (2019). Minimum capital requirements for market risk
3. Jorion, P. (2006). Value at Risk: The New Benchmark for Managing Financial Risk, 3rd Edition
4. McNeil, A.J., Frey, R., & Embrechts, P. (2015). Quantitative Risk Management: Concepts, Techniques and Tools

---

## Example Workflow

```rust
use ag_risk::advanced::*;

// 1. Load historical returns
let returns = load_historical_returns()?;

// 2. Create VaR engine
let config = VarConfig {
    default_simulations: 10000,
    min_observations: 30,
    random_seed: Some(42), // For reproducibility
};
let engine = VarEngine::with_historical_returns(config, returns);

// 3. Calculate VaR using multiple methods
let historical = engine.calculate_historical_var(100000.0, 0.95, 1)?;
let parametric = engine.calculate_parametric_var(100000.0, 0.02, 0.95, 1)?;
let monte_carlo = engine.calculate_monte_carlo_var(
    100000.0, 0.0, 0.02, 0.95, 1, 10000
)?;

// 4. Compare results
println!("Historical VaR: ${:.2}", historical.var_amount);
println!("Parametric VaR: ${:.2}", parametric.var_amount);
println!("Monte Carlo VaR: ${:.2}", monte_carlo.var_amount);

// 5. Calculate CVaR for tail risk
let cvar = engine.calculate_cvar(100000.0, 0.95, 1)?;
println!("CVaR (Expected Shortfall): ${:.2}", cvar);

// 6. Backtest the model
let backtest = engine.backtest_var(predictions, actual_losses)?;
println!("Validation: {}", if backtest.validated { "PASS" } else { "FAIL" });
```

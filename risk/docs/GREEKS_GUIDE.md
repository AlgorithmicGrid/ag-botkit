# Options Greeks Calculation Guide

## Overview

Greeks measure the sensitivity of an option's price to various factors. This guide covers the Black-Scholes Greeks implementation in ag-risk.

## The Five Primary Greeks

### 1. Delta (Δ)

**Definition:** Rate of change of option price with respect to underlying price.

**Formula:**
```
Call Delta: Δ = N(d₁)
Put Delta:  Δ = N(d₁) - 1

where d₁ = [ln(S/K) + (r + σ²/2)×T] / (σ×√T)
```

**Interpretation:**
- **Call Delta:** 0 to 1
  - Deep OTM: ~0 (moves little with underlying)
  - ATM: ~0.5 (moves half as much as underlying)
  - Deep ITM: ~1 (moves one-to-one with underlying)

- **Put Delta:** -1 to 0
  - Deep OTM: ~0
  - ATM: ~-0.5
  - Deep ITM: ~-1

**Portfolio Delta:**
Total delta exposure = Σ(position_size × option_delta × contract_size)

**Delta Hedging:**
To neutralize delta, hold:
```
Hedge_quantity = -Portfolio_Delta / Underlying_Delta
```
(Underlying delta = 1.0)

**Example:**
```rust
let option = Option {
    option_type: OptionType::Call,
    strike: 100.0,
    time_to_expiry: 1.0,  // 1 year
    contract_size: 100.0,
};

let greeks = engine.calculate_greeks(
    &option,
    100.0,  // underlying at 100 (ATM)
    0.20,   // 20% volatility
    0.05,   // 5% risk-free rate
)?;

// ATM call delta ≈ 0.5
println!("Delta: {:.4}", greeks.delta);
```

---

### 2. Gamma (Γ)

**Definition:** Rate of change of delta with respect to underlying price.

**Formula:**
```
Γ = N'(d₁) / (S × σ × √T)

where N'(x) = (1/√2π) × e^(-x²/2)  (standard normal PDF)
```

**Interpretation:**
- Gamma is same for calls and puts (at same strike/expiry)
- Maximum gamma occurs at-the-money
- Gamma approaches 0 deep in/out of the money
- Measures delta convexity

**Why Gamma Matters:**
- High gamma → Delta changes rapidly → Frequent rehedging needed
- Long options → Positive gamma (beneficial)
- Short options → Negative gamma (risk)

**Example:**
For a portfolio with gamma = 50:
- If underlying moves +$1, delta increases by ~50
- If underlying moves -$1, delta decreases by ~50

**Implementation:**
```rust
// Gamma tells us how much delta will change
let current_delta = greeks.delta;
let underlying_move = 1.0;  // $1 move
let new_delta_approx = current_delta + greeks.gamma * underlying_move;
```

---

### 3. Vega (ν)

**Definition:** Sensitivity to implied volatility changes.

**Formula:**
```
ν = S × N'(d₁) × √T
```

**Interpretation:**
- Vega per 1% change in volatility
- Always positive for long options (calls and puts)
- Maximum vega at-the-money
- Increases with time to expiry

**Vega Risk:**
```
P&L_change ≈ Vega × Δσ

where Δσ = change in implied volatility
```

**Example:**
```
Vega = 500
If volatility increases from 20% to 21% (Δσ = 0.01):
P&L change ≈ 500 × 0.01 = $5.00
```

**Portfolio Vega Management:**
- Long vega: Profit from volatility increase
- Short vega: Profit from volatility decrease
- Vega neutral: Hedged against vol changes

**Implementation:**
```rust
// Calculate impact of volatility change
let vol_change = 0.01;  // 1% increase
let pnl_impact = greeks.vega * vol_change;
println!("P&L impact from vol change: ${:.2}", pnl_impact);
```

---

### 4. Theta (Θ)

**Definition:** Time decay - change in option value per day.

**Formula:**
```
Call Theta: Θ = -[S×N'(d₁)×σ / (2×√T)] - r×K×e^(-r×T)×N(d₂)

Put Theta:  Θ = -[S×N'(d₁)×σ / (2×√T)] + r×K×e^(-r×T)×N(-d₂)
```

**Interpretation:**
- Theta is typically negative for long options (time decay)
- Expressed per day (365-day year)
- Largest (most negative) at-the-money
- Accelerates as expiration approaches

**Theta Decay Pattern:**
- Far from expiry: Slow, linear decay
- Near expiry: Accelerating decay
- Last week: Exponential decay

**Example:**
```
Theta = -10.0 (per day)
After 1 day (all else equal): Option value decreases by ~$10
After 1 week: Option value decreases by ~$70
```

**Managing Theta:**
- Long options: Pay theta (losing time value daily)
- Short options: Earn theta (collect time value daily)
- Calendar spreads: Exploit theta differences

---

### 5. Rho (ρ)

**Definition:** Sensitivity to risk-free interest rate changes.

**Formula:**
```
Call Rho: ρ = K × T × e^(-r×T) × N(d₂)
Put Rho:  ρ = -K × T × e^(-r×T) × N(-d₂)
```

**Interpretation:**
- Rho per 1% change in interest rate
- Call rho: Positive (calls benefit from higher rates)
- Put rho: Negative (puts benefit from lower rates)
- Generally smallest Greek in magnitude

**When Rho Matters:**
- Long-dated options (high T)
- High strike prices (high K)
- Significant rate changes expected
- Interest rate derivatives

**Example:**
```
Rho = 25.0
If risk-free rate increases from 5% to 6% (Δr = 0.01):
Option value change ≈ 25 × 0.01 = $0.25
```

---

## Black-Scholes Foundation

### Model Assumptions
1. Log-normal distribution of returns
2. Constant volatility
3. No dividends
4. Frictionless markets (no transaction costs)
5. Continuous trading
6. Constant risk-free rate

### d₁ and d₂ Calculation

```
d₁ = [ln(S/K) + (r + σ²/2)×T] / (σ×√T)
d₂ = d₁ - σ×√T
```

**Where:**
- S = Spot price of underlying
- K = Strike price
- r = Risk-free interest rate (annualized)
- σ = Volatility (annualized)
- T = Time to expiry (in years)
- N(x) = Cumulative normal distribution
- N'(x) = Normal probability density function

---

## Portfolio Greeks Aggregation

### Total Portfolio Greeks

```rust
let positions = vec![
    OptionPosition {
        option: call_100,
        quantity: 10.0,  // Long 10 contracts
        underlying: "SPY".to_string(),
    },
    OptionPosition {
        option: put_100,
        quantity: -5.0,  // Short 5 contracts
        underlying: "SPY".to_string(),
    },
];

let portfolio_greeks = engine.calculate_portfolio_greeks(
    &positions,
    &market_data,
)?;

println!("Total Delta: {:.2}", portfolio_greeks.total_delta);
println!("Total Gamma: {:.2}", portfolio_greeks.total_gamma);
println!("Total Vega: {:.2}", portfolio_greeks.total_vega);
```

### Greeks by Underlying

```rust
// Breakdown by underlying asset
for (underlying, greeks) in &portfolio_greeks.by_underlying {
    println!("{}:", underlying);
    println!("  Delta: {:.2}", greeks.delta);
    println!("  Gamma: {:.2}", greeks.gamma);
    println!("  Vega: {:.2}", greeks.vega);
}
```

---

## Greeks-Based Hedging

### Delta Hedging Strategy

**Objective:** Neutralize directional exposure

```rust
let current_greeks = engine.calculate_portfolio_greeks(&positions, &market_data)?;

if current_greeks.total_delta.abs() > 10.0 {  // Delta threshold
    // Hedge with underlying
    let hedge_quantity = -current_greeks.total_delta;

    println!("Portfolio Delta: {:.2}", current_greeks.total_delta);
    println!("Hedge: {} shares of underlying", hedge_quantity);
}
```

### Dynamic Hedging

Due to gamma, delta changes with price:

```rust
// Rehedge when delta drift exceeds threshold
let delta_drift_threshold = 5.0;
let price_move = new_price - old_price;
let delta_change = current_greeks.total_gamma * price_move;

if delta_change.abs() > delta_drift_threshold {
    rehedge_portfolio()?;
}
```

### Gamma Scalping

Profit from gamma by rehedging after price moves:

1. Long gamma position (long options)
2. Delta hedge to neutralize directional risk
3. As price moves, gamma causes delta to change
4. Rehedge at favorable prices
5. Collect small profits repeatedly

### Vega Hedging

Neutralize volatility exposure using ATM options:

```rust
let target_vega = 0.0;  // Vega neutral
let current_vega = portfolio_greeks.total_vega;
let vega_diff = target_vega - current_vega;

// ATM options have highest vega
let atm_option_vega = 100.0;  // Vega per contract
let hedge_contracts = vega_diff / atm_option_vega;

println!("Hedge with {} ATM option contracts", hedge_contracts);
```

---

## Common Strategies and Their Greeks

### 1. Long Call

| Greek | Sign | Meaning |
|-------|------|---------|
| Delta | + | Profits from upside |
| Gamma | + | Delta increases with price |
| Vega | + | Profits from vol increase |
| Theta | - | Loses time value daily |

**Profile:** Bullish, benefits from upside and volatility

### 2. Covered Call (Long Stock + Short Call)

| Greek | Sign | Meaning |
|-------|------|---------|
| Delta | + (reduced) | Less upside than stock alone |
| Gamma | - | Delta decreases as price rises |
| Vega | - | Hurt by vol increase |
| Theta | + | Collects time decay |

**Profile:** Moderately bullish, income generation

### 3. Straddle (Long Call + Long Put, same strike)

| Greek | Sign | Meaning |
|-------|------|---------|
| Delta | ~0 | Neutral (if ATM) |
| Gamma | ++ | High convexity |
| Vega | ++ | Very sensitive to vol |
| Theta | -- | Expensive time decay |

**Profile:** Volatility play, neutral direction

### 4. Iron Condor

| Greek | Sign | Meaning |
|-------|------|---------|
| Delta | ~0 | Neutral |
| Gamma | - | Short gamma (risky) |
| Vega | - | Short vega |
| Theta | + | Collects time decay |

**Profile:** Range-bound, low volatility

---

## Practical Examples

### Example 1: Risk Analysis

```rust
use ag_risk::advanced::*;

let engine = GreeksEngine::new(GreeksConfig::default());

// SPY call option
let call = Option {
    option_type: OptionType::Call,
    strike: 450.0,
    time_to_expiry: 30.0 / 365.0,  // 30 days
    contract_size: 100.0,
};

let greeks = engine.calculate_greeks(
    &call,
    445.0,  // SPY at $445
    0.15,   // 15% IV
    0.05,   // 5% rate
)?;

println!("=== Option Greeks ===");
println!("Delta: {:.4} (moves ${:.2} per $1 move in SPY)",
    greeks.delta, greeks.delta * 100.0);
println!("Gamma: {:.6} (delta changes by {:.4} per $1 move)",
    greeks.gamma, greeks.gamma);
println!("Vega: {:.2} (changes ${:.2} per 1% vol change)",
    greeks.vega, greeks.vega);
println!("Theta: {:.2} (loses ${:.2} per day)",
    greeks.theta, greeks.theta.abs());
```

### Example 2: Hedging Recommendation

```rust
let current_greeks = PortfolioGreeks {
    total_delta: 150.0,
    total_gamma: 5.0,
    total_vega: 200.0,
    total_theta: -50.0,
    total_rho: 10.0,
    by_underlying: HashMap::new(),
};

let target_greeks = PortfolioGreeks {
    total_delta: 0.0,  // Delta neutral
    ..Default::default()
};

let available = vec!["SPY".to_string()];
let hedges = engine.suggest_hedge(&current_greeks, &target_greeks, &available)?;

for hedge in hedges {
    println!("{}: {} shares", hedge.instrument, hedge.quantity);
}
```

---

## Performance Considerations

### Calculation Speed

**Target performance:**
- Single option Greeks: <1ms
- Portfolio Greeks (100 positions): <100ms

**Optimization tips:**
1. Batch calculations for multiple strikes
2. Cache market data (prices, vols)
3. Use parallel processing for large portfolios
4. Pre-compute d₁, d₂ for reuse

### Numerical Accuracy

**Precision:**
- Delta: 4 decimal places
- Gamma: 6 decimal places (small values)
- Vega: 2 decimal places
- Theta: 2 decimal places

**Edge cases:**
- Very short time to expiry (<1 hour): Use minimum threshold
- Very high/low volatility: Clamp to reasonable range
- Deep ITM/OTM: Greeks approach limits smoothly

---

## Limitations and Considerations

### Model Limitations

1. **Black-Scholes Assumptions:**
   - European exercise only
   - No dividends (can be extended)
   - Constant volatility (violated in practice)

2. **Implied vs. Realized:**
   - Greeks use implied volatility
   - Actual P&L depends on realized volatility

3. **Large Moves:**
   - Greeks are local approximations (first/second order)
   - For large moves, higher-order Greeks matter

### Practical Adjustments

**Volatility smile:**
Black-Scholes assumes constant vol across strikes. In reality:
```
OTM puts: Higher IV (smile)
ATM: Baseline IV
OTM calls: Lower or similar IV
```

**American options:**
For American options (early exercise), use:
- Binomial tree models
- Finite difference methods
- Monte Carlo with early exercise

**Dividends:**
Adjust spot price:
```
S_adjusted = S × e^(-q×T)
where q = continuous dividend yield
```

---

## References

1. Black, F., & Scholes, M. (1973). "The Pricing of Options and Corporate Liabilities"
2. Hull, J.C. (2018). "Options, Futures, and Other Derivatives", 10th Edition
3. Taleb, N.N. (1997). "Dynamic Hedging: Managing Vanilla and Exotic Options"
4. Wilmott, P. (2006). "Paul Wilmott on Quantitative Finance"

---

## Quick Reference Card

| Greek | Measures | Formula | Call Sign | Put Sign |
|-------|----------|---------|-----------|----------|
| Delta | Price sensitivity | N(d₁) | + | - |
| Gamma | Delta curvature | N'(d₁)/(S×σ×√T) | + | + |
| Vega | Vol sensitivity | S×N'(d₁)×√T | + | + |
| Theta | Time decay | Complex | - | - |
| Rho | Rate sensitivity | K×T×e^(-r×T)×N(d₂) | + | - |

**Units:**
- Delta: Per $1 move in underlying
- Gamma: Per $1 move in underlying
- Vega: Per 1% change in volatility
- Theta: Per day
- Rho: Per 1% change in interest rate

//! Technical indicators for signal generation

use crate::types::{MarketData, Signal, SignalType, SignalMetadata, SignalGenerator};
use std::collections::{HashMap, VecDeque};
use chrono::Utc;

/// Simple Moving Average indicator
pub struct SimpleMovingAverage {
    period: usize,
    prices: VecDeque<f64>,
}

impl SimpleMovingAverage {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prices: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, price: f64) {
        self.prices.push_back(price);
        if self.prices.len() > self.period {
            self.prices.pop_front();
        }
    }

    pub fn value(&self) -> Option<f64> {
        if self.prices.len() < self.period {
            return None;
        }
        Some(self.prices.iter().sum::<f64>() / self.period as f64)
    }
}

impl SignalGenerator for SimpleMovingAverage {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        // Get latest price
        let price = data.ticks.last()
            .and_then(|t| t.last)
            .unwrap_or(0.0);

        self.update(price);

        let sma = self.value().unwrap_or(price);

        // Generate signal based on price vs SMA
        let (signal_type, strength) = if price > sma * 1.01 {
            (SignalType::Long, (price / sma - 1.0).min(1.0))
        } else if price < sma * 0.99 {
            (SignalType::Short, (1.0 - price / sma).min(1.0))
        } else {
            (SignalType::Neutral, 0.0)
        };

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if self.prices.len() >= self.period { 1.0 } else { 0.5 },
            metadata: HashMap::new(),
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("period".to_string(), self.period.to_string());

        SignalMetadata {
            name: "SMA".to_string(),
            description: format!("Simple Moving Average ({})", self.period),
            params,
        }
    }
}

/// Exponential Moving Average indicator
pub struct ExponentialMovingAverage {
    period: usize,
    alpha: f64,
    ema: Option<f64>,
}

impl ExponentialMovingAverage {
    pub fn new(period: usize) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            alpha,
            ema: None,
        }
    }

    pub fn update(&mut self, price: f64) {
        self.ema = Some(match self.ema {
            None => price,
            Some(prev) => self.alpha * price + (1.0 - self.alpha) * prev,
        });
    }

    pub fn value(&self) -> Option<f64> {
        self.ema
    }
}

impl SignalGenerator for ExponentialMovingAverage {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        let price = data.ticks.last()
            .and_then(|t| t.last)
            .unwrap_or(0.0);

        self.update(price);

        let ema = self.value().unwrap_or(price);

        let (signal_type, strength) = if price > ema * 1.01 {
            (SignalType::Long, (price / ema - 1.0).min(1.0))
        } else if price < ema * 0.99 {
            (SignalType::Short, (1.0 - price / ema).min(1.0))
        } else {
            (SignalType::Neutral, 0.0)
        };

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if self.ema.is_some() { 1.0 } else { 0.5 },
            metadata: HashMap::new(),
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("period".to_string(), self.period.to_string());

        SignalMetadata {
            name: "EMA".to_string(),
            description: format!("Exponential Moving Average ({})", self.period),
            params,
        }
    }
}

/// Relative Strength Index indicator
pub struct RelativeStrengthIndex {
    period: usize,
    gains: VecDeque<f64>,
    losses: VecDeque<f64>,
    prev_price: Option<f64>,
}

impl RelativeStrengthIndex {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            gains: VecDeque::with_capacity(period),
            losses: VecDeque::with_capacity(period),
            prev_price: None,
        }
    }

    pub fn update(&mut self, price: f64) {
        if let Some(prev) = self.prev_price {
            let change = price - prev;
            if change > 0.0 {
                self.gains.push_back(change);
                self.losses.push_back(0.0);
            } else {
                self.gains.push_back(0.0);
                self.losses.push_back(-change);
            }

            if self.gains.len() > self.period {
                self.gains.pop_front();
                self.losses.pop_front();
            }
        }
        self.prev_price = Some(price);
    }

    pub fn value(&self) -> Option<f64> {
        if self.gains.len() < self.period {
            return None;
        }

        let avg_gain = self.gains.iter().sum::<f64>() / self.period as f64;
        let avg_loss = self.losses.iter().sum::<f64>() / self.period as f64;

        if avg_loss < 1e-10 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }
}

impl SignalGenerator for RelativeStrengthIndex {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        let price = data.ticks.last()
            .and_then(|t| t.last)
            .unwrap_or(0.0);

        self.update(price);

        let rsi = self.value().unwrap_or(50.0);

        // RSI signals: oversold < 30, overbought > 70
        let (signal_type, strength) = if rsi < 30.0 {
            (SignalType::Long, (30.0 - rsi) / 30.0)
        } else if rsi > 70.0 {
            (SignalType::Short, (rsi - 70.0) / 30.0)
        } else {
            (SignalType::Neutral, 0.0)
        };

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if self.gains.len() >= self.period { 1.0 } else { 0.5 },
            metadata: {
                let mut m = HashMap::new();
                m.insert("rsi".to_string(), format!("{:.2}", rsi));
                m
            },
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("period".to_string(), self.period.to_string());

        SignalMetadata {
            name: "RSI".to_string(),
            description: format!("Relative Strength Index ({})", self.period),
            params,
        }
    }
}

/// Bollinger Bands indicator
pub struct BollingerBands {
    period: usize,
    std_dev: f64,
    sma: SimpleMovingAverage,
    prices: VecDeque<f64>,
}

impl BollingerBands {
    pub fn new(period: usize, std_dev: f64) -> Self {
        Self {
            period,
            std_dev,
            sma: SimpleMovingAverage::new(period),
            prices: VecDeque::with_capacity(period),
        }
    }

    pub fn update(&mut self, price: f64) {
        self.sma.update(price);
        self.prices.push_back(price);
        if self.prices.len() > self.period {
            self.prices.pop_front();
        }
    }

    pub fn bands(&self) -> Option<(f64, f64, f64)> {
        let middle = self.sma.value()?;

        if self.prices.len() < self.period {
            return None;
        }

        // Calculate standard deviation
        let variance = self.prices.iter()
            .map(|p| (p - middle).powi(2))
            .sum::<f64>() / self.period as f64;
        let std = variance.sqrt();

        let upper = middle + self.std_dev * std;
        let lower = middle - self.std_dev * std;

        Some((lower, middle, upper))
    }
}

impl SignalGenerator for BollingerBands {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        let price = data.ticks.last()
            .and_then(|t| t.last)
            .unwrap_or(0.0);

        self.update(price);

        let (signal_type, strength) = if let Some((lower, middle, upper)) = self.bands() {
            if price < lower {
                (SignalType::Long, ((lower - price) / (upper - lower)).min(1.0))
            } else if price > upper {
                (SignalType::Short, ((price - upper) / (upper - lower)).min(1.0))
            } else {
                (SignalType::Neutral, 0.0)
            }
        } else {
            (SignalType::Neutral, 0.0)
        };

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if self.prices.len() >= self.period { 1.0 } else { 0.5 },
            metadata: HashMap::new(),
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("period".to_string(), self.period.to_string());
        params.insert("std_dev".to_string(), self.std_dev.to_string());

        SignalMetadata {
            name: "BollingerBands".to_string(),
            description: format!("Bollinger Bands ({}, {}σ)", self.period, self.std_dev),
            params,
        }
    }
}

/// MACD (Moving Average Convergence Divergence) indicator
pub struct MovingAverageConvergenceDivergence {
    fast_ema: ExponentialMovingAverage,
    slow_ema: ExponentialMovingAverage,
    signal_ema: ExponentialMovingAverage,
    macd_values: VecDeque<f64>,
}

impl MovingAverageConvergenceDivergence {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_ema: ExponentialMovingAverage::new(fast_period),
            slow_ema: ExponentialMovingAverage::new(slow_period),
            signal_ema: ExponentialMovingAverage::new(signal_period),
            macd_values: VecDeque::new(),
        }
    }

    pub fn update(&mut self, price: f64) {
        self.fast_ema.update(price);
        self.slow_ema.update(price);

        if let (Some(fast), Some(slow)) = (self.fast_ema.value(), self.slow_ema.value()) {
            let macd = fast - slow;
            self.macd_values.push_back(macd);
            self.signal_ema.update(macd);
        }
    }

    pub fn value(&self) -> Option<(f64, f64, f64)> {
        let macd = self.macd_values.back().copied()?;
        let signal = self.signal_ema.value()?;
        let histogram = macd - signal;
        Some((macd, signal, histogram))
    }
}

impl SignalGenerator for MovingAverageConvergenceDivergence {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        let price = data.ticks.last()
            .and_then(|t| t.last)
            .unwrap_or(0.0);

        self.update(price);

        let (signal_type, strength) = if let Some((macd, signal_line, histogram)) = self.value() {
            if histogram > 0.0 && macd > signal_line {
                (SignalType::Long, histogram.abs().min(1.0))
            } else if histogram < 0.0 && macd < signal_line {
                (SignalType::Short, histogram.abs().min(1.0))
            } else {
                (SignalType::Neutral, 0.0)
            }
        } else {
            (SignalType::Neutral, 0.0)
        };

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if !self.macd_values.is_empty() { 1.0 } else { 0.5 },
            metadata: HashMap::new(),
        }
    }

    fn metadata(&self) -> SignalMetadata {
        SignalMetadata {
            name: "MACD".to_string(),
            description: "Moving Average Convergence Divergence".to_string(),
            params: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma() {
        let mut sma = SimpleMovingAverage::new(3);
        assert_eq!(sma.value(), None);

        sma.update(10.0);
        sma.update(20.0);
        assert_eq!(sma.value(), None); // Not enough data

        sma.update(30.0);
        assert_eq!(sma.value(), Some(20.0)); // (10+20+30)/3 = 20

        sma.update(40.0);
        assert_eq!(sma.value(), Some(30.0)); // (20+30+40)/3 = 30
    }

    #[test]
    fn test_ema() {
        let mut ema = ExponentialMovingAverage::new(3);
        assert_eq!(ema.value(), None);

        ema.update(10.0);
        assert!(ema.value().is_some());

        ema.update(20.0);
        ema.update(30.0);
        let value = ema.value().unwrap();
        assert!(value > 10.0 && value < 30.0);
    }

    #[test]
    fn test_rsi() {
        let mut rsi = RelativeStrengthIndex::new(3);

        rsi.update(100.0);
        rsi.update(110.0);
        rsi.update(105.0);
        rsi.update(115.0);

        let value = rsi.value();
        assert!(value.is_some());
        let v = value.unwrap();
        assert!(v >= 0.0 && v <= 100.0);
    }
}

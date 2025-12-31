//! Market microstructure signals

use crate::types::{MarketData, Signal, SignalType, SignalMetadata, SignalGenerator};
use std::collections::HashMap;
use chrono::Utc;

/// Order book imbalance signal
///
/// Generates signals based on the imbalance between bid and ask volumes
pub struct OrderImbalance {
    threshold: f64,
}

impl OrderImbalance {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    pub fn calculate_imbalance(bid_size: f64, ask_size: f64) -> f64 {
        let total = bid_size + ask_size;
        if total < 1e-8 {
            return 0.0;
        }
        (bid_size - ask_size) / total
    }
}

impl SignalGenerator for OrderImbalance {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        let tick = match data.ticks.last() {
            Some(t) => t,
            None => {
                return Signal {
                    timestamp: Utc::now(),
                    market_id: data.market.clone(),
                    signal_type: SignalType::Neutral,
                    strength: 0.0,
                    confidence: 0.0,
                    metadata: HashMap::new(),
                };
            }
        };

        let bid_size = tick.bid_size.unwrap_or(0.0);
        let ask_size = tick.ask_size.unwrap_or(0.0);

        let imbalance = Self::calculate_imbalance(bid_size, ask_size);

        let (signal_type, strength) = if imbalance > self.threshold {
            (SignalType::Long, imbalance)
        } else if imbalance < -self.threshold {
            (SignalType::Short, -imbalance)
        } else {
            (SignalType::Neutral, 0.0)
        };

        let mut metadata = HashMap::new();
        metadata.insert("imbalance".to_string(), format!("{:.4}", imbalance));
        metadata.insert("bid_size".to_string(), format!("{:.2}", bid_size));
        metadata.insert("ask_size".to_string(), format!("{:.2}", ask_size));

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if bid_size > 0.0 && ask_size > 0.0 { 1.0 } else { 0.5 },
            metadata,
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("threshold".to_string(), self.threshold.to_string());

        SignalMetadata {
            name: "OrderImbalance".to_string(),
            description: "Order book imbalance signal".to_string(),
            params,
        }
    }
}

/// Spread analyzer signal
///
/// Generates signals based on bid-ask spread dynamics
pub struct SpreadAnalyzer {
    avg_spread_bps: f64,
    tight_threshold: f64,
    wide_threshold: f64,
}

impl SpreadAnalyzer {
    pub fn new(avg_spread_bps: f64, tight_threshold: f64, wide_threshold: f64) -> Self {
        Self {
            avg_spread_bps,
            tight_threshold,
            wide_threshold,
        }
    }
}

impl SignalGenerator for SpreadAnalyzer {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        let tick = match data.ticks.last() {
            Some(t) => t,
            None => {
                return Signal {
                    timestamp: Utc::now(),
                    market_id: data.market.clone(),
                    signal_type: SignalType::Neutral,
                    strength: 0.0,
                    confidence: 0.0,
                    metadata: HashMap::new(),
                };
            }
        };

        let spread_bps = tick.spread_bps().unwrap_or(0.0);

        // Tight spread might indicate opportunity to provide liquidity
        // Wide spread might indicate uncertainty or volatility
        let (signal_type, strength) = if spread_bps < self.avg_spread_bps * self.tight_threshold {
            // Tight spread - good time to provide liquidity
            (SignalType::Neutral, 0.5)
        } else if spread_bps > self.avg_spread_bps * self.wide_threshold {
            // Wide spread - market may be moving, avoid or be cautious
            (SignalType::Close, (spread_bps / (self.avg_spread_bps * self.wide_threshold) - 1.0).min(1.0))
        } else {
            (SignalType::Neutral, 0.0)
        };

        let mut metadata = HashMap::new();
        metadata.insert("spread_bps".to_string(), format!("{:.2}", spread_bps));
        metadata.insert("avg_spread_bps".to_string(), format!("{:.2}", self.avg_spread_bps));

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength,
            confidence: if spread_bps > 0.0 { 1.0 } else { 0.5 },
            metadata,
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("avg_spread_bps".to_string(), self.avg_spread_bps.to_string());
        params.insert("tight_threshold".to_string(), self.tight_threshold.to_string());
        params.insert("wide_threshold".to_string(), self.wide_threshold.to_string());

        SignalMetadata {
            name: "SpreadAnalyzer".to_string(),
            description: "Bid-ask spread dynamics signal".to_string(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_imbalance_calculation() {
        // More bids than asks -> positive imbalance
        let imbalance = OrderImbalance::calculate_imbalance(60.0, 40.0);
        assert_eq!(imbalance, 0.2);

        // More asks than bids -> negative imbalance
        let imbalance = OrderImbalance::calculate_imbalance(40.0, 60.0);
        assert_eq!(imbalance, -0.2);

        // Balanced
        let imbalance = OrderImbalance::calculate_imbalance(50.0, 50.0);
        assert_eq!(imbalance, 0.0);

        // Zero volumes
        let imbalance = OrderImbalance::calculate_imbalance(0.0, 0.0);
        assert_eq!(imbalance, 0.0);
    }
}

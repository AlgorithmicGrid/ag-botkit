//! Composite signal generation

use crate::types::{MarketData, Signal, SignalType, SignalMetadata, SignalGenerator};
use std::collections::HashMap;
use chrono::Utc;

/// Composite signal combining multiple signals
pub struct CompositeSignal {
    generators: Vec<Box<dyn SignalGenerator>>,
    weights: Vec<f64>,
}

impl CompositeSignal {
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
            weights: Vec::new(),
        }
    }

    pub fn add_generator(&mut self, generator: Box<dyn SignalGenerator>, weight: f64) {
        self.generators.push(generator);
        self.weights.push(weight);
    }
}

impl SignalGenerator for CompositeSignal {
    fn generate_signal(&mut self, data: &MarketData) -> Signal {
        if self.generators.is_empty() {
            return Signal {
                timestamp: Utc::now(),
                market_id: data.market.clone(),
                signal_type: SignalType::Neutral,
                strength: 0.0,
                confidence: 0.0,
                metadata: HashMap::new(),
            };
        }

        // Generate signals from all generators
        let signals: Vec<Signal> = self.generators
            .iter_mut()
            .map(|gen| gen.generate_signal(data))
            .collect();

        // Calculate weighted average
        let total_weight: f64 = self.weights.iter().sum();
        let mut weighted_strength = 0.0;
        let mut weighted_confidence = 0.0;

        for (signal, &weight) in signals.iter().zip(self.weights.iter()) {
            let signal_value = match signal.signal_type {
                SignalType::Long => signal.strength,
                SignalType::Short => -signal.strength,
                SignalType::Neutral | SignalType::Close => 0.0,
            };

            weighted_strength += signal_value * weight;
            weighted_confidence += signal.confidence * weight;
        }

        weighted_strength /= total_weight;
        weighted_confidence /= total_weight;

        let signal_type = if weighted_strength > 0.1 {
            SignalType::Long
        } else if weighted_strength < -0.1 {
            SignalType::Short
        } else {
            SignalType::Neutral
        };

        Signal {
            timestamp: Utc::now(),
            market_id: data.market.clone(),
            signal_type,
            strength: weighted_strength.abs(),
            confidence: weighted_confidence,
            metadata: HashMap::new(),
        }
    }

    fn metadata(&self) -> SignalMetadata {
        let mut params = HashMap::new();
        params.insert("num_generators".to_string(), self.generators.len().to_string());

        SignalMetadata {
            name: "CompositeSignal".to_string(),
            description: "Weighted combination of multiple signals".to_string(),
            params,
        }
    }
}

/// Signal aggregator that combines signals using different strategies
pub struct SignalAggregator;

impl SignalAggregator {
    /// Calculate consensus signal (majority vote)
    pub fn consensus(signals: &[Signal]) -> Signal {
        if signals.is_empty() {
            return Signal {
                timestamp: Utc::now(),
                market_id: String::new(),
                signal_type: SignalType::Neutral,
                strength: 0.0,
                confidence: 0.0,
                metadata: HashMap::new(),
            };
        }

        let mut long_count = 0;
        let mut short_count = 0;
        let mut neutral_count = 0;

        for signal in signals {
            match signal.signal_type {
                SignalType::Long => long_count += 1,
                SignalType::Short => short_count += 1,
                SignalType::Neutral | SignalType::Close => neutral_count += 1,
            }
        }

        let total = signals.len() as f64;
        let signal_type = if long_count > short_count && long_count > neutral_count {
            SignalType::Long
        } else if short_count > long_count && short_count > neutral_count {
            SignalType::Short
        } else {
            SignalType::Neutral
        };

        let strength = signals.iter()
            .map(|s| s.strength)
            .sum::<f64>() / total;

        let confidence = signals.iter()
            .map(|s| s.confidence)
            .sum::<f64>() / total;

        Signal {
            timestamp: Utc::now(),
            market_id: signals[0].market_id.clone(),
            signal_type,
            strength,
            confidence,
            metadata: HashMap::new(),
        }
    }

    /// Calculate strongest signal
    pub fn strongest(signals: &[Signal]) -> Option<Signal> {
        signals.iter()
            .max_by(|a, b| {
                let a_score = a.strength * a.confidence;
                let b_score = b.strength * b.confidence;
                a_score.partial_cmp(&b_score).unwrap()
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_signal(signal_type: SignalType, strength: f64, confidence: f64) -> Signal {
        Signal {
            timestamp: Utc::now(),
            market_id: "test".to_string(),
            signal_type,
            strength,
            confidence,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_signal_consensus() {
        let signals = vec![
            create_test_signal(SignalType::Long, 0.8, 1.0),
            create_test_signal(SignalType::Long, 0.6, 0.9),
            create_test_signal(SignalType::Short, 0.4, 0.7),
        ];

        let consensus = SignalAggregator::consensus(&signals);
        assert_eq!(consensus.signal_type, SignalType::Long);
    }

    #[test]
    fn test_strongest_signal() {
        let signals = vec![
            create_test_signal(SignalType::Long, 0.5, 0.8),
            create_test_signal(SignalType::Short, 0.9, 0.95),
            create_test_signal(SignalType::Long, 0.6, 0.7),
        ];

        let strongest = SignalAggregator::strongest(&signals).unwrap();
        assert_eq!(strongest.signal_type, SignalType::Short);
    }
}

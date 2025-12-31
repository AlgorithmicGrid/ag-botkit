//! Strategy metrics for monitoring and analysis

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metric type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter metric (monotonically increasing)
    Counter,
    /// Gauge metric (can go up or down)
    Gauge,
    /// Histogram metric (distribution of values)
    Histogram,
}

/// Strategy metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetric {
    /// Metric timestamp
    pub timestamp: DateTime<Utc>,

    /// Strategy ID
    pub strategy_id: String,

    /// Metric type
    pub metric_type: MetricType,

    /// Metric name (e.g., "strategy.pnl_usd", "strategy.signals_generated")
    pub metric_name: String,

    /// Metric value
    pub value: f64,

    /// Additional labels for dimensions
    pub labels: HashMap<String, String>,
}

impl StrategyMetric {
    /// Create a new counter metric
    pub fn counter(
        strategy_id: String,
        name: String,
        value: f64,
        labels: HashMap<String, String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            strategy_id,
            metric_type: MetricType::Counter,
            metric_name: name,
            value,
            labels,
        }
    }

    /// Create a new gauge metric
    pub fn gauge(
        strategy_id: String,
        name: String,
        value: f64,
        labels: HashMap<String, String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            strategy_id,
            metric_type: MetricType::Gauge,
            metric_name: name,
            value,
            labels,
        }
    }

    /// Create a new histogram metric
    pub fn histogram(
        strategy_id: String,
        name: String,
        value: f64,
        labels: HashMap<String, String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            strategy_id,
            metric_type: MetricType::Histogram,
            metric_name: name,
            value,
            labels,
        }
    }
}

/// Standard strategy metric names
pub mod metric_names {
    /// Strategy PnL in USD
    pub const STRATEGY_PNL_USD: &str = "strategy.pnl_usd";

    /// Strategy unrealized PnL in USD
    pub const STRATEGY_UNREALIZED_PNL_USD: &str = "strategy.unrealized_pnl_usd";

    /// Strategy realized PnL in USD
    pub const STRATEGY_REALIZED_PNL_USD: &str = "strategy.realized_pnl_usd";

    /// Position size
    pub const POSITION_SIZE: &str = "strategy.position_size";

    /// Position value in USD
    pub const POSITION_VALUE_USD: &str = "strategy.position_value_usd";

    /// Number of signals generated
    pub const SIGNALS_GENERATED: &str = "strategy.signals_generated";

    /// Number of orders placed
    pub const ORDERS_PLACED: &str = "strategy.orders_placed";

    /// Number of orders filled
    pub const ORDERS_FILLED: &str = "strategy.orders_filled";

    /// Number of orders cancelled
    pub const ORDERS_CANCELLED: &str = "strategy.orders_cancelled";

    /// Order latency in milliseconds
    pub const ORDER_LATENCY_MS: &str = "strategy.order_latency_ms";

    /// Signal strength
    pub const SIGNAL_STRENGTH: &str = "strategy.signal_strength";

    /// Signal confidence
    pub const SIGNAL_CONFIDENCE: &str = "strategy.signal_confidence";

    /// Fill rate (fills / orders)
    pub const FILL_RATE: &str = "strategy.fill_rate";

    /// Sharpe ratio
    pub const SHARPE_RATIO: &str = "strategy.sharpe_ratio";

    /// Max drawdown
    pub const MAX_DRAWDOWN: &str = "strategy.max_drawdown";

    /// Win rate
    pub const WIN_RATE: &str = "strategy.win_rate";
}

/// Helper to create common strategy metrics
pub struct MetricBuilder {
    strategy_id: String,
}

impl MetricBuilder {
    /// Create a new metric builder
    pub fn new(strategy_id: String) -> Self {
        Self { strategy_id }
    }

    /// Build a PnL metric
    pub fn pnl(&self, market_id: &str, value: f64) -> StrategyMetric {
        let mut labels = HashMap::new();
        labels.insert("market".to_string(), market_id.to_string());

        StrategyMetric::gauge(
            self.strategy_id.clone(),
            metric_names::STRATEGY_PNL_USD.to_string(),
            value,
            labels,
        )
    }

    /// Build a position size metric
    pub fn position_size(&self, market_id: &str, size: f64) -> StrategyMetric {
        let mut labels = HashMap::new();
        labels.insert("market".to_string(), market_id.to_string());

        StrategyMetric::gauge(
            self.strategy_id.clone(),
            metric_names::POSITION_SIZE.to_string(),
            size,
            labels,
        )
    }

    /// Build a signal generated metric
    pub fn signal_generated(&self, market_id: &str, signal_type: &str) -> StrategyMetric {
        let mut labels = HashMap::new();
        labels.insert("market".to_string(), market_id.to_string());
        labels.insert("signal_type".to_string(), signal_type.to_string());

        StrategyMetric::counter(
            self.strategy_id.clone(),
            metric_names::SIGNALS_GENERATED.to_string(),
            1.0,
            labels,
        )
    }

    /// Build an order placed metric
    pub fn order_placed(&self, market_id: &str, side: &str) -> StrategyMetric {
        let mut labels = HashMap::new();
        labels.insert("market".to_string(), market_id.to_string());
        labels.insert("side".to_string(), side.to_string());

        StrategyMetric::counter(
            self.strategy_id.clone(),
            metric_names::ORDERS_PLACED.to_string(),
            1.0,
            labels,
        )
    }

    /// Build an order filled metric
    pub fn order_filled(&self, market_id: &str) -> StrategyMetric {
        let mut labels = HashMap::new();
        labels.insert("market".to_string(), market_id.to_string());

        StrategyMetric::counter(
            self.strategy_id.clone(),
            metric_names::ORDERS_FILLED.to_string(),
            1.0,
            labels,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_creation() {
        let metric = StrategyMetric::gauge(
            "test_strategy".to_string(),
            "test.metric".to_string(),
            42.0,
            HashMap::new(),
        );

        assert_eq!(metric.strategy_id, "test_strategy");
        assert_eq!(metric.metric_name, "test.metric");
        assert_eq!(metric.value, 42.0);
        assert_eq!(metric.metric_type, MetricType::Gauge);
    }

    #[test]
    fn test_metric_builder() {
        let builder = MetricBuilder::new("test_strategy".to_string());

        let pnl_metric = builder.pnl("market1", 125.50);
        assert_eq!(pnl_metric.metric_name, metric_names::STRATEGY_PNL_USD);
        assert_eq!(pnl_metric.value, 125.50);
        assert_eq!(pnl_metric.labels.get("market"), Some(&"market1".to_string()));

        let signal_metric = builder.signal_generated("market1", "long");
        assert_eq!(signal_metric.metric_name, metric_names::SIGNALS_GENERATED);
        assert_eq!(signal_metric.value, 1.0);
    }
}

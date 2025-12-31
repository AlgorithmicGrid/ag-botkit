use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Metric data point for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub metric_name: String,
    pub value: f64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

impl MetricPoint {
    /// Create a new metric point
    pub fn new(metric_name: impl Into<String>, value: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            metric_name: metric_name.into(),
            value,
            labels: HashMap::new(),
        }
    }

    /// Add a label to the metric
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Set timestamp
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Aggregation types for metrics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Aggregation {
    Avg,
    Min,
    Max,
    Sum,
    Count,
    Median,
    P95,
    P99,
    StdDev,
}

impl Aggregation {
    pub fn as_sql(&self) -> &str {
        match self {
            Aggregation::Avg => "AVG",
            Aggregation::Min => "MIN",
            Aggregation::Max => "MAX",
            Aggregation::Sum => "SUM",
            Aggregation::Count => "COUNT",
            Aggregation::Median => "PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY value)",
            Aggregation::P95 => "PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY value)",
            Aggregation::P99 => "PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY value)",
            Aggregation::StdDev => "STDDEV",
        }
    }
}

/// Aggregated metric result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub bucket: DateTime<Utc>,
    pub metric_name: String,
    pub labels: HashMap<String, String>,
    pub avg_value: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub median_value: Option<f64>,
    pub p95_value: Option<f64>,
    pub p99_value: Option<f64>,
    pub stddev_value: Option<f64>,
    pub count: i64,
}

/// Order side
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "buy"),
            Side::Sell => write!(f, "sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Limit,
    Market,
    StopLimit,
    StopMarket,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Limit => write!(f, "limit"),
            OrderType::Market => write!(f, "market"),
            OrderType::StopLimit => write!(f, "stop_limit"),
            OrderType::StopMarket => write!(f, "stop_market"),
        }
    }
}

/// Order status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Open,
    Partial,
    Filled,
    Cancelled,
    Rejected,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Open => write!(f, "open"),
            OrderStatus::Partial => write!(f, "partial"),
            OrderStatus::Filled => write!(f, "filled"),
            OrderStatus::Cancelled => write!(f, "cancelled"),
            OrderStatus::Rejected => write!(f, "rejected"),
        }
    }
}

/// Order record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub venue: String,
    pub market: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub size: f64,
    pub status: OrderStatus,
    pub client_order_id: String,
    pub venue_order_id: Option<String>,
    pub time_in_force: Option<String>,
}

impl Order {
    pub fn new(
        venue: impl Into<String>,
        market: impl Into<String>,
        side: Side,
        order_type: OrderType,
        size: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            venue: venue.into(),
            market: market.into(),
            side,
            order_type,
            price: None,
            size,
            status: OrderStatus::Open,
            client_order_id: Uuid::new_v4().to_string(),
            venue_order_id: None,
            time_in_force: None,
        }
    }

    pub fn with_price(mut self, price: f64) -> Self {
        self.price = Some(price);
        self
    }

    pub fn with_status(mut self, status: OrderStatus) -> Self {
        self.status = status;
        self
    }
}

/// Fill/Trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub order_id: Uuid,
    pub venue: String,
    pub market: String,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub fee_currency: String,
    pub trade_id: Option<String>,
    pub liquidity: Option<String>,
}

impl Fill {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        order_id: Uuid,
        venue: impl Into<String>,
        market: impl Into<String>,
        side: Side,
        price: f64,
        size: f64,
        fee: f64,
        fee_currency: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            order_id,
            venue: venue.into(),
            market: market.into(),
            side,
            price,
            size,
            fee,
            fee_currency: fee_currency.into(),
            trade_id: None,
            liquidity: None,
        }
    }
}

/// Position snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub timestamp: DateTime<Utc>,
    pub market: String,
    pub venue: String,
    pub size: f64,
    pub avg_entry_price: f64,
    pub unrealized_pnl: Option<f64>,
    pub realized_pnl: Option<f64>,
    pub mark_price: Option<f64>,
}

impl PositionSnapshot {
    pub fn new(
        venue: impl Into<String>,
        market: impl Into<String>,
        size: f64,
        avg_entry_price: f64,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            market: market.into(),
            venue: venue.into(),
            size,
            avg_entry_price,
            unrealized_pnl: None,
            realized_pnl: None,
            mark_price: None,
        }
    }

    pub fn with_pnl(mut self, unrealized: f64, realized: f64) -> Self {
        self.unrealized_pnl = Some(unrealized);
        self.realized_pnl = Some(realized);
        self
    }
}

/// Filter for querying orders
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderFilters {
    pub venue: Option<String>,
    pub market: Option<String>,
    pub side: Option<Side>,
    pub status: Option<OrderStatus>,
    pub client_order_id: Option<String>,
}

/// Retention policy report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionReport {
    pub metrics_deleted: i64,
    pub orders_deleted: i64,
    pub fills_deleted: i64,
    pub positions_deleted: i64,
    pub executed_at: DateTime<Utc>,
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_point() {
        let metric = MetricPoint::new("test.metric", 123.45)
            .with_label("venue", "polymarket")
            .with_label("market", "0x123abc");

        assert_eq!(metric.metric_name, "test.metric");
        assert_eq!(metric.value, 123.45);
        assert_eq!(metric.labels.get("venue"), Some(&"polymarket".to_string()));
    }

    #[test]
    fn test_order() {
        let order = Order::new("polymarket", "0x123abc", Side::Buy, OrderType::Limit, 100.0)
            .with_price(0.52)
            .with_status(OrderStatus::Open);

        assert_eq!(order.venue, "polymarket");
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.price, Some(0.52));
        assert_eq!(order.status, OrderStatus::Open);
    }

    #[test]
    fn test_fill() {
        let order_id = Uuid::new_v4();
        let fill = Fill::new(
            order_id,
            "polymarket",
            "0x123abc",
            Side::Buy,
            0.52,
            100.0,
            0.1,
            "USDC",
        );

        assert_eq!(fill.order_id, order_id);
        assert_eq!(fill.price, 0.52);
        assert_eq!(fill.size, 100.0);
    }
}

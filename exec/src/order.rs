//! Order types and related data structures
//!
//! This module defines the core order types used throughout the execution gateway.
//! All order types are venue-agnostic and normalized to a common representation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub Uuid);

impl OrderId {
    /// Generate a new random order ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an OrderId from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Venue identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VenueId(pub String);

impl VenueId {
    /// Create a new VenueId
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the venue identifier string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VenueId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Market identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

impl MarketId {
    /// Create a new MarketId
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the market identifier string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Side {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "BUY"),
            Side::Sell => write!(f, "SELL"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Limit order with specified price
    Limit,
    /// Market order (execute at current market price)
    Market,
    /// Post-only order (always maker, never taker)
    PostOnly,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Limit => write!(f, "LIMIT"),
            OrderType::Market => write!(f, "MARKET"),
            OrderType::PostOnly => write!(f, "POST_ONLY"),
        }
    }
}

/// Time in force policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till cancelled
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
        }
    }
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is pending submission
    Pending,
    /// Order is being submitted to venue
    Submitting,
    /// Order is working on the exchange
    Working,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is completely filled
    Filled,
    /// Order is being cancelled
    Cancelling,
    /// Order has been cancelled
    Cancelled,
    /// Order was rejected by venue
    Rejected,
    /// Order expired
    Expired,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "PENDING"),
            OrderStatus::Submitting => write!(f, "SUBMITTING"),
            OrderStatus::Working => write!(f, "WORKING"),
            OrderStatus::PartiallyFilled => write!(f, "PARTIALLY_FILLED"),
            OrderStatus::Filled => write!(f, "FILLED"),
            OrderStatus::Cancelling => write!(f, "CANCELLING"),
            OrderStatus::Cancelled => write!(f, "CANCELLED"),
            OrderStatus::Rejected => write!(f, "REJECTED"),
            OrderStatus::Expired => write!(f, "EXPIRED"),
        }
    }
}

/// Unified order representation across all venues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order identifier
    pub id: OrderId,

    /// Venue where order should be placed
    pub venue: VenueId,

    /// Market identifier
    pub market: MarketId,

    /// Order side (buy/sell)
    pub side: Side,

    /// Order type
    pub order_type: OrderType,

    /// Limit price (None for market orders)
    pub price: Option<f64>,

    /// Order size
    pub size: f64,

    /// Time in force policy
    pub time_in_force: TimeInForce,

    /// Client-specified order ID for tracking
    pub client_order_id: String,

    /// Current order status
    pub status: OrderStatus,

    /// Filled quantity
    pub filled_size: f64,

    /// Average fill price
    pub avg_fill_price: Option<f64>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Order {
    /// Create a new order
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        venue: VenueId,
        market: MarketId,
        side: Side,
        order_type: OrderType,
        price: Option<f64>,
        size: f64,
        time_in_force: TimeInForce,
        client_order_id: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: OrderId::new(),
            venue,
            market,
            side,
            order_type,
            price,
            size,
            time_in_force,
            client_order_id,
            status: OrderStatus::Pending,
            filled_size: 0.0,
            avg_fill_price: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if order is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected | OrderStatus::Expired
        )
    }

    /// Check if order is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Working | OrderStatus::PartiallyFilled
        )
    }

    /// Update order status
    pub fn update_status(&mut self, status: OrderStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Record a fill
    pub fn record_fill(&mut self, fill_size: f64, fill_price: f64) {
        self.filled_size += fill_size;

        // Update average fill price
        if let Some(avg_price) = self.avg_fill_price {
            let total_value = avg_price * (self.filled_size - fill_size) + fill_price * fill_size;
            self.avg_fill_price = Some(total_value / self.filled_size);
        } else {
            self.avg_fill_price = Some(fill_price);
        }

        // Update status based on fill
        if (self.filled_size - self.size).abs() < f64::EPSILON {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }

        self.updated_at = Utc::now();
    }

    /// Get remaining size
    pub fn remaining_size(&self) -> f64 {
        self.size - self.filled_size
    }
}

/// Order acknowledgement from venue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAck {
    /// Order ID
    pub order_id: OrderId,

    /// Venue-specific order ID
    pub venue_order_id: Option<String>,

    /// Order status
    pub status: OrderStatus,

    /// Acknowledgement timestamp
    pub timestamp: DateTime<Utc>,

    /// Optional message from venue
    pub message: Option<String>,
}

/// Cancel acknowledgement from venue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAck {
    /// Order ID that was cancelled
    pub order_id: OrderId,

    /// Venue-specific order ID
    pub venue_order_id: Option<String>,

    /// Cancel success status
    pub success: bool,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Optional message from venue
    pub message: Option<String>,
}

/// Fill notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    /// Fill ID
    pub fill_id: String,

    /// Order ID
    pub order_id: OrderId,

    /// Venue-specific order ID
    pub venue_order_id: Option<String>,

    /// Fill price
    pub price: f64,

    /// Fill size
    pub size: f64,

    /// Fee amount
    pub fee: f64,

    /// Fee currency
    pub fee_currency: String,

    /// Fill timestamp
    pub timestamp: DateTime<Utc>,

    /// Liquidity role (maker/taker)
    pub liquidity: Option<Liquidity>,
}

/// Liquidity role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Liquidity {
    /// Order was maker (added liquidity)
    Maker,
    /// Order was taker (removed liquidity)
    Taker,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_id_generation() {
        let id1 = OrderId::new();
        let id2 = OrderId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_order_creation() {
        let order = Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        );

        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(order.filled_size, 0.0);
        assert_eq!(order.size, 100.0);
    }

    #[test]
    fn test_order_fill_recording() {
        let mut order = Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        );

        // Record partial fill
        order.record_fill(50.0, 0.51);
        assert_eq!(order.filled_size, 50.0);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.avg_fill_price, Some(0.51));

        // Record second fill
        order.record_fill(50.0, 0.53);
        assert_eq!(order.filled_size, 100.0);
        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.avg_fill_price, Some(0.52));
    }

    #[test]
    fn test_order_terminal_status() {
        let mut order = Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        );

        assert!(!order.is_terminal());

        order.update_status(OrderStatus::Working);
        assert!(!order.is_terminal());
        assert!(order.is_active());

        order.update_status(OrderStatus::Filled);
        assert!(order.is_terminal());
        assert!(!order.is_active());
    }

    #[test]
    fn test_remaining_size() {
        let mut order = Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        );

        assert_eq!(order.remaining_size(), 100.0);

        order.record_fill(30.0, 0.51);
        assert_eq!(order.remaining_size(), 70.0);
    }
}

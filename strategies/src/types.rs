//! Core types for the strategy framework

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

pub use crate::{Strategy, StrategyError, StrategyResult};

/// Unique identifier for an order
pub type OrderId = String;

/// Unique identifier for a market
pub type MarketId = String;

/// Unique identifier for a venue
pub type VenueId = String;

/// Order side (buy/sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order - execute at best available price
    Market,
    /// Limit order - execute at specified price or better
    Limit,
    /// Stop order - trigger market order when price reached
    Stop,
    /// Stop-limit order - trigger limit order when price reached
    StopLimit,
}

/// Time in force specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till cancelled
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
    /// Good till date
    GTD,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order created but not yet submitted
    Pending,
    /// Order submitted to venue
    Submitted,
    /// Order acknowledged by venue
    Acknowledged,
    /// Order partially filled
    PartiallyFilled,
    /// Order fully filled
    Filled,
    /// Order cancelled
    Cancelled,
    /// Order rejected
    Rejected,
    /// Order expired
    Expired,
}

/// Order structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Optional order ID (assigned by execution engine)
    pub id: Option<OrderId>,
    /// Venue identifier
    pub venue: VenueId,
    /// Market identifier
    pub market: MarketId,
    /// Order side
    pub side: Side,
    /// Order type
    pub order_type: OrderType,
    /// Price (None for market orders)
    pub price: Option<f64>,
    /// Order size
    pub size: f64,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Client order ID for tracking
    pub client_order_id: Option<String>,
    /// Order creation timestamp
    pub timestamp: DateTime<Utc>,
    /// Order status
    pub status: OrderStatus,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            id: None,
            venue: String::new(),
            market: String::new(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: None,
            size: 0.0,
            time_in_force: TimeInForce::GTC,
            client_order_id: None,
            timestamp: Utc::now(),
            status: OrderStatus::Pending,
        }
    }
}

/// Fill notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    /// Order ID
    pub order_id: OrderId,
    /// Market identifier
    pub market: MarketId,
    /// Fill price
    pub price: f64,
    /// Fill size
    pub size: f64,
    /// Side
    pub side: Side,
    /// Trading fee
    pub fee: f64,
    /// Fill timestamp
    pub timestamp: DateTime<Utc>,
}

/// Trade (completed fill)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Trade ID
    pub id: String,
    /// Market identifier
    pub market: MarketId,
    /// Trade price
    pub price: f64,
    /// Trade size
    pub size: f64,
    /// Side
    pub side: Side,
    /// Trading fee
    pub fee: f64,
    /// Trade timestamp
    pub timestamp: DateTime<Utc>,
}

/// Position in a market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Market identifier
    pub market: MarketId,
    /// Position size (positive = long, negative = short)
    pub size: f64,
    /// Average entry price
    pub entry_price: f64,
    /// Current mark price
    pub mark_price: f64,
    /// Unrealized PnL
    pub unrealized_pnl: f64,
    /// Realized PnL
    pub realized_pnl: f64,
    /// Position value in USD
    pub value_usd: f64,
    /// Last update timestamp
    pub timestamp: DateTime<Utc>,
}

impl Position {
    /// Create a new empty position
    pub fn new(market: MarketId) -> Self {
        Self {
            market,
            size: 0.0,
            entry_price: 0.0,
            mark_price: 0.0,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            value_usd: 0.0,
            timestamp: Utc::now(),
        }
    }

    /// Check if position is flat (no exposure)
    pub fn is_flat(&self) -> bool {
        self.size.abs() < 1e-8
    }

    /// Check if position is long
    pub fn is_long(&self) -> bool {
        self.size > 1e-8
    }

    /// Check if position is short
    pub fn is_short(&self) -> bool {
        self.size < -1e-8
    }
}

/// Market tick data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTick {
    /// Market identifier
    pub market: MarketId,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Best bid price
    pub bid: Option<f64>,
    /// Best bid size
    pub bid_size: Option<f64>,
    /// Best ask price
    pub ask: Option<f64>,
    /// Best ask size
    pub ask_size: Option<f64>,
    /// Last trade price
    pub last: Option<f64>,
    /// 24h volume
    pub volume_24h: Option<f64>,
}

impl MarketTick {
    /// Calculate mid price
    pub fn mid_price(&self) -> f64 {
        match (self.bid, self.ask) {
            (Some(bid), Some(ask)) => (bid + ask) / 2.0,
            (Some(bid), None) => bid,
            (None, Some(ask)) => ask,
            (None, None) => self.last.unwrap_or(0.0),
        }
    }

    /// Calculate spread
    pub fn spread(&self) -> Option<f64> {
        match (self.bid, self.ask) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Calculate spread in basis points
    pub fn spread_bps(&self) -> Option<f64> {
        self.spread().map(|s| s / self.mid_price() * 10000.0)
    }
}

/// Historical market data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Market identifier
    pub market: MarketId,
    /// Historical ticks
    pub ticks: Vec<MarketTick>,
    /// OHLCV bars (if available)
    pub bars: Vec<OhlcvBar>,
}

/// OHLCV bar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvBar {
    /// Timestamp (bar start)
    pub timestamp: DateTime<Utc>,
    /// Open price
    pub open: f64,
    /// High price
    pub high: f64,
    /// Low price
    pub low: f64,
    /// Close price
    pub close: f64,
    /// Volume
    pub volume: f64,
}

/// Signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    /// Long signal
    Long,
    /// Short signal
    Short,
    /// Neutral/no signal
    Neutral,
    /// Close position signal
    Close,
}

/// Trading signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Signal timestamp
    pub timestamp: DateTime<Utc>,
    /// Market identifier
    pub market_id: String,
    /// Signal type
    pub signal_type: SignalType,
    /// Signal strength (-1.0 to 1.0)
    pub strength: f64,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Signal metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMetadata {
    /// Signal generator name
    pub name: String,
    /// Description
    pub description: String,
    /// Parameters
    pub params: HashMap<String, String>,
}

/// Signal generator trait
pub trait SignalGenerator: Send + Sync {
    /// Generate signal from market data
    fn generate_signal(&mut self, data: &MarketData) -> Signal;

    /// Get signal metadata
    fn metadata(&self) -> SignalMetadata;
}

/// Strategy metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetadata {
    /// Strategy name
    pub name: String,
    /// Strategy version
    pub version: String,
    /// Strategy description
    pub description: String,
    /// Markets this strategy trades
    pub markets: Vec<String>,
    /// Required parameters
    pub required_params: Vec<String>,
}

/// Strategy parameters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyParams {
    /// Parameter key-value map
    pub params: HashMap<String, String>,
}

impl StrategyParams {
    /// Create new empty parameters
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Get parameter value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Get parameter as typed value
    pub fn get_typed<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.params.get(key).and_then(|s| s.parse().ok())
    }

    /// Set parameter value
    pub fn set(&mut self, key: String, value: String) {
        self.params.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_states() {
        let mut pos = Position::new("market1".to_string());
        assert!(pos.is_flat());

        pos.size = 100.0;
        assert!(pos.is_long());
        assert!(!pos.is_short());

        pos.size = -50.0;
        assert!(pos.is_short());
        assert!(!pos.is_long());
    }

    #[test]
    fn test_market_tick_calculations() {
        let tick = MarketTick {
            market: "market1".to_string(),
            timestamp: Utc::now(),
            bid: Some(100.0),
            ask: Some(101.0),
            bid_size: Some(10.0),
            ask_size: Some(10.0),
            last: Some(100.5),
            volume_24h: Some(1000.0),
        };

        assert_eq!(tick.mid_price(), 100.5);
        assert_eq!(tick.spread(), Some(1.0));
        assert!((tick.spread_bps().unwrap() - 99.50).abs() < 0.1);
    }

    #[test]
    fn test_strategy_params() {
        let mut params = StrategyParams::new();
        params.set("max_position".to_string(), "1000".to_string());
        params.set("spread".to_string(), "0.01".to_string());

        assert_eq!(params.get("max_position"), Some("1000"));
        assert_eq!(params.get_typed::<f64>("spread"), Some(0.01));
        assert_eq!(params.get_typed::<i32>("max_position"), Some(1000));
    }
}

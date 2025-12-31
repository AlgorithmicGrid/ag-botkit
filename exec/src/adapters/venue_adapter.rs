//! Venue adapter trait and common functionality
//!
//! This module defines the VenueAdapter trait that all venue-specific implementations
//! must implement. It provides a uniform interface for interacting with different exchanges.

use async_trait::async_trait;

use crate::error::ExecResult;
use crate::order::{CancelAck, Order, OrderAck, OrderId, OrderStatus, VenueId};

/// Venue adapter trait
///
/// All venue-specific implementations (Polymarket, Binance, etc.) must implement
/// this trait to provide a uniform interface for order operations.
#[async_trait]
pub trait VenueAdapter: Send + Sync {
    /// Get the venue identifier
    fn venue_id(&self) -> VenueId;

    /// Place an order on the venue
    ///
    /// # Arguments
    /// * `order` - The order to place
    ///
    /// # Returns
    /// * `Ok(OrderAck)` - Order was successfully placed
    /// * `Err(ExecError)` - Order placement failed
    async fn place_order(&mut self, order: &Order) -> ExecResult<OrderAck>;

    /// Cancel an order on the venue
    ///
    /// # Arguments
    /// * `order_id` - The order ID to cancel
    ///
    /// # Returns
    /// * `Ok(CancelAck)` - Order was successfully cancelled
    /// * `Err(ExecError)` - Order cancellation failed
    async fn cancel_order(&mut self, order_id: &OrderId) -> ExecResult<CancelAck>;

    /// Get the current status of an order
    ///
    /// # Arguments
    /// * `order_id` - The order ID to query
    ///
    /// # Returns
    /// * `Ok(OrderStatus)` - Current order status
    /// * `Err(ExecError)` - Status query failed
    async fn get_order_status(&mut self, order_id: &OrderId) -> ExecResult<OrderStatus>;

    /// Get all open orders
    ///
    /// # Returns
    /// * `Ok(Vec<Order>)` - List of open orders
    /// * `Err(ExecError)` - Query failed
    async fn get_open_orders(&mut self) -> ExecResult<Vec<Order>>;

    /// Modify an existing order
    ///
    /// # Arguments
    /// * `order_id` - The order ID to modify
    /// * `new_price` - New price (None to keep current)
    /// * `new_size` - New size (None to keep current)
    ///
    /// # Returns
    /// * `Ok(OrderAck)` - Order was successfully modified
    /// * `Err(ExecError)` - Order modification failed
    async fn modify_order(
        &mut self,
        order_id: &OrderId,
        new_price: Option<f64>,
        new_size: Option<f64>,
    ) -> ExecResult<OrderAck>;

    /// Check if the venue is healthy and reachable
    ///
    /// # Returns
    /// * `Ok(true)` - Venue is healthy
    /// * `Ok(false)` - Venue is unhealthy
    /// * `Err(ExecError)` - Health check failed
    async fn health_check(&mut self) -> ExecResult<bool>;
}

/// Venue configuration
#[derive(Debug, Clone)]
pub struct VenueConfig {
    /// Venue identifier
    pub venue_id: VenueId,

    /// API endpoint URL
    pub api_endpoint: String,

    /// WebSocket endpoint URL (if applicable)
    pub ws_endpoint: Option<String>,

    /// API key
    pub api_key: Option<String>,

    /// API secret
    pub api_secret: Option<String>,

    /// Additional venue-specific configuration
    pub extra: std::collections::HashMap<String, String>,
}

impl VenueConfig {
    /// Create a new venue configuration
    pub fn new(venue_id: VenueId, api_endpoint: String) -> Self {
        Self {
            venue_id,
            api_endpoint,
            ws_endpoint: None,
            api_key: None,
            api_secret: None,
            extra: std::collections::HashMap::new(),
        }
    }

    /// Set API credentials
    pub fn with_credentials(mut self, api_key: String, api_secret: String) -> Self {
        self.api_key = Some(api_key);
        self.api_secret = Some(api_secret);
        self
    }

    /// Set WebSocket endpoint
    pub fn with_ws_endpoint(mut self, ws_endpoint: String) -> Self {
        self.ws_endpoint = Some(ws_endpoint);
        self
    }

    /// Add extra configuration parameter
    pub fn with_extra(mut self, key: String, value: String) -> Self {
        self.extra.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_venue_config_builder() {
        let config = VenueConfig::new(
            VenueId::new("polymarket"),
            "https://clob.polymarket.com".to_string(),
        )
        .with_credentials("api_key".to_string(), "api_secret".to_string())
        .with_ws_endpoint("wss://ws-subscriptions.polymarket.com".to_string())
        .with_extra("chain_id".to_string(), "137".to_string());

        assert_eq!(config.venue_id.as_str(), "polymarket");
        assert_eq!(config.api_endpoint, "https://clob.polymarket.com");
        assert!(config.api_key.is_some());
        assert!(config.api_secret.is_some());
        assert!(config.ws_endpoint.is_some());
        assert_eq!(config.extra.get("chain_id"), Some(&"137".to_string()));
    }
}

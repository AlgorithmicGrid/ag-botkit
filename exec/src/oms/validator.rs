//! Order validation logic
//!
//! This module provides order validation to ensure orders meet basic requirements
//! before being submitted to venues.

use crate::error::{ExecError, ExecResult};
use crate::order::{Order, OrderType};

/// Order validator
pub struct OrderValidator {
    /// Minimum order size
    min_size: f64,
    /// Maximum order size
    max_size: f64,
    /// Minimum price (for limit orders)
    min_price: f64,
    /// Maximum price (for limit orders)
    max_price: f64,
}

impl OrderValidator {
    /// Create a new order validator with default limits
    pub fn new() -> Self {
        Self {
            min_size: 0.01,
            max_size: 1_000_000.0,
            min_price: 0.0001,
            max_price: 1.0,
        }
    }

    /// Create a custom validator
    pub fn custom(min_size: f64, max_size: f64, min_price: f64, max_price: f64) -> Self {
        Self {
            min_size,
            max_size,
            min_price,
            max_price,
        }
    }

    /// Validate an order
    pub fn validate(&self, order: &Order) -> ExecResult<()> {
        // Validate size
        if order.size < self.min_size {
            return Err(ExecError::ValidationError(format!(
                "Order size {} below minimum {}",
                order.size, self.min_size
            )));
        }

        if order.size > self.max_size {
            return Err(ExecError::ValidationError(format!(
                "Order size {} exceeds maximum {}",
                order.size, self.max_size
            )));
        }

        if order.size <= 0.0 {
            return Err(ExecError::ValidationError(
                "Order size must be positive".to_string(),
            ));
        }

        // Validate price for limit orders
        if order.order_type == OrderType::Limit || order.order_type == OrderType::PostOnly {
            match order.price {
                Some(price) => {
                    if price < self.min_price {
                        return Err(ExecError::ValidationError(format!(
                            "Price {} below minimum {}",
                            price, self.min_price
                        )));
                    }

                    if price > self.max_price {
                        return Err(ExecError::ValidationError(format!(
                            "Price {} exceeds maximum {}",
                            price, self.max_price
                        )));
                    }

                    if price <= 0.0 {
                        return Err(ExecError::ValidationError(
                            "Price must be positive".to_string(),
                        ));
                    }
                }
                None => {
                    return Err(ExecError::ValidationError(
                        "Limit order must have a price".to_string(),
                    ));
                }
            }
        }

        // Market orders should not have a price
        if order.order_type == OrderType::Market && order.price.is_some() {
            return Err(ExecError::ValidationError(
                "Market order should not have a price".to_string(),
            ));
        }

        // Validate market ID is not empty
        if order.market.as_str().is_empty() {
            return Err(ExecError::ValidationError(
                "Market ID cannot be empty".to_string(),
            ));
        }

        // Validate venue ID is not empty
        if order.venue.as_str().is_empty() {
            return Err(ExecError::ValidationError(
                "Venue ID cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Set minimum size
    pub fn set_min_size(&mut self, min_size: f64) {
        self.min_size = min_size;
    }

    /// Set maximum size
    pub fn set_max_size(&mut self, max_size: f64) {
        self.max_size = max_size;
    }

    /// Set minimum price
    pub fn set_min_price(&mut self, min_price: f64) {
        self.min_price = min_price;
    }

    /// Set maximum price
    pub fn set_max_price(&mut self, max_price: f64) {
        self.max_price = max_price;
    }
}

impl Default for OrderValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{MarketId, Side, TimeInForce, VenueId};

    fn create_test_order(size: f64, price: Option<f64>, order_type: OrderType) -> Order {
        Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            order_type,
            price,
            size,
            TimeInForce::GTC,
            "client-123".to_string(),
        )
    }

    #[test]
    fn test_valid_limit_order() {
        let validator = OrderValidator::new();
        let order = create_test_order(100.0, Some(0.52), OrderType::Limit);

        assert!(validator.validate(&order).is_ok());
    }

    #[test]
    fn test_valid_market_order() {
        let validator = OrderValidator::new();
        let order = create_test_order(100.0, None, OrderType::Market);

        assert!(validator.validate(&order).is_ok());
    }

    #[test]
    fn test_size_too_small() {
        let validator = OrderValidator::new();
        let order = create_test_order(0.001, Some(0.52), OrderType::Limit);

        let result = validator.validate(&order);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExecError::ValidationError(_))));
    }

    #[test]
    fn test_size_too_large() {
        let validator = OrderValidator::new();
        let order = create_test_order(2_000_000.0, Some(0.52), OrderType::Limit);

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_size() {
        let validator = OrderValidator::new();
        let order = create_test_order(-10.0, Some(0.52), OrderType::Limit);

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_limit_order_without_price() {
        let validator = OrderValidator::new();
        let order = create_test_order(100.0, None, OrderType::Limit);

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_market_order_with_price() {
        let validator = OrderValidator::new();
        let order = create_test_order(100.0, Some(0.52), OrderType::Market);

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_price_too_low() {
        let validator = OrderValidator::new();
        let order = create_test_order(100.0, Some(0.00001), OrderType::Limit);

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_price_too_high() {
        let validator = OrderValidator::new();
        let order = create_test_order(100.0, Some(1.5), OrderType::Limit);

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_validator() {
        let validator = OrderValidator::custom(10.0, 1000.0, 0.01, 0.99);
        let order = create_test_order(50.0, Some(0.5), OrderType::Limit);

        assert!(validator.validate(&order).is_ok());

        let small_order = create_test_order(5.0, Some(0.5), OrderType::Limit);
        assert!(validator.validate(&small_order).is_err());
    }

    #[test]
    fn test_empty_market_id() {
        let validator = OrderValidator::new();
        let mut order = create_test_order(100.0, Some(0.52), OrderType::Limit);
        order.market = MarketId::new("");

        let result = validator.validate(&order);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_venue_id() {
        let validator = OrderValidator::new();
        let mut order = create_test_order(100.0, Some(0.52), OrderType::Limit);
        order.venue = VenueId::new("");

        let result = validator.validate(&order);
        assert!(result.is_err());
    }
}

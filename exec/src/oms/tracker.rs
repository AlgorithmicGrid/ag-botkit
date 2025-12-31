//! Order Management System - Order tracking
//!
//! This module provides order state tracking and lifecycle management.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::error::{ExecError, ExecResult};
use crate::order::{Fill, Order, OrderId, OrderStatus};

/// Order tracker for managing order lifecycle
pub struct OrderTracker {
    orders: Arc<RwLock<HashMap<OrderId, Order>>>,
    fills: Arc<RwLock<HashMap<OrderId, Vec<Fill>>>>,
}

impl OrderTracker {
    /// Create a new order tracker
    pub fn new() -> Self {
        Self {
            orders: Arc::new(RwLock::new(HashMap::new())),
            fills: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Track a new order
    pub fn track_order(&self, order: Order) -> ExecResult<()> {
        let mut orders = self.orders.write().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire write lock: {}", e))
        })?;

        orders.insert(order.id, order);
        Ok(())
    }

    /// Get an order by ID
    pub fn get_order(&self, order_id: &OrderId) -> ExecResult<Order> {
        let orders = self.orders.read().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire read lock: {}", e))
        })?;

        orders
            .get(order_id)
            .cloned()
            .ok_or_else(|| ExecError::OrderNotFound(*order_id))
    }

    /// Update order status
    pub fn update_status(&self, order_id: &OrderId, status: OrderStatus) -> ExecResult<()> {
        let mut orders = self.orders.write().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire write lock: {}", e))
        })?;

        let order = orders
            .get_mut(order_id)
            .ok_or_else(|| ExecError::OrderNotFound(*order_id))?;

        order.update_status(status);
        Ok(())
    }

    /// Record a fill for an order
    pub fn record_fill(&self, order_id: &OrderId, fill: Fill) -> ExecResult<()> {
        // Update order with fill information
        {
            let mut orders = self.orders.write().map_err(|e| {
                ExecError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            let order = orders
                .get_mut(order_id)
                .ok_or_else(|| ExecError::OrderNotFound(*order_id))?;

            order.record_fill(fill.size, fill.price);
        }

        // Store fill record
        {
            let mut fills = self.fills.write().map_err(|e| {
                ExecError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            fills.entry(*order_id).or_insert_with(Vec::new).push(fill);
        }

        Ok(())
    }

    /// Get all fills for an order
    pub fn get_fills(&self, order_id: &OrderId) -> ExecResult<Vec<Fill>> {
        let fills = self.fills.read().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(fills.get(order_id).cloned().unwrap_or_default())
    }

    /// Get all orders
    pub fn get_all_orders(&self) -> ExecResult<Vec<Order>> {
        let orders = self.orders.read().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(orders.values().cloned().collect())
    }

    /// Get active orders (Working or PartiallyFilled)
    pub fn get_active_orders(&self) -> ExecResult<Vec<Order>> {
        let orders = self.orders.read().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(orders
            .values()
            .filter(|o| o.is_active())
            .cloned()
            .collect())
    }

    /// Get terminal orders (Filled, Cancelled, Rejected, Expired)
    pub fn get_terminal_orders(&self) -> ExecResult<Vec<Order>> {
        let orders = self.orders.read().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(orders
            .values()
            .filter(|o| o.is_terminal())
            .cloned()
            .collect())
    }

    /// Remove an order from tracking
    pub fn remove_order(&self, order_id: &OrderId) -> ExecResult<Order> {
        let mut orders = self.orders.write().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire write lock: {}", e))
        })?;

        orders
            .remove(order_id)
            .ok_or_else(|| ExecError::OrderNotFound(*order_id))
    }

    /// Clear all terminal orders
    pub fn clear_terminal_orders(&self) -> ExecResult<usize> {
        let mut orders = self.orders.write().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire write lock: {}", e))
        })?;

        let terminal_ids: Vec<OrderId> = orders
            .iter()
            .filter(|(_, o)| o.is_terminal())
            .map(|(id, _)| *id)
            .collect();

        let count = terminal_ids.len();
        for id in terminal_ids {
            orders.remove(&id);
        }

        Ok(count)
    }

    /// Get order count
    pub fn count(&self) -> ExecResult<usize> {
        let orders = self.orders.read().map_err(|e| {
            ExecError::InternalError(format!("Failed to acquire read lock: {}", e))
        })?;

        Ok(orders.len())
    }
}

impl Default for OrderTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{MarketId, OrderType, Side, TimeInForce, VenueId};
    use chrono::Utc;

    fn create_test_order() -> Order {
        Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        )
    }

    #[test]
    fn test_track_and_retrieve_order() {
        let tracker = OrderTracker::new();
        let order = create_test_order();
        let order_id = order.id;

        tracker.track_order(order.clone()).unwrap();

        let retrieved = tracker.get_order(&order_id).unwrap();
        assert_eq!(retrieved.id, order_id);
        assert_eq!(retrieved.size, 100.0);
    }

    #[test]
    fn test_update_order_status() {
        let tracker = OrderTracker::new();
        let order = create_test_order();
        let order_id = order.id;

        tracker.track_order(order).unwrap();
        tracker.update_status(&order_id, OrderStatus::Working).unwrap();

        let retrieved = tracker.get_order(&order_id).unwrap();
        assert_eq!(retrieved.status, OrderStatus::Working);
    }

    #[test]
    fn test_record_fill() {
        let tracker = OrderTracker::new();
        let order = create_test_order();
        let order_id = order.id;

        tracker.track_order(order).unwrap();

        let fill = Fill {
            fill_id: "fill-1".to_string(),
            order_id,
            venue_order_id: None,
            price: 0.51,
            size: 50.0,
            fee: 0.01,
            fee_currency: "USD".to_string(),
            timestamp: Utc::now(),
            liquidity: None,
        };

        tracker.record_fill(&order_id, fill).unwrap();

        let order = tracker.get_order(&order_id).unwrap();
        assert_eq!(order.filled_size, 50.0);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);

        let fills = tracker.get_fills(&order_id).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].size, 50.0);
    }

    #[test]
    fn test_get_active_orders() {
        let tracker = OrderTracker::new();

        let mut order1 = create_test_order();
        order1.update_status(OrderStatus::Working);
        tracker.track_order(order1).unwrap();

        let mut order2 = create_test_order();
        order2.update_status(OrderStatus::Filled);
        tracker.track_order(order2).unwrap();

        let active_orders = tracker.get_active_orders().unwrap();
        assert_eq!(active_orders.len(), 1);
        assert_eq!(active_orders[0].status, OrderStatus::Working);
    }

    #[test]
    fn test_clear_terminal_orders() {
        let tracker = OrderTracker::new();

        let mut order1 = create_test_order();
        order1.update_status(OrderStatus::Filled);
        tracker.track_order(order1).unwrap();

        let mut order2 = create_test_order();
        order2.update_status(OrderStatus::Working);
        tracker.track_order(order2).unwrap();

        assert_eq!(tracker.count().unwrap(), 2);

        let cleared = tracker.clear_terminal_orders().unwrap();
        assert_eq!(cleared, 1);
        assert_eq!(tracker.count().unwrap(), 1);
    }
}

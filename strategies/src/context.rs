//! Strategy execution context

use crate::{StrategyError, StrategyResult, StrategyParams};
use crate::types::{Order, OrderId, Position, MarketId};
use crate::metrics::StrategyMetric;
use ag_risk::{RiskEngine, RiskContext};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::Utc;

/// Mock execution engine for MVP
/// In production, this will interface with the actual exec/ module
pub struct MockExecutionEngine {
    orders: HashMap<OrderId, Order>,
    next_order_id: u64,
}

impl Default for MockExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MockExecutionEngine {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            next_order_id: 1,
        }
    }

    pub fn submit_order(&mut self, mut order: Order) -> StrategyResult<OrderId> {
        let order_id = format!("order_{}", self.next_order_id);
        self.next_order_id += 1;

        order.id = Some(order_id.clone());
        self.orders.insert(order_id.clone(), order);

        Ok(order_id)
    }

    pub fn cancel_order(&mut self, order_id: &OrderId) -> StrategyResult<()> {
        self.orders.remove(order_id)
            .ok_or_else(|| StrategyError::OrderNotFound(order_id.clone()))?;
        Ok(())
    }

    pub fn get_order(&self, order_id: &OrderId) -> Option<&Order> {
        self.orders.get(order_id)
    }

    pub fn get_all_orders(&self) -> Vec<&Order> {
        self.orders.values().collect()
    }
}

/// Strategy execution context
///
/// Provides strategies with access to execution, risk management, positions,
/// and configuration. This is the primary interface between strategies and
/// the rest of the system.
pub struct StrategyContext {
    /// Unique strategy identifier
    pub strategy_id: String,

    /// Mock execution engine (will be replaced with real exec module)
    exec_engine: Arc<Mutex<MockExecutionEngine>>,

    /// Risk engine
    risk_engine: Arc<Mutex<RiskEngine>>,

    /// Current positions by market
    pub positions: HashMap<MarketId, Position>,

    /// Active orders
    pub orders: HashMap<OrderId, Order>,

    /// Strategy parameters
    pub params: StrategyParams,

    /// Metrics buffer (to be sent to monitor)
    metrics_buffer: Vec<StrategyMetric>,
}

impl StrategyContext {
    /// Create a new strategy context
    pub fn new(
        strategy_id: String,
        risk_engine: Arc<Mutex<RiskEngine>>,
        params: StrategyParams,
    ) -> Self {
        Self {
            strategy_id,
            exec_engine: Arc::new(Mutex::new(MockExecutionEngine::new())),
            risk_engine,
            positions: HashMap::new(),
            orders: HashMap::new(),
            params,
            metrics_buffer: Vec::new(),
        }
    }

    /// Submit an order with risk checks
    ///
    /// This method performs pre-trade risk checks before submitting the order
    /// to the execution engine.
    pub async fn submit_order(&mut self, order: Order) -> StrategyResult<OrderId> {
        // Build risk context
        let position = self.get_position(&order.market)
            .map(|p| p.size)
            .unwrap_or(0.0);

        let proposed_size = match order.side {
            crate::types::Side::Buy => order.size,
            crate::types::Side::Sell => -order.size,
        };

        let inventory_value = self.calculate_total_inventory_value();

        let risk_ctx = RiskContext {
            market_id: order.market.clone(),
            current_position: position,
            proposed_size,
            inventory_value_usd: inventory_value,
        };

        // Evaluate risk
        let risk_decision = {
            let risk_engine = self.risk_engine.lock();
            risk_engine.evaluate(&risk_ctx)
        };

        if !risk_decision.allowed {
            return Err(StrategyError::RiskRejected {
                policies: risk_decision.violated_policies,
            });
        }

        // Submit order to execution engine
        let order_id = {
            let mut exec_engine = self.exec_engine.lock();
            exec_engine.submit_order(order.clone())?
        };

        // Track order
        self.orders.insert(order_id.clone(), order);

        Ok(order_id)
    }

    /// Cancel an order
    pub async fn cancel_order(&mut self, order_id: &OrderId) -> StrategyResult<()> {
        {
            let mut exec_engine = self.exec_engine.lock();
            exec_engine.cancel_order(order_id)?;
        }

        self.orders.remove(order_id);
        Ok(())
    }

    /// Get current position for a market
    pub fn get_position(&self, market_id: &str) -> Option<&Position> {
        self.positions.get(market_id)
    }

    /// Get mutable position for a market
    pub fn get_position_mut(&mut self, market_id: &str) -> Option<&mut Position> {
        self.positions.get_mut(market_id)
    }

    /// Update position (typically called after fills)
    pub fn update_position(&mut self, market_id: &str, size_delta: f64, price: f64) {
        let position = self.positions
            .entry(market_id.to_string())
            .or_insert_with(|| Position::new(market_id.to_string()));

        // Update position size
        let old_size = position.size;
        let new_size = old_size + size_delta;

        // Update entry price (volume-weighted)
        if new_size.abs() > 1e-8 {
            let old_value = old_size * position.entry_price;
            let new_value = size_delta * price;
            position.entry_price = (old_value + new_value) / new_size;
        } else {
            position.entry_price = 0.0;
        }

        position.size = new_size;
        position.mark_price = price;
        position.timestamp = Utc::now();

        // Calculate unrealized PnL
        if position.size.abs() > 1e-8 {
            position.unrealized_pnl = position.size * (price - position.entry_price);
        } else {
            position.unrealized_pnl = 0.0;
        }

        position.value_usd = position.size.abs() * price;
    }

    /// Get all open orders
    pub fn get_open_orders(&self) -> Vec<&Order> {
        self.orders.values().collect()
    }

    /// Get all open orders for a specific market
    pub fn get_open_orders_for_market(&self, market_id: &str) -> Vec<&Order> {
        self.orders
            .values()
            .filter(|o| o.market == market_id)
            .collect()
    }

    /// Emit a strategy metric
    pub async fn emit_metric(&mut self, metric: StrategyMetric) -> StrategyResult<()> {
        // In production, this would send to monitor module
        // For now, buffer the metrics
        self.metrics_buffer.push(metric);
        Ok(())
    }

    /// Get strategy parameter as typed value
    pub fn get_param<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.params.get_typed(key)
    }

    /// Get strategy parameter as string
    pub fn get_param_str(&self, key: &str) -> Option<&str> {
        self.params.get(key)
    }

    /// Calculate total inventory value across all positions
    pub fn calculate_total_inventory_value(&self) -> f64 {
        self.positions.values()
            .map(|p| p.value_usd)
            .sum()
    }

    /// Calculate total unrealized PnL across all positions
    pub fn calculate_total_unrealized_pnl(&self) -> f64 {
        self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum()
    }

    /// Calculate total realized PnL across all positions
    pub fn calculate_total_realized_pnl(&self) -> f64 {
        self.positions.values()
            .map(|p| p.realized_pnl)
            .sum()
    }

    /// Get buffered metrics (for testing)
    pub fn get_metrics_buffer(&self) -> &[StrategyMetric] {
        &self.metrics_buffer
    }

    /// Clear metrics buffer
    pub fn clear_metrics_buffer(&mut self) {
        self.metrics_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Side, OrderType, TimeInForce};
    use ag_risk::{RiskPolicyConfig, PolicyRule};

    fn create_test_context() -> StrategyContext {
        let yaml = r#"
policies:
  - type: PositionLimit
    market_id: "market1"
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 10000.0
"#;
        let risk_engine = RiskEngine::from_yaml(yaml).unwrap();
        let risk_engine = Arc::new(Mutex::new(risk_engine));

        StrategyContext::new(
            "test_strategy".to_string(),
            risk_engine,
            StrategyParams::new(),
        )
    }

    #[tokio::test]
    async fn test_submit_order_with_risk_check() {
        let mut ctx = create_test_context();

        let order = Order {
            venue: "polymarket".to_string(),
            market: "market1".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(100.0),
            size: 500.0,
            time_in_force: TimeInForce::GTC,
            ..Default::default()
        };

        let result = ctx.submit_order(order).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_position_update() {
        let mut ctx = create_test_context();

        // Buy 100 at $100
        ctx.update_position("market1", 100.0, 100.0);
        let pos = ctx.get_position("market1").unwrap();
        assert_eq!(pos.size, 100.0);
        assert_eq!(pos.entry_price, 100.0);

        // Buy 50 more at $102
        ctx.update_position("market1", 50.0, 102.0);
        let pos = ctx.get_position("market1").unwrap();
        assert_eq!(pos.size, 150.0);
        // Entry price should be weighted average
        let expected_entry = (100.0 * 100.0 + 50.0 * 102.0) / 150.0;
        assert!((pos.entry_price - expected_entry).abs() < 0.01);
    }

    #[test]
    fn test_inventory_calculations() {
        let mut ctx = create_test_context();

        ctx.update_position("market1", 100.0, 100.0);
        ctx.update_position("market2", 50.0, 200.0);

        let total_value = ctx.calculate_total_inventory_value();
        // 100 * 100 + 50 * 200 = 20000
        assert_eq!(total_value, 20000.0);
    }
}

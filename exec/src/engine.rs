//! Execution engine orchestrating orders across venues
//!
//! The ExecutionEngine is the main entry point for order execution.
//! It coordinates venue adapters, risk checks, rate limiting, and order tracking.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use ag_risk::{RiskContext, RiskEngine};

use crate::adapters::venue_adapter::VenueAdapter;
use crate::error::{ExecError, ExecResult};
use crate::oms::tracker::OrderTracker;
use crate::oms::validator::OrderValidator;
use crate::order::{CancelAck, Fill, Order, OrderAck, OrderId, OrderStatus, VenueId};
use crate::ratelimit::limiter::RateLimiter;

/// Execution engine configuration
#[derive(Debug, Clone)]
pub struct ExecutionEngineConfig {
    /// Enable pre-trade risk checks
    pub enable_risk_checks: bool,
    /// Enable order validation
    pub enable_validation: bool,
    /// Enable metrics emission
    pub enable_metrics: bool,
}

impl Default for ExecutionEngineConfig {
    fn default() -> Self {
        Self {
            enable_risk_checks: true,
            enable_validation: true,
            enable_metrics: true,
        }
    }
}

/// Execution engine orchestrating orders across venues
pub struct ExecutionEngine {
    /// Venue adapters indexed by venue ID
    adapters: HashMap<VenueId, Arc<Mutex<Box<dyn VenueAdapter>>>>,

    /// Risk engine for pre-trade checks
    risk_engine: Option<Arc<Mutex<RiskEngine>>>,

    /// Rate limiters per venue
    rate_limiters: HashMap<VenueId, RateLimiter>,

    /// Order tracker
    order_tracker: Arc<OrderTracker>,

    /// Order validator
    validator: OrderValidator,

    /// Configuration
    config: ExecutionEngineConfig,

    /// Current positions (market_id -> position size)
    positions: Arc<Mutex<HashMap<String, f64>>>,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(config: ExecutionEngineConfig) -> Self {
        Self {
            adapters: HashMap::new(),
            risk_engine: None,
            rate_limiters: HashMap::new(),
            order_tracker: Arc::new(OrderTracker::new()),
            validator: OrderValidator::new(),
            config,
            positions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a venue adapter
    pub fn register_adapter(
        &mut self,
        adapter: Box<dyn VenueAdapter>,
        rate_limiter: RateLimiter,
    ) {
        let venue_id = adapter.venue_id();
        info!("Registering venue adapter: {}", venue_id);

        self.adapters.insert(venue_id.clone(), Arc::new(Mutex::new(adapter)));
        self.rate_limiters.insert(venue_id, rate_limiter);
    }

    /// Set risk engine
    pub fn set_risk_engine(&mut self, risk_engine: RiskEngine) {
        info!("Setting risk engine");
        self.risk_engine = Some(Arc::new(Mutex::new(risk_engine)));
    }

    /// Submit an order with pre-trade risk checks
    pub async fn submit_order(&self, mut order: Order) -> ExecResult<OrderAck> {
        info!("Submitting order: {:?}", order.id);

        // Validate order
        if self.config.enable_validation {
            debug!("Validating order: {:?}", order.id);
            self.validator.validate(&order)?;
        }

        // Pre-trade risk check
        if self.config.enable_risk_checks {
            if let Some(risk_engine) = &self.risk_engine {
                debug!("Running pre-trade risk check for order: {:?}", order.id);

                let positions = self.positions.lock().await;
                let current_position = positions.get(order.market.as_str()).copied().unwrap_or(0.0);
                let inventory_value = positions.values().sum::<f64>();

                let proposed_size = match order.side {
                    crate::order::Side::Buy => order.size,
                    crate::order::Side::Sell => -order.size,
                };

                let risk_ctx = RiskContext {
                    market_id: order.market.as_str().to_string(),
                    current_position,
                    proposed_size,
                    inventory_value_usd: inventory_value,
                };

                let risk_engine = risk_engine.lock().await;
                let decision = risk_engine.evaluate(&risk_ctx);

                if !decision.allowed {
                    warn!(
                        "Risk check rejected order {:?}: {:?}",
                        order.id, decision.violated_policies
                    );
                    return Err(ExecError::RiskRejected {
                        policies: decision.violated_policies,
                    });
                }

                debug!("Risk check passed for order: {:?}", order.id);
            }
        }

        // Get venue adapter
        let adapter = self
            .adapters
            .get(&order.venue)
            .ok_or_else(|| ExecError::VenueNotSupported(order.venue.to_string()))?;

        // Check rate limit
        if let Some(rate_limiter) = self.rate_limiters.get(&order.venue) {
            debug!("Checking rate limit for venue: {}", order.venue);
            rate_limiter.check().await?;
        }

        // Update order status
        order.update_status(OrderStatus::Submitting);
        self.order_tracker.track_order(order.clone())?;

        // Place order via venue adapter
        let mut adapter = adapter.lock().await;
        let ack = adapter.place_order(&order).await?;

        // Update order status based on ack
        self.order_tracker.update_status(&order.id, ack.status)?;

        info!("Order submitted successfully: {:?}", order.id);
        Ok(ack)
    }

    /// Cancel an order
    pub async fn cancel_order(&self, order_id: OrderId) -> ExecResult<CancelAck> {
        info!("Cancelling order: {:?}", order_id);

        // Get order details
        let order = self.order_tracker.get_order(&order_id)?;

        // Check if order can be cancelled
        if order.is_terminal() {
            return Err(ExecError::InvalidOrderState {
                order_id,
                current_state: order.status.to_string(),
                operation: "cancel".to_string(),
            });
        }

        // Get venue adapter
        let adapter = self
            .adapters
            .get(&order.venue)
            .ok_or_else(|| ExecError::VenueNotSupported(order.venue.to_string()))?;

        // Check rate limit
        if let Some(rate_limiter) = self.rate_limiters.get(&order.venue) {
            rate_limiter.check().await?;
        }

        // Update status
        self.order_tracker.update_status(&order_id, OrderStatus::Cancelling)?;

        // Cancel via venue adapter
        let mut adapter = adapter.lock().await;
        let ack = adapter.cancel_order(&order_id).await?;

        // Update final status
        if ack.success {
            self.order_tracker.update_status(&order_id, OrderStatus::Cancelled)?;
            info!("Order cancelled successfully: {:?}", order_id);
        } else {
            error!("Order cancellation failed: {:?}", order_id);
        }

        Ok(ack)
    }

    /// Get order status
    pub async fn get_status(&self, order_id: OrderId) -> ExecResult<OrderStatus> {
        debug!("Getting status for order: {:?}", order_id);

        let order = self.order_tracker.get_order(&order_id)?;

        // If order is terminal, return cached status
        if order.is_terminal() {
            return Ok(order.status);
        }

        // Otherwise, query venue for latest status
        let adapter = self
            .adapters
            .get(&order.venue)
            .ok_or_else(|| ExecError::VenueNotSupported(order.venue.to_string()))?;

        let mut adapter = adapter.lock().await;
        let status = adapter.get_order_status(&order_id).await?;

        // Update cached status
        self.order_tracker.update_status(&order_id, status)?;

        Ok(status)
    }

    /// Record a fill
    pub async fn record_fill(&self, fill: Fill) -> ExecResult<()> {
        info!("Recording fill for order: {:?}", fill.order_id);

        // Record fill in tracker
        self.order_tracker.record_fill(&fill.order_id, fill.clone())?;

        // Update positions
        let order = self.order_tracker.get_order(&fill.order_id)?;
        let mut positions = self.positions.lock().await;

        let position_delta = match order.side {
            crate::order::Side::Buy => fill.size,
            crate::order::Side::Sell => -fill.size,
        };

        *positions.entry(order.market.as_str().to_string()).or_insert(0.0) += position_delta;

        debug!(
            "Updated position for {}: {}",
            order.market,
            positions.get(order.market.as_str()).unwrap()
        );

        Ok(())
    }

    /// Get current position for a market
    pub async fn get_position(&self, market_id: &str) -> f64 {
        let positions = self.positions.lock().await;
        positions.get(market_id).copied().unwrap_or(0.0)
    }

    /// Get all positions
    pub async fn get_all_positions(&self) -> HashMap<String, f64> {
        let positions = self.positions.lock().await;
        positions.clone()
    }

    /// Get all active orders
    pub fn get_active_orders(&self) -> ExecResult<Vec<Order>> {
        self.order_tracker.get_active_orders()
    }

    /// Get order by ID
    pub fn get_order(&self, order_id: &OrderId) -> ExecResult<Order> {
        self.order_tracker.get_order(order_id)
    }

    /// Get order tracker (for advanced usage)
    pub fn order_tracker(&self) -> &Arc<OrderTracker> {
        &self.order_tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{MarketId, OrderType, Side, TimeInForce, VenueId};

    #[tokio::test]
    async fn test_execution_engine_creation() {
        let config = ExecutionEngineConfig::default();
        let engine = ExecutionEngine::new(config);

        assert!(engine.adapters.is_empty());
        assert!(engine.risk_engine.is_none());
    }

    #[tokio::test]
    async fn test_position_tracking() {
        let config = ExecutionEngineConfig::default();
        let engine = ExecutionEngine::new(config);

        let market_id = "0x123abc";
        let initial_position = engine.get_position(market_id).await;
        assert_eq!(initial_position, 0.0);

        // Simulate a fill
        let order = Order::new(
            VenueId::new("polymarket"),
            MarketId::new(market_id),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        );

        engine.order_tracker.track_order(order.clone()).unwrap();

        let fill = Fill {
            fill_id: "fill-1".to_string(),
            order_id: order.id,
            venue_order_id: None,
            price: 0.51,
            size: 100.0,
            fee: 0.01,
            fee_currency: "USD".to_string(),
            timestamp: chrono::Utc::now(),
            liquidity: None,
        };

        engine.record_fill(fill).await.unwrap();

        let position = engine.get_position(market_id).await;
        assert_eq!(position, 100.0);
    }

    #[test]
    fn test_config_default() {
        let config = ExecutionEngineConfig::default();
        assert!(config.enable_risk_checks);
        assert!(config.enable_validation);
        assert!(config.enable_metrics);
    }
}

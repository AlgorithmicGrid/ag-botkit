//! Integration tests for ExecutionEngine

use ag_exec::{
    adapters::{VenueAdapter, VenueConfig},
    error::ExecResult,
    order::{CancelAck, MarketId, Order, OrderAck, OrderId, OrderStatus, OrderType, Side, TimeInForce, VenueId},
    ratelimit::RateLimiterConfig,
    ExecutionEngine, ExecutionEngineConfig,
};
use ag_risk::RiskEngine;
use async_trait::async_trait;
use chrono::Utc;

/// Mock venue adapter for testing
struct MockVenueAdapter {
    venue_id: VenueId,
}

impl MockVenueAdapter {
    fn new(venue_id: VenueId) -> Self {
        Self { venue_id }
    }
}

#[async_trait]
impl VenueAdapter for MockVenueAdapter {
    fn venue_id(&self) -> VenueId {
        self.venue_id.clone()
    }

    async fn place_order(&mut self, order: &Order) -> ExecResult<OrderAck> {
        Ok(OrderAck {
            order_id: order.id,
            venue_order_id: Some("venue-123".to_string()),
            status: OrderStatus::Working,
            timestamp: Utc::now(),
            message: None,
        })
    }

    async fn cancel_order(&mut self, order_id: &OrderId) -> ExecResult<CancelAck> {
        Ok(CancelAck {
            order_id: *order_id,
            venue_order_id: Some("venue-123".to_string()),
            success: true,
            timestamp: Utc::now(),
            message: None,
        })
    }

    async fn get_order_status(&mut self, _order_id: &OrderId) -> ExecResult<OrderStatus> {
        Ok(OrderStatus::Working)
    }

    async fn get_open_orders(&mut self) -> ExecResult<Vec<Order>> {
        Ok(Vec::new())
    }

    async fn modify_order(
        &mut self,
        order_id: &OrderId,
        _new_price: Option<f64>,
        _new_size: Option<f64>,
    ) -> ExecResult<OrderAck> {
        Ok(OrderAck {
            order_id: *order_id,
            venue_order_id: Some("venue-123".to_string()),
            status: OrderStatus::Working,
            timestamp: Utc::now(),
            message: None,
        })
    }

    async fn health_check(&mut self) -> ExecResult<bool> {
        Ok(true)
    }
}

#[tokio::test]
async fn test_submit_order_without_risk_checks() {
    let config = ExecutionEngineConfig {
        enable_risk_checks: false,
        enable_validation: true,
        enable_metrics: false,
    };

    let mut engine = ExecutionEngine::new(config);

    // Register mock adapter
    let venue_id = VenueId::new("mock_venue");
    let adapter = MockVenueAdapter::new(venue_id.clone());
    let rate_limiter = RateLimiterConfig::new(100, 200).build(venue_id.clone());
    engine.register_adapter(Box::new(adapter), rate_limiter);

    // Create order
    let order = Order::new(
        venue_id,
        MarketId::new("market-1"),
        Side::Buy,
        OrderType::Limit,
        Some(0.52),
        100.0,
        TimeInForce::GTC,
        "client-123".to_string(),
    );

    // Submit order
    let result = engine.submit_order(order).await;
    assert!(result.is_ok());

    let ack = result.unwrap();
    assert_eq!(ack.status, OrderStatus::Working);
    assert!(ack.venue_order_id.is_some());
}

#[tokio::test]
async fn test_submit_order_with_risk_checks() {
    let config = ExecutionEngineConfig::default();
    let mut engine = ExecutionEngine::new(config);

    // Set up risk engine
    let risk_yaml = r#"
policies:
  - type: PositionLimit
    market_id: "market-1"
    max_size: 200.0
  - type: InventoryLimit
    max_value_usd: 10000.0
"#;

    let risk_engine = RiskEngine::from_yaml(risk_yaml).unwrap();
    engine.set_risk_engine(risk_engine);

    // Register mock adapter
    let venue_id = VenueId::new("mock_venue");
    let adapter = MockVenueAdapter::new(venue_id.clone());
    let rate_limiter = RateLimiterConfig::new(100, 200).build(venue_id.clone());
    engine.register_adapter(Box::new(adapter), rate_limiter);

    // Create order (within limits)
    let order = Order::new(
        venue_id.clone(),
        MarketId::new("market-1"),
        Side::Buy,
        OrderType::Limit,
        Some(0.52),
        100.0,
        TimeInForce::GTC,
        "client-123".to_string(),
    );

    // Submit order
    let result = engine.submit_order(order).await;
    assert!(result.is_ok());

    // Create order that exceeds position limit
    let large_order = Order::new(
        venue_id,
        MarketId::new("market-1"),
        Side::Buy,
        OrderType::Limit,
        Some(0.52),
        300.0, // Exceeds max_size of 200
        TimeInForce::GTC,
        "client-456".to_string(),
    );

    // This should be rejected by risk check
    let result = engine.submit_order(large_order).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cancel_order() {
    let config = ExecutionEngineConfig {
        enable_risk_checks: false,
        enable_validation: true,
        enable_metrics: false,
    };

    let mut engine = ExecutionEngine::new(config);

    // Register mock adapter
    let venue_id = VenueId::new("mock_venue");
    let adapter = MockVenueAdapter::new(venue_id.clone());
    let rate_limiter = RateLimiterConfig::new(100, 200).build(venue_id.clone());
    engine.register_adapter(Box::new(adapter), rate_limiter);

    // Submit order
    let order = Order::new(
        venue_id,
        MarketId::new("market-1"),
        Side::Buy,
        OrderType::Limit,
        Some(0.52),
        100.0,
        TimeInForce::GTC,
        "client-123".to_string(),
    );

    let ack = engine.submit_order(order.clone()).await.unwrap();
    let order_id = ack.order_id;

    // Cancel order
    let cancel_result = engine.cancel_order(order_id).await;
    assert!(cancel_result.is_ok());

    let cancel_ack = cancel_result.unwrap();
    assert!(cancel_ack.success);
}

#[tokio::test]
async fn test_position_tracking() {
    let config = ExecutionEngineConfig {
        enable_risk_checks: false,
        enable_validation: true,
        enable_metrics: false,
    };

    let mut engine = ExecutionEngine::new(config);

    // Register mock adapter
    let venue_id = VenueId::new("mock_venue");
    let adapter = MockVenueAdapter::new(venue_id.clone());
    let rate_limiter = RateLimiterConfig::new(100, 200).build(venue_id.clone());
    engine.register_adapter(Box::new(adapter), rate_limiter);

    let market_id = "market-1";

    // Initial position should be 0
    let initial_position = engine.get_position(market_id).await;
    assert_eq!(initial_position, 0.0);

    // Submit buy order
    let order = Order::new(
        venue_id,
        MarketId::new(market_id),
        Side::Buy,
        OrderType::Limit,
        Some(0.52),
        100.0,
        TimeInForce::GTC,
        "client-123".to_string(),
    );

    let ack = engine.submit_order(order.clone()).await.unwrap();

    // Simulate fill
    use ag_exec::Fill;
    let fill = Fill {
        fill_id: "fill-1".to_string(),
        order_id: ack.order_id,
        venue_order_id: Some("venue-123".to_string()),
        price: 0.51,
        size: 100.0,
        fee: 0.01,
        fee_currency: "USD".to_string(),
        timestamp: Utc::now(),
        liquidity: None,
    };

    engine.record_fill(fill).await.unwrap();

    // Position should now be 100
    let position = engine.get_position(market_id).await;
    assert_eq!(position, 100.0);
}

#[tokio::test]
async fn test_validation_error() {
    let config = ExecutionEngineConfig::default();
    let mut engine = ExecutionEngine::new(config);

    // Register mock adapter
    let venue_id = VenueId::new("mock_venue");
    let adapter = MockVenueAdapter::new(venue_id.clone());
    let rate_limiter = RateLimiterConfig::new(100, 200).build(venue_id.clone());
    engine.register_adapter(Box::new(adapter), rate_limiter);

    // Create invalid order (negative size)
    let mut order = Order::new(
        venue_id,
        MarketId::new("market-1"),
        Side::Buy,
        OrderType::Limit,
        Some(0.52),
        -100.0, // Invalid negative size
        TimeInForce::GTC,
        "client-123".to_string(),
    );

    // This should fail validation
    let result = engine.submit_order(order).await;
    assert!(result.is_err());
}

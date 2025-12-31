//! Example: Placing orders on Polymarket CLOB
//!
//! This example demonstrates how to:
//! 1. Set up the execution engine
//! 2. Configure Polymarket venue adapter
//! 3. Set up risk engine with policies
//! 4. Place limit orders
//! 5. Cancel orders
//! 6. Track order status

use ag_exec::{
    adapters::VenueConfig,
    order::{MarketId, Order, OrderType, Side, TimeInForce, VenueId},
    ratelimit::RateLimiterConfig,
    venues::PolymarketAdapter,
    ExecutionEngine, ExecutionEngineConfig,
};
use ag_risk::RiskEngine;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    ag_exec::init_tracing();

    println!("=== Polymarket Order Placement Example ===\n");

    // Get API credentials from environment
    let api_key = env::var("POLYMARKET_API_KEY")
        .unwrap_or_else(|_| "your_api_key".to_string());
    let api_secret = env::var("POLYMARKET_API_SECRET")
        .unwrap_or_else(|_| "your_api_secret".to_string());

    // 1. Create execution engine configuration
    println!("1. Creating execution engine...");
    let config = ExecutionEngineConfig {
        enable_risk_checks: true,
        enable_validation: true,
        enable_metrics: true,
    };
    let mut engine = ExecutionEngine::new(config);
    println!("   ✓ Execution engine created\n");

    // 2. Set up risk engine with policies
    println!("2. Setting up risk engine...");
    let risk_yaml = r#"
policies:
  - type: PositionLimit
    market_id: null  # Global position limit
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 10000.0
  - type: KillSwitch
    enabled: false
"#;

    let risk_engine = RiskEngine::from_yaml(risk_yaml)?;
    engine.set_risk_engine(risk_engine);
    println!("   ✓ Risk engine configured with policies\n");

    // 3. Configure Polymarket venue adapter
    println!("3. Configuring Polymarket CLOB adapter...");
    let venue_config = VenueConfig::new(
        VenueId::new("polymarket"),
        "https://clob.polymarket.com".to_string(),
    )
    .with_credentials(api_key, api_secret)
    .with_ws_endpoint("wss://ws-subscriptions.polymarket.com".to_string());

    let adapter = PolymarketAdapter::new(venue_config)?;
    println!("   ✓ Polymarket adapter created\n");

    // 4. Configure rate limiter
    println!("4. Setting up rate limiter...");
    let rate_limiter = RateLimiterConfig::polymarket_default()
        .build(VenueId::new("polymarket"));
    println!("   ✓ Rate limiter configured (10 req/s, burst 20)\n");

    // 5. Register adapter with engine
    println!("5. Registering venue adapter...");
    engine.register_adapter(Box::new(adapter), rate_limiter);
    println!("   ✓ Venue adapter registered\n");

    // 6. Create a limit order
    println!("6. Creating limit order...");
    let order = Order::new(
        VenueId::new("polymarket"),
        MarketId::new("0x1234567890abcdef"), // Example market ID
        Side::Buy,
        OrderType::Limit,
        Some(0.52), // Limit price
        100.0,      // Size
        TimeInForce::GTC,
        "example-order-1".to_string(),
    );

    println!("   Order details:");
    println!("   - Market: {}", order.market);
    println!("   - Side: {}", order.side);
    println!("   - Type: {}", order.order_type);
    println!("   - Price: {}", order.price.unwrap());
    println!("   - Size: {}", order.size);
    println!("   - Client Order ID: {}\n", order.client_order_id);

    // 7. Submit the order
    println!("7. Submitting order...");
    match engine.submit_order(order.clone()).await {
        Ok(ack) => {
            println!("   ✓ Order submitted successfully!");
            println!("   - Order ID: {}", ack.order_id);
            println!("   - Venue Order ID: {:?}", ack.venue_order_id);
            println!("   - Status: {}", ack.status);
            println!("   - Timestamp: {}\n", ack.timestamp);

            // 8. Check order status
            println!("8. Checking order status...");
            match engine.get_status(ack.order_id).await {
                Ok(status) => {
                    println!("   ✓ Order status: {}\n", status);
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to get status: {}\n", e);
                }
            }

            // 9. Check current position
            println!("9. Checking current position...");
            let position = engine.get_position(order.market.as_str()).await;
            println!("   Current position for market {}: {}\n", order.market, position);

            // 10. Cancel the order (if you want to)
            println!("10. Cancelling order (optional)...");
            match engine.cancel_order(ack.order_id).await {
                Ok(cancel_ack) => {
                    if cancel_ack.success {
                        println!("   ✓ Order cancelled successfully!");
                        println!("   - Order ID: {}", cancel_ack.order_id);
                        println!("   - Timestamp: {}\n", cancel_ack.timestamp);
                    } else {
                        println!("   ✗ Cancellation failed: {:?}\n", cancel_ack.message);
                    }
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to cancel order: {}\n", e);
                }
            }
        }
        Err(e) => {
            eprintln!("   ✗ Order submission failed: {}\n", e);

            // Check if it was a risk rejection
            if let ag_exec::ExecError::RiskRejected { policies } = e {
                eprintln!("   Violated risk policies:");
                for policy in policies {
                    eprintln!("   - {}", policy);
                }
            }
        }
    }

    // 11. Get all active orders
    println!("11. Getting all active orders...");
    match engine.get_active_orders() {
        Ok(orders) => {
            println!("   Active orders: {}", orders.len());
            for order in orders {
                println!("   - {} | {} | {} @ {}",
                    order.id,
                    order.market,
                    order.side,
                    order.price.unwrap_or(0.0)
                );
            }
        }
        Err(e) => {
            eprintln!("   ✗ Failed to get active orders: {}", e);
        }
    }

    println!("\n=== Example completed ===");

    Ok(())
}

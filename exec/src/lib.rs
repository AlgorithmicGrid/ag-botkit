//! # ag-exec: Execution Gateway for Order Placement
//!
//! This library provides order execution infrastructure for trading systems,
//! with support for multiple venues including Polymarket CLOB, CEX, and DEX.
//!
//! ## Core Components
//!
//! - **ExecutionEngine**: Main orchestrator for order execution across venues
//! - **VenueAdapter**: Trait for venue-specific API implementations
//! - **Order Management System (OMS)**: Order lifecycle tracking and validation
//! - **Rate Limiting**: Per-venue API rate limit enforcement
//! - **Risk Integration**: Pre-trade risk checks via ag-risk module
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use ag_exec::{ExecutionEngine, ExecutionEngineConfig};
//! use ag_exec::venues::PolymarketAdapter;
//! use ag_exec::adapters::VenueConfig;
//! use ag_exec::ratelimit::RateLimiterConfig;
//! use ag_exec::order::{Order, VenueId, MarketId, Side, OrderType, TimeInForce};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create execution engine
//!     let config = ExecutionEngineConfig::default();
//!     let mut engine = ExecutionEngine::new(config);
//!
//!     // Configure Polymarket adapter
//!     let venue_config = VenueConfig::new(
//!         VenueId::new("polymarket"),
//!         "https://clob.polymarket.com".to_string(),
//!     ).with_credentials("api_key".to_string(), "api_secret".to_string());
//!
//!     let adapter = PolymarketAdapter::new(venue_config).unwrap();
//!     let rate_limiter = RateLimiterConfig::polymarket_default()
//!         .build(VenueId::new("polymarket"));
//!
//!     // Register adapter
//!     engine.register_adapter(Box::new(adapter), rate_limiter);
//!
//!     // Create and submit order
//!     let order = Order::new(
//!         VenueId::new("polymarket"),
//!         MarketId::new("0x123abc"),
//!         Side::Buy,
//!         OrderType::Limit,
//!         Some(0.52),
//!         100.0,
//!         TimeInForce::GTC,
//!         "my-order-1".to_string(),
//!     );
//!
//!     match engine.submit_order(order).await {
//!         Ok(ack) => println!("Order placed: {:?}", ack),
//!         Err(e) => eprintln!("Order failed: {}", e),
//!     }
//! }
//! ```

// Public modules
pub mod error;
pub mod order;

// Re-export main types
pub use error::{ExecError, ExecResult};
pub use order::{
    CancelAck, Fill, Liquidity, MarketId, Order, OrderAck, OrderId, OrderStatus, OrderType, Side,
    TimeInForce, VenueId,
};

// Internal modules
mod engine;

// OMS modules
pub mod oms {
    pub mod tracker;
    pub mod validator;

    pub use tracker::OrderTracker;
    pub use validator::OrderValidator;
}

// Adapter modules
pub mod adapters {
    pub mod venue_adapter;

    pub use venue_adapter::{VenueAdapter, VenueConfig};
}

// Rate limiting
pub mod ratelimit {
    pub mod limiter;

    pub use limiter::{RateLimiter, RateLimiterConfig};
}

// Venue implementations
pub mod venues {
    pub mod polymarket;

    pub use polymarket::PolymarketAdapter;
}

// Re-export engine
pub use engine::{ExecutionEngine, ExecutionEngineConfig};

// Initialize tracing
pub fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all main types are exported
        let _: OrderId;
        let _: VenueId;
        let _: MarketId;
        let _: Side;
        let _: OrderType;
        let _: TimeInForce;
        let _: OrderStatus;
    }
}

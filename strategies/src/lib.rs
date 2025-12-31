//! # ag-strategies: Multi-Market Trading Strategy Framework
//!
//! This library provides a comprehensive framework for building, testing, and deploying
//! trading strategies across multiple markets and venues.
//!
//! ## Core Components
//!
//! - **Strategy Trait**: Base trait all strategies must implement
//! - **StrategyContext**: Execution context with access to exec/risk engines
//! - **MultiMarketCoordinator**: Orchestrates multiple strategies across markets
//! - **Signal Framework**: Technical indicators and signal generation
//! - **Backtesting Engine**: Event-driven backtesting with realistic fill simulation
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use ag_strategies::{Strategy, StrategyContext, StrategyMetadata};
//! use async_trait::async_trait;
//!
//! struct MyStrategy;
//!
//! #[async_trait]
//! impl Strategy for MyStrategy {
//!     async fn initialize(&mut self, ctx: &mut StrategyContext) -> Result<(), ag_strategies::StrategyError> {
//!         // Initialize strategy
//!         Ok(())
//!     }
//!
//!     async fn on_market_tick(
//!         &mut self,
//!         market_id: &str,
//!         tick: &ag_strategies::MarketTick,
//!         ctx: &mut StrategyContext,
//!     ) -> Result<(), ag_strategies::StrategyError> {
//!         // Process market update
//!         Ok(())
//!     }
//!
//!     async fn on_fill(
//!         &mut self,
//!         fill: &ag_strategies::Fill,
//!         ctx: &mut StrategyContext,
//!     ) -> Result<(), ag_strategies::StrategyError> {
//!         // Handle fill
//!         Ok(())
//!     }
//!
//!     async fn on_cancel(
//!         &mut self,
//!         order_id: &ag_strategies::OrderId,
//!         ctx: &mut StrategyContext,
//!     ) -> Result<(), ag_strategies::StrategyError> {
//!         // Handle cancellation
//!         Ok(())
//!     }
//!
//!     async fn on_timer(
//!         &mut self,
//!         ctx: &mut StrategyContext,
//!     ) -> Result<(), ag_strategies::StrategyError> {
//!         // Periodic housekeeping
//!         Ok(())
//!     }
//!
//!     async fn shutdown(&mut self, ctx: &mut StrategyContext) -> Result<(), ag_strategies::StrategyError> {
//!         // Cleanup
//!         Ok(())
//!     }
//!
//!     fn metadata(&self) -> StrategyMetadata {
//!         StrategyMetadata {
//!             name: "MyStrategy".to_string(),
//!             version: "1.0.0".to_string(),
//!             description: "My custom strategy".to_string(),
//!             markets: vec![],
//!             required_params: vec![],
//!         }
//!     }
//! }
//! ```

pub mod error;
pub mod types;
pub mod context;
pub mod coordinator;
pub mod metrics;

// Re-export main types
pub use error::{StrategyError, StrategyResult};
pub use types::{
    Strategy, StrategyMetadata, StrategyParams,
    Order, OrderId, OrderType, OrderStatus, Side, TimeInForce,
    Fill, Trade, Position,
    MarketTick, MarketData,
    Signal, SignalType, SignalMetadata, SignalGenerator,
};
pub use context::StrategyContext;
pub use coordinator::MultiMarketCoordinator;
pub use metrics::{StrategyMetric, MetricType};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Base strategy trait that all strategies must implement
///
/// This trait defines the lifecycle hooks and event handlers that strategies
/// use to respond to market data and execution events.
#[async_trait]
pub trait Strategy: Send + Sync {
    /// Initialize the strategy
    ///
    /// Called once when the strategy is first loaded. Use this to set up
    /// initial state, subscribe to markets, and validate parameters.
    async fn initialize(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()>;

    /// Process market data update
    ///
    /// Called whenever market data is received for a subscribed market.
    /// This is the primary signal generation and trading logic entry point.
    async fn on_market_tick(
        &mut self,
        market_id: &str,
        tick: &MarketTick,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()>;

    /// Handle order fill notification
    ///
    /// Called when an order is partially or fully filled.
    async fn on_fill(
        &mut self,
        fill: &Fill,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()>;

    /// Handle order cancellation notification
    ///
    /// Called when an order is cancelled.
    async fn on_cancel(
        &mut self,
        order_id: &OrderId,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()>;

    /// Periodic timer callback
    ///
    /// Called at regular intervals for housekeeping tasks like
    /// risk checks, position rebalancing, or metric emission.
    async fn on_timer(
        &mut self,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()>;

    /// Shutdown the strategy
    ///
    /// Called when the strategy is being stopped. Use this to clean up
    /// resources, cancel open orders, and persist state.
    async fn shutdown(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()>;

    /// Get strategy metadata
    ///
    /// Returns information about the strategy including name, version,
    /// and required parameters.
    fn metadata(&self) -> StrategyMetadata;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyStrategy;

    #[async_trait]
    impl Strategy for DummyStrategy {
        async fn initialize(&mut self, _ctx: &mut StrategyContext) -> StrategyResult<()> {
            Ok(())
        }

        async fn on_market_tick(
            &mut self,
            _market_id: &str,
            _tick: &MarketTick,
            _ctx: &mut StrategyContext,
        ) -> StrategyResult<()> {
            Ok(())
        }

        async fn on_fill(
            &mut self,
            _fill: &Fill,
            _ctx: &mut StrategyContext,
        ) -> StrategyResult<()> {
            Ok(())
        }

        async fn on_cancel(
            &mut self,
            _order_id: &OrderId,
            _ctx: &mut StrategyContext,
        ) -> StrategyResult<()> {
            Ok(())
        }

        async fn on_timer(&mut self, _ctx: &mut StrategyContext) -> StrategyResult<()> {
            Ok(())
        }

        async fn shutdown(&mut self, _ctx: &mut StrategyContext) -> StrategyResult<()> {
            Ok(())
        }

        fn metadata(&self) -> StrategyMetadata {
            StrategyMetadata {
                name: "DummyStrategy".to_string(),
                version: "0.1.0".to_string(),
                description: "Test strategy".to_string(),
                markets: vec![],
                required_params: vec![],
            }
        }
    }

    #[tokio::test]
    async fn test_dummy_strategy() {
        let mut strategy = DummyStrategy;
        let metadata = strategy.metadata();
        assert_eq!(metadata.name, "DummyStrategy");
        assert_eq!(metadata.version, "0.1.0");
    }
}

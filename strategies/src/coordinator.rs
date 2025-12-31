//! Multi-market strategy coordinator

use crate::{Strategy, StrategyError, StrategyResult, StrategyContext};
use crate::types::{MarketTick, Fill, OrderId, Position};
use std::collections::HashMap;

/// Multi-market coordinator
///
/// Orchestrates multiple strategies across different markets, routing market data
/// and execution events to the appropriate strategies.
pub struct MultiMarketCoordinator {
    /// Registered strategies by ID
    strategies: HashMap<String, Box<dyn Strategy>>,

    /// Strategy contexts by ID
    contexts: HashMap<String, StrategyContext>,

    /// Market subscriptions: market_id -> strategy_ids
    market_subscriptions: HashMap<String, Vec<String>>,

    /// Strategy subscriptions: strategy_id -> market_ids
    strategy_markets: HashMap<String, Vec<String>>,
}

impl MultiMarketCoordinator {
    /// Create a new coordinator
    pub fn new() -> Self {
        Self {
            strategies: HashMap::new(),
            contexts: HashMap::new(),
            market_subscriptions: HashMap::new(),
            strategy_markets: HashMap::new(),
        }
    }

    /// Register a strategy with markets
    ///
    /// # Arguments
    /// * `strategy_id` - Unique identifier for the strategy
    /// * `strategy` - Strategy implementation
    /// * `context` - Strategy execution context
    /// * `markets` - List of markets to subscribe to
    pub async fn register_strategy(
        &mut self,
        strategy_id: String,
        mut strategy: Box<dyn Strategy>,
        mut context: StrategyContext,
        markets: Vec<String>,
    ) -> StrategyResult<()> {
        // Initialize the strategy
        strategy.initialize(&mut context).await?;

        // Register market subscriptions
        for market in &markets {
            self.market_subscriptions
                .entry(market.clone())
                .or_default()
                .push(strategy_id.clone());
        }

        // Store strategy markets
        self.strategy_markets.insert(strategy_id.clone(), markets);

        // Store strategy and context
        self.strategies.insert(strategy_id.clone(), strategy);
        self.contexts.insert(strategy_id.clone(), context);

        Ok(())
    }

    /// Unregister a strategy
    pub async fn unregister_strategy(&mut self, strategy_id: &str) -> StrategyResult<()> {
        // Get strategy's markets
        let markets = self.strategy_markets.remove(strategy_id)
            .ok_or_else(|| StrategyError::Other(format!("Strategy not found: {}", strategy_id)))?;

        // Remove from market subscriptions
        for market in markets {
            if let Some(subs) = self.market_subscriptions.get_mut(&market) {
                subs.retain(|id| id != strategy_id);
                if subs.is_empty() {
                    self.market_subscriptions.remove(&market);
                }
            }
        }

        // Shutdown the strategy
        if let (Some(mut strategy), Some(mut context)) = (
            self.strategies.remove(strategy_id),
            self.contexts.remove(strategy_id),
        ) {
            strategy.shutdown(&mut context).await?;
        }

        Ok(())
    }

    /// Route market tick to relevant strategies
    pub async fn route_market_tick(
        &mut self,
        market_id: &str,
        tick: &MarketTick,
    ) -> StrategyResult<()> {
        // Get strategies subscribed to this market
        let strategy_ids = match self.market_subscriptions.get(market_id) {
            Some(ids) => ids.clone(),
            None => return Ok(()), // No subscribers
        };

        // Route to each strategy
        for strategy_id in strategy_ids {
            if let (Some(strategy), Some(context)) = (
                self.strategies.get_mut(&strategy_id),
                self.contexts.get_mut(&strategy_id),
            ) {
                strategy.on_market_tick(market_id, tick, context).await?;
            }
        }

        Ok(())
    }

    /// Route fill to a specific strategy
    pub async fn route_fill(
        &mut self,
        strategy_id: &str,
        fill: &Fill,
    ) -> StrategyResult<()> {
        let strategy = self.strategies.get_mut(strategy_id)
            .ok_or_else(|| StrategyError::Other(format!("Strategy not found: {}", strategy_id)))?;

        let context = self.contexts.get_mut(strategy_id)
            .ok_or_else(|| StrategyError::Other(format!("Context not found: {}", strategy_id)))?;

        strategy.on_fill(fill, context).await
    }

    /// Route cancellation to a specific strategy
    pub async fn route_cancel(
        &mut self,
        strategy_id: &str,
        order_id: &OrderId,
    ) -> StrategyResult<()> {
        let strategy = self.strategies.get_mut(strategy_id)
            .ok_or_else(|| StrategyError::Other(format!("Strategy not found: {}", strategy_id)))?;

        let context = self.contexts.get_mut(strategy_id)
            .ok_or_else(|| StrategyError::Other(format!("Context not found: {}", strategy_id)))?;

        strategy.on_cancel(order_id, context).await
    }

    /// Call timer callback for all strategies
    pub async fn on_timer_all(&mut self) -> StrategyResult<()> {
        let strategy_ids: Vec<String> = self.strategies.keys().cloned().collect();

        for strategy_id in strategy_ids {
            if let (Some(strategy), Some(context)) = (
                self.strategies.get_mut(&strategy_id),
                self.contexts.get_mut(&strategy_id),
            ) {
                strategy.on_timer(context).await?;
            }
        }

        Ok(())
    }

    /// Get cross-market positions
    ///
    /// Returns all positions grouped by strategy
    pub fn get_cross_market_positions(&self) -> HashMap<String, Vec<Position>> {
        let mut positions = HashMap::new();

        for (strategy_id, context) in &self.contexts {
            let strategy_positions: Vec<Position> = context.positions
                .values()
                .cloned()
                .collect();
            positions.insert(strategy_id.clone(), strategy_positions);
        }

        positions
    }

    /// Calculate total exposure across all strategies
    pub fn calculate_total_exposure(&self) -> CrossMarketExposure {
        let mut total_value = 0.0;
        let mut total_unrealized_pnl = 0.0;
        let mut total_realized_pnl = 0.0;
        let mut positions_by_market: HashMap<String, f64> = HashMap::new();

        for context in self.contexts.values() {
            total_value += context.calculate_total_inventory_value();
            total_unrealized_pnl += context.calculate_total_unrealized_pnl();
            total_realized_pnl += context.calculate_total_realized_pnl();

            for (market_id, position) in &context.positions {
                *positions_by_market.entry(market_id.clone()).or_insert(0.0) += position.size;
            }
        }

        CrossMarketExposure {
            total_value,
            total_unrealized_pnl,
            total_realized_pnl,
            positions_by_market,
        }
    }

    /// Get number of registered strategies
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    /// Get list of strategy IDs
    pub fn strategy_ids(&self) -> Vec<String> {
        self.strategies.keys().cloned().collect()
    }

    /// Get strategy context (immutable)
    pub fn get_context(&self, strategy_id: &str) -> Option<&StrategyContext> {
        self.contexts.get(strategy_id)
    }
}

impl Default for MultiMarketCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Cross-market exposure summary
#[derive(Debug, Clone)]
pub struct CrossMarketExposure {
    /// Total inventory value in USD
    pub total_value: f64,

    /// Total unrealized PnL
    pub total_unrealized_pnl: f64,

    /// Total realized PnL
    pub total_realized_pnl: f64,

    /// Net positions by market
    pub positions_by_market: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StrategyMetadata, StrategyParams};
    use crate::types::{MarketTick, Side};
    use async_trait::async_trait;
    use chrono::Utc;
    use ag_risk::RiskEngine;
    use std::sync::Arc;
    use parking_lot::Mutex;

    struct TestStrategy {
        ticks_received: usize,
    }

    #[async_trait]
    impl Strategy for TestStrategy {
        async fn initialize(&mut self, _ctx: &mut StrategyContext) -> StrategyResult<()> {
            Ok(())
        }

        async fn on_market_tick(
            &mut self,
            _market_id: &str,
            _tick: &MarketTick,
            _ctx: &mut StrategyContext,
        ) -> StrategyResult<()> {
            self.ticks_received += 1;
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
                name: "TestStrategy".to_string(),
                version: "1.0.0".to_string(),
                description: "Test".to_string(),
                markets: vec![],
                required_params: vec![],
            }
        }
    }

    fn create_test_context(strategy_id: &str) -> StrategyContext {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#;
        let risk_engine = RiskEngine::from_yaml(yaml).unwrap();
        StrategyContext::new(
            strategy_id.to_string(),
            Arc::new(Mutex::new(risk_engine)),
            StrategyParams::new(),
        )
    }

    #[tokio::test]
    async fn test_register_strategy() {
        let mut coordinator = MultiMarketCoordinator::new();
        let strategy = Box::new(TestStrategy { ticks_received: 0 });
        let context = create_test_context("test1");

        let result = coordinator.register_strategy(
            "test1".to_string(),
            strategy,
            context,
            vec!["market1".to_string(), "market2".to_string()],
        ).await;

        assert!(result.is_ok());
        assert_eq!(coordinator.strategy_count(), 1);
    }

    #[tokio::test]
    async fn test_route_market_tick() {
        let mut coordinator = MultiMarketCoordinator::new();
        let strategy = Box::new(TestStrategy { ticks_received: 0 });
        let context = create_test_context("test1");

        coordinator.register_strategy(
            "test1".to_string(),
            strategy,
            context,
            vec!["market1".to_string()],
        ).await.unwrap();

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

        coordinator.route_market_tick("market1", &tick).await.unwrap();

        // Verify tick was routed (we can't easily check ticks_received due to trait object)
        // but we can verify no error occurred
    }

    #[tokio::test]
    async fn test_unregister_strategy() {
        let mut coordinator = MultiMarketCoordinator::new();
        let strategy = Box::new(TestStrategy { ticks_received: 0 });
        let context = create_test_context("test1");

        coordinator.register_strategy(
            "test1".to_string(),
            strategy,
            context,
            vec!["market1".to_string()],
        ).await.unwrap();

        assert_eq!(coordinator.strategy_count(), 1);

        coordinator.unregister_strategy("test1").await.unwrap();

        assert_eq!(coordinator.strategy_count(), 0);
    }
}

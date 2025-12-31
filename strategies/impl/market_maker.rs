//! Market making strategy with inventory skewing

use crate::{Strategy, StrategyContext, StrategyError, StrategyResult, StrategyMetadata};
use crate::types::{MarketTick, Fill, OrderId, Order, Side, OrderType, TimeInForce};
use crate::metrics::MetricBuilder;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Market maker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakerConfig {
    /// Target spread in basis points
    pub target_spread_bps: f64,

    /// Quote size per order
    pub quote_size: f64,

    /// Maximum position size
    pub max_position: f64,

    /// Target inventory (default 0.0 for neutral)
    pub inventory_target: f64,

    /// Inventory skew factor (how much to adjust quotes based on inventory)
    pub skew_factor: f64,

    /// Minimum quote interval in milliseconds
    pub min_quote_interval_ms: u64,
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self {
            target_spread_bps: 20.0,
            quote_size: 100.0,
            max_position: 1000.0,
            inventory_target: 0.0,
            skew_factor: 0.5,
            min_quote_interval_ms: 100,
        }
    }
}

/// Simple market making strategy with inventory skewing
///
/// This strategy continuously quotes bid and ask prices around the mid price,
/// adjusting the quotes based on current inventory to encourage mean reversion.
pub struct MarketMakerStrategy {
    config: MarketMakerConfig,
    market_id: String,
    last_quote_time: Option<i64>,
    metric_builder: Option<MetricBuilder>,
}

impl MarketMakerStrategy {
    pub fn new(market_id: String, config: MarketMakerConfig) -> Self {
        Self {
            config,
            market_id,
            last_quote_time: None,
            metric_builder: None,
        }
    }

    /// Calculate inventory skew
    fn calculate_inventory_skew(&self, position: f64) -> f64 {
        if self.config.max_position < 1e-8 {
            return 0.0;
        }
        (position - self.config.inventory_target) / self.config.max_position
    }

    /// Calculate bid and ask prices based on mid and inventory
    fn calculate_quotes(&self, mid: f64, position: f64) -> (f64, f64) {
        let base_spread = mid * self.config.target_spread_bps / 10000.0;
        let inventory_skew = self.calculate_inventory_skew(position);

        // Adjust spread and skew based on inventory
        // When long, widen ask and narrow bid to encourage selling
        // When short, widen bid and narrow ask to encourage buying
        let spread_adjustment = 1.0 + inventory_skew.abs() * self.config.skew_factor;
        let adjusted_spread = base_spread * spread_adjustment;

        let skew_shift = inventory_skew * adjusted_spread * 0.5;

        let bid_price = mid - adjusted_spread / 2.0 - skew_shift;
        let ask_price = mid + adjusted_spread / 2.0 - skew_shift;

        (bid_price, ask_price)
    }

    /// Check if we should requote (based on time interval)
    fn should_requote(&self) -> bool {
        match self.last_quote_time {
            None => true,
            Some(last_time) => {
                let now = Utc::now().timestamp_millis();
                (now - last_time) >= self.config.min_quote_interval_ms as i64
            }
        }
    }
}

#[async_trait]
impl Strategy for MarketMakerStrategy {
    async fn initialize(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()> {
        self.metric_builder = Some(MetricBuilder::new(ctx.strategy_id.clone()));

        tracing::info!(
            strategy_id = %ctx.strategy_id,
            market_id = %self.market_id,
            "Market maker strategy initialized"
        );

        Ok(())
    }

    async fn on_market_tick(
        &mut self,
        market_id: &str,
        tick: &MarketTick,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Only process our market
        if market_id != self.market_id {
            return Ok(());
        }

        // Check if we should requote
        if !self.should_requote() {
            return Ok(());
        }

        // Get current position
        let position = ctx.get_position(market_id)
            .map(|p| p.size)
            .unwrap_or(0.0);

        // Check position limits
        if position.abs() >= self.config.max_position {
            tracing::warn!(
                market_id = %market_id,
                position = %position,
                max_position = %self.config.max_position,
                "Position limit reached, not quoting"
            );
            return Ok(());
        }

        // Calculate mid price
        let mid = tick.mid_price();
        if mid < 1e-8 {
            return Ok(()); // Invalid price
        }

        // Calculate bid and ask prices
        let (bid_price, ask_price) = self.calculate_quotes(mid, position);

        // Cancel existing orders
        let open_orders: Vec<OrderId> = ctx.get_open_orders_for_market(market_id)
            .iter()
            .filter_map(|o| o.id.clone())
            .collect();

        for order_id in open_orders {
            ctx.cancel_order(&order_id).await?;
        }

        // Submit new quotes if within position limits
        let can_buy = position + self.config.quote_size <= self.config.max_position;
        let can_sell = position - self.config.quote_size >= -self.config.max_position;

        if can_buy {
            let bid_order = Order {
                venue: "polymarket".to_string(),
                market: market_id.to_string(),
                side: Side::Buy,
                order_type: OrderType::Limit,
                price: Some(bid_price),
                size: self.config.quote_size,
                time_in_force: TimeInForce::GTC,
                ..Default::default()
            };

            match ctx.submit_order(bid_order).await {
                Ok(order_id) => {
                    if let Some(ref builder) = self.metric_builder {
                        let metric = builder.order_placed(market_id, "buy");
                        ctx.emit_metric(metric).await?;
                    }
                }
                Err(StrategyError::RiskRejected { policies }) => {
                    tracing::warn!(
                        market_id = %market_id,
                        policies = ?policies,
                        "Bid order rejected by risk engine"
                    );
                }
                Err(e) => return Err(e),
            }
        }

        if can_sell {
            let ask_order = Order {
                venue: "polymarket".to_string(),
                market: market_id.to_string(),
                side: Side::Sell,
                order_type: OrderType::Limit,
                price: Some(ask_price),
                size: self.config.quote_size,
                time_in_force: TimeInForce::GTC,
                ..Default::default()
            };

            match ctx.submit_order(ask_order).await {
                Ok(order_id) => {
                    if let Some(ref builder) = self.metric_builder {
                        let metric = builder.order_placed(market_id, "sell");
                        ctx.emit_metric(metric).await?;
                    }
                }
                Err(StrategyError::RiskRejected { policies }) => {
                    tracing::warn!(
                        market_id = %market_id,
                        policies = ?policies,
                        "Ask order rejected by risk engine"
                    );
                }
                Err(e) => return Err(e),
            }
        }

        self.last_quote_time = Some(Utc::now().timestamp_millis());

        Ok(())
    }

    async fn on_fill(
        &mut self,
        fill: &Fill,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Update position
        let size_delta = match fill.side {
            Side::Buy => fill.size,
            Side::Sell => -fill.size,
        };

        ctx.update_position(&fill.market, size_delta, fill.price);

        // Emit metrics
        if let Some(ref builder) = self.metric_builder {
            let metric = builder.order_filled(&fill.market);
            ctx.emit_metric(metric).await?;

            if let Some(pos) = ctx.get_position(&fill.market) {
                let pnl_metric = builder.pnl(&fill.market, pos.unrealized_pnl);
                ctx.emit_metric(pnl_metric).await?;

                let position_metric = builder.position_size(&fill.market, pos.size);
                ctx.emit_metric(position_metric).await?;
            }
        }

        tracing::info!(
            market_id = %fill.market,
            side = ?fill.side,
            price = %fill.price,
            size = %fill.size,
            "Fill received"
        );

        Ok(())
    }

    async fn on_cancel(
        &mut self,
        order_id: &OrderId,
        _ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        tracing::debug!(order_id = %order_id, "Order cancelled");
        Ok(())
    }

    async fn on_timer(
        &mut self,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Emit periodic position metrics
        if let Some(ref builder) = self.metric_builder {
            if let Some(pos) = ctx.get_position(&self.market_id) {
                let pnl_metric = builder.pnl(&self.market_id, pos.unrealized_pnl);
                ctx.emit_metric(pnl_metric).await?;
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()> {
        // Cancel all open orders
        let open_orders: Vec<OrderId> = ctx.get_open_orders()
            .iter()
            .filter_map(|o| o.id.clone())
            .collect();

        for order_id in open_orders {
            ctx.cancel_order(&order_id).await?;
        }

        tracing::info!(
            strategy_id = %ctx.strategy_id,
            market_id = %self.market_id,
            "Market maker strategy shutdown"
        );

        Ok(())
    }

    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            name: "MarketMaker".to_string(),
            version: "1.0.0".to_string(),
            description: "Market making with inventory skewing".to_string(),
            markets: vec![self.market_id.clone()],
            required_params: vec![
                "target_spread_bps".to_string(),
                "quote_size".to_string(),
                "max_position".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StrategyParams;
    use ag_risk::RiskEngine;
    use std::sync::Arc;
    use parking_lot::Mutex;

    fn create_test_context() -> StrategyContext {
        let yaml = r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
"#;
        let risk_engine = RiskEngine::from_yaml(yaml).unwrap();
        StrategyContext::new(
            "test_mm".to_string(),
            Arc::new(Mutex::new(risk_engine)),
            StrategyParams::new(),
        )
    }

    #[test]
    fn test_inventory_skew() {
        let config = MarketMakerConfig {
            max_position: 1000.0,
            inventory_target: 0.0,
            ..Default::default()
        };
        let strategy = MarketMakerStrategy::new("market1".to_string(), config);

        // Neutral position
        let skew = strategy.calculate_inventory_skew(0.0);
        assert_eq!(skew, 0.0);

        // Long position
        let skew = strategy.calculate_inventory_skew(500.0);
        assert_eq!(skew, 0.5);

        // Short position
        let skew = strategy.calculate_inventory_skew(-500.0);
        assert_eq!(skew, -0.5);
    }

    #[test]
    fn test_quote_calculation() {
        let config = MarketMakerConfig {
            target_spread_bps: 20.0, // 0.2%
            max_position: 1000.0,
            inventory_target: 0.0,
            skew_factor: 0.5,
            ..Default::default()
        };
        let strategy = MarketMakerStrategy::new("market1".to_string(), config);

        // Neutral position
        let (bid, ask) = strategy.calculate_quotes(100.0, 0.0);
        assert!(bid < 100.0);
        assert!(ask > 100.0);
        assert!((ask - bid - 0.2).abs() < 0.01); // Spread should be ~0.2

        // Long position - should encourage selling
        let (bid_long, ask_long) = strategy.calculate_quotes(100.0, 500.0);
        assert!(ask_long < ask); // Lower ask to encourage selling
    }
}

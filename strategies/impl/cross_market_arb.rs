//! Cross-market arbitrage strategy

use crate::{Strategy, StrategyContext, StrategyError, StrategyResult, StrategyMetadata};
use crate::types::{MarketTick, Fill, OrderId, Order, Side, OrderType, TimeInForce};
use crate::metrics::MetricBuilder;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cross-market arbitrage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossMarketArbConfig {
    /// Minimum spread in basis points to execute arbitrage
    pub min_spread_bps: f64,

    /// Size to trade
    pub size: f64,

    /// Maximum position size per market
    pub max_position: f64,
}

impl Default for CrossMarketArbConfig {
    fn default() -> Self {
        Self {
            min_spread_bps: 10.0,
            size: 50.0,
            max_position: 500.0,
        }
    }
}

/// Cross-market arbitrage strategy
///
/// Monitors two markets for price discrepancies and executes arbitrage
/// when the spread exceeds the minimum threshold.
pub struct CrossMarketArbStrategy {
    config: CrossMarketArbConfig,
    market_a: String,
    market_b: String,
    last_prices: HashMap<String, f64>,
    metric_builder: Option<MetricBuilder>,
}

impl CrossMarketArbStrategy {
    pub fn new(market_a: String, market_b: String, config: CrossMarketArbConfig) -> Self {
        Self {
            config,
            market_a,
            market_b,
            last_prices: HashMap::new(),
            metric_builder: None,
        }
    }

    /// Get the other market ID
    fn get_other_market(&self, market_id: &str) -> Option<&str> {
        if market_id == self.market_a {
            Some(&self.market_b)
        } else if market_id == self.market_b {
            Some(&self.market_a)
        } else {
            None
        }
    }

    /// Calculate spread in basis points
    fn calculate_spread_bps(&self, price_a: f64, price_b: f64) -> f64 {
        let mid = (price_a + price_b) / 2.0;
        if mid < 1e-8 {
            return 0.0;
        }
        ((price_a - price_b).abs() / mid) * 10000.0
    }

    /// Execute arbitrage trade
    async fn execute_arbitrage(
        &self,
        buy_market: &str,
        sell_market: &str,
        buy_price: f64,
        sell_price: f64,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Check position limits
        let buy_position = ctx.get_position(buy_market).map(|p| p.size).unwrap_or(0.0);
        let sell_position = ctx.get_position(sell_market).map(|p| p.size).unwrap_or(0.0);

        if buy_position + self.config.size > self.config.max_position {
            tracing::warn!(
                market = %buy_market,
                position = %buy_position,
                "Buy position limit would be exceeded"
            );
            return Ok(());
        }

        if sell_position - self.config.size < -self.config.max_position {
            tracing::warn!(
                market = %sell_market,
                position = %sell_position,
                "Sell position limit would be exceeded"
            );
            return Ok(());
        }

        // Submit buy order
        let buy_order = Order {
            venue: "polymarket".to_string(),
            market: buy_market.to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(buy_price),
            size: self.config.size,
            time_in_force: TimeInForce::IOC,
            ..Default::default()
        };

        // Submit sell order
        let sell_order = Order {
            venue: "polymarket".to_string(),
            market: sell_market.to_string(),
            side: Side::Sell,
            order_type: OrderType::Limit,
            price: Some(sell_price),
            size: self.config.size,
            time_in_force: TimeInForce::IOC,
            ..Default::default()
        };

        // Execute both legs
        match ctx.submit_order(buy_order).await {
            Ok(buy_order_id) => {
                tracing::info!(
                    market = %buy_market,
                    price = %buy_price,
                    size = %self.config.size,
                    "Buy leg submitted"
                );

                if let Some(ref builder) = self.metric_builder {
                    let metric = builder.order_placed(buy_market, "buy");
                    ctx.emit_metric(metric).await?;
                }
            }
            Err(e) => {
                tracing::error!(error = ?e, market = %buy_market, "Buy leg failed");
                return Err(e);
            }
        }

        match ctx.submit_order(sell_order).await {
            Ok(sell_order_id) => {
                tracing::info!(
                    market = %sell_market,
                    price = %sell_price,
                    size = %self.config.size,
                    "Sell leg submitted"
                );

                if let Some(ref builder) = self.metric_builder {
                    let metric = builder.order_placed(sell_market, "sell");
                    ctx.emit_metric(metric).await?;
                }
            }
            Err(e) => {
                tracing::error!(error = ?e, market = %sell_market, "Sell leg failed");
                return Err(e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Strategy for CrossMarketArbStrategy {
    async fn initialize(&mut self, ctx: &mut StrategyContext) -> StrategyResult<()> {
        self.metric_builder = Some(MetricBuilder::new(ctx.strategy_id.clone()));

        tracing::info!(
            strategy_id = %ctx.strategy_id,
            market_a = %self.market_a,
            market_b = %self.market_b,
            "Cross-market arbitrage strategy initialized"
        );

        Ok(())
    }

    async fn on_market_tick(
        &mut self,
        market_id: &str,
        tick: &MarketTick,
        ctx: &mut StrategyContext,
    ) -> StrategyResult<()> {
        // Only process our markets
        if market_id != self.market_a && market_id != self.market_b {
            return Ok(());
        }

        // Update last price for this market
        let price = tick.mid_price();
        if price < 1e-8 {
            return Ok(());
        }
        self.last_prices.insert(market_id.to_string(), price);

        // Check if we have prices for both markets
        let price_a = match self.last_prices.get(&self.market_a) {
            Some(&p) => p,
            None => return Ok(()),
        };

        let price_b = match self.last_prices.get(&self.market_b) {
            Some(&p) => p,
            None => return Ok(()),
        };

        // Calculate spread
        let spread_bps = self.calculate_spread_bps(price_a, price_b);

        // Emit signal metric
        if let Some(ref builder) = self.metric_builder {
            let mut labels = HashMap::new();
            labels.insert("market_a".to_string(), self.market_a.clone());
            labels.insert("market_b".to_string(), self.market_b.clone());

            let metric = crate::metrics::StrategyMetric::gauge(
                ctx.strategy_id.clone(),
                "strategy.arb_spread_bps".to_string(),
                spread_bps,
                labels,
            );
            ctx.emit_metric(metric).await?;
        }

        // Check if spread exceeds threshold
        if spread_bps >= self.config.min_spread_bps {
            tracing::info!(
                spread_bps = %spread_bps,
                price_a = %price_a,
                price_b = %price_b,
                "Arbitrage opportunity detected"
            );

            // Determine which market to buy and which to sell
            let (buy_market, sell_market, buy_price, sell_price) = if price_a < price_b {
                (&self.market_a, &self.market_b, price_a, price_b)
            } else {
                (&self.market_b, &self.market_a, price_b, price_a)
            };

            // Execute arbitrage
            if let Some(ref builder) = self.metric_builder {
                let metric = builder.signal_generated(buy_market, "arbitrage");
                ctx.emit_metric(metric).await?;
            }

            self.execute_arbitrage(buy_market, sell_market, buy_price, sell_price, ctx).await?;
        }

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
        // Emit periodic metrics
        if let Some(ref builder) = self.metric_builder {
            for market_id in &[&self.market_a, &self.market_b] {
                // Extract position values to avoid borrow checker issues
                let position_data = ctx.get_position(market_id)
                    .map(|pos| (pos.unrealized_pnl, pos.size));

                if let Some((unrealized_pnl, position_size)) = position_data {
                    let pnl_metric = builder.pnl(market_id, unrealized_pnl);
                    ctx.emit_metric(pnl_metric).await?;

                    let position_metric = builder.position_size(market_id, position_size);
                    ctx.emit_metric(position_metric).await?;
                }
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
            "Cross-market arbitrage strategy shutdown"
        );

        Ok(())
    }

    fn metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            name: "CrossMarketArbitrage".to_string(),
            version: "1.0.0".to_string(),
            description: "Arbitrage between two markets".to_string(),
            markets: vec![self.market_a.clone(), self.market_b.clone()],
            required_params: vec![
                "market_a".to_string(),
                "market_b".to_string(),
                "min_spread_bps".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spread_calculation() {
        let config = CrossMarketArbConfig::default();
        let strategy = CrossMarketArbStrategy::new(
            "market_a".to_string(),
            "market_b".to_string(),
            config,
        );

        // 1% spread
        let spread = strategy.calculate_spread_bps(101.0, 100.0);
        assert!((spread - 99.5).abs() < 1.0);

        // No spread
        let spread = strategy.calculate_spread_bps(100.0, 100.0);
        assert_eq!(spread, 0.0);
    }

    #[test]
    fn test_get_other_market() {
        let config = CrossMarketArbConfig::default();
        let strategy = CrossMarketArbStrategy::new(
            "market_a".to_string(),
            "market_b".to_string(),
            config,
        );

        assert_eq!(strategy.get_other_market("market_a"), Some("market_b"));
        assert_eq!(strategy.get_other_market("market_b"), Some("market_a"));
        assert_eq!(strategy.get_other_market("market_c"), None);
    }
}

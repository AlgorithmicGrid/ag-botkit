//! Backtesting engine implementation

use crate::{Strategy, StrategyContext, StrategyError, StrategyResult, StrategyParams};
use crate::types::{MarketTick, Trade};
use crate::backtest::fill_simulator::{FillSimulator, FillSimulatorConfig};
use ag_risk::RiskEngine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;

/// Backtesting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    /// Starting capital in USD
    pub initial_capital: f64,

    /// Fill simulator configuration
    #[serde(skip)]
    pub fill_simulator: FillSimulatorConfig,

    /// Risk policy YAML
    pub risk_policy_yaml: String,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: 10000.0,
            fill_simulator: FillSimulatorConfig::default(),
            risk_policy_yaml: r#"
policies:
  - type: PositionLimit
    max_size: 1000.0
  - type: InventoryLimit
    max_value_usd: 10000.0
"#.to_string(),
        }
    }
}

/// Backtesting result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    /// Total return (absolute)
    pub total_return: f64,

    /// Total return (percentage)
    pub total_return_pct: f64,

    /// Sharpe ratio (annualized)
    pub sharpe_ratio: f64,

    /// Maximum drawdown (absolute)
    pub max_drawdown: f64,

    /// Maximum drawdown (percentage)
    pub max_drawdown_pct: f64,

    /// Win rate
    pub win_rate: f64,

    /// Number of trades
    pub num_trades: usize,

    /// Average trade PnL
    pub avg_trade_pnl: f64,

    /// Daily PnL series
    pub pnl_by_day: Vec<(DateTime<Utc>, f64)>,

    /// All trades
    pub trades: Vec<Trade>,

    /// Final capital
    pub final_capital: f64,
}

/// Event-driven backtesting engine
pub struct BacktestEngine {
    config: BacktestConfig,
    fill_simulator: FillSimulator,
    risk_engine: Arc<Mutex<RiskEngine>>,
}

impl BacktestEngine {
    /// Create a new backtest engine
    pub fn new(config: BacktestConfig) -> StrategyResult<Self> {
        let risk_engine = RiskEngine::from_yaml(&config.risk_policy_yaml)
            .map_err(|e| StrategyError::ConfigError(e))?;

        let fill_simulator = FillSimulator::new(config.fill_simulator.clone());

        Ok(Self {
            config,
            fill_simulator,
            risk_engine: Arc::new(Mutex::new(risk_engine)),
        })
    }

    /// Run backtest for a strategy
    ///
    /// # Arguments
    /// * `strategy` - Strategy to backtest
    /// * `historical_ticks` - Historical market data
    /// * `params` - Strategy parameters
    pub async fn run_backtest(
        &mut self,
        mut strategy: Box<dyn Strategy>,
        historical_ticks: Vec<MarketTick>,
        params: StrategyParams,
    ) -> StrategyResult<BacktestResult> {
        if historical_ticks.is_empty() {
            return Err(StrategyError::InsufficientData(
                "No historical data provided".to_string()
            ));
        }

        // Create strategy context
        let mut ctx = StrategyContext::new(
            "backtest_strategy".to_string(),
            self.risk_engine.clone(),
            params,
        );

        // Initialize strategy
        strategy.initialize(&mut ctx).await?;

        let mut trades = Vec::new();
        let mut equity_curve = Vec::new();
        let start_time = historical_ticks[0].timestamp;

        // Process each tick
        for tick in historical_ticks {
            // Update strategy with market data
            strategy.on_market_tick(&tick.market, &tick, &mut ctx).await?;

            // Simulate fills for any submitted orders
            let orders_to_fill: Vec<_> = ctx.get_open_orders()
                .iter()
                .filter_map(|o| {
                    if o.market == tick.market {
                        Some((*o).clone())
                    } else {
                        None
                    }
                })
                .collect();

            for order in orders_to_fill {
                if let Some(fill) = self.fill_simulator.simulate_fill(&order, &tick) {
                    // Notify strategy of fill
                    strategy.on_fill(&fill, &mut ctx).await?;

                    // Record trade
                    trades.push(Trade {
                        id: fill.order_id.clone(),
                        market: fill.market.clone(),
                        price: fill.price,
                        size: fill.size,
                        side: fill.side,
                        fee: fill.fee,
                        timestamp: fill.timestamp,
                    });

                    // Remove filled order
                    if let Some(order_id) = &order.id {
                        ctx.orders.remove(order_id);
                    }
                }
            }

            // Record equity
            let total_pnl = ctx.calculate_total_realized_pnl() + ctx.calculate_total_unrealized_pnl();
            let equity = self.config.initial_capital + total_pnl;
            equity_curve.push((tick.timestamp, equity));

            // Periodic timer (every 100 ticks)
            if equity_curve.len() % 100 == 0 {
                strategy.on_timer(&mut ctx).await?;
            }
        }

        // Shutdown strategy
        strategy.shutdown(&mut ctx).await?;

        // Calculate performance metrics
        self.calculate_metrics(trades, equity_curve)
    }

    /// Calculate performance metrics from trades and equity curve
    fn calculate_metrics(
        &self,
        trades: Vec<Trade>,
        equity_curve: Vec<(DateTime<Utc>, f64)>,
    ) -> StrategyResult<BacktestResult> {
        let initial_capital = self.config.initial_capital;
        let final_capital = equity_curve.last()
            .map(|(_, eq)| *eq)
            .unwrap_or(initial_capital);

        let total_return = final_capital - initial_capital;
        let total_return_pct = (total_return / initial_capital) * 100.0;

        // Calculate max drawdown
        let mut peak = initial_capital;
        let mut max_drawdown = 0.0;
        for (_, equity) in &equity_curve {
            if *equity > peak {
                peak = *equity;
            }
            let drawdown = peak - equity;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        let max_drawdown_pct = (max_drawdown / peak) * 100.0;

        // Calculate daily returns for Sharpe ratio
        let daily_returns = self.calculate_daily_returns(&equity_curve);
        let sharpe_ratio = self.calculate_sharpe_ratio(&daily_returns);

        // Calculate win rate
        let winning_trades = trades.iter()
            .filter(|t| t.price > 0.0)
            .count();
        let win_rate = if trades.is_empty() {
            0.0
        } else {
            (winning_trades as f64 / trades.len() as f64) * 100.0
        };

        // Calculate average trade PnL
        let avg_trade_pnl = if trades.is_empty() {
            0.0
        } else {
            total_return / trades.len() as f64
        };

        // Group PnL by day
        let pnl_by_day = self.group_pnl_by_day(&equity_curve);

        Ok(BacktestResult {
            total_return,
            total_return_pct,
            sharpe_ratio,
            max_drawdown,
            max_drawdown_pct,
            win_rate,
            num_trades: trades.len(),
            avg_trade_pnl,
            pnl_by_day,
            trades,
            final_capital,
        })
    }

    /// Calculate daily returns
    fn calculate_daily_returns(&self, equity_curve: &[(DateTime<Utc>, f64)]) -> Vec<f64> {
        let mut daily_returns = Vec::new();
        if equity_curve.len() < 2 {
            return daily_returns;
        }

        for window in equity_curve.windows(2) {
            let prev_equity = window[0].1;
            let curr_equity = window[1].1;
            if prev_equity > 0.0 {
                let return_pct = (curr_equity - prev_equity) / prev_equity;
                daily_returns.push(return_pct);
            }
        }

        daily_returns
    }

    /// Calculate annualized Sharpe ratio
    fn calculate_sharpe_ratio(&self, daily_returns: &[f64]) -> f64 {
        if daily_returns.is_empty() {
            return 0.0;
        }

        let mean = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
        let variance = daily_returns.iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / daily_returns.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev < 1e-10 {
            return 0.0;
        }

        // Annualize (assuming 252 trading days)
        mean / std_dev * (252.0_f64).sqrt()
    }

    /// Group equity by day
    fn group_pnl_by_day(&self, equity_curve: &[(DateTime<Utc>, f64)]) -> Vec<(DateTime<Utc>, f64)> {
        // For simplicity, just return the equity curve
        // In production, we'd actually group by day
        equity_curve.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StrategyMetadata, Strategy};
    use crate::types::{Fill, OrderId};
    use async_trait::async_trait;

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
                name: "Dummy".to_string(),
                version: "1.0.0".to_string(),
                description: "Test".to_string(),
                markets: vec![],
                required_params: vec![],
            }
        }
    }

    #[tokio::test]
    async fn test_backtest_engine() {
        let config = BacktestConfig::default();
        let mut engine = BacktestEngine::new(config).unwrap();

        let ticks = vec![
            MarketTick {
                market: "test".to_string(),
                timestamp: Utc::now(),
                bid: Some(100.0),
                ask: Some(101.0),
                bid_size: Some(10.0),
                ask_size: Some(10.0),
                last: Some(100.5),
                volume_24h: Some(1000.0),
            },
        ];

        let strategy = Box::new(DummyStrategy);
        let params = StrategyParams::new();

        let result = engine.run_backtest(strategy, ticks, params).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.num_trades, 0);
        assert_eq!(result.final_capital, 10000.0);
    }
}

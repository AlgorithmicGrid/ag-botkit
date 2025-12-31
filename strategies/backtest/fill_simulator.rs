//! Fill simulation for backtesting

use crate::types::{Order, Fill, MarketTick, Side};
use chrono::Utc;

/// Fill simulator configuration
#[derive(Debug, Clone)]
pub struct FillSimulatorConfig {
    /// Slippage model (bps of price movement)
    pub slippage_bps: f64,

    /// Fill probability for limit orders
    pub fill_probability: f64,

    /// Taker fee (bps)
    pub taker_fee_bps: f64,

    /// Maker fee (bps), negative if rebate
    pub maker_fee_bps: f64,
}

impl Default for FillSimulatorConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 5.0,
            fill_probability: 0.8,
            taker_fee_bps: 10.0,
            maker_fee_bps: -5.0, // Maker rebate
        }
    }
}

/// Fill simulator for backtesting
pub struct FillSimulator {
    config: FillSimulatorConfig,
}

impl FillSimulator {
    pub fn new(config: FillSimulatorConfig) -> Self {
        Self { config }
    }

    /// Simulate fill for an order given market tick
    ///
    /// Returns None if order wouldn't be filled, Some(Fill) if filled
    pub fn simulate_fill(&self, order: &Order, tick: &MarketTick) -> Option<Fill> {
        match order.order_type {
            crate::types::OrderType::Market => self.simulate_market_order_fill(order, tick),
            crate::types::OrderType::Limit => self.simulate_limit_order_fill(order, tick),
            _ => None, // Stop orders not implemented yet
        }
    }

    /// Simulate market order fill
    fn simulate_market_order_fill(&self, order: &Order, tick: &MarketTick) -> Option<Fill> {
        // Market orders always fill (in backtest)
        let fill_price = match order.side {
            Side::Buy => {
                // Buy at ask + slippage
                let ask = tick.ask.unwrap_or_else(|| tick.mid_price());
                ask * (1.0 + self.config.slippage_bps / 10000.0)
            }
            Side::Sell => {
                // Sell at bid - slippage
                let bid = tick.bid.unwrap_or_else(|| tick.mid_price());
                bid * (1.0 - self.config.slippage_bps / 10000.0)
            }
        };

        let fee = order.size * fill_price * self.config.taker_fee_bps / 10000.0;

        Some(Fill {
            order_id: order.id.clone().unwrap_or_default(),
            market: order.market.clone(),
            price: fill_price,
            size: order.size,
            side: order.side,
            fee,
            timestamp: Utc::now(),
        })
    }

    /// Simulate limit order fill
    fn simulate_limit_order_fill(&self, order: &Order, tick: &MarketTick) -> Option<Fill> {
        let order_price = order.price?;

        // Check if order price crosses the market
        let would_fill = match order.side {
            Side::Buy => {
                // Buy limit fills if order price >= ask
                tick.ask.map(|ask| order_price >= ask).unwrap_or(false)
            }
            Side::Sell => {
                // Sell limit fills if order price <= bid
                tick.bid.map(|bid| order_price <= bid).unwrap_or(false)
            }
        };

        if !would_fill {
            // Order rests on book - probabilistic fill
            if rand::random::<f64>() > self.config.fill_probability {
                return None;
            }
        }

        // Determine if maker or taker
        let is_maker = !would_fill;
        let fee_bps = if is_maker {
            self.config.maker_fee_bps
        } else {
            self.config.taker_fee_bps
        };

        let fill_price = if would_fill {
            // Aggressive order - fill at order price (price improvement)
            order_price
        } else {
            // Passive order - fill at order price
            order_price
        };

        let fee = order.size * fill_price * fee_bps / 10000.0;

        Some(Fill {
            order_id: order.id.clone().unwrap_or_default(),
            market: order.market.clone(),
            price: fill_price,
            size: order.size,
            side: order.side,
            fee,
            timestamp: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderType, TimeInForce};
    use chrono::Utc;

    fn create_test_tick(bid: f64, ask: f64) -> MarketTick {
        MarketTick {
            market: "test".to_string(),
            timestamp: Utc::now(),
            bid: Some(bid),
            ask: Some(ask),
            bid_size: Some(100.0),
            ask_size: Some(100.0),
            last: Some((bid + ask) / 2.0),
            volume_24h: Some(1000.0),
        }
    }

    #[test]
    fn test_market_order_fill() {
        let simulator = FillSimulator::new(FillSimulatorConfig::default());
        let tick = create_test_tick(100.0, 101.0);

        // Buy market order
        let buy_order = Order {
            id: Some("test1".to_string()),
            venue: "test".to_string(),
            market: "test".to_string(),
            side: Side::Buy,
            order_type: OrderType::Market,
            price: None,
            size: 10.0,
            time_in_force: TimeInForce::IOC,
            ..Default::default()
        };

        let fill = simulator.simulate_fill(&buy_order, &tick).unwrap();
        assert!(fill.price > 101.0); // Should fill at ask + slippage
        assert_eq!(fill.size, 10.0);
        assert!(fill.fee > 0.0); // Taker fee

        // Sell market order
        let sell_order = Order {
            id: Some("test2".to_string()),
            venue: "test".to_string(),
            market: "test".to_string(),
            side: Side::Sell,
            order_type: OrderType::Market,
            price: None,
            size: 10.0,
            time_in_force: TimeInForce::IOC,
            ..Default::default()
        };

        let fill = simulator.simulate_fill(&sell_order, &tick).unwrap();
        assert!(fill.price < 100.0); // Should fill at bid - slippage
    }

    #[test]
    fn test_limit_order_fill() {
        let simulator = FillSimulator::new(FillSimulatorConfig::default());
        let tick = create_test_tick(100.0, 101.0);

        // Buy limit at 101.0 (crosses market, should fill)
        let buy_order = Order {
            id: Some("test3".to_string()),
            venue: "test".to_string(),
            market: "test".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(101.0),
            size: 10.0,
            time_in_force: TimeInForce::GTC,
            ..Default::default()
        };

        let fill = simulator.simulate_fill(&buy_order, &tick);
        assert!(fill.is_some());

        // Buy limit at 99.0 (doesn't cross, may not fill)
        let buy_order_low = Order {
            id: Some("test4".to_string()),
            venue: "test".to_string(),
            market: "test".to_string(),
            side: Side::Buy,
            order_type: OrderType::Limit,
            price: Some(99.0),
            size: 10.0,
            time_in_force: TimeInForce::GTC,
            ..Default::default()
        };

        // May or may not fill due to probability
        let _ = simulator.simulate_fill(&buy_order_low, &tick);
    }
}

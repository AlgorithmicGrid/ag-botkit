//! Polymarket position simulator
//!
//! This module provides a simulator for tracking positions and calculating
//! PnL for Polymarket binary outcome (YES/NO) markets.

use std::collections::HashMap;

/// Position tracking for Polymarket markets
///
/// This simulator maintains position state for multiple markets,
/// tracking both YES and NO positions, average entry prices, and PnL.
#[derive(Debug, Clone)]
pub struct PolymarketSimulator {
    /// Position data per market
    positions: HashMap<String, MarketPosition>,
}

/// Position details for a single market
#[derive(Debug, Clone)]
struct MarketPosition {
    /// Net position size (positive = long YES, negative = short YES/long NO)
    size: f64,

    /// Weighted average entry price
    avg_price: f64,

    /// Total invested capital
    invested_capital: f64,

    /// Current market price (last update)
    current_price: f64,
}

impl PolymarketSimulator {
    /// Create a new simulator with no positions
    ///
    /// # Example
    ///
    /// ```
    /// use ag_risk::PolymarketSimulator;
    ///
    /// let mut sim = PolymarketSimulator::new();
    /// sim.update_position("0x123", 100.0, 0.55);
    /// assert_eq!(sim.get_position("0x123"), 100.0);
    /// ```
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }

    /// Update position for a market
    ///
    /// This method applies a fill (buy or sell) to the position tracking.
    /// Positive size = buy, negative size = sell.
    ///
    /// # Arguments
    ///
    /// * `market_id` - Market identifier
    /// * `size` - Size of the fill (signed: +buy, -sell)
    /// * `price` - Execution price (0.0 to 1.0 for binary markets)
    ///
    /// # Example
    ///
    /// ```
    /// use ag_risk::PolymarketSimulator;
    ///
    /// let mut sim = PolymarketSimulator::new();
    ///
    /// // Buy 100 shares at 0.55
    /// sim.update_position("0x123", 100.0, 0.55);
    ///
    /// // Sell 50 shares at 0.60
    /// sim.update_position("0x123", -50.0, 0.60);
    ///
    /// assert_eq!(sim.get_position("0x123"), 50.0);
    /// ```
    pub fn update_position(&mut self, market_id: &str, size: f64, price: f64) {
        let position = self
            .positions
            .entry(market_id.to_string())
            .or_insert(MarketPosition {
                size: 0.0,
                avg_price: 0.0,
                invested_capital: 0.0,
                current_price: price,
            });

        // Update current market price
        position.current_price = price;

        let new_size = position.size + size;

        // Calculate new average price and invested capital
        if new_size.abs() < 1e-10 {
            // Position closed
            position.size = 0.0;
            position.avg_price = 0.0;
            position.invested_capital = 0.0;
        } else if position.size.abs() < 1e-10 {
            // Opening new position from zero
            position.size = new_size;
            position.avg_price = price;
            position.invested_capital = new_size * price;
        } else if (position.size > 0.0 && new_size > 0.0) || (position.size < 0.0 && new_size < 0.0) {
            // Staying in same direction (adding or reducing)
            if size.signum() == position.size.signum() {
                // Adding to position
                let new_capital = position.invested_capital + (size * price);
                position.avg_price = new_capital / new_size;
                position.invested_capital = new_capital;
            } else {
                // Reducing position
                let reduction_ratio = size.abs() / position.size.abs();
                position.invested_capital *= 1.0 - reduction_ratio;
            }
            position.size = new_size;
        } else {
            // Reversing position (was long, now short, or vice versa)
            position.size = new_size;
            position.avg_price = price;
            position.invested_capital = new_size * price;
        }
    }

    /// Get current position for a market
    ///
    /// Returns 0.0 if no position exists.
    pub fn get_position(&self, market_id: &str) -> f64 {
        self.positions
            .get(market_id)
            .map(|p| p.size)
            .unwrap_or(0.0)
    }

    /// Get average entry price for a market
    ///
    /// Returns 0.0 if no position exists.
    pub fn get_avg_price(&self, market_id: &str) -> f64 {
        self.positions
            .get(market_id)
            .map(|p| p.avg_price)
            .unwrap_or(0.0)
    }

    /// Get unrealized PnL for a market
    ///
    /// Calculates PnL based on current position, average entry price,
    /// and last known market price.
    pub fn get_unrealized_pnl(&self, market_id: &str) -> f64 {
        if let Some(position) = self.positions.get(market_id) {
            if position.size.abs() < 1e-10 {
                return 0.0;
            }
            let market_value = position.size * position.current_price;
            market_value - position.invested_capital
        } else {
            0.0
        }
    }

    /// Get total inventory value across all markets
    ///
    /// Calculates the sum of absolute position values at current prices.
    ///
    /// # Example
    ///
    /// ```
    /// use ag_risk::PolymarketSimulator;
    ///
    /// let mut sim = PolymarketSimulator::new();
    /// sim.update_position("0x123", 100.0, 0.55);
    /// sim.update_position("0x456", 200.0, 0.40);
    ///
    /// // |100 * 0.55| + |200 * 0.40| = 55 + 80 = 135
    /// assert_eq!(sim.get_inventory_value_usd(), 135.0);
    /// ```
    pub fn get_inventory_value_usd(&self) -> f64 {
        self.positions
            .values()
            .map(|p| (p.size * p.current_price).abs())
            .sum()
    }

    /// Get total unrealized PnL across all markets
    pub fn get_total_pnl(&self) -> f64 {
        self.positions
            .keys()
            .map(|market_id| self.get_unrealized_pnl(market_id))
            .sum()
    }

    /// Reset all positions to zero
    pub fn reset(&mut self) {
        self.positions.clear();
    }

    /// Get all active market IDs
    pub fn get_active_markets(&self) -> Vec<String> {
        self.positions
            .iter()
            .filter(|(_, p)| p.size.abs() > 1e-10)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get detailed position info for a market
    pub fn get_position_details(&self, market_id: &str) -> Option<PositionDetails> {
        self.positions.get(market_id).map(|p| PositionDetails {
            size: p.size,
            avg_price: p.avg_price,
            current_price: p.current_price,
            invested_capital: p.invested_capital,
            unrealized_pnl: self.get_unrealized_pnl(market_id),
        })
    }
}

impl Default for PolymarketSimulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Detailed position information
#[derive(Debug, Clone)]
pub struct PositionDetails {
    pub size: f64,
    pub avg_price: f64,
    pub current_price: f64,
    pub invested_capital: f64,
    pub unrealized_pnl: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_simulator() {
        let sim = PolymarketSimulator::new();
        assert_eq!(sim.get_position("0x123"), 0.0);
        assert_eq!(sim.get_inventory_value_usd(), 0.0);
    }

    #[test]
    fn test_single_buy() {
        let mut sim = PolymarketSimulator::new();
        sim.update_position("0x123", 100.0, 0.55);

        assert_eq!(sim.get_position("0x123"), 100.0);
        assert_eq!(sim.get_avg_price("0x123"), 0.55);
        assert!((sim.get_inventory_value_usd() - 55.0).abs() < 1e-10);
    }

    #[test]
    fn test_multiple_buys_same_market() {
        let mut sim = PolymarketSimulator::new();

        // First buy: 100 @ 0.50
        sim.update_position("0x123", 100.0, 0.50);

        // Second buy: 100 @ 0.60
        sim.update_position("0x123", 100.0, 0.60);

        assert_eq!(sim.get_position("0x123"), 200.0);
        // Average: (100*0.50 + 100*0.60) / 200 = 0.55
        assert_eq!(sim.get_avg_price("0x123"), 0.55);
    }

    #[test]
    fn test_buy_then_sell() {
        let mut sim = PolymarketSimulator::new();

        // Buy 100 @ 0.50
        sim.update_position("0x123", 100.0, 0.50);

        // Sell 50 @ 0.60
        sim.update_position("0x123", -50.0, 0.60);

        assert_eq!(sim.get_position("0x123"), 50.0);
        // PnL should be positive from selling at higher price
        let pnl = sim.get_unrealized_pnl("0x123");
        assert!(pnl > 0.0);
    }

    #[test]
    fn test_close_position() {
        let mut sim = PolymarketSimulator::new();

        // Buy 100 @ 0.50
        sim.update_position("0x123", 100.0, 0.50);

        // Sell all @ 0.60
        sim.update_position("0x123", -100.0, 0.60);

        assert_eq!(sim.get_position("0x123"), 0.0);
        assert_eq!(sim.get_avg_price("0x123"), 0.0);
    }

    #[test]
    fn test_multiple_markets() {
        let mut sim = PolymarketSimulator::new();

        sim.update_position("0x123", 100.0, 0.55);
        sim.update_position("0x456", 200.0, 0.40);

        assert_eq!(sim.get_position("0x123"), 100.0);
        assert_eq!(sim.get_position("0x456"), 200.0);

        // Total inventory: |100*0.55| + |200*0.40| = 55 + 80 = 135
        assert_eq!(sim.get_inventory_value_usd(), 135.0);
    }

    #[test]
    fn test_pnl_calculation() {
        let mut sim = PolymarketSimulator::new();

        // Buy 100 @ 0.50
        sim.update_position("0x123", 100.0, 0.50);

        // Price moves to 0.60
        sim.update_position("0x123", 0.0, 0.60);

        let pnl = sim.get_unrealized_pnl("0x123");
        // PnL = (100 * 0.60) - (100 * 0.50) = 10.0
        assert_eq!(pnl, 10.0);
    }

    #[test]
    fn test_negative_pnl() {
        let mut sim = PolymarketSimulator::new();

        // Buy 100 @ 0.60
        sim.update_position("0x123", 100.0, 0.60);

        // Price drops to 0.50
        sim.update_position("0x123", 0.0, 0.50);

        let pnl = sim.get_unrealized_pnl("0x123");
        // PnL = (100 * 0.50) - (100 * 0.60) = -10.0
        assert_eq!(pnl, -10.0);
    }

    #[test]
    fn test_reset() {
        let mut sim = PolymarketSimulator::new();

        sim.update_position("0x123", 100.0, 0.55);
        sim.update_position("0x456", 200.0, 0.40);

        sim.reset();

        assert_eq!(sim.get_position("0x123"), 0.0);
        assert_eq!(sim.get_position("0x456"), 0.0);
        assert_eq!(sim.get_inventory_value_usd(), 0.0);
    }

    #[test]
    fn test_get_active_markets() {
        let mut sim = PolymarketSimulator::new();

        sim.update_position("0x123", 100.0, 0.55);
        sim.update_position("0x456", 200.0, 0.40);
        sim.update_position("0x789", 0.0, 0.50); // Zero position

        let active = sim.get_active_markets();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"0x123".to_string()));
        assert!(active.contains(&"0x456".to_string()));
    }

    #[test]
    fn test_position_details() {
        let mut sim = PolymarketSimulator::new();

        sim.update_position("0x123", 100.0, 0.50);
        sim.update_position("0x123", 0.0, 0.60); // Update price

        let details = sim.get_position_details("0x123").unwrap();
        assert_eq!(details.size, 100.0);
        assert_eq!(details.avg_price, 0.50);
        assert_eq!(details.current_price, 0.60);
        assert_eq!(details.unrealized_pnl, 10.0);
    }

    #[test]
    fn test_short_position() {
        let mut sim = PolymarketSimulator::new();

        // Sell short 100 @ 0.60
        sim.update_position("0x123", -100.0, 0.60);

        assert_eq!(sim.get_position("0x123"), -100.0);

        // Price drops to 0.50 (favorable for short)
        sim.update_position("0x123", 0.0, 0.50);

        let pnl = sim.get_unrealized_pnl("0x123");
        // For short: PnL = invested - current_value
        // invested = 100 * 0.60 = 60
        // current = -100 * 0.50 = -50
        // PnL = -50 - (-60) = 10
        assert!(pnl > 0.0);
    }

    #[test]
    fn test_total_pnl() {
        let mut sim = PolymarketSimulator::new();

        // Market 1: Buy 100 @ 0.50, price -> 0.60 (+10 PnL)
        sim.update_position("0x123", 100.0, 0.50);
        sim.update_position("0x123", 0.0, 0.60);

        // Market 2: Buy 100 @ 0.60, price -> 0.50 (-10 PnL)
        sim.update_position("0x456", 100.0, 0.60);
        sim.update_position("0x456", 0.0, 0.50);

        let total = sim.get_total_pnl();
        assert_eq!(total, 0.0); // +10 + (-10) = 0
    }

    #[test]
    fn test_position_reversal() {
        let mut sim = PolymarketSimulator::new();

        // Long 100 @ 0.50
        sim.update_position("0x123", 100.0, 0.50);

        // Sell 200 @ 0.60 (close long, open short 100)
        sim.update_position("0x123", -200.0, 0.60);

        assert_eq!(sim.get_position("0x123"), -100.0);
        assert_eq!(sim.get_avg_price("0x123"), 0.60);
    }
}

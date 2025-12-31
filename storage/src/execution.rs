use crate::config::StorageConfig;
use crate::error::{Result, StorageError};
use crate::timescale::{ConnectionPool, QueryBuilder};
use crate::types::{Fill, Order, OrderFilters, PositionSnapshot};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

/// Execution history storage
pub struct ExecutionStore {
    pool: Arc<ConnectionPool>,
    config: StorageConfig,
}

impl ExecutionStore {
    /// Create new execution store
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!("Initializing ExecutionStore");

        let pool = ConnectionPool::new(&config.database).await?;
        let pool = Arc::new(pool);

        Ok(Self { pool, config })
    }

    /// Store order placement
    pub async fn store_order(&mut self, order: Order) -> Result<()> {
        debug!("Storing order: {}", order.id);

        let client = self.pool.get().await?;

        client
            .execute(
                r#"
                INSERT INTO orders (
                    id, timestamp, venue, market, side, order_type,
                    price, size, status, client_order_id, venue_order_id, time_in_force
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (client_order_id) DO UPDATE SET
                    status = EXCLUDED.status,
                    venue_order_id = EXCLUDED.venue_order_id,
                    updated_at = NOW()
                "#,
                &[
                    &order.id,
                    &order.timestamp,
                    &order.venue,
                    &order.market,
                    &order.side.to_string(),
                    &order.order_type.to_string(),
                    &order.price,
                    &order.size,
                    &order.status.to_string(),
                    &order.client_order_id,
                    &order.venue_order_id,
                    &order.time_in_force,
                ],
            )
            .await?;

        Ok(())
    }

    /// Store execution fill
    pub async fn store_fill(&mut self, fill: Fill) -> Result<()> {
        debug!("Storing fill: {}", fill.id);

        let client = self.pool.get().await?;

        client
            .execute(
                r#"
                INSERT INTO fills (
                    id, timestamp, order_id, venue, market, side,
                    price, size, fee, fee_currency, trade_id, liquidity
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                "#,
                &[
                    &fill.id,
                    &fill.timestamp,
                    &fill.order_id,
                    &fill.venue,
                    &fill.market,
                    &fill.side.to_string(),
                    &fill.price,
                    &fill.size,
                    &fill.fee,
                    &fill.fee_currency,
                    &fill.trade_id,
                    &fill.liquidity,
                ],
            )
            .await?;

        Ok(())
    }

    /// Store position snapshot
    pub async fn store_position(&mut self, position: PositionSnapshot) -> Result<()> {
        debug!("Storing position: {} @ {}", position.market, position.venue);

        let client = self.pool.get().await?;

        client
            .execute(
                r#"
                INSERT INTO positions (
                    timestamp, market, venue, size, avg_entry_price,
                    unrealized_pnl, realized_pnl, mark_price
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                &[
                    &position.timestamp,
                    &position.market,
                    &position.venue,
                    &position.size,
                    &position.avg_entry_price,
                    &position.unrealized_pnl,
                    &position.realized_pnl,
                    &position.mark_price,
                ],
            )
            .await?;

        Ok(())
    }

    /// Query order history
    pub async fn query_orders(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        filters: OrderFilters,
    ) -> Result<Vec<Order>> {
        debug!("Querying orders from {} to {}", start, end);

        let client = self.pool.get().await?;

        let mut builder = QueryBuilder::new("orders")
            .time_range(start, end)
            .order_by("timestamp", true)
            .limit(self.config.query.max_results);

        if let Some(venue) = filters.venue {
            builder = builder.eq("venue", &venue);
        }

        if let Some(market) = filters.market {
            builder = builder.eq("market", &market);
        }

        if let Some(side) = filters.side {
            builder = builder.eq("side", &side.to_string());
        }

        if let Some(status) = filters.status {
            builder = builder.eq("status", &status.to_string());
        }

        if let Some(client_order_id) = filters.client_order_id {
            builder = builder.eq("client_order_id", &client_order_id);
        }

        let (query, param_strings) = builder.build_select(&[
            "id",
            "timestamp",
            "venue",
            "market",
            "side",
            "order_type",
            "price",
            "size",
            "status",
            "client_order_id",
            "venue_order_id",
            "time_in_force",
        ]);

        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = param_strings
            .iter()
            .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client.query(&query, &params).await?;

        let orders: Vec<Order> = rows
            .iter()
            .map(|row| {
                let side_str: String = row.get(4);
                let order_type_str: String = row.get(5);
                let status_str: String = row.get(8);

                Order {
                    id: row.get(0),
                    timestamp: row.get(1),
                    venue: row.get(2),
                    market: row.get(3),
                    side: parse_side(&side_str),
                    order_type: parse_order_type(&order_type_str),
                    price: row.get(6),
                    size: row.get(7),
                    status: parse_order_status(&status_str),
                    client_order_id: row.get(9),
                    venue_order_id: row.get(10),
                    time_in_force: row.get(11),
                }
            })
            .collect();

        debug!("Found {} orders", orders.len());

        Ok(orders)
    }

    /// Query fills by order ID
    pub async fn query_fills_by_order(&self, order_id: Uuid) -> Result<Vec<Fill>> {
        debug!("Querying fills for order: {}", order_id);

        let client = self.pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT id, timestamp, order_id, venue, market, side,
                       price, size, fee, fee_currency, trade_id, liquidity
                FROM fills
                WHERE order_id = $1
                ORDER BY timestamp ASC
                "#,
                &[&order_id],
            )
            .await?;

        let fills: Vec<Fill> = rows
            .iter()
            .map(|row| {
                let side_str: String = row.get(5);

                Fill {
                    id: row.get(0),
                    timestamp: row.get(1),
                    order_id: row.get(2),
                    venue: row.get(3),
                    market: row.get(4),
                    side: parse_side(&side_str),
                    price: row.get(6),
                    size: row.get(7),
                    fee: row.get(8),
                    fee_currency: row.get(9),
                    trade_id: row.get(10),
                    liquidity: row.get(11),
                }
            })
            .collect();

        Ok(fills)
    }

    /// Get position history
    pub async fn query_positions(
        &self,
        market_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PositionSnapshot>> {
        debug!("Querying positions for market: {}", market_id);

        let client = self.pool.get().await?;

        let rows = client
            .query(
                r#"
                SELECT timestamp, market, venue, size, avg_entry_price,
                       unrealized_pnl, realized_pnl, mark_price
                FROM positions
                WHERE market = $1 AND timestamp >= $2 AND timestamp <= $3
                ORDER BY timestamp DESC
                LIMIT $4
                "#,
                &[
                    &market_id,
                    &start,
                    &end,
                    &(self.config.query.max_results as i64),
                ],
            )
            .await?;

        let positions: Vec<PositionSnapshot> = rows
            .iter()
            .map(|row| PositionSnapshot {
                timestamp: row.get(0),
                market: row.get(1),
                venue: row.get(2),
                size: row.get(3),
                avg_entry_price: row.get(4),
                unrealized_pnl: row.get(5),
                realized_pnl: row.get(6),
                mark_price: row.get(7),
            })
            .collect();

        debug!("Found {} position snapshots", positions.len());

        Ok(positions)
    }

    /// Get latest position for a market
    pub async fn get_latest_position(&self, venue: &str, market: &str) -> Result<Option<PositionSnapshot>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                r#"
                SELECT timestamp, market, venue, size, avg_entry_price,
                       unrealized_pnl, realized_pnl, mark_price
                FROM positions
                WHERE venue = $1 AND market = $2
                ORDER BY timestamp DESC
                LIMIT 1
                "#,
                &[&venue, &market],
            )
            .await?;

        Ok(row.map(|row| PositionSnapshot {
            timestamp: row.get(0),
            market: row.get(1),
            venue: row.get(2),
            size: row.get(3),
            avg_entry_price: row.get(4),
            unrealized_pnl: row.get(5),
            realized_pnl: row.get(6),
            mark_price: row.get(7),
        }))
    }
}

// Helper functions to parse enum types
use crate::types::{OrderStatus, OrderType, Side};

fn parse_side(s: &str) -> Side {
    match s.to_lowercase().as_str() {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        _ => Side::Buy,
    }
}

fn parse_order_type(s: &str) -> OrderType {
    match s.to_lowercase().as_str() {
        "market" => OrderType::Market,
        "limit" => OrderType::Limit,
        "stop_limit" => OrderType::StopLimit,
        "stop_market" => OrderType::StopMarket,
        _ => OrderType::Limit,
    }
}

fn parse_order_status(s: &str) -> OrderStatus {
    match s.to_lowercase().as_str() {
        "open" => OrderStatus::Open,
        "partial" => OrderStatus::Partial,
        "filled" => OrderStatus::Filled,
        "cancelled" => OrderStatus::Cancelled,
        "rejected" => OrderStatus::Rejected,
        _ => OrderStatus::Open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_side() {
        assert_eq!(parse_side("buy"), Side::Buy);
        assert_eq!(parse_side("sell"), Side::Sell);
        assert_eq!(parse_side("BUY"), Side::Buy);
    }

    #[test]
    fn test_parse_order_type() {
        assert_eq!(parse_order_type("limit"), OrderType::Limit);
        assert_eq!(parse_order_type("market"), OrderType::Market);
    }

    #[test]
    fn test_parse_order_status() {
        assert_eq!(parse_order_status("open"), OrderStatus::Open);
        assert_eq!(parse_order_status("filled"), OrderStatus::Filled);
    }
}

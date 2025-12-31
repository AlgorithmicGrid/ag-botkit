-- TimescaleDB Execution Schema
-- Tables for orders, fills, and positions history

-- Orders table
CREATE TABLE IF NOT EXISTS orders (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    order_type TEXT NOT NULL,
    price DOUBLE PRECISION,
    size DOUBLE PRECISION NOT NULL,
    status TEXT NOT NULL,
    client_order_id TEXT UNIQUE NOT NULL,
    venue_order_id TEXT,
    time_in_force TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create hypertable for orders
SELECT create_hypertable('orders', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Indexes for orders
CREATE INDEX IF NOT EXISTS idx_orders_market_time
    ON orders (market, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_orders_status_time
    ON orders (status, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_orders_venue
    ON orders (venue, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_orders_client_order_id
    ON orders (client_order_id);

CREATE INDEX IF NOT EXISTS idx_orders_venue_order_id
    ON orders (venue_order_id);

-- Fills table
CREATE TABLE IF NOT EXISTS fills (
    id UUID PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    order_id UUID NOT NULL,
    venue TEXT NOT NULL,
    market TEXT NOT NULL,
    side TEXT NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    fee DOUBLE PRECISION NOT NULL,
    fee_currency TEXT NOT NULL,
    trade_id TEXT,
    liquidity TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create hypertable for fills
SELECT create_hypertable('fills', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Indexes for fills
CREATE INDEX IF NOT EXISTS idx_fills_order_id
    ON fills (order_id);

CREATE INDEX IF NOT EXISTS idx_fills_market_time
    ON fills (market, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_fills_venue
    ON fills (venue, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_fills_trade_id
    ON fills (trade_id);

-- Positions table (snapshots)
CREATE TABLE IF NOT EXISTS positions (
    timestamp TIMESTAMPTZ NOT NULL,
    market TEXT NOT NULL,
    venue TEXT NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    avg_entry_price DOUBLE PRECISION NOT NULL,
    unrealized_pnl DOUBLE PRECISION,
    realized_pnl DOUBLE PRECISION,
    mark_price DOUBLE PRECISION,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create hypertable for positions
SELECT create_hypertable('positions', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Indexes for positions
CREATE INDEX IF NOT EXISTS idx_positions_market_time
    ON positions (market, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_positions_venue_time
    ON positions (venue, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_positions_venue_market
    ON positions (venue, market, timestamp DESC);

-- Compression policies for execution data
ALTER TABLE orders SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'venue, market, status',
    timescaledb.compress_orderby = 'timestamp DESC'
);

ALTER TABLE fills SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'venue, market',
    timescaledb.compress_orderby = 'timestamp DESC'
);

ALTER TABLE positions SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'venue, market',
    timescaledb.compress_orderby = 'timestamp DESC'
);

-- Compress after 30 days
SELECT add_compression_policy('orders', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('fills', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('positions', INTERVAL '30 days', if_not_exists => TRUE);

-- Retention policies (365 days for compliance/audit)
SELECT add_retention_policy('orders', INTERVAL '365 days', if_not_exists => TRUE);
SELECT add_retention_policy('fills', INTERVAL '365 days', if_not_exists => TRUE);
SELECT add_retention_policy('positions', INTERVAL '365 days', if_not_exists => TRUE);

-- Continuous aggregate for daily order statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS orders_daily_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS bucket,
    venue,
    market,
    status,
    COUNT(*) AS order_count,
    SUM(CASE WHEN side = 'buy' THEN 1 ELSE 0 END) AS buy_count,
    SUM(CASE WHEN side = 'sell' THEN 1 ELSE 0 END) AS sell_count,
    AVG(size) AS avg_size,
    SUM(size) AS total_size
FROM orders
GROUP BY bucket, venue, market, status
WITH NO DATA;

SELECT add_continuous_aggregate_policy('orders_daily_stats',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Continuous aggregate for daily fill statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS fills_daily_stats
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS bucket,
    venue,
    market,
    COUNT(*) AS fill_count,
    SUM(size) AS total_volume,
    SUM(size * price) AS total_notional,
    SUM(fee) AS total_fees,
    AVG(price) AS vwap,
    MIN(price) AS min_price,
    MAX(price) AS max_price
FROM fills
GROUP BY bucket, venue, market
WITH NO DATA;

SELECT add_continuous_aggregate_policy('fills_daily_stats',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- View for active orders
CREATE OR REPLACE VIEW active_orders AS
SELECT
    id,
    timestamp,
    venue,
    market,
    side,
    order_type,
    price,
    size,
    status,
    client_order_id,
    venue_order_id
FROM orders
WHERE status IN ('open', 'partial')
ORDER BY timestamp DESC;

-- View for latest positions per market
CREATE OR REPLACE VIEW latest_positions AS
SELECT DISTINCT ON (venue, market)
    timestamp,
    market,
    venue,
    size,
    avg_entry_price,
    unrealized_pnl,
    realized_pnl,
    mark_price
FROM positions
ORDER BY venue, market, timestamp DESC;

-- Trigger to update updated_at on orders
CREATE OR REPLACE FUNCTION update_orders_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_orders_updated_at
BEFORE UPDATE ON orders
FOR EACH ROW
EXECUTE FUNCTION update_orders_updated_at();

-- Comments for documentation
COMMENT ON TABLE orders IS 'Order placement history with full lifecycle tracking';
COMMENT ON TABLE fills IS 'Execution fills/trades history';
COMMENT ON TABLE positions IS 'Position snapshots over time for PnL tracking';

COMMENT ON COLUMN orders.status IS 'Order status: open, partial, filled, cancelled, rejected';
COMMENT ON COLUMN fills.liquidity IS 'Liquidity type: maker, taker';
COMMENT ON COLUMN positions.unrealized_pnl IS 'Unrealized PnL based on current mark price';
COMMENT ON COLUMN positions.realized_pnl IS 'Cumulative realized PnL from closed positions';

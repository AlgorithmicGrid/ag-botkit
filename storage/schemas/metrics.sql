-- TimescaleDB Metrics Schema
-- Hypertable for time-series metrics from ag-botkit monitoring

-- Metrics hypertable
CREATE TABLE IF NOT EXISTS metrics (
    timestamp TIMESTAMPTZ NOT NULL,
    metric_name TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    labels JSONB DEFAULT '{}'::jsonb
);

-- Create hypertable partitioned by time (1 day chunks)
SELECT create_hypertable('metrics', 'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_metrics_name_time
    ON metrics (metric_name, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_metrics_labels
    ON metrics USING GIN (labels);

CREATE INDEX IF NOT EXISTS idx_metrics_time
    ON metrics (timestamp DESC);

-- Compression policy (compress data older than 7 days)
-- This reduces storage size by ~90% for historical data
ALTER TABLE metrics SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'metric_name',
    timescaledb.compress_orderby = 'timestamp DESC'
);

SELECT add_compression_policy('metrics', INTERVAL '7 days', if_not_exists => TRUE);

-- Retention policy (drop data older than 90 days)
-- Automatically removes old data to manage storage
SELECT add_retention_policy('metrics', INTERVAL '90 days', if_not_exists => TRUE);

-- Continuous aggregate for hourly metrics
-- Pre-aggregates data for faster queries
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', timestamp) AS bucket,
    metric_name,
    labels,
    AVG(value) AS avg_value,
    MIN(value) AS min_value,
    MAX(value) AS max_value,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY value) AS median_value,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY value) AS p95_value,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY value) AS p99_value,
    STDDEV(value) AS stddev_value,
    COUNT(*) AS count
FROM metrics
GROUP BY bucket, metric_name, labels
WITH NO DATA;

-- Refresh policy for continuous aggregate
-- Updates the materialized view every hour
SELECT add_continuous_aggregate_policy('metrics_hourly',
    start_offset => INTERVAL '3 hours',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists => TRUE
);

-- Continuous aggregate for daily metrics
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_daily
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 day', timestamp) AS bucket,
    metric_name,
    labels,
    AVG(value) AS avg_value,
    MIN(value) AS min_value,
    MAX(value) AS max_value,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY value) AS median_value,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY value) AS p95_value,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY value) AS p99_value,
    STDDEV(value) AS stddev_value,
    COUNT(*) AS count
FROM metrics
GROUP BY bucket, metric_name, labels
WITH NO DATA;

-- Refresh policy for daily aggregate
SELECT add_continuous_aggregate_policy('metrics_daily',
    start_offset => INTERVAL '7 days',
    end_offset => INTERVAL '1 day',
    schedule_interval => INTERVAL '1 day',
    if_not_exists => TRUE
);

-- Compression for continuous aggregates
ALTER MATERIALIZED VIEW metrics_hourly SET (
    timescaledb.compress = true
);

SELECT add_compression_policy('metrics_hourly', INTERVAL '30 days', if_not_exists => TRUE);

-- Helper view for latest metrics
CREATE OR REPLACE VIEW latest_metrics AS
SELECT DISTINCT ON (metric_name, labels)
    timestamp,
    metric_name,
    value,
    labels
FROM metrics
ORDER BY metric_name, labels, timestamp DESC;

-- Comment documentation
COMMENT ON TABLE metrics IS 'Time-series metrics from ag-botkit monitoring system';
COMMENT ON COLUMN metrics.timestamp IS 'Metric timestamp in UTC';
COMMENT ON COLUMN metrics.metric_name IS 'Metric identifier (e.g., polymarket.rtds.lag_ms)';
COMMENT ON COLUMN metrics.value IS 'Metric value';
COMMENT ON COLUMN metrics.labels IS 'JSONB key-value pairs for metric dimensions';

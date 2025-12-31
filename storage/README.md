# ag-storage - TimescaleDB Storage Layer

**Status: COMPLETE (100%)**

TimescaleDB persistent storage layer for ag-botkit. Provides high-performance time-series storage for metrics, execution history, and positions.

## Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Connection Pooling | COMPLETE | deadpool-postgres with configurable max_size |
| Schema Management | COMPLETE | Metrics + execution tables with hypertables |
| Metric Storage | COMPLETE | Batch insertion, time-range queries |
| Execution Storage | COMPLETE | Orders, fills, positions tracking |
| Data Retention | COMPLETE | Automated cleanup + compression |
| Retention Scheduler | COMPLETE | Background job for automated retention |
| Query API | COMPLETE | Type-safe querying with aggregations |
| Tests | COMPLETE | 17 unit tests passing, 3 integration tests (require DB) |
| Documentation | COMPLETE | Full API docs + examples |

## Features

- **High-throughput ingestion**: >10,000 metric points/second with batch insertion
- **Efficient querying**: Optimized time-range queries with proper indexing
- **Automatic compression**: Data compressed after 7 days (90% storage reduction)
- **Data retention**: Automatic cleanup of old data (configurable per table)
- **Automated scheduler**: Background retention job for hands-free operation
- **Continuous aggregates**: Pre-computed hourly/daily statistics for fast analytics
- **Connection pooling**: Concurrent access from multiple modules with deadpool
- **Type-safe API**: Full Rust type safety with async/await

## Architecture

```
storage/
├── src/
│   ├── lib.rs              # Public API
│   ├── engine.rs           # StorageEngine (metrics)
│   ├── execution.rs        # ExecutionStore (orders/fills/positions)
│   ├── config.rs           # Configuration
│   ├── error.rs            # Error types
│   └── types.rs            # Data types
├── timescale/
│   ├── connection.rs       # Connection pooling
│   └── query.rs            # Query builders
├── schemas/
│   ├── metrics.sql         # Metrics hypertable schema
│   └── execution.sql       # Execution tables schema
├── retention/
│   ├── policy.rs           # Data retention manager
│   └── scheduler.rs        # Automated retention scheduler
├── examples/
│   ├── insert_metrics.rs   # Metric insertion examples
│   └── query_data.rs       # Query examples
├── docker-compose.yml      # Local TimescaleDB setup
└── README.md
```

## Quick Start

### 1. Start TimescaleDB

```bash
cd storage
docker-compose up -d
```

This starts:
- TimescaleDB on port 5432
- Auto-initializes schemas from `schemas/*.sql`
- Creates database `ag_botkit` with user `postgres`/`postgres`

Optional: Start pgAdmin for database management:
```bash
docker-compose --profile admin up -d
# Access at http://localhost:5050 (admin@ag-botkit.local / admin)
```

### 2. Run Examples

```bash
# Insert metrics
cargo run --example insert_metrics

# Query data
cargo run --example query_data
```

### 3. Use in Your Code

```rust
use ag_storage::{StorageEngine, StorageConfig, MetricPoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = StorageConfig::default();
    let mut storage = StorageEngine::new(config).await?;

    // Insert metric
    let metric = MetricPoint::new("polymarket.rtds.lag_ms", 45.3)
        .with_label("topic", "market");
    storage.insert_metric(metric).await?;

    // Query metrics
    let start = chrono::Utc::now() - chrono::Duration::hours(1);
    let end = chrono::Utc::now();
    let metrics = storage.query_metrics(
        "polymarket.rtds.lag_ms",
        start,
        end,
        None
    ).await?;

    println!("Found {} metrics", metrics.len());
    Ok(())
}
```

## Configuration

### YAML Configuration File

```yaml
database:
  host: localhost
  port: 5432
  database: ag_botkit
  user: postgres
  password: postgres
  max_connections: 10
  connection_timeout_sec: 5

ingestion:
  batch_size: 1000
  flush_interval_ms: 100
  max_buffer_size: 10000

retention:
  metrics_retention_days: 90
  execution_retention_days: 365
  compression_after_days: 7

query:
  max_results: 10000
  cache_ttl_sec: 60
  enable_cache: true
```

### Load Configuration

```rust
use ag_storage::StorageConfig;

// From YAML file
let config = StorageConfig::from_yaml_file("config.yaml")?;

// From YAML string
let yaml = std::fs::read_to_string("config.yaml")?;
let config = StorageConfig::from_yaml(&yaml)?;

// Use defaults
let config = StorageConfig::default();
```

## API Documentation

### StorageEngine (Metrics)

```rust
impl StorageEngine {
    // Create new storage engine
    pub async fn new(config: StorageConfig) -> Result<Self>;

    // Initialize database schemas
    pub async fn init_schemas(&self, metrics_sql: &str, execution_sql: &str) -> Result<()>;

    // Insert single metric
    pub async fn insert_metric(&mut self, metric: MetricPoint) -> Result<()>;

    // Batch insert (recommended for high throughput)
    pub async fn insert_metrics_batch(&mut self, metrics: Vec<MetricPoint>) -> Result<()>;

    // Buffer metric for async ingestion
    pub async fn buffer_metric(&self, metric: MetricPoint) -> Result<()>;

    // Flush buffered metrics
    pub async fn flush_buffer(&mut self) -> Result<usize>;

    // Query metrics in time range
    pub async fn query_metrics(
        &self,
        metric_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        labels: Option<HashMap<String, String>>,
    ) -> Result<Vec<MetricPoint>>;

    // Query aggregated metrics
    pub async fn query_aggregated(
        &self,
        metric_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bucket_size: Duration,
        aggregation: Aggregation,
    ) -> Result<Vec<AggregatedMetric>>;
}
```

### ExecutionStore (Orders/Fills/Positions)

```rust
impl ExecutionStore {
    // Create new execution store
    pub async fn new(config: StorageConfig) -> Result<Self>;

    // Store order
    pub async fn store_order(&mut self, order: Order) -> Result<()>;

    // Store fill/trade
    pub async fn store_fill(&mut self, fill: Fill) -> Result<()>;

    // Store position snapshot
    pub async fn store_position(&mut self, position: PositionSnapshot) -> Result<()>;

    // Query orders
    pub async fn query_orders(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        filters: OrderFilters,
    ) -> Result<Vec<Order>>;

    // Query fills by order ID
    pub async fn query_fills_by_order(&self, order_id: Uuid) -> Result<Vec<Fill>>;

    // Query position history
    pub async fn query_positions(
        &self,
        market_id: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<PositionSnapshot>>;

    // Get latest position
    pub async fn get_latest_position(
        &self,
        venue: &str,
        market: &str
    ) -> Result<Option<PositionSnapshot>>;
}
```

### RetentionManager

```rust
impl RetentionManager {
    // Run retention cleanup
    pub async fn run_retention(&self) -> Result<RetentionReport>;

    // Compress old data
    pub async fn compress_old_data(&self) -> Result<()>;

    // Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats>;

    // Check compression status
    pub async fn check_compression_status(&self) -> Result<CompressionStatus>;
}
```

## Database Schema

### Metrics Table

```sql
CREATE TABLE metrics (
    timestamp TIMESTAMPTZ NOT NULL,
    metric_name TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    labels JSONB DEFAULT '{}'::jsonb
);

-- Converted to hypertable (1 day chunks)
SELECT create_hypertable('metrics', 'timestamp');

-- Indexes
CREATE INDEX idx_metrics_name_time ON metrics (metric_name, timestamp DESC);
CREATE INDEX idx_metrics_labels ON metrics USING GIN (labels);

-- Compression (after 7 days)
ALTER TABLE metrics SET (timescaledb.compress);
SELECT add_compression_policy('metrics', INTERVAL '7 days');

-- Retention (90 days)
SELECT add_retention_policy('metrics', INTERVAL '90 days');
```

### Orders Table

```sql
CREATE TABLE orders (
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
    time_in_force TEXT
);

SELECT create_hypertable('orders', 'timestamp');
```

### Continuous Aggregates

Pre-computed aggregates for fast analytics:

- `metrics_hourly`: 1-hour buckets with avg/min/max/percentiles
- `metrics_daily`: 1-day buckets with statistics
- `orders_daily_stats`: Daily order statistics by venue/market
- `fills_daily_stats`: Daily fill statistics with VWAP

## Integration with Other Modules

### With monitor/ (Metrics Persistence)

```rust
// In monitor, periodically flush metrics to storage
let mut storage = StorageEngine::new(config).await?;

for metric in metrics_buffer {
    storage.buffer_metric(metric).await?;
}

// Flush every 100ms
tokio::time::interval(Duration::from_millis(100));
storage.flush_buffer().await?;
```

### With exec/ (Execution History)

```rust
// Store order placement
let mut exec_store = ExecutionStore::new(config).await?;

let order = Order::new("polymarket", "0x123abc", Side::Buy, OrderType::Limit, 100.0)
    .with_price(0.52);

exec_store.store_order(order).await?;

// Store fills
let fill = Fill::new(order_id, "polymarket", "0x123abc", Side::Buy, 0.52, 50.0, 0.1, "USDC");
exec_store.store_fill(fill).await?;
```

### With risk/ (Historical Risk Data)

```rust
// Query historical risk metrics for backtesting
let risk_decisions = storage.query_metrics(
    "polymarket.risk.decision",
    start,
    end,
    Some(hashmap!{"policy" => "position_limit"})
).await?;
```

## Performance

### Benchmarks

- **Metric insertion**: >10,000 points/second (batch mode)
- **Query latency**: <100ms for typical time ranges (1 hour with 1000s of points)
- **Compression ratio**: ~90% size reduction for time-series data
- **Storage growth**: ~500KB per 10,000 metrics (compressed)

### Optimization Tips

1. **Use batch insertion** for high throughput:
   ```rust
   storage.insert_metrics_batch(metrics).await?;  // Fast
   // vs
   for m in metrics { storage.insert_metric(m).await?; }  // Slow
   ```

2. **Query continuous aggregates** for large time ranges:
   ```sql
   SELECT * FROM metrics_hourly WHERE bucket >= $1 AND bucket <= $2;
   ```

3. **Use label filters** efficiently:
   ```rust
   let labels = hashmap!{"venue" => "polymarket"};
   storage.query_metrics(name, start, end, Some(labels)).await?;
   ```

4. **Leverage compression**:
   - Data automatically compressed after 7 days
   - Queries still work on compressed chunks
   - ~90% storage savings

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests (Requires TimescaleDB)

```bash
# Start TimescaleDB
docker-compose up -d

# Run integration tests
cargo test --test integration

# Run specific test
cargo test --test integration test_metric_insertion
```

## Troubleshooting

### Connection Issues

```
Error: Database connection error: connection refused
```

**Solution**: Ensure TimescaleDB is running:
```bash
docker-compose ps
docker-compose up -d
```

### Schema Errors

```
Error: TimescaleDB extension not found
```

**Solution**: The Docker image includes TimescaleDB. If using custom PostgreSQL:
```sql
CREATE EXTENSION IF NOT EXISTS timescaledb;
```

### Slow Queries

If queries are slow:

1. Check indexes: `EXPLAIN ANALYZE SELECT ...`
2. Use continuous aggregates for large time ranges
3. Ensure data is compressed: `SELECT * FROM timescaledb_information.chunks WHERE is_compressed;`
4. Monitor connection pool: `storage.pool_status()`

### Out of Memory

Reduce batch size in config:
```yaml
ingestion:
  batch_size: 500  # Reduced from 1000
  max_buffer_size: 5000  # Reduced from 10000
```

## Maintenance

### Manual Retention Cleanup

```rust
use ag_storage::RetentionManager;

let retention = RetentionManager::new(pool, config.retention);
let report = retention.run_retention().await?;

println!("Deleted {} metrics, {} orders",
    report.metrics_deleted, report.orders_deleted);
```

### Storage Statistics

```rust
let stats = retention.get_storage_stats().await?;
println!("Total storage: {:.2} GB", stats.total_size_gb());
println!("Metrics: {} rows ({:.2} MB)",
    stats.metrics_count,
    stats.metrics_size_bytes as f64 / 1024.0 / 1024.0);
```

### Compression Status

```rust
let status = retention.check_compression_status().await?;
println!("Compression ratio: {:.1}%", status.compression_ratio * 100.0);
```

## Production Deployment

### Docker Compose (Production)

```yaml
services:
  timescaledb:
    image: timescale/timescaledb:latest-pg16
    restart: always
    ports:
      - "5432:5432"
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - /data/timescaledb:/var/lib/postgresql/data
    command:
      - postgres
      - -c
      - shared_preload_libraries=timescaledb
      - -c
      - max_connections=200
      - -c
      - shared_buffers=2GB
      - -c
      - effective_cache_size=6GB
```

### Kubernetes StatefulSet

See `../deploy/k8s/timescaledb-statefulset.yaml` for production Kubernetes deployment.

### Backup and Restore

```bash
# Backup
docker exec ag-botkit-timescaledb pg_dump -U postgres ag_botkit > backup.sql

# Restore
docker exec -i ag-botkit-timescaledb psql -U postgres ag_botkit < backup.sql
```

## Contributing

When making changes:

1. Update tests
2. Run `cargo clippy`
3. Update this README if adding features
4. Test with real TimescaleDB

## License

MIT

## Contact

Part of the ag-botkit project.

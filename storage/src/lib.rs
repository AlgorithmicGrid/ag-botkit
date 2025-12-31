//! TimescaleDB persistent storage layer for ag-botkit
//!
//! This module provides persistent storage for metrics, execution history, and positions
//! using TimescaleDB (PostgreSQL with time-series extensions).
//!
//! # Features
//!
//! - High-throughput metric ingestion (>10,000 points/second)
//! - Efficient time-range queries with indexes
//! - Automatic data compression and retention policies
//! - Continuous aggregates for downsampling
//! - Execution history (orders, fills, positions)
//! - Connection pooling for concurrent access
//!
//! # Example
//!
//! ```no_run
//! use ag_storage::{StorageEngine, StorageConfig, MetricPoint};
//! use chrono::Utc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = StorageConfig::default();
//!     let mut storage = StorageEngine::new(config).await?;
//!
//!     // Insert a metric
//!     let metric = MetricPoint::new("polymarket.rtds.lag_ms", 45.3)
//!         .with_label("topic", "market");
//!
//!     storage.insert_metric(metric).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod engine;
pub mod error;
pub mod execution;
pub mod types;

// Include timescale module from parent directory
#[path = "../timescale/mod.rs"]
pub mod timescale_impl;
pub mod timescale {
    pub use super::timescale_impl::*;
}

// Include retention module from parent directory
#[path = "../retention/mod.rs"]
pub mod retention_impl;
pub mod retention {
    pub use super::retention_impl::*;
}

// Re-export main types
pub use config::{
    DatabaseConfig, IngestionConfig, QueryConfig, RetentionConfig, StorageConfig,
};
pub use engine::StorageEngine;
pub use error::{Result, StorageError};
pub use execution::ExecutionStore;
pub use timescale::{ConnectionPool, PoolStatus, QueryBuilder};
pub use types::{
    AggregatedMetric, Aggregation, Fill, MetricPoint, Order, OrderFilters, OrderStatus,
    OrderType, PositionSnapshot, RetentionReport, Side,
};

// Re-export retention types
pub use retention::{CompressionStatus, RetentionManager, RetentionScheduler, StorageStats};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize tracing subscriber (for examples and tests)
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ag_storage=info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = StorageConfig::default();
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
    }
}

use crate::config::RetentionConfig;
use crate::error::{Result, StorageError};
use crate::timescale::ConnectionPool;
use crate::types::RetentionReport;
use chrono::{Duration, Utc};
use std::sync::Arc;
use tracing::{info, warn};

/// Data retention manager
pub struct RetentionManager {
    pool: Arc<ConnectionPool>,
    config: RetentionConfig,
}

impl RetentionManager {
    /// Create new retention manager
    pub fn new(pool: Arc<ConnectionPool>, config: RetentionConfig) -> Self {
        Self { pool, config }
    }

    /// Run data retention cleanup
    pub async fn run_retention(&self) -> Result<RetentionReport> {
        info!("Running data retention cleanup");

        let start_time = Utc::now();

        // Calculate cutoff timestamps
        let metrics_cutoff = Utc::now() - Duration::days(self.config.metrics_retention_days as i64);
        let execution_cutoff =
            Utc::now() - Duration::days(self.config.execution_retention_days as i64);

        let client = self.pool.get().await?;

        // Clean up old metrics
        let metrics_deleted = client
            .execute(
                "DELETE FROM metrics WHERE timestamp < $1",
                &[&metrics_cutoff],
            )
            .await
            .map_err(|e| StorageError::RetentionError(e.to_string()))?;

        info!("Deleted {} old metric records", metrics_deleted);

        // Clean up old orders
        let orders_deleted = client
            .execute(
                "DELETE FROM orders WHERE timestamp < $1",
                &[&execution_cutoff],
            )
            .await
            .map_err(|e| StorageError::RetentionError(e.to_string()))?;

        info!("Deleted {} old order records", orders_deleted);

        // Clean up old fills
        let fills_deleted = client
            .execute(
                "DELETE FROM fills WHERE timestamp < $1",
                &[&execution_cutoff],
            )
            .await
            .map_err(|e| StorageError::RetentionError(e.to_string()))?;

        info!("Deleted {} old fill records", fills_deleted);

        // Clean up old positions
        let positions_deleted = client
            .execute(
                "DELETE FROM positions WHERE timestamp < $1",
                &[&execution_cutoff],
            )
            .await
            .map_err(|e| StorageError::RetentionError(e.to_string()))?;

        info!("Deleted {} old position records", positions_deleted);

        let end_time = Utc::now();
        let duration = end_time - start_time;

        let report = RetentionReport {
            metrics_deleted: metrics_deleted as i64,
            orders_deleted: orders_deleted as i64,
            fills_deleted: fills_deleted as i64,
            positions_deleted: positions_deleted as i64,
            executed_at: end_time,
            duration,
        };

        info!(
            "Retention cleanup complete: {} total records deleted in {:?}",
            report.metrics_deleted
                + report.orders_deleted
                + report.fills_deleted
                + report.positions_deleted,
            duration
        );

        Ok(report)
    }

    /// Manually compress chunks older than threshold
    pub async fn compress_old_data(&self) -> Result<()> {
        info!("Running manual data compression");

        let compression_cutoff =
            Utc::now() - Duration::days(self.config.compression_after_days as i64);

        let client = self.pool.get().await?;

        // Compress metrics chunks
        let metrics_compressed = client
            .execute(
                r#"
                SELECT compress_chunk(i)
                FROM show_chunks('metrics', older_than => $1::timestamptz) i
                "#,
                &[&compression_cutoff],
            )
            .await
            .map_err(|e| StorageError::RetentionError(e.to_string()))?;

        info!("Compressed {} metrics chunks", metrics_compressed);

        // Compress orders chunks
        let orders_compressed = client
            .execute(
                r#"
                SELECT compress_chunk(i)
                FROM show_chunks('orders', older_than => $1::timestamptz) i
                "#,
                &[&compression_cutoff],
            )
            .await
            .map_err(|e| StorageError::RetentionError(e.to_string()))?;

        info!("Compressed {} orders chunks", orders_compressed);

        Ok(())
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let client = self.pool.get().await?;

        // Get table sizes
        let metrics_size: i64 = client
            .query_one(
                "SELECT pg_total_relation_size('metrics')",
                &[],
            )
            .await?
            .get(0);

        let orders_size: i64 = client
            .query_one(
                "SELECT pg_total_relation_size('orders')",
                &[],
            )
            .await?
            .get(0);

        let fills_size: i64 = client
            .query_one(
                "SELECT pg_total_relation_size('fills')",
                &[],
            )
            .await?
            .get(0);

        let positions_size: i64 = client
            .query_one(
                "SELECT pg_total_relation_size('positions')",
                &[],
            )
            .await?
            .get(0);

        // Get row counts
        let metrics_count: i64 = client
            .query_one("SELECT COUNT(*) FROM metrics", &[])
            .await?
            .get(0);

        let orders_count: i64 = client
            .query_one("SELECT COUNT(*) FROM orders", &[])
            .await?
            .get(0);

        let fills_count: i64 = client
            .query_one("SELECT COUNT(*) FROM fills", &[])
            .await?
            .get(0);

        let positions_count: i64 = client
            .query_one("SELECT COUNT(*) FROM positions", &[])
            .await?
            .get(0);

        Ok(StorageStats {
            metrics_size_bytes: metrics_size,
            orders_size_bytes: orders_size,
            fills_size_bytes: fills_size,
            positions_size_bytes: positions_size,
            total_size_bytes: metrics_size + orders_size + fills_size + positions_size,
            metrics_count,
            orders_count,
            fills_count,
            positions_count,
        })
    }

    /// Check compression status
    pub async fn check_compression_status(&self) -> Result<CompressionStatus> {
        let client = self.pool.get().await?;

        // Check compressed chunks for metrics
        let metrics_compressed: i64 = client
            .query_one(
                r#"
                SELECT COUNT(*)
                FROM timescaledb_information.chunks
                WHERE hypertable_name = 'metrics' AND is_compressed = true
                "#,
                &[],
            )
            .await?
            .get(0);

        let metrics_total: i64 = client
            .query_one(
                r#"
                SELECT COUNT(*)
                FROM timescaledb_information.chunks
                WHERE hypertable_name = 'metrics'
                "#,
                &[],
            )
            .await?
            .get(0);

        Ok(CompressionStatus {
            metrics_compressed_chunks: metrics_compressed,
            metrics_total_chunks: metrics_total,
            compression_ratio: if metrics_total > 0 {
                metrics_compressed as f64 / metrics_total as f64
            } else {
                0.0
            },
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub metrics_size_bytes: i64,
    pub orders_size_bytes: i64,
    pub fills_size_bytes: i64,
    pub positions_size_bytes: i64,
    pub total_size_bytes: i64,
    pub metrics_count: i64,
    pub orders_count: i64,
    pub fills_count: i64,
    pub positions_count: i64,
}

impl StorageStats {
    /// Get total size in MB
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Get total size in GB
    pub fn total_size_gb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Compression status
#[derive(Debug, Clone)]
pub struct CompressionStatus {
    pub metrics_compressed_chunks: i64,
    pub metrics_total_chunks: i64,
    pub compression_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_stats_conversion() {
        let stats = StorageStats {
            metrics_size_bytes: 1024 * 1024 * 100, // 100 MB
            orders_size_bytes: 1024 * 1024 * 50,   // 50 MB
            fills_size_bytes: 1024 * 1024 * 25,    // 25 MB
            positions_size_bytes: 1024 * 1024 * 25, // 25 MB
            total_size_bytes: 1024 * 1024 * 200,   // 200 MB
            metrics_count: 1000000,
            orders_count: 50000,
            fills_count: 75000,
            positions_count: 10000,
        };

        assert_eq!(stats.total_size_mb(), 200.0);
        assert!((stats.total_size_gb() - 0.1953).abs() < 0.01);
    }
}

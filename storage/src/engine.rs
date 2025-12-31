use crate::config::StorageConfig;
use crate::error::{Result, StorageError};
use crate::timescale::ConnectionPool;
use crate::types::{AggregatedMetric, Aggregation, MetricPoint};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Storage engine for time-series metrics
pub struct StorageEngine {
    pool: Arc<ConnectionPool>,
    config: StorageConfig,
    buffer: Arc<RwLock<Vec<MetricPoint>>>,
}

impl StorageEngine {
    /// Create new storage engine with database connection
    pub async fn new(config: StorageConfig) -> Result<Self> {
        info!("Initializing StorageEngine");

        let pool = ConnectionPool::new(&config.database).await?;
        let pool = Arc::new(pool);

        let buffer = Arc::new(RwLock::new(Vec::with_capacity(
            config.ingestion.max_buffer_size,
        )));

        Ok(Self {
            pool,
            config,
            buffer,
        })
    }

    /// Initialize database schemas
    pub async fn init_schemas(&self, metrics_sql: &str, execution_sql: &str) -> Result<()> {
        self.pool.init_schemas(metrics_sql, execution_sql).await
    }

    /// Insert single metric point
    pub async fn insert_metric(&mut self, metric: MetricPoint) -> Result<()> {
        debug!("Inserting metric: {}", metric.metric_name);

        let client = self.pool.get().await?;

        let labels_json = serde_json::to_value(&metric.labels)?;

        client
            .execute(
                "INSERT INTO metrics (timestamp, metric_name, value, labels) VALUES ($1, $2, $3, $4)",
                &[
                    &metric.timestamp,
                    &metric.metric_name,
                    &metric.value,
                    &labels_json,
                ],
            )
            .await?;

        Ok(())
    }

    /// Batch insert metrics (more efficient)
    pub async fn insert_metrics_batch(&mut self, metrics: Vec<MetricPoint>) -> Result<()> {
        if metrics.is_empty() {
            return Ok(());
        }

        debug!("Batch inserting {} metrics", metrics.len());

        let client = self.pool.get().await?;

        // Use COPY for high-performance bulk insert
        let stmt = "COPY metrics (timestamp, metric_name, value, labels) FROM STDIN BINARY";

        let sink = client
            .copy_in(stmt)
            .await
            .map_err(|e| StorageError::QueryError(e.to_string()))?;

        // Prepare binary data
        let writer = tokio_postgres::binary_copy::BinaryCopyInWriter::new(
            sink,
            &[
                tokio_postgres::types::Type::TIMESTAMPTZ,
                tokio_postgres::types::Type::TEXT,
                tokio_postgres::types::Type::FLOAT8,
                tokio_postgres::types::Type::JSONB,
            ],
        );

        // This would require more complex binary encoding, so let's use multi-row INSERT instead
        drop(writer);

        // Build multi-row INSERT statement
        let mut query = String::from(
            "INSERT INTO metrics (timestamp, metric_name, value, labels) VALUES "
        );

        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_idx = 1;

        for (i, _metric) in metrics.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }

            query.push_str(&format!(
                "(${}, ${}, ${}, ${})",
                param_idx,
                param_idx + 1,
                param_idx + 2,
                param_idx + 3
            ));

            param_idx += 4;
        }

        // Execute with temporary lifetime extension
        let mut timestamps: Vec<DateTime<Utc>> = Vec::with_capacity(metrics.len());
        let mut names: Vec<String> = Vec::with_capacity(metrics.len());
        let mut values: Vec<f64> = Vec::with_capacity(metrics.len());
        let mut labels_json: Vec<serde_json::Value> = Vec::with_capacity(metrics.len());

        for metric in metrics {
            timestamps.push(metric.timestamp);
            names.push(metric.metric_name.clone());
            values.push(metric.value);
            labels_json.push(serde_json::to_value(&metric.labels)?);
        }

        // Build params vector
        for i in 0..timestamps.len() {
            params.push(&timestamps[i]);
            params.push(&names[i]);
            params.push(&values[i]);
            params.push(&labels_json[i]);
        }

        client.execute(&query, &params).await?;

        info!("Successfully inserted {} metrics", timestamps.len());

        Ok(())
    }

    /// Add metric to buffer (for async ingestion)
    pub async fn buffer_metric(&self, metric: MetricPoint) -> Result<()> {
        let mut buffer = self.buffer.write().await;

        if buffer.len() >= self.config.ingestion.max_buffer_size {
            warn!("Buffer full, dropping metric: {}", metric.metric_name);
            return Err(StorageError::Internal("Buffer full".to_string()));
        }

        buffer.push(metric);

        Ok(())
    }

    /// Flush buffered metrics to database
    pub async fn flush_buffer(&mut self) -> Result<usize> {
        let mut buffer = self.buffer.write().await;

        if buffer.is_empty() {
            return Ok(0);
        }

        let metrics = std::mem::take(&mut *buffer);
        let count = metrics.len();

        drop(buffer); // Release lock before async operation

        self.insert_metrics_batch(metrics).await?;

        Ok(count)
    }

    /// Query metrics in time range
    pub async fn query_metrics(
        &self,
        metric_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        labels: Option<HashMap<String, String>>,
    ) -> Result<Vec<MetricPoint>> {
        debug!(
            "Querying metrics: {} from {} to {}",
            metric_name, start, end
        );

        let client = self.pool.get().await?;

        // Build query manually with proper type handling
        let mut query = String::from(
            "SELECT timestamp, metric_name, value, labels FROM metrics WHERE timestamp >= $1 AND timestamp <= $2 AND metric_name = $3"
        );

        let mut param_idx = 4;
        let label_conditions = if let Some(ref labels) = labels {
            labels.keys().map(|key| {
                    let cond = format!(" AND labels->>'{}' = ${}", key, param_idx);
                    param_idx += 1;
                    cond
                })
                .collect::<Vec<_>>()
                .join("")
        } else {
            String::new()
        };

        query.push_str(&label_conditions);
        query.push_str(&format!(" ORDER BY timestamp ASC LIMIT {}", self.config.query.max_results));

        // Build parameters vector
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = vec![&start, &end, &metric_name];

        let label_values: Vec<String> = if let Some(ref labels) = labels {
            labels.values().cloned().collect()
        } else {
            Vec::new()
        };

        for label_val in &label_values {
            params.push(label_val);
        }

        let rows = client.query(&query, &params).await?;

        let metrics: Vec<MetricPoint> = rows
            .iter()
            .map(|row| {
                let labels_value: serde_json::Value = row.get(3);
                let labels: HashMap<String, String> =
                    serde_json::from_value(labels_value).unwrap_or_default();

                MetricPoint {
                    timestamp: row.get(0),
                    metric_name: row.get(1),
                    value: row.get(2),
                    labels,
                }
            })
            .collect();

        debug!("Found {} metrics", metrics.len());

        Ok(metrics)
    }

    /// Query aggregated metrics
    pub async fn query_aggregated(
        &self,
        metric_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        bucket_size: Duration,
        _aggregation: Aggregation,
    ) -> Result<Vec<AggregatedMetric>> {
        debug!(
            "Querying aggregated metrics: {} from {} to {}, bucket: {:?}",
            metric_name, start, end, bucket_size
        );

        let client = self.pool.get().await?;

        // Convert bucket_size to PostgreSQL interval
        let bucket_interval = format!("{} seconds", bucket_size.num_seconds());

        let query = r#"
            SELECT
                time_bucket($1, timestamp) AS bucket,
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
            WHERE timestamp >= $2 AND timestamp <= $3 AND metric_name = $4
            GROUP BY bucket, metric_name, labels
            ORDER BY bucket DESC
            LIMIT $5
            "#.to_string();

        let rows = client
            .query(
                &query,
                &[
                    &bucket_interval,
                    &start,
                    &end,
                    &metric_name,
                    &(self.config.query.max_results as i64),
                ],
            )
            .await?;

        let aggregated: Vec<AggregatedMetric> = rows
            .iter()
            .map(|row| {
                let labels_value: serde_json::Value = row.get(2);
                let labels: HashMap<String, String> =
                    serde_json::from_value(labels_value).unwrap_or_default();

                AggregatedMetric {
                    bucket: row.get(0),
                    metric_name: row.get(1),
                    labels,
                    avg_value: row.get(3),
                    min_value: row.get(4),
                    max_value: row.get(5),
                    median_value: row.get(6),
                    p95_value: row.get(7),
                    p99_value: row.get(8),
                    stddev_value: row.get(9),
                    count: row.get(10),
                }
            })
            .collect();

        debug!("Found {} aggregated buckets", aggregated.len());

        Ok(aggregated)
    }

    /// Get pool status
    pub fn pool_status(&self) -> String {
        self.pool.status().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running TimescaleDB
    async fn test_metric_buffering() {
        let config = StorageConfig::default();
        let mut engine = StorageEngine::new(config).await.unwrap();

        let metric = MetricPoint::new("test.metric", 42.0);
        engine.buffer_metric(metric).await.unwrap();

        let buffer = engine.buffer.read().await;
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_metric_point_builder() {
        let metric = MetricPoint::new("test.cpu", 75.5)
            .with_label("host", "server1")
            .with_label("region", "us-east");

        assert_eq!(metric.metric_name, "test.cpu");
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.labels.len(), 2);
    }
}

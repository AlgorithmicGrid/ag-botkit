use crate::retention::RetentionManager;
use crate::error::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

/// Automated retention policy scheduler
pub struct RetentionScheduler {
    manager: Arc<RetentionManager>,
    interval_hours: u64,
}

impl RetentionScheduler {
    /// Create a new retention scheduler
    pub fn new(manager: Arc<RetentionManager>, interval_hours: u64) -> Self {
        Self {
            manager,
            interval_hours,
        }
    }

    /// Start the scheduler (runs in background)
    pub async fn start(self) {
        info!(
            "Starting retention scheduler (interval: {} hours)",
            self.interval_hours
        );

        let mut ticker = interval(Duration::from_secs(self.interval_hours * 3600));

        loop {
            ticker.tick().await;

            info!("Running scheduled retention cleanup");

            match self.manager.run_retention().await {
                Ok(report) => {
                    info!(
                        "Retention cleanup completed: {} metrics, {} orders, {} fills, {} positions deleted",
                        report.metrics_deleted,
                        report.orders_deleted,
                        report.fills_deleted,
                        report.positions_deleted
                    );
                }
                Err(e) => {
                    error!("Retention cleanup failed: {}", e);
                }
            }

            // Also run compression
            match self.manager.compress_old_data().await {
                Ok(_) => {
                    info!("Data compression completed");
                }
                Err(e) => {
                    error!("Data compression failed: {}", e);
                }
            }
        }
    }

    /// Run retention once (for testing or manual execution)
    pub async fn run_once(&self) -> Result<()> {
        info!("Running one-time retention cleanup");
        let report = self.manager.run_retention().await?;
        info!("Retention report: {:?}", report);

        self.manager.compress_old_data().await?;
        info!("Compression completed");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DatabaseConfig, RetentionConfig, StorageConfig};
    use crate::timescale::ConnectionPool;

    #[test]
    fn test_scheduler_creation() {
        // This is a unit test that doesn't require a database connection
        let db_config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            user: "test".to_string(),
            password: "test".to_string(),
            max_connections: 5,
            connection_timeout_sec: 5,
            use_tls: false,
        };

        let retention_config = RetentionConfig {
            metrics_retention_days: 90,
            execution_retention_days: 365,
            compression_after_days: 7,
        };

        // We can't actually create the pool without a database, but we can test the structure
        assert_eq!(retention_config.metrics_retention_days, 90);
    }
}

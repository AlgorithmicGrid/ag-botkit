use crate::error::{Result, StorageError};
use crate::config::DatabaseConfig;
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;
use tracing::{debug, info};

/// Connection pool manager for TimescaleDB
pub struct ConnectionPool {
    pool: Pool,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!(
            "Creating connection pool to {}:{}/{} (max_connections: {})",
            config.host, config.port, config.database, config.max_connections
        );

        let mut pg_config = Config::new();
        pg_config.host = Some(config.host.clone());
        pg_config.port = Some(config.port);
        pg_config.dbname = Some(config.database.clone());
        pg_config.user = Some(config.user.clone());
        pg_config.password = Some(config.password.clone());

        pg_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        // Set pool size limits
        pg_config.pool = Some(deadpool_postgres::PoolConfig::new(config.max_connections));

        // Create pool with configured max connections
        let pool = pg_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        // Verify connection by getting a client
        let client = pool.get().await?;
        let version: String = client
            .query_one("SELECT version()", &[])
            .await
            .map(|row| row.get(0))?;

        info!("Connected to PostgreSQL: {}", version);

        // Check if TimescaleDB extension is available
        let has_timescale: bool = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'timescaledb')",
                &[],
            )
            .await
            .map(|row| row.get(0))?;

        if has_timescale {
            let ts_version: String = client
                .query_one("SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'", &[])
                .await
                .map(|row| row.get(0))?;
            info!("TimescaleDB extension version: {}", ts_version);
        } else {
            return Err(StorageError::SchemaError(
                "TimescaleDB extension not found. Please install TimescaleDB.".to_string(),
            ));
        }

        debug!("Connection pool created successfully");

        Ok(Self { pool })
    }

    /// Get a connection from the pool
    pub async fn get(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.map_err(|e| e.into())
    }

    /// Get pool status
    pub fn status(&self) -> PoolStatus {
        let status = self.pool.status();
        PoolStatus {
            size: status.size,
            available: status.available,
            waiting: status.waiting,
            max_size: status.max_size,
        }
    }

    /// Execute a schema migration script
    pub async fn execute_schema(&self, sql: &str) -> Result<()> {
        let mut client = self.get().await?;

        // Execute in a transaction
        let transaction = client
            .transaction()
            .await
            .map_err(|e| StorageError::SchemaError(e.to_string()))?;

        transaction
            .batch_execute(sql)
            .await
            .map_err(|e| StorageError::SchemaError(e.to_string()))?;

        transaction
            .commit()
            .await
            .map_err(|e| StorageError::SchemaError(e.to_string()))?;

        info!("Schema executed successfully");
        Ok(())
    }

    /// Initialize database schemas
    pub async fn init_schemas(&self, metrics_sql: &str, execution_sql: &str) -> Result<()> {
        info!("Initializing database schemas");

        // Execute metrics schema
        self.execute_schema(metrics_sql).await?;
        info!("Metrics schema initialized");

        // Execute execution schema
        self.execute_schema(execution_sql).await?;
        info!("Execution schema initialized");

        Ok(())
    }

    /// Test database connectivity
    pub async fn test_connection(&self) -> Result<bool> {
        let client = self.get().await?;
        let result: i32 = client.query_one("SELECT 1", &[]).await?.get(0);
        Ok(result == 1)
    }
}

/// Pool status information
#[derive(Debug, Clone)]
pub struct PoolStatus {
    /// Current pool size
    pub size: usize,
    /// Available connections
    pub available: usize,
    /// Waiting requests
    pub waiting: usize,
    /// Maximum pool size
    pub max_size: usize,
}

impl std::fmt::Display for PoolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool[size={}, available={}, waiting={}, max={}]",
            self.size, self.available, self.waiting, self.max_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running TimescaleDB
    async fn test_connection_pool() {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "ag_botkit".to_string(),
            user: "postgres".to_string(),
            password: "postgres".to_string(),
            max_connections: 5,
            connection_timeout_sec: 5,
            use_tls: false,
        };

        let pool = ConnectionPool::new(&config).await.unwrap();
        let status = pool.status();
        assert!(status.max_size > 0);
    }

    #[tokio::test]
    #[ignore] // Requires running TimescaleDB
    async fn test_connection() {
        let config = DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "ag_botkit".to_string(),
            user: "postgres".to_string(),
            password: "postgres".to_string(),
            max_connections: 5,
            connection_timeout_sec: 5,
            use_tls: false,
        };

        let pool = ConnectionPool::new(&config).await.unwrap();
        let result = pool.test_connection().await.unwrap();
        assert!(result);
    }
}

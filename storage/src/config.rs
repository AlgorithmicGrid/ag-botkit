use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Storage engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database connection configuration
    pub database: DatabaseConfig,

    /// Ingestion configuration
    pub ingestion: IngestionConfig,

    /// Retention configuration
    pub retention: RetentionConfig,

    /// Query configuration
    pub query: QueryConfig,
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database host
    pub host: String,

    /// Database port
    pub port: u16,

    /// Database name
    pub database: String,

    /// Database user
    pub user: String,

    /// Database password
    pub password: String,

    /// Maximum number of connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout_sec")]
    pub connection_timeout_sec: u64,

    /// Enable TLS/SSL
    #[serde(default)]
    pub use_tls: bool,
}

impl DatabaseConfig {
    /// Build PostgreSQL connection string
    pub fn connection_string(&self) -> String {
        format!(
            "host={} port={} dbname={} user={} password={} connect_timeout={}",
            self.host,
            self.port,
            self.database,
            self.user,
            self.password,
            self.connection_timeout_sec
        )
    }
}

/// Ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionConfig {
    /// Batch size for bulk inserts
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Flush interval in milliseconds
    #[serde(default = "default_flush_interval_ms")]
    pub flush_interval_ms: u64,

    /// Maximum buffer size before blocking
    #[serde(default = "default_max_buffer_size")]
    pub max_buffer_size: usize,
}

impl IngestionConfig {
    /// Get flush interval as Duration
    pub fn flush_interval(&self) -> Duration {
        Duration::from_millis(self.flush_interval_ms)
    }
}

/// Data retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Metrics retention in days
    #[serde(default = "default_metrics_retention_days")]
    pub metrics_retention_days: u32,

    /// Execution data retention in days
    #[serde(default = "default_execution_retention_days")]
    pub execution_retention_days: u32,

    /// Compression threshold in days
    #[serde(default = "default_compression_after_days")]
    pub compression_after_days: u32,
}

/// Query configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    /// Maximum number of results to return
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl_sec")]
    pub cache_ttl_sec: u64,

    /// Enable query result caching
    #[serde(default = "default_enable_cache")]
    pub enable_cache: bool,
}

impl QueryConfig {
    /// Get cache TTL as Duration
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_sec)
    }
}

// Default value functions
fn default_max_connections() -> usize {
    10
}

fn default_connection_timeout_sec() -> u64 {
    5
}

fn default_batch_size() -> usize {
    1000
}

fn default_flush_interval_ms() -> u64 {
    100
}

fn default_max_buffer_size() -> usize {
    10000
}

fn default_metrics_retention_days() -> u32 {
    90
}

fn default_execution_retention_days() -> u32 {
    365
}

fn default_compression_after_days() -> u32 {
    7
}

fn default_max_results() -> usize {
    10000
}

fn default_cache_ttl_sec() -> u64 {
    60
}

fn default_enable_cache() -> bool {
    true
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "ag_botkit".to_string(),
                user: "postgres".to_string(),
                password: "postgres".to_string(),
                max_connections: default_max_connections(),
                connection_timeout_sec: default_connection_timeout_sec(),
                use_tls: false,
            },
            ingestion: IngestionConfig {
                batch_size: default_batch_size(),
                flush_interval_ms: default_flush_interval_ms(),
                max_buffer_size: default_max_buffer_size(),
            },
            retention: RetentionConfig {
                metrics_retention_days: default_metrics_retention_days(),
                execution_retention_days: default_execution_retention_days(),
                compression_after_days: default_compression_after_days(),
            },
            query: QueryConfig {
                max_results: default_max_results(),
                cache_ttl_sec: default_cache_ttl_sec(),
                enable_cache: default_enable_cache(),
            },
        }
    }
}

impl StorageConfig {
    /// Load configuration from YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self, crate::error::StorageError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| crate::error::StorageError::ConfigError(e.to_string()))?;

        let config: StorageConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Load configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, crate::error::StorageError> {
        let config: StorageConfig = serde_yaml::from_str(yaml)?;
        Ok(config)
    }

    /// Save configuration to YAML file
    pub fn to_yaml_file(&self, path: &str) -> Result<(), crate::error::StorageError> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)
            .map_err(|e| crate::error::StorageError::ConfigError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StorageConfig::default();
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.ingestion.batch_size, 1000);
        assert_eq!(config.retention.metrics_retention_days, 90);
    }

    #[test]
    fn test_connection_string() {
        let config = DatabaseConfig {
            host: "db.example.com".to_string(),
            port: 5433,
            database: "testdb".to_string(),
            user: "testuser".to_string(),
            password: "testpass".to_string(),
            max_connections: 5,
            connection_timeout_sec: 10,
            use_tls: false,
        };

        let conn_str = config.connection_string();
        assert!(conn_str.contains("host=db.example.com"));
        assert!(conn_str.contains("port=5433"));
        assert!(conn_str.contains("dbname=testdb"));
    }

    #[test]
    fn test_yaml_serialization() {
        let config = StorageConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("database:"));
        assert!(yaml.contains("ingestion:"));

        // Deserialize back
        let parsed: StorageConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.database.host, config.database.host);
    }
}

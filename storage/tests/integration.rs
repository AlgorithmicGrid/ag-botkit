// Integration tests for ag-storage
// Requires running TimescaleDB instance (docker-compose up -d)

use ag_storage::{
    ExecutionStore, MetricPoint, Order, OrderFilters, OrderStatus, OrderType, PositionSnapshot,
    Side, StorageConfig, StorageEngine,
};
use chrono::{Duration, Utc};
use std::collections::HashMap;

// Helper to get test config
fn test_config() -> StorageConfig {
    StorageConfig::default()
}

#[tokio::test]
#[ignore] // Requires running TimescaleDB
async fn test_storage_engine_connection() {
    let config = test_config();
    let storage = StorageEngine::new(config).await;
    assert!(storage.is_ok(), "Should connect to TimescaleDB");
}

#[tokio::test]
#[ignore]
async fn test_insert_single_metric() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    let metric = MetricPoint::new("test.metric", 42.0).with_label("env", "test");

    let result = storage.insert_metric(metric).await;
    assert!(result.is_ok(), "Should insert metric successfully");
}

#[tokio::test]
#[ignore]
async fn test_batch_insert_metrics() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    let mut metrics = Vec::new();
    for i in 0..100 {
        let metric = MetricPoint::new("test.batch", i as f64)
            .with_label("batch", "test")
            .with_label("index", &i.to_string());
        metrics.push(metric);
    }

    let result = storage.insert_metrics_batch(metrics).await;
    assert!(result.is_ok(), "Should batch insert 100 metrics");
}

#[tokio::test]
#[ignore]
async fn test_query_metrics() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    // Insert test data
    let metric = MetricPoint::new("test.query", 123.45).with_label("test", "query");
    storage.insert_metric(metric).await.unwrap();

    // Query
    let end = Utc::now();
    let start = end - Duration::hours(1);

    let results = storage
        .query_metrics("test.query", start, end, None)
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find inserted metrics");
}

#[tokio::test]
#[ignore]
async fn test_query_metrics_with_labels() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    // Insert metrics with different labels
    storage
        .insert_metric(
            MetricPoint::new("test.labels", 1.0)
                .with_label("env", "prod")
                .with_label("region", "us-east"),
        )
        .await
        .unwrap();

    storage
        .insert_metric(
            MetricPoint::new("test.labels", 2.0)
                .with_label("env", "dev")
                .with_label("region", "us-west"),
        )
        .await
        .unwrap();

    // Query with label filter
    let end = Utc::now();
    let start = end - Duration::hours(1);

    let mut labels = HashMap::new();
    labels.insert("env".to_string(), "prod".to_string());

    let results = storage
        .query_metrics("test.labels", start, end, Some(labels))
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find metrics with labels");
    assert_eq!(
        results[0].labels.get("env"),
        Some(&"prod".to_string()),
        "Should filter by label"
    );
}

#[tokio::test]
#[ignore]
async fn test_query_aggregated_metrics() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    // Insert multiple metrics
    for i in 0..50 {
        let metric = MetricPoint::new("test.aggregate", (i as f64) * 1.5);
        storage.insert_metric(metric).await.unwrap();
    }

    // Query aggregated
    let end = Utc::now();
    let start = end - Duration::hours(1);
    let bucket_size = Duration::minutes(5);

    let results = storage
        .query_aggregated(
            "test.aggregate",
            start,
            end,
            bucket_size,
            ag_storage::Aggregation::Avg,
        )
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should have aggregated buckets");
}

#[tokio::test]
#[ignore]
async fn test_metric_buffering() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    // Buffer metrics
    for i in 0..10 {
        let metric = MetricPoint::new("test.buffer", i as f64);
        storage.buffer_metric(metric).await.unwrap();
    }

    // Flush
    let flushed = storage.flush_buffer().await.unwrap();
    assert_eq!(flushed, 10, "Should flush 10 buffered metrics");
}

#[tokio::test]
#[ignore]
async fn test_execution_store_order() {
    let config = test_config();
    let mut exec_store = ExecutionStore::new(config).await.unwrap();

    let order = Order::new("test_venue", "test_market", Side::Buy, OrderType::Limit, 100.0)
        .with_price(0.52)
        .with_status(OrderStatus::Open);

    let result = exec_store.store_order(order).await;
    assert!(result.is_ok(), "Should store order successfully");
}

#[tokio::test]
#[ignore]
async fn test_query_orders() {
    let config = test_config();
    let mut exec_store = ExecutionStore::new(config).await.unwrap();

    // Store test order
    let order = Order::new("polymarket", "0x123abc", Side::Buy, OrderType::Limit, 100.0)
        .with_price(0.52);

    exec_store.store_order(order).await.unwrap();

    // Query
    let end = Utc::now();
    let start = end - Duration::hours(1);

    let filters = OrderFilters {
        venue: Some("polymarket".to_string()),
        ..Default::default()
    };

    let results = exec_store.query_orders(start, end, filters).await.unwrap();

    assert!(!results.is_empty(), "Should find orders");
}

#[tokio::test]
#[ignore]
async fn test_store_position() {
    let config = test_config();
    let mut exec_store = ExecutionStore::new(config).await.unwrap();

    let position =
        PositionSnapshot::new("polymarket", "0x123abc", 100.0, 0.52).with_pnl(5.0, 10.0);

    let result = exec_store.store_position(position).await;
    assert!(result.is_ok(), "Should store position successfully");
}

#[tokio::test]
#[ignore]
async fn test_get_latest_position() {
    let config = test_config();
    let mut exec_store = ExecutionStore::new(config).await.unwrap();

    // Store position
    let position = PositionSnapshot::new("test_venue", "test_market", 50.0, 1.0);
    exec_store.store_position(position).await.unwrap();

    // Get latest
    let latest = exec_store
        .get_latest_position("test_venue", "test_market")
        .await
        .unwrap();

    assert!(latest.is_some(), "Should find latest position");
    assert_eq!(latest.unwrap().size, 50.0);
}

#[tokio::test]
#[ignore]
async fn test_pool_status() {
    let config = test_config();
    let storage = StorageEngine::new(config).await.unwrap();

    let status = storage.pool_status();
    assert!(!status.is_empty(), "Should return pool status");
}

#[tokio::test]
#[ignore]
async fn test_high_throughput_batch_insert() {
    let config = test_config();
    let mut storage = StorageEngine::new(config).await.unwrap();

    let start = std::time::Instant::now();

    // Insert 10,000 metrics in batches
    for batch_num in 0..10 {
        let mut batch = Vec::new();
        for i in 0..1000 {
            let metric = MetricPoint::new("test.throughput", (batch_num * 1000 + i) as f64)
                .with_label("batch", &batch_num.to_string());
            batch.push(metric);
        }
        storage.insert_metrics_batch(batch).await.unwrap();
    }

    let elapsed = start.elapsed();
    let throughput = 10_000.0 / elapsed.as_secs_f64();

    println!(
        "Inserted 10,000 metrics in {:.2}s ({:.0} metrics/sec)",
        elapsed.as_secs_f64(),
        throughput
    );

    assert!(
        throughput > 5000.0,
        "Should achieve >5000 metrics/sec throughput"
    );
}

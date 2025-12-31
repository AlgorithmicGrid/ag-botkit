use ag_storage::{MetricPoint, StorageConfig, StorageEngine};
use chrono::Utc;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    ag_storage::init_tracing();

    println!("=== Metric Insertion Example ===\n");

    // Load configuration (or use default)
    let config = StorageConfig::default();
    println!("Database: {}:{}/{}", config.database.host, config.database.port, config.database.database);

    // Create storage engine
    let mut storage = StorageEngine::new(config).await?;
    println!("Connected to TimescaleDB\n");

    // Example 1: Insert single metric
    println!("1. Inserting single metric...");
    let metric = MetricPoint::new("polymarket.rtds.lag_ms", 45.3)
        .with_label("topic", "market")
        .with_label("venue", "polymarket");

    storage.insert_metric(metric).await?;
    println!("   ✓ Inserted metric: polymarket.rtds.lag_ms = 45.3\n");

    // Example 2: Batch insert multiple metrics
    println!("2. Batch inserting 100 metrics...");
    let mut metrics = Vec::new();

    for i in 0..100 {
        let metric = MetricPoint::new("polymarket.rtds.messages_received", (i as f64))
            .with_label("topic", "market")
            .with_timestamp(Utc::now());

        metrics.push(metric);
    }

    storage.insert_metrics_batch(metrics).await?;
    println!("   ✓ Inserted 100 metrics in batch\n");

    // Example 3: Insert metrics with different labels
    println!("3. Inserting metrics for multiple markets...");
    let markets = vec!["0x123abc", "0x456def", "0x789ghi"];

    for market in markets {
        let metric = MetricPoint::new("polymarket.position.size", 150.0)
            .with_label("market_id", market)
            .with_label("side", "long");

        storage.insert_metric(metric).await?;
    }
    println!("   ✓ Inserted position metrics for 3 markets\n");

    // Example 4: Insert various metric types
    println!("4. Inserting different metric types...");

    let metrics_to_insert = vec![
        MetricPoint::new("polymarket.rtds.msgs_per_second", 23.5),
        MetricPoint::new("polymarket.inventory.value_usd", 1250.75),
        MetricPoint::new("polymarket.risk.decision", 1.0)
            .with_label("policy", "position_limit"),
        MetricPoint::new("polymarket.risk.kill_switch", 0.0),
    ];

    storage.insert_metrics_batch(metrics_to_insert).await?;
    println!("   ✓ Inserted 4 different metric types\n");

    // Example 5: Demonstrate buffering (for high-throughput scenarios)
    println!("5. Demonstrating metric buffering...");

    for i in 0..50 {
        let metric = MetricPoint::new("test.high_frequency", (i as f64) * 1.1);
        storage.buffer_metric(metric).await?;
    }

    let flushed = storage.flush_buffer().await?;
    println!("   ✓ Buffered and flushed {} metrics\n", flushed);

    println!("=== All metrics inserted successfully! ===");
    println!("\nTip: Use query_data example to retrieve these metrics");

    Ok(())
}

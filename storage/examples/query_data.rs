use ag_storage::{Aggregation, StorageConfig, StorageEngine};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    ag_storage::init_tracing();

    println!("=== Metric Query Example ===\n");

    // Load configuration
    let config = StorageConfig::default();
    println!("Database: {}:{}/{}", config.database.host, config.database.port, config.database.database);

    // Create storage engine
    let storage = StorageEngine::new(config).await?;
    println!("Connected to TimescaleDB\n");

    // Example 1: Query metrics in time range
    println!("1. Querying metrics from last hour...");
    let end = Utc::now();
    let start = end - Duration::hours(1);

    let metrics = storage
        .query_metrics("polymarket.rtds.lag_ms", start, end, None)
        .await?;

    println!("   Found {} metrics", metrics.len());
    if let Some(first) = metrics.first() {
        println!("   Latest: {} = {}", first.metric_name, first.value);
    }
    println!();

    // Example 2: Query metrics with label filter
    println!("2. Querying metrics with label filter...");
    let mut labels = HashMap::new();
    labels.insert("topic".to_string(), "market".to_string());

    let filtered_metrics = storage
        .query_metrics("polymarket.rtds.lag_ms", start, end, Some(labels))
        .await?;

    println!("   Found {} metrics with label topic=market", filtered_metrics.len());
    println!();

    // Example 3: Query aggregated metrics
    println!("3. Querying aggregated metrics (5-minute buckets)...");
    let bucket_size = Duration::minutes(5);

    let aggregated = storage
        .query_aggregated(
            "polymarket.rtds.messages_received",
            start,
            end,
            bucket_size,
            Aggregation::Avg,
        )
        .await?;

    println!("   Found {} buckets", aggregated.len());
    for bucket in aggregated.iter().take(5) {
        println!(
            "   {} | avg: {:.2}, min: {:.2}, max: {:.2}, count: {}",
            bucket.bucket.format("%H:%M:%S"),
            bucket.avg_value.unwrap_or(0.0),
            bucket.min_value.unwrap_or(0.0),
            bucket.max_value.unwrap_or(0.0),
            bucket.count
        );
    }
    println!();

    // Example 4: Query recent metrics (last 100)
    println!("4. Querying last 100 metrics...");
    let recent_end = Utc::now();
    let recent_start = recent_end - Duration::days(7);

    let recent_metrics = storage
        .query_metrics(
            "polymarket.position.size",
            recent_start,
            recent_end,
            None,
        )
        .await?;

    println!("   Found {} position metrics", recent_metrics.len());
    if !recent_metrics.is_empty() {
        println!("   Sample metrics:");
        for metric in recent_metrics.iter().take(3) {
            let market = metric.labels.get("market_id").map(|s| s.as_str()).unwrap_or("unknown");
            println!("     Market {}: size = {}", market, metric.value);
        }
    }
    println!();

    // Example 5: Query different aggregation windows
    println!("5. Comparing different aggregation windows...");

    let windows = vec![
        ("1 minute", Duration::minutes(1)),
        ("5 minutes", Duration::minutes(5)),
        ("15 minutes", Duration::minutes(15)),
        ("1 hour", Duration::hours(1)),
    ];

    for (name, window) in windows {
        let agg = storage
            .query_aggregated(
                "polymarket.rtds.lag_ms",
                start,
                end,
                window,
                Aggregation::Avg,
            )
            .await?;

        println!("   {} buckets: {}", name, agg.len());
    }
    println!();

    // Example 6: Get pool status
    println!("6. Connection pool status:");
    println!("   {}", storage.pool_status());
    println!();

    println!("=== Query examples complete! ===");

    Ok(())
}

use anyhow::Result;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::{interval, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

mod config;
mod metrics;
mod rtds;

use config::Config;
use metrics::{MetricSender, MetricType};
use rtds::RtdsMessage;

#[derive(Parser, Debug)]
#[clap(name = "minibot", about = "Polymarket RTDS demo bot")]
struct Args {
    #[clap(short, long, default_value = "config.yaml")]
    config: PathBuf,
}

struct BotState {
    message_count: u64,
    last_second_count: u64,
    last_second_time: Instant,
    simulator: ag_risk::PolymarketSimulator,
    risk_engine: ag_risk::RiskEngine,
}

impl BotState {
    fn new(risk_engine: ag_risk::RiskEngine) -> Self {
        Self {
            message_count: 0,
            last_second_count: 0,
            last_second_time: Instant::now(),
            simulator: ag_risk::PolymarketSimulator::new(),
            risk_engine,
        }
    }

    fn increment_message_count(&mut self) -> u64 {
        self.message_count += 1;
        self.last_second_count += 1;
        self.message_count
    }

    fn get_messages_per_second(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_second_time);

        if elapsed >= Duration::from_secs(1) {
            let msgs_per_sec = self.last_second_count as f64 / elapsed.as_secs_f64();
            self.last_second_count = 0;
            self.last_second_time = now;
            msgs_per_sec
        } else {
            0.0
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Load configuration
    info!("Loading configuration from {:?}", args.config);
    let config = Config::load(&args.config)?;

    // Load risk policies
    info!("Loading risk policies from {:?}", config.risk.policy_file);
    let policy_yaml = std::fs::read_to_string(&config.risk.policy_file)?;
    let risk_engine = ag_risk::RiskEngine::from_yaml(&policy_yaml)
        .map_err(|e| anyhow::anyhow!("Failed to load risk policy: {}", e))?;

    // Initialize state
    let state = Arc::new(RwLock::new(BotState::new(risk_engine)));

    // Connect to monitor
    info!("Connecting to monitor at {}", config.monitor.endpoint);
    let metric_sender = MetricSender::connect(&config.monitor.endpoint).await?;
    let metric_sender = Arc::new(metric_sender);
    info!("Connected to monitor");

    // Connect to RTDS
    info!("Connecting to Polymarket RTDS at {}", config.rtds.endpoint);
    let (ws_stream, _) = connect_async(&config.rtds.endpoint).await?;
    info!("Connected to RTDS");

    let (write, mut read) = ws_stream.split();
    let write = Arc::new(tokio::sync::Mutex::new(write));

    // Subscribe to RTDS topics
    let mut w = write.lock().await;

    // Subscribe to crypto_prices (fully supported topic)
    let crypto_sub = rtds::SubscriptionMessage {
        action: "subscribe".to_string(),
        subscriptions: vec![
            rtds::Subscription {
                topic: "crypto_prices".to_string(),
                msg_type: "*".to_string(),
                filters: None,
            },
        ],
    };

    let json = serde_json::to_string(&crypto_sub)?;
    info!("Subscribing to crypto_prices");
    w.send(Message::Text(json)).await?;

    // Also try subscribing to activity/trades (unsupported but may work)
    let activity_sub = rtds::SubscriptionMessage {
        action: "subscribe".to_string(),
        subscriptions: vec![
            rtds::Subscription {
                topic: "activity".to_string(),
                msg_type: "trades".to_string(),
                filters: None,
            },
        ],
    };

    let json = serde_json::to_string(&activity_sub)?;
    info!("Subscribing to activity/trades");
    w.send(Message::Text(json)).await?;

    drop(w); // Release lock

    // Spawn ping task
    let write_clone = Arc::clone(&write);
    let ping_interval = config.rtds.ping_interval_sec;
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(ping_interval));
        loop {
            interval.tick().await;
            let mut w = write_clone.lock().await;
            if let Err(e) = w.send(Message::Text(r#"{"type":"ping"}"#.to_string())).await {
                error!("Failed to send ping: {}", e);
                break;
            }
        }
    });

    // Spawn metrics reporting task
    let state_clone = Arc::clone(&state);
    let metric_sender_clone = Arc::clone(&metric_sender);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            let mut state = state_clone.write().await;
            let msgs_per_sec = state.get_messages_per_second();

            if msgs_per_sec > 0.0 {
                if let Err(e) = metric_sender_clone.send(
                    "polymarket.rtds.msgs_per_second",
                    MetricType::Gauge,
                    msgs_per_sec,
                    std::collections::HashMap::new(),
                ).await {
                    warn!("Failed to send msgs_per_second metric: {}", e);
                }
            }

            // Send inventory metrics
            let inventory_value = state.simulator.get_inventory_value_usd();
            if let Err(e) = metric_sender_clone.send(
                "polymarket.inventory.value_usd",
                MetricType::Gauge,
                inventory_value,
                std::collections::HashMap::new(),
            ).await {
                warn!("Failed to send inventory metric: {}", e);
            }
        }
    });

    // Main message loop
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_message(&text, &state, &metric_sender, &config).await {
                    warn!("Failed to handle message: {}", e);
                }
            }
            Ok(Message::Ping(_)) => {
                // Handled automatically by tungstenite
            }
            Ok(Message::Pong(_)) => {
                // Response to our ping
            }
            Ok(Message::Close(_)) => {
                info!("RTDS connection closed");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    info!("Minibot shutting down");
    Ok(())
}

async fn handle_message(
    text: &str,
    state: &Arc<RwLock<BotState>>,
    metric_sender: &Arc<MetricSender>,
    _config: &Config,
) -> Result<()> {
    // Parse RTDS message
    let rtds_msg: RtdsMessage = serde_json::from_str(text)?;

    // Skip pong messages
    if rtds_msg.msg_type == "pong" {
        return Ok(());
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis() as u64;

    // Increment message count
    let mut state = state.write().await;
    let total_count = state.increment_message_count();

    // Calculate lag
    if let Some(server_timestamp) = rtds_msg.timestamp {
        let lag_ms = (now_ms as i64 - server_timestamp as i64).abs() as f64;

        let mut labels = std::collections::HashMap::new();
        if let Some(topic) = &rtds_msg.topic {
            labels.insert("topic".to_string(), topic.clone());
        }

        metric_sender.send(
            "polymarket.rtds.lag_ms",
            MetricType::Gauge,
            lag_ms,
            labels.clone(),
        ).await?;
    }

    // Send message received counter
    let mut labels = std::collections::HashMap::new();
    if let Some(topic) = &rtds_msg.topic {
        labels.insert("topic".to_string(), topic.clone());
    }
    labels.insert("message_type".to_string(), rtds_msg.msg_type.clone());

    metric_sender.send(
        "polymarket.rtds.messages_received",
        MetricType::Counter,
        1.0,
        labels,
    ).await?;

    // Simulate position updates and risk checks
    if rtds_msg.msg_type == "book" && rtds_msg.topic == Some("market".to_string()) {
        if let Some(payload) = rtds_msg.payload {
            if let Some(market_id) = payload.get("market").and_then(|v| v.as_str()) {
                // Simulate a small position update
                let mock_size = 10.0;
                let mock_price = 0.5;

                state.simulator.update_position(market_id, mock_size, mock_price);
                let position = state.simulator.get_position(market_id);

                // Send position metric
                let mut labels = std::collections::HashMap::new();
                labels.insert("market_id".to_string(), market_id.to_string());

                metric_sender.send(
                    "polymarket.position.size",
                    MetricType::Gauge,
                    position,
                    labels.clone(),
                ).await?;

                // Evaluate risk
                let risk_context = ag_risk::RiskContext {
                    market_id: market_id.to_string(),
                    current_position: position,
                    proposed_size: mock_size,
                    inventory_value_usd: state.simulator.get_inventory_value_usd(),
                };

                let decision = state.risk_engine.evaluate(&risk_context);

                labels.insert("policy".to_string(), "combined".to_string());
                metric_sender.send(
                    "polymarket.risk.decision",
                    MetricType::Gauge,
                    if decision.allowed { 1.0 } else { 0.0 },
                    labels,
                ).await?;

                if !decision.allowed {
                    warn!("Risk check BLOCKED: {:?}", decision.violated_policies);
                }
            }
        }
    }

    if total_count % 100 == 0 {
        info!("Processed {} messages", total_count);
    }

    Ok(())
}

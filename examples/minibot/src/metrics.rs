use anyhow::Result;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug, Serialize)]
pub struct MetricMessage {
    pub timestamp: u64,
    pub metric_type: MetricType,
    pub metric_name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
}

pub struct MetricSender {
    ws: Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl MetricSender {
    pub async fn connect(endpoint: &str) -> Result<Self> {
        let (ws_stream, _) = connect_async(endpoint).await?;
        Ok(Self {
            ws: Mutex::new(ws_stream),
        })
    }

    pub async fn send(
        &self,
        metric_name: &str,
        metric_type: MetricType,
        value: f64,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis() as u64;

        let msg = MetricMessage {
            timestamp,
            metric_type,
            metric_name: metric_name.to_string(),
            value,
            labels,
        };

        let json = serde_json::to_string(&msg)?;

        let mut ws = self.ws.lock().await;
        ws.send(Message::Text(json)).await?;

        Ok(())
    }
}

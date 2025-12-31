use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RtdsMessage {
    #[serde(rename = "type")]
    pub msg_type: String,

    #[serde(default)]
    pub topic: Option<String>,

    #[serde(default)]
    pub timestamp: Option<u64>,

    #[serde(default)]
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct Subscription {
    pub topic: String,

    #[serde(rename = "type")]
    pub msg_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionMessage {
    pub action: String,
    pub subscriptions: Vec<Subscription>,
}

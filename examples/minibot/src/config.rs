use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rtds: RtdsConfig,
    pub monitor: MonitorConfig,
    pub risk: RiskConfig,
}

#[derive(Debug, Deserialize)]
pub struct RtdsConfig {
    pub endpoint: String,
    pub ping_interval_sec: u64,
    pub reconnect_delay_sec: u64,
    pub subscribe_topics: Vec<MarketSubscription>,
}

#[derive(Debug, Deserialize)]
pub struct MarketSubscription {
    pub market: String,
}

#[derive(Debug, Deserialize)]
pub struct MonitorConfig {
    pub endpoint: String,
    pub buffer_size: usize,
    pub reconnect_delay_sec: u64,
}

#[derive(Debug, Deserialize)]
pub struct RiskConfig {
    pub policy_file: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}

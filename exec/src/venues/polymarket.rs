//! Polymarket CLOB venue adapter
//!
//! This module implements the VenueAdapter trait for Polymarket CLOB API.
//! Documentation: https://docs.polymarket.com

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;

use crate::adapters::venue_adapter::{VenueAdapter, VenueConfig};
use crate::error::{ExecError, ExecResult};
use crate::order::{
    CancelAck, Liquidity, Order, OrderAck, OrderId, OrderStatus, OrderType, Side, TimeInForce,
    VenueId,
};

type HmacSha256 = Hmac<Sha256>;

/// Polymarket CLOB adapter
pub struct PolymarketAdapter {
    config: VenueConfig,
    client: Client,
    /// Map of our OrderId to Polymarket order ID
    order_id_map: HashMap<OrderId, String>,
}

impl PolymarketAdapter {
    /// Create a new Polymarket adapter
    pub fn new(config: VenueConfig) -> ExecResult<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ExecError::ConfigError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            order_id_map: HashMap::new(),
        })
    }

    /// Sign a request for authentication
    fn sign_request(&self, timestamp: i64, method: &str, path: &str, body: &str) -> ExecResult<String> {
        let api_secret = self
            .config
            .api_secret
            .as_ref()
            .ok_or_else(|| ExecError::AuthenticationError("API secret not configured".to_string()))?;

        let message = format!("{}{}{}{}", timestamp, method, path, body);

        let mut mac = HmacSha256::new_from_slice(api_secret.as_bytes())
            .map_err(|e| ExecError::InternalError(format!("HMAC error: {}", e)))?;
        mac.update(message.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }

    /// Convert our Order to Polymarket API format
    fn to_polymarket_order(&self, order: &Order) -> ExecResult<PolymarketOrderRequest> {
        let side = match order.side {
            Side::Buy => "BUY",
            Side::Sell => "SELL",
        };

        let order_type = match order.order_type {
            OrderType::Limit => "GTC",
            OrderType::Market => "FOK", // Polymarket uses FOK for market orders
            OrderType::PostOnly => "GTD",
        };

        Ok(PolymarketOrderRequest {
            market: order.market.as_str().to_string(),
            side: side.to_string(),
            price: order.price.map(|p| p.to_string()),
            size: order.size.to_string(),
            order_type: order_type.to_string(),
            client_order_id: Some(order.client_order_id.clone()),
        })
    }

    /// Convert Polymarket order status to our status
    fn from_polymarket_status(status: &str) -> OrderStatus {
        match status {
            "PENDING" => OrderStatus::Pending,
            "LIVE" => OrderStatus::Working,
            "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
            "FILLED" => OrderStatus::Filled,
            "CANCELLED" => OrderStatus::Cancelled,
            "REJECTED" => OrderStatus::Rejected,
            "EXPIRED" => OrderStatus::Expired,
            _ => OrderStatus::Rejected, // Unknown status treated as rejected
        }
    }
}

#[async_trait]
impl VenueAdapter for PolymarketAdapter {
    fn venue_id(&self) -> VenueId {
        self.config.venue_id.clone()
    }

    async fn place_order(&mut self, order: &Order) -> ExecResult<OrderAck> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| ExecError::AuthenticationError("API key not configured".to_string()))?;

        // Convert to Polymarket format
        let pm_order = self.to_polymarket_order(order)?;
        let body = serde_json::to_string(&pm_order)?;

        // Sign request
        let timestamp = Utc::now().timestamp();
        let path = "/orders";
        let signature = self.sign_request(timestamp, "POST", path, &body)?;

        // Build request
        let url = format!("{}{}", self.config.api_endpoint, path);
        let response = self
            .client
            .post(&url)
            .header("X-API-Key", api_key)
            .header("X-Signature", signature)
            .header("X-Timestamp", timestamp.to_string())
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        // Handle response
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ExecError::VenueError {
                venue: self.venue_id().to_string(),
                message: format!("Order placement failed: {}", error_text),
                code: Some(status.as_str().to_string()),
            });
        }

        let pm_response: PolymarketOrderResponse = response.json().await?;

        // Store order ID mapping
        self.order_id_map.insert(order.id, pm_response.order_id.clone());

        Ok(OrderAck {
            order_id: order.id,
            venue_order_id: Some(pm_response.order_id),
            status: Self::from_polymarket_status(&pm_response.status),
            timestamp: Utc::now(),
            message: None,
        })
    }

    async fn cancel_order(&mut self, order_id: &OrderId) -> ExecResult<CancelAck> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| ExecError::AuthenticationError("API key not configured".to_string()))?;

        // Get Polymarket order ID
        let pm_order_id = self
            .order_id_map
            .get(order_id)
            .ok_or_else(|| ExecError::OrderNotFound(*order_id))?;

        // Sign request
        let timestamp = Utc::now().timestamp();
        let path = format!("/orders/{}", pm_order_id);
        let signature = self.sign_request(timestamp, "DELETE", &path, "")?;

        // Build request
        let url = format!("{}{}", self.config.api_endpoint, path);
        let response = self
            .client
            .delete(&url)
            .header("X-API-Key", api_key)
            .header("X-Signature", signature)
            .header("X-Timestamp", timestamp.to_string())
            .send()
            .await?;

        // Handle response
        let success = response.status().is_success();
        let message = if !success {
            Some(response.text().await.unwrap_or_else(|_| "Cancel failed".to_string()))
        } else {
            None
        };

        Ok(CancelAck {
            order_id: *order_id,
            venue_order_id: Some(pm_order_id.clone()),
            success,
            timestamp: Utc::now(),
            message,
        })
    }

    async fn get_order_status(&mut self, order_id: &OrderId) -> ExecResult<OrderStatus> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| ExecError::AuthenticationError("API key not configured".to_string()))?;

        // Get Polymarket order ID
        let pm_order_id = self
            .order_id_map
            .get(order_id)
            .ok_or_else(|| ExecError::OrderNotFound(*order_id))?;

        // Sign request
        let timestamp = Utc::now().timestamp();
        let path = format!("/orders/{}", pm_order_id);
        let signature = self.sign_request(timestamp, "GET", &path, "")?;

        // Build request
        let url = format!("{}{}", self.config.api_endpoint, path);
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", api_key)
            .header("X-Signature", signature)
            .header("X-Timestamp", timestamp.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ExecError::VenueError {
                venue: self.venue_id().to_string(),
                message: "Failed to get order status".to_string(),
                code: Some(response.status().as_str().to_string()),
            });
        }

        let pm_response: PolymarketOrderResponse = response.json().await?;
        Ok(Self::from_polymarket_status(&pm_response.status))
    }

    async fn get_open_orders(&mut self) -> ExecResult<Vec<Order>> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| ExecError::AuthenticationError("API key not configured".to_string()))?;

        // Sign request
        let timestamp = Utc::now().timestamp();
        let path = "/orders?status=LIVE";
        let signature = self.sign_request(timestamp, "GET", path, "")?;

        // Build request
        let url = format!("{}{}", self.config.api_endpoint, path);
        let response = self
            .client
            .get(&url)
            .header("X-API-Key", api_key)
            .header("X-Signature", signature)
            .header("X-Timestamp", timestamp.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ExecError::VenueError {
                venue: self.venue_id().to_string(),
                message: "Failed to get open orders".to_string(),
                code: Some(response.status().as_str().to_string()),
            });
        }

        let pm_orders: Vec<PolymarketOrderResponse> = response.json().await?;

        // Convert to our Order type
        // Note: This is simplified - in production you'd need full order reconstruction
        let orders = pm_orders
            .into_iter()
            .map(|_pm_order| {
                // Simplified conversion - in production this would need complete mapping
                // For now, return empty vec as this is primarily for demonstration
                Vec::new()
            })
            .flatten()
            .collect();

        Ok(orders)
    }

    async fn modify_order(
        &mut self,
        order_id: &OrderId,
        new_price: Option<f64>,
        new_size: Option<f64>,
    ) -> ExecResult<OrderAck> {
        // Polymarket typically requires cancel + replace for modifications
        // This is a simplified implementation
        let cancel_ack = self.cancel_order(order_id).await?;

        if !cancel_ack.success {
            return Err(ExecError::VenueError {
                venue: self.venue_id().to_string(),
                message: "Failed to cancel order for modification".to_string(),
                code: None,
            });
        }

        // Note: In production, you'd need to recreate and place the modified order
        // This requires storing original order details
        Err(ExecError::VenueError {
            venue: self.venue_id().to_string(),
            message: "Order modification requires full order context - not yet implemented".to_string(),
            code: None,
        })
    }

    async fn health_check(&mut self) -> ExecResult<bool> {
        // Simple health check - try to reach the API
        let url = format!("{}/health", self.config.api_endpoint);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

// Polymarket API request/response types

#[derive(Debug, Serialize)]
struct PolymarketOrderRequest {
    market: String,
    side: String,
    price: Option<String>,
    size: String,
    #[serde(rename = "type")]
    order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_order_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PolymarketOrderResponse {
    order_id: String,
    status: String,
    #[serde(default)]
    filled_size: String,
    #[serde(default)]
    avg_fill_price: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::{MarketId, VenueId};

    #[test]
    fn test_polymarket_adapter_creation() {
        let config = VenueConfig::new(
            VenueId::new("polymarket"),
            "https://clob.polymarket.com".to_string(),
        );

        let adapter = PolymarketAdapter::new(config);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_status_conversion() {
        assert_eq!(
            PolymarketAdapter::from_polymarket_status("LIVE"),
            OrderStatus::Working
        );
        assert_eq!(
            PolymarketAdapter::from_polymarket_status("FILLED"),
            OrderStatus::Filled
        );
        assert_eq!(
            PolymarketAdapter::from_polymarket_status("CANCELLED"),
            OrderStatus::Cancelled
        );
    }

    #[test]
    fn test_order_conversion() {
        let config = VenueConfig::new(
            VenueId::new("polymarket"),
            "https://clob.polymarket.com".to_string(),
        );
        let adapter = PolymarketAdapter::new(config).unwrap();

        let order = Order::new(
            VenueId::new("polymarket"),
            MarketId::new("0x123abc"),
            Side::Buy,
            OrderType::Limit,
            Some(0.52),
            100.0,
            TimeInForce::GTC,
            "client-123".to_string(),
        );

        let pm_order = adapter.to_polymarket_order(&order);
        assert!(pm_order.is_ok());

        let pm_order = pm_order.unwrap();
        assert_eq!(pm_order.market, "0x123abc");
        assert_eq!(pm_order.side, "BUY");
        assert_eq!(pm_order.size, "100");
    }
}

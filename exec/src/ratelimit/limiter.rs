//! Rate limiting implementation
//!
//! This module provides rate limiting functionality to prevent API violations
//! when communicating with exchanges. It uses a token bucket algorithm.

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter as GovRateLimiter};
use nonzero_ext::nonzero;
use std::num::NonZeroU32;
use std::time::Duration;

use crate::error::{ExecError, ExecResult};
use crate::order::VenueId;

/// Rate limiter for API requests
pub struct RateLimiter {
    venue_id: VenueId,
    limiter: DefaultDirectRateLimiter,
    requests_per_second: u32,
    burst_size: u32,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `venue_id` - Venue identifier
    /// * `requests_per_second` - Maximum requests per second
    /// * `burst_size` - Maximum burst capacity
    pub fn new(venue_id: VenueId, requests_per_second: u32, burst_size: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap())
            .allow_burst(NonZeroU32::new(burst_size).unwrap());

        Self {
            venue_id,
            limiter: GovRateLimiter::direct(quota),
            requests_per_second,
            burst_size,
        }
    }

    /// Check if a request is allowed and wait if necessary
    ///
    /// This method will block until the rate limit allows the request.
    pub async fn check(&self) -> ExecResult<()> {
        match self.limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => {
                // Wait for the next available slot
                self.limiter.until_ready().await;
                Ok(())
            }
        }
    }

    /// Try to acquire permission without waiting
    ///
    /// # Returns
    /// * `Ok(())` - Permission granted
    /// * `Err(ExecError::RateLimitExceeded)` - Rate limit exceeded
    pub fn try_check(&self) -> ExecResult<()> {
        self.limiter.check().map_err(|_| ExecError::RateLimitExceeded {
            venue: self.venue_id.to_string(),
            message: format!(
                "Rate limit exceeded: {} requests/sec, burst {}",
                self.requests_per_second, self.burst_size
            ),
        })
    }

    /// Get venue ID
    pub fn venue_id(&self) -> &VenueId {
        &self.venue_id
    }

    /// Get requests per second limit
    pub fn requests_per_second(&self) -> u32 {
        self.requests_per_second
    }

    /// Get burst size
    pub fn burst_size(&self) -> u32 {
        self.burst_size
    }
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Requests per second
    pub requests_per_second: u32,
    /// Burst capacity
    pub burst_size: u32,
}

impl RateLimiterConfig {
    /// Create a new rate limiter configuration
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            requests_per_second,
            burst_size,
        }
    }

    /// Default configuration for Polymarket CLOB
    pub fn polymarket_default() -> Self {
        Self {
            requests_per_second: 10,
            burst_size: 20,
        }
    }

    /// Default configuration for Binance
    pub fn binance_default() -> Self {
        Self {
            requests_per_second: 20,
            burst_size: 50,
        }
    }

    /// Build a rate limiter with this configuration
    pub fn build(&self, venue_id: VenueId) -> RateLimiter {
        RateLimiter::new(venue_id, self.requests_per_second, self.burst_size)
    }
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_size: 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Instant};

    #[tokio::test]
    async fn test_rate_limiter_allows_requests() {
        let limiter = RateLimiter::new(VenueId::new("test"), 10, 10);

        // First request should succeed immediately
        let result = limiter.try_check();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_enforces_limit() {
        let limiter = RateLimiter::new(VenueId::new("test"), 2, 2);

        // First two requests should succeed
        assert!(limiter.try_check().is_ok());
        assert!(limiter.try_check().is_ok());

        // Third request should fail
        let result = limiter.try_check();
        assert!(result.is_err());
        assert!(matches!(result, Err(ExecError::RateLimitExceeded { .. })));
    }

    #[tokio::test]
    async fn test_rate_limiter_check_waits() {
        let limiter = RateLimiter::new(VenueId::new("test"), 2, 2);

        // Exhaust burst capacity
        limiter.try_check().unwrap();
        limiter.try_check().unwrap();

        // This should wait but eventually succeed
        let start = Instant::now();
        limiter.check().await.unwrap();
        let elapsed = start.elapsed();

        // Should have waited at least some time
        assert!(elapsed > Duration::from_millis(100));
    }

    #[test]
    fn test_rate_limiter_config() {
        let config = RateLimiterConfig::new(15, 30);
        assert_eq!(config.requests_per_second, 15);
        assert_eq!(config.burst_size, 30);

        let polymarket_config = RateLimiterConfig::polymarket_default();
        assert_eq!(polymarket_config.requests_per_second, 10);
        assert_eq!(polymarket_config.burst_size, 20);
    }

    #[test]
    fn test_rate_limiter_build() {
        let config = RateLimiterConfig::new(5, 10);
        let limiter = config.build(VenueId::new("test"));

        assert_eq!(limiter.venue_id().as_str(), "test");
        assert_eq!(limiter.requests_per_second(), 5);
        assert_eq!(limiter.burst_size(), 10);
    }
}

//! Rate limiting
//!
//! This module provides rate limiting functionality to prevent API violations.

pub mod limiter;

pub use limiter::{RateLimiter, RateLimiterConfig};

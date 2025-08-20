// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Rate limiting implementations for the REST API gateway
//! Implements multiple rate limiting algorithms:
//! - Token Bucket
//! - Sliding Window
//! - Fixed Window Counter

use crate::error::{ApiError, ApiResult};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests allowed in the time window
    pub max_requests: u32,

    /// Time window for rate limiting
    #[serde(with = "serde_duration")]
    pub window: Duration,

    /// Algorithm to use for rate limiting
    pub algorithm: RateLimitAlgorithm,

    /// Whether to apply rate limiting per IP address
    pub per_ip: bool,

    /// Whether to apply rate limiting per user
    pub per_user: bool,

    /// Whether to apply rate limiting per API key
    pub per_api_key: bool,
}

/// Rate limiting algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RateLimitAlgorithm {
    /// Token bucket algorithm
    TokenBucket,

    /// Sliding window algorithm
    SlidingWindow,

    /// Fixed window counter algorithm
    FixedWindowCounter,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Maximum requests allowed
    pub limit: u32,

    /// Remaining requests in current window
    pub remaining: u32,

    /// Time until rate limit resets (in seconds)
    pub reset_in: u64,

    /// Whether the request was allowed
    pub allowed: bool,
}

/// Token bucket rate limiter
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum tokens in the bucket
    max_tokens: u32,

    /// Tokens added per second
    tokens_per_second: f64,

    /// Current tokens
    tokens: DashMap<String, (f64, Instant)>,
}

impl TokenBucket {
    /// Create a new token bucket rate limiter
    pub fn new(max_tokens: u32, window: Duration) -> Self {
        let tokens_per_second = max_tokens as f64 / window.as_secs_f64();

        Self {
            max_tokens,
            tokens_per_second,
            tokens: DashMap::new(),
        }
    }

    /// Check if a request is allowed
    pub fn is_allowed(&self, key: &str, cost: u32) -> RateLimitInfo {
        let now = Instant::now();
        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0)).as_secs();

        let mut entry = self.tokens.entry(key.to_string()).or_insert((self.max_tokens as f64, now));
        let (tokens, last_refill) = *entry.value();

        // Calculate tokens to add based on time passed
        let elapsed = now.duration_since(last_refill);
        let tokens_to_add = elapsed.as_secs_f64() * self.tokens_per_second;

        // Refill tokens
        let mut current_tokens = tokens + tokens_to_add;
        if current_tokens > self.max_tokens as f64 {
            current_tokens = self.max_tokens as f64;
        }

        // Check if request is allowed
        let allowed = current_tokens >= cost as f64;
        let remaining = if allowed { (current_tokens - cost as f64) as u32 } else { current_tokens as u32 };

        // Update tokens if allowed
        if allowed {
            *entry.value_mut() = (current_tokens - cost as f64, now);
        } else {
            *entry.value_mut() = (current_tokens, last_refill);
        }

        RateLimitInfo {
            limit: self.max_tokens,
            remaining,
            reset_in: 1, // Approximate reset time
            allowed,
        }
    }
}

/// Sliding window rate limiter
#[derive(Debug)]
pub struct SlidingWindow {
    /// Maximum requests allowed in window
    max_requests: u32,

    /// Window duration
    window: Duration,

    /// Request timestamps per key
    requests: DashMap<String, Vec<Instant>>,
}

impl SlidingWindow {
    /// Create a new sliding window rate limiter
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: DashMap::new(),
        }
    }

    /// Check if a request is allowed
    pub fn is_allowed(&self, key: &str) -> RateLimitInfo {
        let now = Instant::now();
        let window_start = now - self.window;

        // Get or create request history for this key
        let mut requests = self.requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove old requests outside the window
        requests.retain(|&timestamp| timestamp > window_start);

        // Check if we're within the limit
        let count = requests.len() as u32;
        let allowed = count < self.max_requests;

        // Add current request if allowed
        if allowed {
            requests.push(now);
        }

        // Calculate reset time (when oldest request expires)
        let reset_in = if let Some(&oldest) = requests.first() {
            let oldest_expires = oldest + self.window;
            if oldest_expires > now { oldest_expires.duration_since(now).as_secs() } else { 0 }
        } else {
            0
        };

        RateLimitInfo {
            limit: self.max_requests,
            remaining: self.max_requests.saturating_sub(count + if allowed { 1 } else { 0 }),
            reset_in,
            allowed,
        }
    }
}

/// Fixed window counter rate limiter
#[derive(Debug)]
pub struct FixedWindowCounter {
    /// Maximum requests allowed per window
    max_requests: u32,

    /// Window duration
    window: Duration,

    /// Request counts per key and window
    counts: DashMap<String, (u32, Instant)>,
}

impl FixedWindowCounter {
    /// Create a new fixed window counter rate limiter
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            counts: DashMap::new(),
        }
    }

    /// Check if a request is allowed
    pub fn is_allowed(&self, key: &str) -> RateLimitInfo {
        let now = Instant::now();

        let mut entry = self.counts.entry(key.to_string()).or_insert((0, now));
        let (count, window_start) = *entry.value();

        // Check if we're in a new window
        if now.duration_since(window_start) > self.window {
            // Reset count for new window
            *entry.value_mut() = (1, now);

            RateLimitInfo {
                limit: self.max_requests,
                remaining: self.max_requests - 1,
                reset_in: self.window.as_secs(),
                allowed: true,
            }
        } else {
            // Same window, check limit
            let allowed = count < self.max_requests;
            let remaining = self.max_requests.saturating_sub(count + if allowed { 1 } else { 0 });

            if allowed {
                *entry.value_mut() = (count + 1, window_start);
            }

            // Calculate time until window reset
            let reset_in = self.window.saturating_sub(now.duration_since(window_start)).as_secs();

            RateLimitInfo {
                limit: self.max_requests,
                remaining,
                reset_in,
                allowed,
            }
        }
    }
}

/// Rate limiter that supports multiple algorithms
#[derive(Debug)]
pub struct RateLimiter {
    /// Token bucket rate limiter
    token_bucket: Option<TokenBucket>,

    /// Sliding window rate limiter
    sliding_window: Option<SlidingWindow>,

    /// Fixed window counter rate limiter
    fixed_window: Option<FixedWindowCounter>,

    /// Algorithm to use
    algorithm: RateLimitAlgorithm,

    /// Configuration
    config: RateLimitConfig,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let token_bucket = if config.algorithm == RateLimitAlgorithm::TokenBucket {
            Some(TokenBucket::new(config.max_requests, config.window))
        } else {
            None
        };

        let sliding_window = if config.algorithm == RateLimitAlgorithm::SlidingWindow {
            Some(SlidingWindow::new(config.max_requests, config.window))
        } else {
            None
        };

        let fixed_window = if config.algorithm == RateLimitAlgorithm::FixedWindowCounter {
            Some(FixedWindowCounter::new(config.max_requests, config.window))
        } else {
            None
        };

        Self {
            token_bucket,
            sliding_window,
            fixed_window,
            algorithm: config.algorithm,
            config,
        }
    }

    /// Check if a request is allowed
    pub fn is_allowed(&self, key: &str) -> ApiResult<RateLimitInfo> {
        let info = match self.algorithm {
            RateLimitAlgorithm::TokenBucket => self.token_bucket.as_ref().unwrap().is_allowed(key, 1),
            RateLimitAlgorithm::SlidingWindow => self.sliding_window.as_ref().unwrap().is_allowed(key),
            RateLimitAlgorithm::FixedWindowCounter => self.fixed_window.as_ref().unwrap().is_allowed(key),
        };

        if info.allowed {
            Ok(info)
        } else {
            Err(ApiError::TooManyRequests {
                message: format!("Rate limit exceeded. Try again in {} seconds", info.reset_in),
            })
        }
    }

    /// Get rate limit information without consuming a request
    pub fn peek(&self, key: &str) -> RateLimitInfo {
        match self.algorithm {
            RateLimitAlgorithm::TokenBucket => self.token_bucket.as_ref().unwrap().is_allowed(key, 0),
            RateLimitAlgorithm::SlidingWindow => self.sliding_window.as_ref().unwrap().is_allowed(key),
            RateLimitAlgorithm::FixedWindowCounter => self.fixed_window.as_ref().unwrap().is_allowed(key),
        }
    }
}

/// Global rate limiter manager
#[derive(Debug, Clone)]
pub struct RateLimiterManager {
    /// Rate limiters for different configurations
    limiters: Arc<DashMap<String, Arc<RateLimiter>>>,

    /// Default configuration
    default_config: RateLimitConfig,
}

impl RateLimiterManager {
    /// Create a new rate limiter manager
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            limiters: Arc::new(DashMap::new()),
            default_config,
        }
    }

    /// Get or create a rate limiter for a specific configuration
    pub fn get_limiter(&self, name: &str, config: Option<RateLimitConfig>) -> Arc<RateLimiter> {
        let config = config.unwrap_or_else(|| self.default_config.clone());

        if let Some(limiter) = self.limiters.get(name) {
            return limiter.clone();
        }

        let limiter = Arc::new(RateLimiter::new(config));
        self.limiters.insert(name.to_string(), limiter.clone());
        limiter
    }

    /// Check if a request is allowed for a specific rate limiter
    pub fn is_allowed(&self, limiter_name: &str, key: &str) -> ApiResult<RateLimitInfo> {
        let limiter = self.get_limiter(limiter_name, None);
        limiter.is_allowed(key)
    }
}

/// Serde helper for Duration serialization
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_token_bucket() {
        let bucket = TokenBucket::new(10, Duration::from_secs(1));
        let key = "test_key";

        // First request should be allowed
        let info = bucket.is_allowed(key, 1);
        assert!(info.allowed);
        assert_eq!(info.remaining, 9);

        // Second request should also be allowed
        let info = bucket.is_allowed(key, 1);
        assert!(info.allowed);
        assert_eq!(info.remaining, 8);
    }

    #[test]
    fn test_sliding_window() {
        let window = SlidingWindow::new(5, Duration::from_secs(1));
        let key = "test_key";

        // First 5 requests should be allowed
        for i in 0..5 {
            let info = window.is_allowed(key);
            assert!(info.allowed, "Request {} should be allowed", i);
        }

        // 6th request should be denied
        let info = window.is_allowed(key);
        assert!(!info.allowed);
    }

    #[test]
    fn test_fixed_window_counter() {
        let counter = FixedWindowCounter::new(3, Duration::from_secs(1));
        let key = "test_key";

        // First 3 requests should be allowed
        for i in 0..3 {
            let info = counter.is_allowed(key);
            assert!(info.allowed, "Request {} should be allowed", i);
        }

        // 4th request should be denied
        let info = counter.is_allowed(key);
        assert!(!info.allowed);
    }
}

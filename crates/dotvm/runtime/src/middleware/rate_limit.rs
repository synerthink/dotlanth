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

//! Rate limiting middleware for gRPC services

use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use tokio::sync::RwLock;
use tonic::{Request, Status};
use tracing::{warn, debug};

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub window_size: Duration,
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 1000,
            burst_size: 100,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            last_refill: Instant::now(),
            max_tokens,
            refill_rate,
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    fn tokens_available(&mut self) -> f64 {
        self.refill();
        self.tokens
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    buckets: DashMap<String, Arc<RwLock<TokenBucket>>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let limiter = Self {
            buckets: DashMap::new(),
            config,
        };

        // Start cleanup task
        let limiter_clone = Arc::new(limiter);
        let cleanup_limiter = limiter_clone.clone();
        tokio::spawn(async move {
            cleanup_limiter.cleanup_task().await;
        });

        Arc::try_unwrap(limiter_clone).unwrap_or_else(|_| unreachable!())
    }

    pub async fn check_rate_limit(&self, client_id: &str) -> Result<(), Status> {
        let bucket = self.get_or_create_bucket(client_id);
        let mut bucket_guard = bucket.write().await;

        if bucket_guard.try_consume(1.0) {
            debug!("Rate limit check passed for client: {}", client_id);
            Ok(())
        } else {
            warn!("Rate limit exceeded for client: {}", client_id);
            Err(Status::resource_exhausted(format!(
                "Rate limit exceeded. Try again in {} seconds",
                self.calculate_retry_after(&mut *bucket_guard)
            )))
        }
    }

    pub async fn get_remaining_tokens(&self, client_id: &str) -> f64 {
        let bucket = self.get_or_create_bucket(client_id);
        let mut bucket_guard = bucket.write().await;
        bucket_guard.tokens_available()
    }

    fn get_or_create_bucket(&self, client_id: &str) -> Arc<RwLock<TokenBucket>> {
        self.buckets
            .entry(client_id.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(TokenBucket::new(
                    self.config.burst_size as f64,
                    self.config.requests_per_minute as f64 / 60.0, // per second
                )))
            })
            .clone()
    }

    fn calculate_retry_after(&self, bucket: &mut TokenBucket) -> u64 {
        let tokens_needed = 1.0 - bucket.tokens_available();
        if tokens_needed <= 0.0 {
            0
        } else {
            (tokens_needed / bucket.refill_rate).ceil() as u64
        }
    }

    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(self.config.cleanup_interval);
        
        loop {
            interval.tick().await;
            self.cleanup_old_buckets().await;
        }
    }

    async fn cleanup_old_buckets(&self) {
        let cutoff = Instant::now() - self.config.window_size * 2;
        let mut to_remove = Vec::new();

        for entry in self.buckets.iter() {
            let bucket = entry.value();
            let bucket_guard = bucket.read().await;
            
            if bucket_guard.last_refill < cutoff {
                to_remove.push(entry.key().clone());
            }
        }

        for key in to_remove {
            self.buckets.remove(&key);
            debug!("Cleaned up rate limit bucket for: {}", key);
        }
    }
}

/// Rate limiting interceptor
pub struct RateLimitInterceptor {
    limiter: Arc<RateLimiter>,
    extract_client_id: Box<dyn Fn(&tonic::metadata::MetadataMap) -> String + Send + Sync>,
}

impl RateLimitInterceptor {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(config)),
            extract_client_id: Box::new(|metadata| {
                // Default: use IP address or user ID from metadata
                metadata
                    .get("x-forwarded-for")
                    .or_else(|| metadata.get("x-real-ip"))
                    .or_else(|| metadata.get("user-id"))
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string()
            }),
        }
    }

    pub fn with_client_id_extractor<F>(mut self, extractor: F) -> Self
    where
        F: Fn(&tonic::metadata::MetadataMap) -> String + Send + Sync + 'static,
    {
        self.extract_client_id = Box::new(extractor);
        self
    }

    pub async fn intercept<T>(&self, request: Request<T>) -> Result<Request<T>, Status> {
        let client_id = (self.extract_client_id)(request.metadata());
        
        self.limiter.check_rate_limit(&client_id).await?;
        
        Ok(request)
    }

    pub async fn get_client_stats(&self, client_id: &str) -> ClientRateLimitStats {
        let remaining = self.limiter.get_remaining_tokens(client_id).await;
        
        ClientRateLimitStats {
            client_id: client_id.to_string(),
            remaining_tokens: remaining,
            max_tokens: self.limiter.config.burst_size as f64,
            refill_rate: self.limiter.config.requests_per_minute as f64 / 60.0,
        }
    }
}

/// Client rate limit statistics
#[derive(Debug, Clone)]
pub struct ClientRateLimitStats {
    pub client_id: String,
    pub remaining_tokens: f64,
    pub max_tokens: f64,
    pub refill_rate: f64,
}

/// Global rate limiting middleware
pub struct GlobalRateLimiter {
    limiter: Arc<RateLimiter>,
}

impl GlobalRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: Arc::new(RateLimiter::new(config)),
        }
    }

    pub async fn check_global_limit(&self) -> Result<(), Status> {
        self.limiter.check_rate_limit("global").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0); // 10 tokens, 1 per second
        
        // Should be able to consume initial tokens
        assert!(bucket.try_consume(5.0));
        assert_eq!(bucket.tokens, 5.0);
        
        // Should not be able to consume more than available
        assert!(!bucket.try_consume(10.0));
        assert_eq!(bucket.tokens, 5.0);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            requests_per_minute: 60, // 1 per second
            burst_size: 5,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300),
        };
        
        let limiter = RateLimiter::new(config);
        
        // Should allow initial burst
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("test_client").await.is_ok());
        }
        
        // Should reject next request
        assert!(limiter.check_rate_limit("test_client").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_interceptor() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 2,
            window_size: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(300),
        };
        
        let interceptor = RateLimitInterceptor::new(config);
        
        let mut request = Request::new(());
        request.metadata_mut().insert("user-id", "test_user".parse().unwrap());
        
        // Should allow first requests
        assert!(interceptor.intercept(request.clone()).await.is_ok());
        assert!(interceptor.intercept(request.clone()).await.is_ok());
        
        // Should reject third request
        assert!(interceptor.intercept(request).await.is_err());
    }
}
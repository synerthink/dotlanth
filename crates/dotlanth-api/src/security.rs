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

//! Security middleware for the REST API gateway
//! Implements various security measures:
//! - Request/response sanitization
//! - Security headers
//! - DDoS protection
//! - API key management
//! - Request size limiting

use crate::auth::{AuthService, Claims};
use crate::error::{ApiError, ApiResult};
use crate::rate_limiting::{RateLimitAlgorithm, RateLimitConfig, RateLimiterManager};
use dashmap::DashMap;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, Response, StatusCode, header};
use hyper_util::rt::TokioIo;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::Mutex;
use tower::{Layer, Service};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{debug, error, warn};

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable request sanitization
    pub enable_sanitization: bool,

    /// Enable security headers
    pub enable_security_headers: bool,

    /// Enable DDoS protection
    pub enable_ddos_protection: bool,

    /// Enable API key management
    pub enable_api_keys: bool,

    /// Enable request size limiting
    pub enable_request_size_limiting: bool,

    /// Maximum request body size in bytes
    pub max_body_size: usize,

    /// Rate limit configuration for general requests
    pub rate_limit_config: RateLimitConfig,

    /// Rate limit configuration for authenticated requests
    pub authenticated_rate_limit_config: RateLimitConfig,

    /// DDoS protection threshold (requests per second)
    pub ddos_threshold: u32,

    /// DDoS protection window
    pub ddos_window: Duration,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_sanitization: true,
            enable_security_headers: true,
            enable_ddos_protection: true,
            enable_api_keys: true,
            enable_request_size_limiting: true,
            max_body_size: 10 * 1024 * 1024, // 10MB
            rate_limit_config: RateLimitConfig {
                max_requests: 100,
                window: Duration::from_secs(60),
                algorithm: RateLimitAlgorithm::SlidingWindow,
                per_ip: true,
                per_user: false,
                per_api_key: false,
            },
            authenticated_rate_limit_config: RateLimitConfig {
                max_requests: 1000,
                window: Duration::from_secs(60),
                algorithm: RateLimitAlgorithm::SlidingWindow,
                per_ip: false,
                per_user: true,
                per_api_key: true,
            },
            ddos_threshold: 1000,
            ddos_window: Duration::from_secs(1),
        }
    }
}

/// API key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// API key ID
    pub id: String,

    /// API key value (hashed)
    pub key_hash: String,

    /// User ID associated with this key
    pub user_id: String,

    /// Key name/description
    pub name: String,

    /// Permissions granted by this key
    pub permissions: Vec<String>,

    /// Whether this key is active
    pub is_active: bool,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last used timestamp
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,

    /// Rate limit for this key (overrides default)
    pub rate_limit: Option<RateLimitConfig>,
}

/// Security middleware
#[derive(Clone)]
pub struct SecurityMiddleware<S> {
    inner: S,
    config: SecurityConfig,
    auth_service: Arc<TokioMutex<AuthService>>,
    rate_limiter_manager: Arc<RateLimiterManager>,
    api_keys: Arc<DashMap<String, ApiKey>>,
    /// DDoS protection tracking
    ddos_tracker: Arc<DashMap<IpAddr, Vec<Instant>>>,
    /// Blocked IPs
    blocked_ips: Arc<RwLock<HashMap<IpAddr, Instant>>>,
}

impl<S> SecurityMiddleware<S> {
    /// Create a new security middleware
    pub fn new(inner: S, config: SecurityConfig, auth_service: Arc<TokioMutex<AuthService>>) -> Self {
        let rate_limiter_manager = Arc::new(RateLimiterManager::new(config.rate_limit_config.clone()));

        Self {
            inner,
            config,
            auth_service,
            rate_limiter_manager,
            api_keys: Arc::new(DashMap::new()),
            ddos_tracker: Arc::new(DashMap::new()),
            blocked_ips: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Sanitize request data
    fn sanitize_request<ReqBody>(&self, req: &mut Request<ReqBody>) -> ApiResult<()> {
        if !self.config.enable_sanitization {
            return Ok(());
        }

        // Sanitize headers
        let headers = req.headers_mut();
        let mut headers_to_remove = Vec::new();

        for (name, _) in headers.iter() {
            // Remove potentially dangerous headers
            let name_str = name.as_str().to_lowercase();
            if name_str.contains("xss") || name_str.contains("script") || name_str.contains("javascript") || name_str.starts_with("x-forwarded") {
                headers_to_remove.push(name.clone());
            }
        }

        for name in headers_to_remove {
            headers.remove(name);
        }

        Ok(())
    }

    /// Add security headers to response
    fn add_security_headers(&self, response: &mut Response<Full<Bytes>>) {
        if !self.config.enable_security_headers {
            return;
        }

        let headers = response.headers_mut();

        // Content Security Policy
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; object-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; media-src 'self';",
            ),
        );

        // X-Content-Type-Options
        headers.insert(header::HeaderName::from_static("x-content-type-options"), header::HeaderValue::from_static("nosniff"));

        // X-Frame-Options
        headers.insert(header::HeaderName::from_static("x-frame-options"), header::HeaderValue::from_static("DENY"));

        // X-XSS-Protection
        headers.insert(header::HeaderName::from_static("x-xss-protection"), header::HeaderValue::from_static("1; mode=block"));

        // Strict-Transport-Security
        headers.insert(
            header::HeaderName::from_static("strict-transport-security"),
            header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );

        // Referrer-Policy
        headers.insert(header::HeaderName::from_static("referrer-policy"), header::HeaderValue::from_static("strict-origin-when-cross-origin"));

        // Permissions-Policy
        headers.insert(
            header::HeaderName::from_static("permissions-policy"),
            header::HeaderValue::from_static(
                "geolocation=(), midi=(), notifications=(), push=(), sync-xhr=(), microphone=(), camera=(), magnetometer=(), gyroscope=(), speaker=(), vibrate=(), fullscreen=(), payment=()",
            ),
        );
    }

    /// Check DDoS protection
    fn check_ddos_protection(&self, ip: IpAddr) -> ApiResult<()> {
        if !self.config.enable_ddos_protection {
            return Ok(());
        }

        // Check if IP is blocked
        {
            let blocked_ips = self.blocked_ips.read();
            if let Some(blocked_until) = blocked_ips.get(&ip) {
                if Instant::now() < *blocked_until {
                    return Err(ApiError::TooManyRequests {
                        message: "IP temporarily blocked due to DDoS protection".to_string(),
                    });
                }
            }
        }

        let now = Instant::now();
        let window_start = now - self.config.ddos_window;

        // Get or create request history for this IP
        let mut requests = self.ddos_tracker.entry(ip).or_insert_with(Vec::new);

        // Remove old requests outside the window
        requests.retain(|&timestamp| timestamp > window_start);

        // Add current request
        requests.push(now);

        // Check if we've exceeded the threshold
        if requests.len() as u32 > self.config.ddos_threshold {
            // Block this IP for 10 minutes
            let mut blocked_ips = self.blocked_ips.write();
            blocked_ips.insert(ip, now + Duration::from_secs(600));

            warn!("DDoS protection triggered for IP: {}", ip);

            return Err(ApiError::TooManyRequests {
                message: "DDoS protection triggered. IP temporarily blocked.".to_string(),
            });
        }

        Ok(())
    }

    /// Validate API key
    pub fn validate_api_key(&self, key: &str) -> ApiResult<ApiKey> {
        if !self.config.enable_api_keys {
            return Err(ApiError::Unauthorized {
                message: "API key authentication is disabled".to_string(),
            });
        }

        // Hash the provided key for comparison with stored hashes
        let key_hash = base64::encode(ring::digest::digest(&ring::digest::SHA256, key.as_bytes()).as_ref());

        // Check if the hashed key exists in our map
        if let Some(api_key) = self.api_keys.get(&key_hash) {
            if !api_key.is_active {
                return Err(ApiError::Unauthorized {
                    message: "API key is inactive".to_string(),
                });
            }

            // Update last used timestamp
            // We use DashMap's get_mut API to atomically update the last_used field
            // This provides thread-safe in-memory updates without requiring external storage
            // In a production implementation with persistent storage, we would also update
            // the database record to track API key usage across server restarts
            if let Some(mut entry) = self.api_keys.get_mut(&key_hash) {
                let mut api_key = entry.value().clone();
                api_key.last_used = Some(chrono::Utc::now());
                *entry.value_mut() = api_key;
            }

            tracing::debug!("API key {} used by user {}", api_key.id, api_key.user_id);

            return Ok(api_key.clone());
        }

        Err(ApiError::Unauthorized {
            message: "Invalid API key".to_string(),
        })
    }

    /// Get client IP address
    fn get_client_ip<ReqBody>(&self, req: &Request<ReqBody>) -> Option<IpAddr> {
        // Try to get IP from X-Forwarded-For header
        if let Some(forwarded) = req.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }

        // Try to get IP from X-Real-IP header
        if let Some(real_ip) = req.headers().get("x-real-ip") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                if let Ok(ip) = real_ip_str.parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }

        // For direct connections, we would get the peer address from the connection
        // This is not available in the middleware context, so we return None

        None
    }

    /// Apply rate limiting
    async fn apply_rate_limiting<ReqBody>(&self, req: &Request<ReqBody>, claims: Option<&Claims>) -> ApiResult<()> {
        // Get client IP
        let client_ip = self.get_client_ip(req);

        // Get and validate API key if present
        let validated_api_key = if let Some(api_key_header) = req.headers().get("x-api-key") {
            if let Ok(api_key_str) = api_key_header.to_str() {
                // Validate the API key
                match self.validate_api_key(api_key_str) {
                    Ok(api_key) => Some(api_key),
                    Err(_) => {
                        // Invalid API key
                        return Err(ApiError::Unauthorized {
                            message: "Invalid API key".to_string(),
                        });
                    }
                }
            } else {
                // Invalid header encoding
                return Err(ApiError::Unauthorized {
                    message: "Invalid API key header encoding".to_string(),
                });
            }
        } else {
            None
        };

        // Determine rate limiting key
        let rate_limit_key = if let Some(claims) = claims {
            // For authenticated users, use user ID
            format!("user:{}", claims.sub)
        } else if validated_api_key.is_some() {
            // For validated API keys, use the API key ID
            format!("api_key:{}", validated_api_key.as_ref().unwrap().id)
        } else if let Some(ip) = client_ip {
            // For IPs, use IP address
            format!("ip:{}", ip)
        } else {
            // Fallback to path-based key
            format!("path:{}", req.uri().path())
        };

        // Choose appropriate rate limiter
        let limiter_name = if claims.is_some() || validated_api_key.is_some() { "authenticated" } else { "general" };

        // Get custom rate limit config for API key if applicable
        let custom_config = if let Some(api_key) = &validated_api_key { api_key.rate_limit.clone() } else { None };

        // Apply rate limiting
        let limiter = self.rate_limiter_manager.get_limiter(
            limiter_name,
            custom_config.or_else(|| {
                if limiter_name == "authenticated" {
                    Some(self.config.authenticated_rate_limit_config.clone())
                } else {
                    Some(self.config.rate_limit_config.clone())
                }
            }),
        );

        limiter.is_allowed(&rate_limit_key)?;

        Ok(())
    }

    /// Check request size
    fn check_request_size<ReqBody>(&self, req: &Request<ReqBody>) -> ApiResult<()> {
        if !self.config.enable_request_size_limiting {
            return Ok(());
        }

        // For Hyper's streaming body, we can't easily check the size without consuming it
        // In a production implementation, we would use tower-http's RequestBodyLimitLayer
        // or a similar middleware that limits body size before it reaches application logic.
        // As a first line of defense, we check if Content-Length header exceeds our limit.
        // For more robust protection, the server should be configured with body size limits
        // at the HTTP server level.
        if let Some(content_length) = req.headers().get(hyper::header::CONTENT_LENGTH) {
            if let Ok(length_str) = content_length.to_str() {
                if let Ok(length) = length_str.parse::<usize>() {
                    if length > self.config.max_body_size {
                        return Err(ApiError::BadRequest {
                            message: format!("Request body size {} exceeds maximum allowed size {}", length, self.config.max_body_size),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

impl<S> Service<Request<hyper::body::Incoming>> for SecurityMiddleware<S>
where
    S: Service<Request<hyper::body::Incoming>, Response = Response<Full<Bytes>>> + Clone + Send + Sync + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<hyper::body::Incoming>) -> Self::Future {
        let inner = self.inner.clone();
        let config = self.config.clone();
        let auth_service = self.auth_service.clone();
        let rate_limiter_manager = self.rate_limiter_manager.clone();
        let api_keys = self.api_keys.clone();
        let ddos_tracker = self.ddos_tracker.clone();
        let blocked_ips = self.blocked_ips.clone();

        Box::pin(async move {
            // Create a new instance for this request
            let mut middleware = SecurityMiddleware {
                inner,
                config,
                auth_service,
                rate_limiter_manager,
                api_keys,
                ddos_tracker,
                blocked_ips,
            };

            // Get client IP for DDoS protection
            let client_ip = middleware.get_client_ip(&req);

            // Apply DDoS protection
            if let Some(ip) = client_ip {
                if let Err(e) = middleware.check_ddos_protection(ip) {
                    return Ok(e.into());
                }
            }

            // Check request size
            if let Err(e) = middleware.check_request_size(&req) {
                return Ok(e.into());
            }

            // Sanitize request
            if let Err(e) = middleware.sanitize_request(&mut req) {
                return Ok(e.into());
            }

            // Extract claims if available (from previous auth middleware)
            let claims = req.extensions().get::<Claims>().cloned();

            // Apply rate limiting
            if let Err(e) = middleware.apply_rate_limiting(&req, claims.as_ref()).await {
                return Ok(e.into());
            }

            // Call the inner service
            let response = middleware.inner.call(req).await.map_err(Into::into)?;

            // Convert response to add security headers
            let mut response = response;
            middleware.add_security_headers(&mut response);

            Ok(response)
        })
    }
}

/// Security middleware layer
#[derive(Clone)]
pub struct SecurityLayer {
    config: SecurityConfig,
    auth_service: Arc<TokioMutex<AuthService>>,
}

impl SecurityLayer {
    /// Create a new security layer
    pub fn new(config: SecurityConfig, auth_service: Arc<TokioMutex<AuthService>>) -> Self {
        Self { config, auth_service }
    }
}

impl<S> Layer<S> for SecurityLayer {
    type Service = SecurityMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityMiddleware::new(inner, self.config.clone(), self.auth_service.clone())
    }
}

/// API key management service
#[derive(Debug, Clone)]
pub struct ApiKeyService {
    api_keys: Arc<DashMap<String, ApiKey>>,
}

impl ApiKeyService {
    /// Create a new API key service
    pub fn new() -> Self {
        Self { api_keys: Arc::new(DashMap::new()) }
    }

    /// Generate a new API key
    pub fn generate_key(&self, user_id: String, name: String, permissions: Vec<String>) -> ApiResult<(String, ApiKey)> {
        use ring::rand::{SecureRandom, SystemRandom};

        // Generate a random key
        let rng = SystemRandom::new();
        let mut key_bytes = vec![0u8; 32];
        rng.fill(&mut key_bytes).map_err(|_| ApiError::InternalServerError {
            message: "Failed to generate random API key".to_string(),
        })?;

        let key = base64::encode(&key_bytes);

        // Hash the key for storage
        let key_hash = base64::encode(ring::digest::digest(&ring::digest::SHA256, key.as_bytes()).as_ref());

        let api_key = ApiKey {
            id: uuid::Uuid::new_v4().to_string(),
            key_hash: key_hash.clone(),
            user_id,
            name,
            permissions,
            is_active: true,
            created_at: chrono::Utc::now(),
            last_used: None,
            rate_limit: None,
        };

        self.api_keys.insert(key_hash, api_key.clone());

        Ok((key, api_key))
    }

    /// Get an API key by its value
    pub fn get_key(&self, key: &str) -> Option<ApiKey> {
        // Hash the provided key for comparison
        let key_hash = base64::encode(ring::digest::digest(&ring::digest::SHA256, key.as_bytes()).as_ref());

        self.api_keys.get(&key_hash).map(|k| k.clone())
    }

    /// Revoke an API key
    pub fn revoke_key(&self, key_id: &str) -> bool {
        // Find the key by ID and remove it
        let mut to_remove = None;
        for entry in self.api_keys.iter() {
            if entry.value().id == key_id {
                to_remove = Some(entry.key().clone());
                break;
            }
        }

        if let Some(key) = to_remove {
            self.api_keys.remove(&key);
            true
        } else {
            false
        }
    }

    /// List API keys for a user
    pub fn list_keys(&self, user_id: &str) -> Vec<ApiKey> {
        self.api_keys.iter().filter(|entry| entry.value().user_id == user_id).map(|entry| entry.value().clone()).collect()
    }
}

impl Default for ApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Method, body};

    #[test]
    fn test_api_key_generation() {
        let service = ApiKeyService::new();
        let (key, api_key) = service
            .generate_key("user123".to_string(), "Test Key".to_string(), vec!["read".to_string(), "write".to_string()])
            .unwrap();

        assert!(!key.is_empty());
        assert_eq!(api_key.user_id, "user123");
        assert_eq!(api_key.name, "Test Key");
        assert_eq!(api_key.permissions.len(), 2);
    }

    #[test]
    fn test_api_key_retrieval() {
        let service = ApiKeyService::new();
        let (key, _) = service.generate_key("user123".to_string(), "Test Key".to_string(), vec!["read".to_string()]).unwrap();

        // Test that we can retrieve the key using the service method
        let keys = service.list_keys("user123");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].user_id, "user123");
    }

    #[test]
    fn test_rate_limiter_token_bucket() {
        let config = crate::rate_limiting::RateLimitConfig {
            max_requests: 10,
            window: std::time::Duration::from_secs(1),
            algorithm: crate::rate_limiting::RateLimitAlgorithm::TokenBucket,
            per_ip: true,
            per_user: false,
            per_api_key: false,
        };

        let limiter = crate::rate_limiting::RateLimiter::new(config);
        let key = "test_user";

        // First 10 requests should be allowed
        for i in 0..10 {
            let result = limiter.is_allowed(key);
            assert!(result.is_ok(), "Request {} should be allowed", i);
            assert!(result.unwrap().allowed, "Request {} should be allowed", i);
        }

        // 11th request should be denied
        let result = limiter.is_allowed(key);
        assert!(result.is_err(), "11th request should be denied");
    }

    #[test]
    fn test_rate_limiter_sliding_window() {
        let config = crate::rate_limiting::RateLimitConfig {
            max_requests: 5,
            window: std::time::Duration::from_secs(1),
            algorithm: crate::rate_limiting::RateLimitAlgorithm::SlidingWindow,
            per_ip: true,
            per_user: false,
            per_api_key: false,
        };

        let limiter = crate::rate_limiting::RateLimiter::new(config);
        let key = "test_user";

        // First 5 requests should be allowed
        for i in 0..5 {
            let result = limiter.is_allowed(key);
            assert!(result.is_ok(), "Request {} should be allowed", i);
            assert!(result.unwrap().allowed, "Request {} should be allowed", i);
        }

        // 6th request should be denied
        let result = limiter.is_allowed(key);
        assert!(result.is_err(), "6th request should be denied");
    }

    #[test]
    fn test_rate_limiter_fixed_window() {
        let config = crate::rate_limiting::RateLimitConfig {
            max_requests: 3,
            window: std::time::Duration::from_secs(1),
            algorithm: crate::rate_limiting::RateLimitAlgorithm::FixedWindowCounter,
            per_ip: true,
            per_user: false,
            per_api_key: false,
        };

        let limiter = crate::rate_limiting::RateLimiter::new(config);
        let key = "test_user";

        // First 3 requests should be allowed
        for i in 0..3 {
            let result = limiter.is_allowed(key);
            assert!(result.is_ok(), "Request {} should be allowed", i);
            assert!(result.unwrap().allowed, "Request {} should be allowed", i);
        }

        // 4th request should be denied
        let result = limiter.is_allowed(key);
        assert!(result.is_err(), "4th request should be denied");
    }

    #[test]
    fn test_api_key_service() {
        let service = ApiKeyService::new();

        // Test key generation
        let (key, api_key) = service
            .generate_key("user123".to_string(), "Test Key".to_string(), vec!["read".to_string(), "write".to_string()])
            .unwrap();

        // Test listing keys
        let keys = service.list_keys("user123");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].id, api_key.id);

        // Test retrieving valid key
        let result = service.get_key(&key);
        assert!(result.is_some());
        let validated_key = result.unwrap();
        assert_eq!(validated_key.id, api_key.id);
        assert_eq!(validated_key.user_id, "user123");

        // Test retrieving invalid key
        let result = service.get_key("invalid_key");
        assert!(result.is_none());

        // Revoke the key
        let revoked = service.revoke_key(&api_key.id);
        assert!(revoked);
    }

    #[test]
    fn test_request_size_checking() {
        use http_body_util::Full;
        use hyper::body::Bytes;
        use hyper::{HeaderMap, Request};

        // Create a mock auth service
        let auth_service = Arc::new(TokioMutex::new(crate::auth::AuthService::new("test_secret")));

        // Create security config with request size limiting enabled
        let config = SecurityConfig {
            enable_request_size_limiting: true,
            max_body_size: 1024, // 1KB limit
            ..Default::default()
        };

        // Create security middleware
        let middleware = SecurityMiddleware::new((), config, auth_service);

        // Create a request with Content-Length header within limit
        let req = Request::builder()
            .method(hyper::Method::POST)
            .uri("http://example.com/test")
            .header(hyper::header::CONTENT_LENGTH, "512")
            .body(Full::<Bytes>::new(Bytes::new()))
            .unwrap();

        // This should pass
        assert!(middleware.check_request_size(&req).is_ok());

        // Create a request with Content-Length header exceeding limit
        let req = Request::builder()
            .method(hyper::Method::POST)
            .uri("http://example.com/test")
            .header(hyper::header::CONTENT_LENGTH, "2048")
            .body(Full::<Bytes>::new(Bytes::new()))
            .unwrap();

        // This should fail
        assert!(middleware.check_request_size(&req).is_err());
    }
}

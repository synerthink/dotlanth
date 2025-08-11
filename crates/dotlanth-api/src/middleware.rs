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

//! Middleware for the REST API gateway

use crate::auth::{AuthService, Claims, extract_token_from_header};
use crate::error::{ApiError, ApiResult};
use http_body_util::Full;
use hyper::{Request, Response, body::Bytes};
use std::sync::Arc;
use std::time::Instant;
use tower::{Layer, Service};
use tracing::{error, info, warn};

/// Authentication middleware
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    auth_service: Arc<tokio::sync::Mutex<AuthService>>,
    public_paths: Vec<String>,
}

impl<S> AuthMiddleware<S> {
    /// Create a new authentication middleware
    pub fn new(inner: S, auth_service: Arc<tokio::sync::Mutex<AuthService>>) -> Self {
        let public_paths = vec![
            "/api/v1/health".to_string(),
            "/api/v1/version".to_string(),
            "/api/v1/auth/login".to_string(),
            "/docs".to_string(),
            "/docs/".to_string(),
            "/api-docs".to_string(),
            "/openapi.json".to_string(),
        ];

        Self { inner, auth_service, public_paths }
    }

    /// Check if a path is public (doesn't require authentication)
    fn is_public_path(&self, path: &str) -> bool {
        self.public_paths.iter().any(|public_path| path == public_path || path.starts_with(&format!("{}/", public_path)))
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AuthMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let inner = self.inner.clone();
        let auth_service = self.auth_service.clone();
        let public_paths = self.public_paths.clone();

        Box::pin(async move {
            let path = req.uri().path();

            // Check if this is a public path
            let is_public = public_paths.iter().any(|public_path| path == public_path || path.starts_with(&format!("{}/", public_path)));

            if !is_public {
                // Extract and validate JWT token
                if let Some(auth_header) = req.headers().get("authorization") {
                    match auth_header.to_str() {
                        Ok(auth_str) => {
                            match extract_token_from_header(auth_str) {
                                Ok(token) => {
                                    let auth_service = auth_service.lock().await;
                                    match auth_service.validate_token(token) {
                                        Ok(claims) => {
                                            // Add claims to request extensions
                                            req.extensions_mut().insert(claims);
                                        }
                                        Err(e) => {
                                            error!("Token validation failed: {}", e);
                                            let error_response = Response::from(ApiError::Unauthorized {
                                                message: "Invalid or expired token".to_string(),
                                            });
                                            // This is a type conversion issue, but for now we'll return the error
                                            return Err(format!("Authentication failed: {}", e).into());
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Invalid authorization header format: {}", e);
                                    return Err(format!("Invalid authorization header: {}", e).into());
                                }
                            }
                        }
                        Err(_) => {
                            warn!("Authorization header contains invalid UTF-8");
                            return Err("Invalid authorization header encoding".into());
                        }
                    }
                } else {
                    warn!("Missing authorization header for protected path: {}", path);
                    return Err("Missing authorization header".into());
                }
            }

            // Call the inner service
            let mut inner_service = inner;
            inner_service.call(req).await.map_err(Into::into)
        })
    }
}

/// Authentication middleware layer
pub struct AuthLayer {
    auth_service: Arc<tokio::sync::Mutex<AuthService>>,
}

impl AuthLayer {
    /// Create a new authentication layer
    pub fn new(auth_service: Arc<tokio::sync::Mutex<AuthService>>) -> Self {
        Self { auth_service }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware::new(inner, self.auth_service.clone())
    }
}

/// Request logging middleware
#[derive(Clone)]
pub struct LoggingMiddleware<S> {
    inner: S,
}

impl<S> LoggingMiddleware<S> {
    /// Create a new logging middleware
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for LoggingMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let inner = self.inner.clone();
        let method = req.method().clone();
        let uri = req.uri().clone();
        let start_time = Instant::now();

        Box::pin(async move {
            info!("Request: {} {}", method, uri);

            let mut inner_service = inner;
            let result = inner_service.call(req).await;

            let duration = start_time.elapsed();

            match &result {
                Ok(response) => {
                    info!("Response: {} {} - {} in {:?}", method, uri, response.status(), duration);
                }
                Err(e) => {
                    error!("Error: {} {} - {} in {:?}", method, uri, "error", duration);
                }
            }

            result.map_err(Into::into)
        })
    }
}

/// Logging middleware layer
pub struct LoggingLayer;

impl LoggingLayer {
    /// Create a new logging layer
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for LoggingLayer {
    type Service = LoggingMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggingMiddleware::new(inner)
    }
}

/// Extract authenticated user claims from request
pub fn extract_claims(req: &Request<impl hyper::body::Body>) -> ApiResult<&Claims> {
    req.extensions().get::<Claims>().ok_or_else(|| ApiError::Unauthorized {
        message: "No authentication information found".to_string(),
    })
}

/// Check if the authenticated user has required permissions
pub fn check_permissions(claims: &Claims, required_permissions: &[&str]) -> ApiResult<()> {
    for permission in required_permissions {
        if !claims.has_permission(permission) {
            return Err(ApiError::Forbidden {
                message: format!("Missing required permission: {}", permission),
            });
        }
    }
    Ok(())
}

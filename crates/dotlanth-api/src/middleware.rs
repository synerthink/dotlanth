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

use crate::auth::{AuthService, Claims, JWTAuthSystem, extract_token_from_header};
use crate::error::{ApiError, ApiResult};
use crate::versioning::{CompatibilityChecker, DeprecationManager, ProtocolType, SchemaEvolutionManager, ServiceType, VersionContext, VersionNegotiator, VersionRegistry};
use http_body_util::Full;
use hyper::{HeaderMap, Request, Response, body::Bytes, body::Incoming};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
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

/// Versioning middleware for handling API version negotiation and compatibility
#[derive(Clone)]
pub struct VersioningMiddleware {
    negotiator: VersionNegotiator,
    compatibility_checker: Arc<RwLock<CompatibilityChecker>>,
    deprecation_manager: Arc<RwLock<DeprecationManager>>,
    schema_manager: Arc<RwLock<SchemaEvolutionManager>>,
}

impl VersioningMiddleware {
    /// Create new versioning middleware
    pub fn new(registry: VersionRegistry, compatibility_checker: CompatibilityChecker, deprecation_manager: DeprecationManager, schema_manager: SchemaEvolutionManager) -> Self {
        Self {
            negotiator: VersionNegotiator::new(registry),
            compatibility_checker: Arc::new(RwLock::new(compatibility_checker)),
            deprecation_manager: Arc::new(RwLock::new(deprecation_manager)),
            schema_manager: Arc::new(RwLock::new(schema_manager)),
        }
    }

    /// Process versioning for incoming request
    pub async fn process_request(&self, mut req: Request<Incoming>, service: ServiceType) -> ApiResult<(Request<Incoming>, VersionContext)> {
        let headers = req.headers();

        // Negotiate version
        let negotiation_result = self.negotiator.negotiate_from_headers(headers, ProtocolType::Rest, service.clone()).map_err(|e| ApiError::BadRequest {
            message: format!("Version negotiation failed: {}", e),
        })?;

        // Extract client preferences for context
        let client_prefs = self.extract_client_preferences(headers)?;

        // Create version context
        let version_context = VersionContext::new(negotiation_result, client_prefs);

        // Check deprecation warnings
        let deprecation_manager = self.deprecation_manager.read().await;
        let used_features = self.extract_used_features(&req, &service);
        let deprecation_warnings = deprecation_manager.generate_warnings(&ProtocolType::Rest, &service, &version_context.negotiated_version, &used_features);

        if !deprecation_warnings.is_empty() {
            for warning in &deprecation_warnings {
                warn!("Deprecation warning: {}", warning);
            }
        }

        // Add version context to request extensions
        req.extensions_mut().insert(version_context.clone());

        info!(
            "Request processed with API version {} for {}/{}",
            version_context.negotiated_version, version_context.protocol, version_context.service
        );

        Ok((req, version_context))
    }

    /// Validate request data against schema
    pub async fn validate_request_data(&self, version_context: &VersionContext, data: &Value) -> ApiResult<()> {
        let mut schema_manager = self.schema_manager.write().await;

        schema_manager
            .validate_data(&version_context.protocol, &version_context.service, &version_context.negotiated_version, data)
            .map_err(|e| ApiError::BadRequest {
                message: format!("Schema validation failed: {}", e),
            })?;

        Ok(())
    }

    /// Check compatibility for request features
    pub async fn check_request_compatibility(&self, version_context: &VersionContext, features: &[String]) -> ApiResult<()> {
        let compatibility_checker = self.compatibility_checker.read().await;

        compatibility_checker
            .validate_request_compatibility(&version_context.protocol, &version_context.service, &version_context.negotiated_version, features)
            .map_err(|e| ApiError::BadRequest {
                message: format!("Compatibility check failed: {}", e),
            })?;

        Ok(())
    }

    /// Extract client preferences from headers
    fn extract_client_preferences(&self, headers: &HeaderMap) -> ApiResult<crate::versioning::ClientVersionPreferences> {
        // This would typically parse Accept-Version headers or similar
        // For now, return default preferences
        Ok(crate::versioning::ClientVersionPreferences::default())
    }

    /// Extract used features from request
    fn extract_used_features(&self, _req: &Request<Incoming>, service: &ServiceType) -> Vec<String> {
        // Extract features based on request path, body, etc.
        // For now, return common features based on service
        match service {
            ServiceType::Vm => vec!["execute_dot".to_string(), "deploy_dot".to_string()],
            ServiceType::Database => vec!["get".to_string(), "put".to_string()],
            _ => vec![],
        }
    }

    /// Get compatibility checker
    pub async fn compatibility_checker(&self) -> tokio::sync::RwLockReadGuard<'_, CompatibilityChecker> {
        self.compatibility_checker.read().await
    }

    /// Get deprecation manager
    pub async fn deprecation_manager(&self) -> tokio::sync::RwLockReadGuard<'_, DeprecationManager> {
        self.deprecation_manager.read().await
    }

    /// Get schema manager
    pub async fn schema_manager(&self) -> tokio::sync::RwLockReadGuard<'_, SchemaEvolutionManager> {
        self.schema_manager.read().await
    }
}

/// Extension trait for extracting version context from requests
pub trait VersionContextExt {
    fn version_context(&self) -> Option<&VersionContext>;
}

impl<T> VersionContextExt for Request<T> {
    fn version_context(&self) -> Option<&VersionContext> {
        self.extensions().get::<VersionContext>()
    }
}

/// JWT Authentication middleware for protecting routes
#[derive(Clone)]
pub struct JwtMiddleware<S> {
    inner: S,
    jwt_auth: Arc<JWTAuthSystem>,
    public_paths: Vec<String>,
    required_permissions: Vec<String>,
}

impl<S> JwtMiddleware<S> {
    /// Create a new JWT middleware
    pub fn new(inner: S, jwt_auth: Arc<JWTAuthSystem>) -> Self {
        let public_paths = vec![
            "/api/v1/health".to_string(),
            "/api/v1/version".to_string(),
            "/api/v1/auth/login".to_string(),
            "/api/v1/auth/register".to_string(),
            "/api/v1/auth/refresh".to_string(),
            "/docs".to_string(),
            "/docs/".to_string(),
            "/api-docs".to_string(),
            "/openapi.json".to_string(),
        ];

        Self {
            inner,
            jwt_auth,
            public_paths,
            required_permissions: vec![],
        }
    }

    /// Create JWT middleware with required permissions
    pub fn with_permissions(inner: S, jwt_auth: Arc<JWTAuthSystem>, permissions: Vec<String>) -> Self {
        let mut middleware = Self::new(inner, jwt_auth);
        middleware.required_permissions = permissions;
        middleware
    }

    /// Check if path is public (doesn't require authentication)
    fn is_public_path(&self, path: &str) -> bool {
        self.public_paths.iter().any(|public_path| path == public_path || path.starts_with(&format!("{}/", public_path)))
    }
}

impl<S> Service<Request<Incoming>> for JwtMiddleware<S>
where
    S: Service<Request<Incoming>, Response = Response<Full<Bytes>>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response<Full<Bytes>>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<Incoming>) -> Self::Future {
        let jwt_auth = self.jwt_auth.clone();
        let public_paths = self.public_paths.clone();
        let required_permissions = self.required_permissions.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let path = req.uri().path();

            // Check if this is a public path
            let is_public = public_paths.iter().any(|public_path| path == public_path || path.starts_with(&format!("{}/", public_path)));

            if is_public {
                return inner.call(req).await.map_err(Into::into);
            }

            // Extract and validate JWT token
            let auth_header = req.headers().get("authorization").and_then(|h| h.to_str().ok()).ok_or_else(|| ApiError::Unauthorized {
                message: "Missing authorization header".to_string(),
            })?;

            let token = extract_token_from_header(auth_header).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let claims = jwt_auth.validate_token(token).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            // Check required permissions
            for permission in &required_permissions {
                if !claims.has_permission(permission) {
                    return Err(Box::new(ApiError::Forbidden {
                        message: format!("Missing required permission: {}", permission),
                    }) as Box<dyn std::error::Error + Send + Sync>);
                }
            }

            // Add claims to request extensions for use in handlers
            req.extensions_mut().insert(claims);

            inner.call(req).await.map_err(Into::into)
        })
    }
}

/// JWT middleware layer
#[derive(Clone)]
pub struct JwtMiddlewareLayer {
    jwt_auth: Arc<JWTAuthSystem>,
    required_permissions: Vec<String>,
}

impl JwtMiddlewareLayer {
    /// Create a new JWT middleware layer
    pub fn new(jwt_auth: Arc<JWTAuthSystem>) -> Self {
        Self {
            jwt_auth,
            required_permissions: vec![],
        }
    }

    /// Create JWT middleware layer with required permissions
    pub fn with_permissions(jwt_auth: Arc<JWTAuthSystem>, permissions: Vec<String>) -> Self {
        Self {
            jwt_auth,
            required_permissions: permissions,
        }
    }
}

impl<S> Layer<S> for JwtMiddlewareLayer {
    type Service = JwtMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        if self.required_permissions.is_empty() {
            JwtMiddleware::new(inner, self.jwt_auth.clone())
        } else {
            JwtMiddleware::with_permissions(inner, self.jwt_auth.clone(), self.required_permissions.clone())
        }
    }
}

/// Extension trait for extracting JWT claims from requests
pub trait JwtClaimsExt {
    fn jwt_claims(&self) -> Option<&Claims>;
}

impl<T> JwtClaimsExt for Request<T> {
    fn jwt_claims(&self) -> Option<&Claims> {
        self.extensions().get::<Claims>()
    }
}

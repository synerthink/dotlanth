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

//! RBAC middleware for permission checking

use crate::auth::Claims;
use crate::error::{ApiError, ApiResult};
use crate::rbac::permissions::PermissionContext;
use crate::rbac::system::RBACSystem;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Request, Response, body::Incoming};
use std::sync::Arc;
use tower::{Layer, Service};
use tracing::{debug, warn};

/// RBAC middleware for enforcing permissions
#[derive(Clone)]
pub struct RBACMiddleware<S> {
    inner: S,
    rbac_system: Arc<RBACSystem>,
    required_permissions: Vec<String>,
    required_dot_permissions: Vec<(String, String)>, // (dot_id, operation)
}

impl<S> RBACMiddleware<S> {
    /// Create new RBAC middleware
    pub fn new(inner: S, rbac_system: Arc<RBACSystem>) -> Self {
        Self {
            inner,
            rbac_system,
            required_permissions: Vec::new(),
            required_dot_permissions: Vec::new(),
        }
    }

    /// Create RBAC middleware with required permissions
    pub fn with_permissions(inner: S, rbac_system: Arc<RBACSystem>, permissions: Vec<String>) -> Self {
        Self {
            inner,
            rbac_system,
            required_permissions: permissions,
            required_dot_permissions: Vec::new(),
        }
    }

    /// Create RBAC middleware with required dot permissions
    pub fn with_dot_permissions(inner: S, rbac_system: Arc<RBACSystem>, dot_permissions: Vec<(String, String)>) -> Self {
        Self {
            inner,
            rbac_system,
            required_permissions: Vec::new(),
            required_dot_permissions: dot_permissions,
        }
    }

    /// Create RBAC middleware with both permission types
    pub fn with_all_permissions(inner: S, rbac_system: Arc<RBACSystem>, permissions: Vec<String>, dot_permissions: Vec<(String, String)>) -> Self {
        Self {
            inner,
            rbac_system,
            required_permissions: permissions,
            required_dot_permissions: dot_permissions,
        }
    }
}

impl<S> Service<Request<Incoming>> for RBACMiddleware<S>
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

    fn call(&mut self, req: Request<Incoming>) -> Self::Future {
        let rbac_system = self.rbac_system.clone();
        let required_permissions = self.required_permissions.clone();
        let required_dot_permissions = self.required_dot_permissions.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Extract JWT claims from request
            let claims = req.extensions().get::<Claims>().ok_or_else(|| ApiError::Unauthorized {
                message: "No authentication information found".to_string(),
            })?;

            // Extract client information for permission context
            let client_ip = extract_client_ip(&req);
            let user_agent = extract_user_agent(&req);
            let request_id = extract_request_id(&req);

            // Create permission context
            let context = PermissionContext::new(claims.sub.clone()).with_client_ip(client_ip.clone()).with_additional_data({
                let mut data = std::collections::HashMap::new();
                if let Some(ua) = &user_agent {
                    data.insert("user_agent".to_string(), ua.clone());
                }
                if let Some(req_id) = &request_id {
                    data.insert("request_id".to_string(), req_id.clone());
                }
                data
            });

            // Check required permissions
            for permission in &required_permissions {
                let parts: Vec<&str> = permission.split(':').collect();
                if parts.len() != 2 {
                    return Err(Box::new(ApiError::InternalServerError {
                        message: format!("Invalid permission format: {}", permission),
                    }) as Box<dyn std::error::Error + Send + Sync>);
                }

                let resource = parts[0];
                let action = parts[1];

                let has_permission = rbac_system
                    .check_permission(&claims.sub, resource, action, &context)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                if !has_permission {
                    warn!(
                        user_id = %claims.sub,
                        resource = %resource,
                        action = %action,
                        client_ip = ?client_ip,
                        "Permission denied"
                    );

                    return Err(Box::new(ApiError::Forbidden {
                        message: format!("Missing required permission: {}:{}", resource, action),
                    }) as Box<dyn std::error::Error + Send + Sync>);
                }

                debug!(
                    user_id = %claims.sub,
                    resource = %resource,
                    action = %action,
                    "Permission granted"
                );
            }

            // Check required dot permissions
            for (dot_id, operation) in &required_dot_permissions {
                let has_permission = rbac_system
                    .check_dot_permission(&claims.sub, dot_id, operation, &context)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                if !has_permission {
                    warn!(
                        user_id = %claims.sub,
                        dot_id = %dot_id,
                        operation = %operation,
                        client_ip = ?client_ip,
                        "Dot permission denied"
                    );

                    return Err(Box::new(ApiError::Forbidden {
                        message: format!("Missing required dot permission: {} on {}", operation, dot_id),
                    }) as Box<dyn std::error::Error + Send + Sync>);
                }

                debug!(
                    user_id = %claims.sub,
                    dot_id = %dot_id,
                    operation = %operation,
                    "Dot permission granted"
                );
            }

            // All permissions granted, proceed with request
            inner.call(req).await.map_err(Into::into)
        })
    }
}

/// RBAC middleware layer
#[derive(Clone)]
pub struct RBACLayer {
    rbac_system: Arc<RBACSystem>,
    required_permissions: Vec<String>,
    required_dot_permissions: Vec<(String, String)>,
}

impl RBACLayer {
    /// Create new RBAC layer
    pub fn new(rbac_system: Arc<RBACSystem>) -> Self {
        Self {
            rbac_system,
            required_permissions: Vec::new(),
            required_dot_permissions: Vec::new(),
        }
    }

    /// Create RBAC layer with required permissions
    pub fn with_permissions(rbac_system: Arc<RBACSystem>, permissions: Vec<String>) -> Self {
        Self {
            rbac_system,
            required_permissions: permissions,
            required_dot_permissions: Vec::new(),
        }
    }

    /// Create RBAC layer with required dot permissions
    pub fn with_dot_permissions(rbac_system: Arc<RBACSystem>, dot_permissions: Vec<(String, String)>) -> Self {
        Self {
            rbac_system,
            required_permissions: Vec::new(),
            required_dot_permissions: dot_permissions,
        }
    }

    /// Create RBAC layer with both permission types
    pub fn with_all_permissions(rbac_system: Arc<RBACSystem>, permissions: Vec<String>, dot_permissions: Vec<(String, String)>) -> Self {
        Self {
            rbac_system,
            required_permissions: permissions,
            required_dot_permissions: dot_permissions,
        }
    }
}

impl<S> Layer<S> for RBACLayer {
    type Service = RBACMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        if !self.required_permissions.is_empty() || !self.required_dot_permissions.is_empty() {
            RBACMiddleware::with_all_permissions(inner, self.rbac_system.clone(), self.required_permissions.clone(), self.required_dot_permissions.clone())
        } else {
            RBACMiddleware::new(inner, self.rbac_system.clone())
        }
    }
}

/// Permission checking functions for use in handlers
pub struct PermissionChecker {
    rbac_system: Arc<RBACSystem>,
}

impl PermissionChecker {
    /// Create new permission checker
    pub fn new(rbac_system: Arc<RBACSystem>) -> Self {
        Self { rbac_system }
    }

    /// Check if user has permission
    pub async fn check_permission(&self, user_id: &str, resource: &str, action: &str, context: &PermissionContext) -> ApiResult<bool> {
        self.rbac_system.check_permission(user_id, resource, action, context).await
    }

    /// Check if user has dot permission
    pub async fn check_dot_permission(&self, user_id: &str, dot_id: &str, operation: &str, context: &PermissionContext) -> ApiResult<bool> {
        self.rbac_system.check_dot_permission(user_id, dot_id, operation, context).await
    }

    /// Require permission (throws error if not granted)
    pub async fn require_permission(&self, user_id: &str, resource: &str, action: &str, context: &PermissionContext) -> ApiResult<()> {
        let has_permission = self.check_permission(user_id, resource, action, context).await?;

        if !has_permission {
            return Err(ApiError::Forbidden {
                message: format!("Missing required permission: {}:{}", resource, action),
            });
        }

        Ok(())
    }

    /// Require dot permission (throws error if not granted)
    pub async fn require_dot_permission(&self, user_id: &str, dot_id: &str, operation: &str, context: &PermissionContext) -> ApiResult<()> {
        let has_permission = self.check_dot_permission(user_id, dot_id, operation, context).await?;

        if !has_permission {
            return Err(ApiError::Forbidden {
                message: format!("Missing required dot permission: {} on {}", operation, dot_id),
            });
        }

        Ok(())
    }
}

/// Extract client IP from request
fn extract_client_ip<T>(req: &Request<T>) -> Option<String> {
    // Try X-Forwarded-For header first
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take the first IP in the chain
            if let Some(ip) = forwarded_str.split(',').next() {
                return Some(ip.trim().to_string());
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return Some(ip_str.to_string());
        }
    }

    // Could extract from connection info if available
    None
}

/// Extract user agent from request
fn extract_user_agent<T>(req: &Request<T>) -> Option<String> {
    req.headers().get("user-agent").and_then(|ua| ua.to_str().ok()).map(|s| s.to_string())
}

/// Extract request ID from request
fn extract_request_id<T>(req: &Request<T>) -> Option<String> {
    req.headers().get("x-request-id").and_then(|id| id.to_str().ok()).map(|s| s.to_string()).or_else(|| {
        // Generate a request ID if not present
        Some(uuid::Uuid::new_v4().to_string())
    })
}

/// Extension trait for extracting permission context from requests
pub trait PermissionContextExt {
    fn permission_context(&self) -> PermissionContext;
}

impl<T> PermissionContextExt for Request<T> {
    fn permission_context(&self) -> PermissionContext {
        let user_id = self.extensions().get::<Claims>().map(|claims| claims.sub.clone()).unwrap_or_else(|| "anonymous".to_string());

        PermissionContext::new(user_id).with_client_ip(extract_client_ip(self)).with_additional_data({
            let mut data = std::collections::HashMap::new();
            if let Some(ua) = extract_user_agent(self) {
                data.insert("user_agent".to_string(), ua);
            }
            if let Some(req_id) = extract_request_id(self) {
                data.insert("request_id".to_string(), req_id);
            }
            data
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::audit::AuditLogger;
    use crate::rbac::cache::PermissionCache;
    use crate::rbac::manager::RoleManager;
    use crate::rbac::permissions::PermissionChecker as PermChecker;
    use std::sync::Arc;

    async fn create_test_rbac_system() -> Arc<RBACSystem> {
        let audit_logger = Arc::new(AuditLogger::new());
        let cache = Arc::new(PermissionCache::new());
        let role_manager = Arc::new(RoleManager::new(audit_logger.clone()));
        let permission_checker = Arc::new(PermChecker::new(cache.clone()));

        Arc::new(RBACSystem::new(
            role_manager,
            permission_checker,
            audit_logger.clone(),
            cache,
            Arc::new(crate::rbac::dot_permissions::DotPermissionManager::new(audit_logger)),
        ))
    }

    #[tokio::test]
    async fn test_permission_checker() {
        let rbac_system = create_test_rbac_system().await;
        let checker = PermissionChecker::new(rbac_system);

        let context = PermissionContext::new("user123".to_string());

        // This would require setting up roles and permissions first
        // For now, just test that the methods don't panic
        let result = checker.check_permission("user123", "dots", "read", &context).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_client_ip() {
        use http_body_util::Empty;
        use hyper::body::Bytes;

        let req = Request::builder().header("x-forwarded-for", "192.168.1.1, 10.0.0.1").body(Empty::<Bytes>::new()).unwrap();

        let ip = extract_client_ip(&req);
        assert_eq!(ip, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_user_agent() {
        use http_body_util::Empty;
        use hyper::body::Bytes;

        let req = Request::builder().header("user-agent", "Mozilla/5.0 Test Browser").body(Empty::<Bytes>::new()).unwrap();

        let ua = extract_user_agent(&req);
        assert_eq!(ua, Some("Mozilla/5.0 Test Browser".to_string()));
    }
}

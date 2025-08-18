// Dotlanth
// Copyright (C) 2025 Synerthink

//! gRPC versioning interceptors and compatibility management

use crate::services::database::DatabaseServiceImpl;
use crate::services::vm_service::VmServiceImpl;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::metadata::MetadataMap;
use tonic::{Request, Response, Status};
use tracing::{info, warn};

/// API version for gRPC services
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrpcApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl GrpcApiVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn is_compatible_with(&self, other: &GrpcApiVersion) -> bool {
        self.major == other.major && self >= other
    }
}

impl PartialOrd for GrpcApiVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GrpcApiVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major).then_with(|| self.minor.cmp(&other.minor)).then_with(|| self.patch.cmp(&other.patch))
    }
}

impl std::fmt::Display for GrpcApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for GrpcApiVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0].parse::<u32>().map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1].parse::<u32>().map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2].parse::<u32>().map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(GrpcApiVersion::new(major, minor, patch))
    }
}

/// gRPC service versioning registry
#[derive(Debug, Clone)]
pub struct GrpcVersionRegistry {
    supported_versions: HashMap<String, Vec<GrpcApiVersion>>,
    default_versions: HashMap<String, GrpcApiVersion>,
}

impl Default for GrpcVersionRegistry {
    fn default() -> Self {
        let mut registry = Self {
            supported_versions: HashMap::new(),
            default_versions: HashMap::new(),
        };

        // Initialize default versions for gRPC services
        let services = ["VmService", "DatabaseService", "ClusterService", "MetricsService"];
        for service in &services {
            let version = GrpcApiVersion::new(1, 0, 0);
            registry.register_version(service.to_string(), version.clone());
            registry.set_default_version(service.to_string(), version);
        }

        registry
    }
}

impl GrpcVersionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_version(&mut self, service: String, version: GrpcApiVersion) {
        self.supported_versions.entry(service).or_insert_with(Vec::new).push(version);
    }

    pub fn set_default_version(&mut self, service: String, version: GrpcApiVersion) {
        self.default_versions.insert(service, version);
    }

    pub fn get_supported_versions(&self, service: &str) -> Vec<GrpcApiVersion> {
        self.supported_versions.get(service).cloned().unwrap_or_default()
    }

    pub fn get_default_version(&self, service: &str) -> Option<GrpcApiVersion> {
        self.default_versions.get(service).cloned()
    }

    pub fn is_version_supported(&self, service: &str, version: &GrpcApiVersion) -> bool {
        self.get_supported_versions(service).contains(version)
    }
}

/// gRPC version context
#[derive(Debug, Clone)]
pub struct GrpcVersionContext {
    pub service_name: String,
    pub negotiated_version: GrpcApiVersion,
    pub client_version: Option<GrpcApiVersion>,
    pub compatibility_warnings: Vec<String>,
}

/// gRPC versioning interceptor
#[derive(Clone)]
pub struct GrpcVersioningInterceptor {
    registry: Arc<RwLock<GrpcVersionRegistry>>,
}

impl GrpcVersioningInterceptor {
    pub fn new(registry: GrpcVersionRegistry) -> Self {
        Self {
            registry: Arc::new(RwLock::new(registry)),
        }
    }

    /// Intercept incoming gRPC request for version negotiation
    pub async fn intercept_request<T>(&self, mut request: Request<T>, service_name: &str) -> Result<(Request<T>, GrpcVersionContext), Status> {
        let metadata = request.metadata();

        // Extract client version from metadata
        let client_version = self.extract_client_version(metadata)?;

        // Negotiate version
        let registry = self.registry.read().await;
        let negotiated_version = self.negotiate_version(&registry, service_name, client_version.as_ref())?;

        // Create version context
        let version_context = GrpcVersionContext {
            service_name: service_name.to_string(),
            negotiated_version: negotiated_version.clone(),
            client_version,
            compatibility_warnings: Vec::new(),
        };

        // Add version context to request extensions
        request.extensions_mut().insert(version_context.clone());

        // Add server version to metadata
        request.metadata_mut().insert("server-api-version", negotiated_version.to_string().parse().unwrap());

        info!("gRPC request negotiated version {} for service {}", negotiated_version, service_name);

        Ok((request, version_context))
    }

    /// Intercept outgoing gRPC response to add version information
    pub async fn intercept_response<T>(&self, mut response: Response<T>, version_context: &GrpcVersionContext) -> Response<T> {
        // Add version headers to response
        response.metadata_mut().insert("api-version", version_context.negotiated_version.to_string().parse().unwrap());
        response.metadata_mut().insert("service-name", version_context.service_name.parse().unwrap());

        // Add compatibility warnings if any
        if !version_context.compatibility_warnings.is_empty() {
            let warnings = version_context.compatibility_warnings.join("; ");
            response.metadata_mut().insert("api-warnings", warnings.parse().unwrap());
        }

        response
    }

    /// Extract client version from gRPC metadata
    fn extract_client_version(&self, metadata: &MetadataMap) -> Result<Option<GrpcApiVersion>, Status> {
        if let Some(version_value) = metadata.get("api-version") {
            let version_str = version_value.to_str().map_err(|_| Status::invalid_argument("Invalid API version header"))?;

            let version = version_str
                .parse::<GrpcApiVersion>()
                .map_err(|e| Status::invalid_argument(format!("Invalid API version format: {}", e)))?;

            Ok(Some(version))
        } else {
            Ok(None)
        }
    }

    /// Negotiate version between client and server
    fn negotiate_version(&self, registry: &GrpcVersionRegistry, service_name: &str, client_version: Option<&GrpcApiVersion>) -> Result<GrpcApiVersion, Status> {
        let supported_versions = registry.get_supported_versions(service_name);

        if supported_versions.is_empty() {
            return Err(Status::unimplemented(format!("Service {} not available", service_name)));
        }

        // If client didn't specify version, use default
        let client_version = match client_version {
            Some(v) => v,
            None => {
                return Ok(registry.get_default_version(service_name).unwrap_or_else(|| supported_versions[0].clone()));
            }
        };

        // Check if client version is supported
        if registry.is_version_supported(service_name, client_version) {
            return Ok(client_version.clone());
        }

        // Find best compatible version
        let mut compatible_versions: Vec<_> = supported_versions.iter().filter(|v| client_version.is_compatible_with(v)).cloned().collect();

        if compatible_versions.is_empty() {
            return Err(Status::failed_precondition(format!("No compatible version found for client version {}", client_version)));
        }

        // Sort by newest first and pick the best
        compatible_versions.sort_by(|a, b| b.cmp(a));
        Ok(compatible_versions[0].clone())
    }

    /// Get registry
    pub async fn registry(&self) -> tokio::sync::RwLockReadGuard<'_, GrpcVersionRegistry> {
        self.registry.read().await
    }

    /// Update registry
    pub async fn registry_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, GrpcVersionRegistry> {
        self.registry.write().await
    }
}

/// Extension trait for extracting version context from gRPC requests
pub trait GrpcVersionContextExt {
    fn version_context(&self) -> Option<&GrpcVersionContext>;
}

impl<T> GrpcVersionContextExt for Request<T> {
    fn version_context(&self) -> Option<&GrpcVersionContext> {
        self.extensions().get::<GrpcVersionContext>()
    }
}

/// Macro for creating versioned gRPC service implementations
#[macro_export]
macro_rules! versioned_grpc_service {
    ($service:ident, $interceptor:expr) => {
        impl $service {
            async fn with_version_check<T, F, Fut>(&self, request: tonic::Request<T>, service_name: &str, handler: F) -> Result<tonic::Response<impl prost::Message>, tonic::Status>
            where
                F: FnOnce(tonic::Request<T>, GrpcVersionContext) -> Fut,
                Fut: std::future::Future<Output = Result<tonic::Response<impl prost::Message>, tonic::Status>>,
            {
                let (request, version_context) = $interceptor.intercept_request(request, service_name).await?;

                let response = handler(request, version_context.clone()).await?;

                Ok($interceptor.intercept_response(response, &version_context).await)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!("1.0.0".parse::<GrpcApiVersion>().unwrap(), GrpcApiVersion::new(1, 0, 0));
        assert_eq!("2.1.3".parse::<GrpcApiVersion>().unwrap(), GrpcApiVersion::new(2, 1, 3));
        assert!("invalid".parse::<GrpcApiVersion>().is_err());
    }

    #[test]
    fn test_version_compatibility() {
        let v1_0_0 = GrpcApiVersion::new(1, 0, 0);
        let v1_1_0 = GrpcApiVersion::new(1, 1, 0);
        let v2_0_0 = GrpcApiVersion::new(2, 0, 0);

        assert!(v1_1_0.is_compatible_with(&v1_0_0));
        assert!(!v1_0_0.is_compatible_with(&v1_1_0));
        assert!(!v2_0_0.is_compatible_with(&v1_0_0));
    }

    #[test]
    fn test_registry() {
        let mut registry = GrpcVersionRegistry::new();
        let version = GrpcApiVersion::new(1, 1, 0);

        registry.register_version("TestService".to_string(), version.clone());

        assert!(registry.is_version_supported("TestService", &version));
        assert_eq!(registry.get_supported_versions("TestService"), vec![version]);
    }
}

// Dotlanth
// Copyright (C) 2025 Synerthink

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Semantic version for API endpoints
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ApiVersion {
    /// Create a new API version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Check if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &ApiVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major && self >= other
    }

    /// Check if this version is a breaking change from another
    pub fn is_breaking_change_from(&self, other: &ApiVersion) -> bool {
        self.major > other.major
    }

    /// Get the next major version
    pub fn next_major(&self) -> ApiVersion {
        ApiVersion::new(self.major + 1, 0, 0)
    }

    /// Get the next minor version
    pub fn next_minor(&self) -> ApiVersion {
        ApiVersion::new(self.major, self.minor + 1, 0)
    }

    /// Get the next patch version
    pub fn next_patch(&self) -> ApiVersion {
        ApiVersion::new(self.major, self.minor, self.patch + 1)
    }

    /// Get version as string with v prefix
    pub fn to_version_string(&self) -> String {
        format!("v{}", self)
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl PartialOrd for ApiVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ApiVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major.cmp(&other.major).then_with(|| self.minor.cmp(&other.minor)).then_with(|| self.patch.cmp(&other.patch))
    }
}

#[derive(Error, Debug)]
pub enum VersionParseError {
    #[error("Invalid version format: {0}")]
    InvalidFormat(String),
    #[error("Invalid number in version: {0}")]
    InvalidNumber(String),
}

impl FromStr for ApiVersion {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() != 3 {
            return Err(VersionParseError::InvalidFormat(s.to_string()));
        }

        let major = parts[0].parse::<u32>().map_err(|_| VersionParseError::InvalidNumber(parts[0].to_string()))?;
        let minor = parts[1].parse::<u32>().map_err(|_| VersionParseError::InvalidNumber(parts[1].to_string()))?;
        let patch = parts[2].parse::<u32>().map_err(|_| VersionParseError::InvalidNumber(parts[2].to_string()))?;

        Ok(ApiVersion::new(major, minor, patch))
    }
}

/// Protocol type for API versioning
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolType {
    Rest,
    GraphQL,
    Grpc,
    WebSocket,
}

impl fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolType::Rest => write!(f, "REST"),
            ProtocolType::GraphQL => write!(f, "GraphQL"),
            ProtocolType::Grpc => write!(f, "gRPC"),
            ProtocolType::WebSocket => write!(f, "WebSocket"),
        }
    }
}

/// Service type for API versioning
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    Vm,
    Database,
    Runtime,
    Cluster,
    Metrics,
    Abi,
    Dots,
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceType::Vm => write!(f, "VM"),
            ServiceType::Database => write!(f, "Database"),
            ServiceType::Runtime => write!(f, "Runtime"),
            ServiceType::Cluster => write!(f, "Cluster"),
            ServiceType::Metrics => write!(f, "Metrics"),
            ServiceType::Abi => write!(f, "ABI"),
            ServiceType::Dots => write!(f, "Dots"),
        }
    }
}

/// Version registry for managing API versions
#[derive(Debug, Clone)]
pub struct VersionRegistry {
    /// Current supported versions by protocol and service
    versions: std::collections::HashMap<(ProtocolType, ServiceType), Vec<ApiVersion>>,
    /// Default version for each protocol/service combination
    defaults: std::collections::HashMap<(ProtocolType, ServiceType), ApiVersion>,
}

impl Default for VersionRegistry {
    fn default() -> Self {
        let mut registry = Self {
            versions: std::collections::HashMap::new(),
            defaults: std::collections::HashMap::new(),
        };

        // Initialize default versions for all protocol/service combinations
        let protocols = [ProtocolType::Rest, ProtocolType::GraphQL, ProtocolType::Grpc, ProtocolType::WebSocket];
        let services = [
            ServiceType::Vm,
            ServiceType::Database,
            ServiceType::Runtime,
            ServiceType::Cluster,
            ServiceType::Metrics,
            ServiceType::Abi,
            ServiceType::Dots,
        ];

        for protocol in &protocols {
            for service in &services {
                let version = ApiVersion::new(1, 0, 0);
                registry.register_version(protocol.clone(), service.clone(), version.clone());
                registry.set_default_version(protocol.clone(), service.clone(), version);
            }
        }

        registry
    }
}

impl VersionRegistry {
    /// Create a new version registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new version for a protocol/service combination
    pub fn register_version(&mut self, protocol: ProtocolType, service: ServiceType, version: ApiVersion) {
        let key = (protocol.clone(), service.clone());
        self.versions.entry(key.clone()).or_insert_with(Vec::new).push(version);

        // Sort versions in descending order (newest first)
        if let Some(versions) = self.versions.get_mut(&key) {
            versions.sort_by(|a, b| b.cmp(a));
            versions.dedup();
        }
    }

    /// Set default version for a protocol/service combination
    pub fn set_default_version(&mut self, protocol: ProtocolType, service: ServiceType, version: ApiVersion) {
        self.defaults.insert((protocol, service), version);
    }

    /// Get supported versions for a protocol/service combination
    pub fn get_supported_versions(&self, protocol: &ProtocolType, service: &ServiceType) -> Vec<ApiVersion> {
        self.versions.get(&(protocol.clone(), service.clone())).cloned().unwrap_or_default()
    }

    /// Get default version for a protocol/service combination
    pub fn get_default_version(&self, protocol: &ProtocolType, service: &ServiceType) -> Option<ApiVersion> {
        self.defaults.get(&(protocol.clone(), service.clone())).cloned()
    }

    /// Check if a version is supported
    pub fn is_version_supported(&self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion) -> bool {
        self.get_supported_versions(protocol, service).contains(version)
    }

    /// Get the latest version for a protocol/service combination
    pub fn get_latest_version(&self, protocol: &ProtocolType, service: &ServiceType) -> Option<ApiVersion> {
        self.get_supported_versions(protocol, service).into_iter().next()
    }

    /// Get compatible versions for a given version
    pub fn get_compatible_versions(&self, protocol: &ProtocolType, service: &ServiceType, target_version: &ApiVersion) -> Vec<ApiVersion> {
        self.get_supported_versions(protocol, service).into_iter().filter(|v| target_version.is_compatible_with(v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!("1.0.0".parse::<ApiVersion>().unwrap(), ApiVersion::new(1, 0, 0));
        assert_eq!("v2.1.3".parse::<ApiVersion>().unwrap(), ApiVersion::new(2, 1, 3));
        assert!("invalid".parse::<ApiVersion>().is_err());
    }

    #[test]
    fn test_version_compatibility() {
        let v1_0_0 = ApiVersion::new(1, 0, 0);
        let v1_1_0 = ApiVersion::new(1, 1, 0);
        let v2_0_0 = ApiVersion::new(2, 0, 0);

        assert!(v1_1_0.is_compatible_with(&v1_0_0));
        assert!(!v1_0_0.is_compatible_with(&v1_1_0));
        assert!(!v2_0_0.is_compatible_with(&v1_0_0));
    }

    #[test]
    fn test_version_registry() {
        let mut registry = VersionRegistry::new();
        let version = ApiVersion::new(1, 1, 0);

        registry.register_version(ProtocolType::Rest, ServiceType::Vm, version.clone());

        assert!(registry.is_version_supported(&ProtocolType::Rest, &ServiceType::Vm, &version));
        assert_eq!(registry.get_latest_version(&ProtocolType::Rest, &ServiceType::Vm), Some(version));
    }
}

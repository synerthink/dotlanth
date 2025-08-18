// Dotlanth
// Copyright (C) 2025 Synerthink

use crate::versioning::{ApiVersion, ProtocolType, ServiceType, VersionRegistry};
use hyper::{HeaderMap, Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Version negotiation errors
#[derive(Error, Debug)]
pub enum NegotiationError {
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),
    #[error("No compatible version found")]
    NoCompatibleVersion,
    #[error("Invalid version header: {0}")]
    InvalidVersionHeader(String),
    #[error("Protocol not supported: {0}")]
    ProtocolNotSupported(String),
}

/// Client version preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientVersionPreferences {
    pub preferred_version: ApiVersion,
    pub supported_versions: Vec<ApiVersion>,
    pub min_version: Option<ApiVersion>,
    pub max_version: Option<ApiVersion>,
}

impl Default for ClientVersionPreferences {
    fn default() -> Self {
        Self {
            preferred_version: ApiVersion::new(1, 0, 0),
            supported_versions: vec![ApiVersion::new(1, 0, 0)],
            min_version: Some(ApiVersion::new(1, 0, 0)),
            max_version: None,
        }
    }
}

/// Version negotiation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResult {
    pub negotiated_version: ApiVersion,
    pub protocol: ProtocolType,
    pub service: ServiceType,
    pub compatibility_warnings: Vec<String>,
}

/// Version negotiator for handling client-server version negotiation
#[derive(Debug, Clone)]
pub struct VersionNegotiator {
    registry: VersionRegistry,
}

impl VersionNegotiator {
    /// Create a new version negotiator
    pub fn new(registry: VersionRegistry) -> Self {
        Self { registry }
    }

    /// Negotiate version from HTTP request headers
    pub fn negotiate_from_headers(&self, headers: &HeaderMap, protocol: ProtocolType, service: ServiceType) -> Result<NegotiationResult, NegotiationError> {
        let client_prefs = self.extract_version_preferences_from_headers(headers)?;
        self.negotiate_version(client_prefs, protocol, service)
    }

    /// Negotiate version from gRPC metadata
    pub fn negotiate_from_grpc_metadata(&self, metadata: &tonic::metadata::MetadataMap, service: ServiceType) -> Result<NegotiationResult, NegotiationError> {
        let client_prefs = self.extract_version_preferences_from_grpc_metadata(metadata)?;
        self.negotiate_version(client_prefs, ProtocolType::Grpc, service)
    }

    /// Negotiate version from WebSocket headers
    pub fn negotiate_from_websocket_headers(&self, headers: &HeaderMap, service: ServiceType) -> Result<NegotiationResult, NegotiationError> {
        let client_prefs = self.extract_version_preferences_from_headers(headers)?;
        self.negotiate_version(client_prefs, ProtocolType::WebSocket, service)
    }

    /// Negotiate version from GraphQL context
    pub fn negotiate_from_graphql_headers(&self, headers: &HeaderMap, service: ServiceType) -> Result<NegotiationResult, NegotiationError> {
        let client_prefs = self.extract_version_preferences_from_headers(headers)?;
        self.negotiate_version(client_prefs, ProtocolType::GraphQL, service)
    }

    /// Core version negotiation logic
    pub fn negotiate_version(&self, client_prefs: ClientVersionPreferences, protocol: ProtocolType, service: ServiceType) -> Result<NegotiationResult, NegotiationError> {
        let supported_versions = self.registry.get_supported_versions(&protocol, &service);

        if supported_versions.is_empty() {
            return Err(NegotiationError::ProtocolNotSupported(format!("{}/{}", protocol, service)));
        }

        // Try to find exact match for preferred version
        if supported_versions.contains(&client_prefs.preferred_version) {
            return Ok(NegotiationResult {
                negotiated_version: client_prefs.preferred_version,
                protocol,
                service,
                compatibility_warnings: Vec::new(),
            });
        }

        // Find best compatible version
        let mut compatible_versions: Vec<_> = supported_versions
            .iter()
            .filter(|server_version| {
                // Check if server version is in client's supported range
                client_prefs.supported_versions.iter().any(|client_version| {
                    client_version.is_compatible_with(server_version)
                }) &&
                // Check min version constraint
                client_prefs.min_version.as_ref().map_or(true, |min| *server_version >= min) &&
                // Check max version constraint
                client_prefs.max_version.as_ref().map_or(true, |max| *server_version <= max)
            })
            .cloned()
            .collect();

        if compatible_versions.is_empty() {
            return Err(NegotiationError::NoCompatibleVersion);
        }

        // Sort by preference (newest compatible version first)
        compatible_versions.sort_by(|a, b| b.cmp(a));
        let negotiated_version = compatible_versions[0].clone();

        // Generate compatibility warnings
        let mut warnings = Vec::new();
        if negotiated_version != client_prefs.preferred_version {
            warnings.push(format!("Preferred version {} not available, using {}", client_prefs.preferred_version, negotiated_version));
        }

        if negotiated_version.is_breaking_change_from(&client_prefs.preferred_version) {
            warnings.push(format!("Breaking changes may exist between {} and {}", client_prefs.preferred_version, negotiated_version));
        }

        Ok(NegotiationResult {
            negotiated_version,
            protocol,
            service,
            compatibility_warnings: warnings,
        })
    }

    /// Extract version preferences from HTTP headers
    fn extract_version_preferences_from_headers(&self, headers: &HeaderMap) -> Result<ClientVersionPreferences, NegotiationError> {
        // Check for API-Version header (preferred)
        if let Some(version_header) = headers.get("api-version") {
            let version_str = version_header.to_str().map_err(|_| NegotiationError::InvalidVersionHeader("Invalid UTF-8".to_string()))?;

            let preferred_version = version_str.parse::<ApiVersion>().map_err(|e| NegotiationError::InvalidVersionHeader(e.to_string()))?;

            return Ok(ClientVersionPreferences {
                preferred_version: preferred_version.clone(),
                supported_versions: vec![preferred_version],
                min_version: None,
                max_version: None,
            });
        }

        // Check for Accept-Version header (range specification)
        if let Some(accept_header) = headers.get("accept-version") {
            let accept_str = accept_header.to_str().map_err(|_| NegotiationError::InvalidVersionHeader("Invalid UTF-8".to_string()))?;

            return self.parse_accept_version_header(accept_str);
        }

        // Check URL path for version (e.g., /v1/api/...)
        if let Some(path_header) = headers.get("x-original-path") {
            if let Ok(path_str) = path_header.to_str() {
                if let Some(version) = self.extract_version_from_path(path_str) {
                    return Ok(ClientVersionPreferences {
                        preferred_version: version.clone(),
                        supported_versions: vec![version],
                        min_version: None,
                        max_version: None,
                    });
                }
            }
        }

        // Default to latest version if no preference specified
        Ok(ClientVersionPreferences::default())
    }

    /// Extract version preferences from gRPC metadata
    fn extract_version_preferences_from_grpc_metadata(&self, metadata: &tonic::metadata::MetadataMap) -> Result<ClientVersionPreferences, NegotiationError> {
        // Check for api-version metadata
        if let Some(version_value) = metadata.get("api-version") {
            let version_str = version_value.to_str().map_err(|_| NegotiationError::InvalidVersionHeader("Invalid UTF-8".to_string()))?;

            let preferred_version = version_str.parse::<ApiVersion>().map_err(|e| NegotiationError::InvalidVersionHeader(e.to_string()))?;

            return Ok(ClientVersionPreferences {
                preferred_version: preferred_version.clone(),
                supported_versions: vec![preferred_version],
                min_version: None,
                max_version: None,
            });
        }

        Ok(ClientVersionPreferences::default())
    }

    /// Parse Accept-Version header (e.g., "1.0-2.0", ">=1.2", "~1.1")
    fn parse_accept_version_header(&self, accept_str: &str) -> Result<ClientVersionPreferences, NegotiationError> {
        // Simple range parsing - can be extended for more complex semantics
        if accept_str.contains('-') {
            // Range format: "1.0-2.0"
            let parts: Vec<&str> = accept_str.split('-').collect();
            if parts.len() == 2 {
                let min_version = parts[0].parse::<ApiVersion>().map_err(|e| NegotiationError::InvalidVersionHeader(e.to_string()))?;
                let max_version = parts[1].parse::<ApiVersion>().map_err(|e| NegotiationError::InvalidVersionHeader(e.to_string()))?;

                return Ok(ClientVersionPreferences {
                    preferred_version: max_version.clone(),
                    supported_versions: vec![min_version.clone(), max_version.clone()],
                    min_version: Some(min_version),
                    max_version: Some(max_version),
                });
            }
        }

        // Single version
        let version = accept_str.parse::<ApiVersion>().map_err(|e| NegotiationError::InvalidVersionHeader(e.to_string()))?;

        Ok(ClientVersionPreferences {
            preferred_version: version.clone(),
            supported_versions: vec![version],
            min_version: None,
            max_version: None,
        })
    }

    /// Extract version from URL path (e.g., /v1/api/vm/execute)
    fn extract_version_from_path(&self, path: &str) -> Option<ApiVersion> {
        let parts: Vec<&str> = path.split('/').collect();
        for part in parts {
            if part.starts_with('v') && part.len() > 1 {
                if let Ok(version) = part.parse::<ApiVersion>() {
                    return Some(version);
                }
            }
        }
        None
    }

    /// Get version registry
    pub fn registry(&self) -> &VersionRegistry {
        &self.registry
    }
}

/// Version context for request processing
#[derive(Debug, Clone)]
pub struct VersionContext {
    pub negotiated_version: ApiVersion,
    pub protocol: ProtocolType,
    pub service: ServiceType,
    pub client_preferences: ClientVersionPreferences,
    pub compatibility_warnings: Vec<String>,
}

impl VersionContext {
    /// Create new version context
    pub fn new(negotiation_result: NegotiationResult, client_preferences: ClientVersionPreferences) -> Self {
        Self {
            negotiated_version: negotiation_result.negotiated_version,
            protocol: negotiation_result.protocol,
            service: negotiation_result.service,
            client_preferences,
            compatibility_warnings: negotiation_result.compatibility_warnings,
        }
    }

    /// Check if version supports a specific feature
    pub fn supports_feature(&self, feature_name: &str, introduced_in: &ApiVersion) -> bool {
        self.negotiated_version >= *introduced_in
    }

    /// Get response headers for version information
    pub fn get_response_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("api-version".to_string(), self.negotiated_version.to_string());
        headers.insert("api-protocol".to_string(), self.protocol.to_string());
        headers.insert("api-service".to_string(), self.service.to_string());

        if !self.compatibility_warnings.is_empty() {
            headers.insert("api-warnings".to_string(), self.compatibility_warnings.join("; "));
        }

        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_negotiation() {
        let mut registry = VersionRegistry::new();
        registry.register_version(ProtocolType::Rest, ServiceType::Vm, ApiVersion::new(1, 0, 0));
        registry.register_version(ProtocolType::Rest, ServiceType::Vm, ApiVersion::new(1, 1, 0));
        registry.register_version(ProtocolType::Rest, ServiceType::Vm, ApiVersion::new(2, 0, 0));

        let negotiator = VersionNegotiator::new(registry);

        let client_prefs = ClientVersionPreferences {
            preferred_version: ApiVersion::new(1, 1, 0),
            supported_versions: vec![ApiVersion::new(1, 0, 0), ApiVersion::new(1, 1, 0)],
            min_version: Some(ApiVersion::new(1, 0, 0)),
            max_version: None,
        };

        let result = negotiator.negotiate_version(client_prefs, ProtocolType::Rest, ServiceType::Vm).unwrap();
        assert_eq!(result.negotiated_version, ApiVersion::new(1, 1, 0));
    }

    #[test]
    fn test_header_parsing() {
        let negotiator = VersionNegotiator::new(VersionRegistry::new());
        let mut headers = HeaderMap::new();
        headers.insert("api-version", "v1.2.3".parse().unwrap());

        let prefs = negotiator.extract_version_preferences_from_headers(&headers).unwrap();
        assert_eq!(prefs.preferred_version, ApiVersion::new(1, 2, 3));
    }
}

// Dotlanth
// Copyright (C) 2025 Synerthink

//! Versioning-related HTTP handlers

use crate::error::{ApiError, ApiResult};
use crate::versioning::{ApiVersion, ProtocolType, ServiceType, VersionRegistry};
use hyper::{Method, Request, Response, body::Incoming};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Version information response
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current_version: String,
    pub supported_versions: Vec<String>,
    pub protocol: String,
    pub service: String,
    pub deprecation_warnings: Vec<String>,
}

/// API compatibility status
#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityStatus {
    pub from_version: String,
    pub to_version: String,
    pub is_compatible: bool,
    pub compatibility_level: String,
    pub breaking_changes: Vec<String>,
    pub migration_required: bool,
}

/// Handle version information requests
pub async fn handle_version_info(req: Request<Incoming>, version_registry: &VersionRegistry) -> ApiResult<Response<hyper::body::Bytes>> {
    match req.method() {
        &Method::GET => {
            let query_params = extract_query_params(req.uri().query().unwrap_or(""));

            let protocol = query_params
                .get("protocol")
                .and_then(|p| match p.as_str() {
                    "rest" => Some(ProtocolType::Rest),
                    "graphql" => Some(ProtocolType::GraphQL),
                    "grpc" => Some(ProtocolType::Grpc),
                    "websocket" => Some(ProtocolType::WebSocket),
                    _ => None,
                })
                .unwrap_or(ProtocolType::Rest);

            let service = query_params
                .get("service")
                .and_then(|s| match s.as_str() {
                    "vm" => Some(ServiceType::Vm),
                    "database" => Some(ServiceType::Database),
                    "runtime" => Some(ServiceType::Runtime),
                    "cluster" => Some(ServiceType::Cluster),
                    "metrics" => Some(ServiceType::Metrics),
                    "abi" => Some(ServiceType::Abi),
                    "dots" => Some(ServiceType::Dots),
                    _ => None,
                })
                .unwrap_or(ServiceType::Vm);

            let supported_versions = version_registry.get_supported_versions(&protocol, &service);
            let current_version = version_registry.get_latest_version(&protocol, &service);

            let version_info = VersionInfo {
                current_version: current_version.map(|v| v.to_string()).unwrap_or_else(|| "1.0.0".to_string()),
                supported_versions: supported_versions.iter().map(|v| v.to_string()).collect(),
                protocol: protocol.to_string(),
                service: service.to_string(),
                deprecation_warnings: Vec::new(), // Would be populated from deprecation manager
            };

            let response_body = serde_json::to_vec(&version_info).map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to serialize version info: {}", e),
            })?;

            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .header("api-version", version_info.current_version)
                .body(hyper::body::Bytes::from(response_body))
                .map_err(|e| ApiError::InternalServerError {
                    message: format!("Failed to build response: {}", e),
                })
        }
        _ => Err(ApiError::MethodNotAllowed {
            message: format!("Method {} not allowed", req.method()),
        }),
    }
}

/// Handle compatibility check requests
pub async fn handle_compatibility_check(req: Request<Incoming>, version_registry: &VersionRegistry) -> ApiResult<Response<hyper::body::Bytes>> {
    match req.method() {
        &Method::GET => {
            let query_params = extract_query_params(req.uri().query().unwrap_or(""));

            let from_version = query_params
                .get("from")
                .ok_or_else(|| ApiError::BadRequest {
                    message: "Missing 'from' version parameter".to_string(),
                })?
                .parse::<ApiVersion>()
                .map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid 'from' version: {}", e),
                })?;

            let to_version = query_params
                .get("to")
                .ok_or_else(|| ApiError::BadRequest {
                    message: "Missing 'to' version parameter".to_string(),
                })?
                .parse::<ApiVersion>()
                .map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid 'to' version: {}", e),
                })?;

            let protocol = query_params
                .get("protocol")
                .and_then(|p| match p.as_str() {
                    "rest" => Some(ProtocolType::Rest),
                    "graphql" => Some(ProtocolType::GraphQL),
                    "grpc" => Some(ProtocolType::Grpc),
                    "websocket" => Some(ProtocolType::WebSocket),
                    _ => None,
                })
                .unwrap_or(ProtocolType::Rest);

            let service = query_params
                .get("service")
                .and_then(|s| match s.as_str() {
                    "vm" => Some(ServiceType::Vm),
                    "database" => Some(ServiceType::Database),
                    "runtime" => Some(ServiceType::Runtime),
                    "cluster" => Some(ServiceType::Cluster),
                    "metrics" => Some(ServiceType::Metrics),
                    "abi" => Some(ServiceType::Abi),
                    "dots" => Some(ServiceType::Dots),
                    _ => None,
                })
                .unwrap_or(ServiceType::Vm);

            // Check if versions are supported
            let from_supported = version_registry.is_version_supported(&protocol, &service, &from_version);
            let to_supported = version_registry.is_version_supported(&protocol, &service, &to_version);

            if !from_supported {
                return Err(ApiError::BadRequest {
                    message: format!("Version {} is not supported for {}/{}", from_version, protocol, service),
                });
            }

            if !to_supported {
                return Err(ApiError::BadRequest {
                    message: format!("Version {} is not supported for {}/{}", to_version, protocol, service),
                });
            }

            // Check compatibility
            let is_compatible = from_version.is_compatible_with(&to_version);
            let is_breaking = to_version.is_breaking_change_from(&from_version);

            let compatibility_level = if is_compatible {
                if from_version == to_version {
                    "identical"
                } else if from_version.major == to_version.major {
                    "compatible"
                } else {
                    "forward_compatible"
                }
            } else {
                "incompatible"
            };

            let mut breaking_changes = Vec::new();
            if is_breaking {
                breaking_changes.push(format!("Major version change from {} to {} introduces breaking changes", from_version.major, to_version.major));
            }

            let status = CompatibilityStatus {
                from_version: from_version.to_string(),
                to_version: to_version.to_string(),
                is_compatible,
                compatibility_level: compatibility_level.to_string(),
                breaking_changes,
                migration_required: is_breaking,
            };

            let response_body = serde_json::to_vec(&status).map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to serialize compatibility status: {}", e),
            })?;

            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(hyper::body::Bytes::from(response_body))
                .map_err(|e| ApiError::InternalServerError {
                    message: format!("Failed to build response: {}", e),
                })
        }
        _ => Err(ApiError::MethodNotAllowed {
            message: format!("Method {} not allowed", req.method()),
        }),
    }
}

/// Handle API schema requests
pub async fn handle_schema_info(req: Request<Incoming>) -> ApiResult<Response<hyper::body::Bytes>> {
    match req.method() {
        &Method::GET => {
            let query_params = extract_query_params(req.uri().query().unwrap_or(""));

            let version = query_params.get("version").unwrap_or(&"1.0.0".to_string()).parse::<ApiVersion>().map_err(|e| ApiError::BadRequest {
                message: format!("Invalid version: {}", e),
            })?;

            // For now, return a basic schema structure
            // In a real implementation, this would query the SchemaEvolutionManager
            let schema_info = json!({
                "version": version.to_string(),
                "schema_format": "json_schema",
                "endpoints": {
                    "vm": {
                        "execute_dot": {
                            "method": "POST",
                            "path": "/api/v{}/vm/execute",
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "dot_id": {"type": "string"},
                                    "inputs": {"type": "object"},
                                    "paradots_enabled": {"type": "boolean"}
                                },
                                "required": ["dot_id"]
                            }
                        }
                    },
                    "database": {
                        "get": {
                            "method": "GET",
                            "path": "/api/v{}/db/{collection}/{key}",
                            "schema": {
                                "type": "object",
                                "properties": {
                                    "collection": {"type": "string"},
                                    "key": {"type": "string"}
                                },
                                "required": ["collection", "key"]
                            }
                        }
                    }
                }
            });

            let response_body = serde_json::to_vec(&schema_info).map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to serialize schema info: {}", e),
            })?;

            Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .header("api-version", version.to_string())
                .body(hyper::body::Bytes::from(response_body))
                .map_err(|e| ApiError::InternalServerError {
                    message: format!("Failed to build response: {}", e),
                })
        }
        _ => Err(ApiError::MethodNotAllowed {
            message: format!("Method {} not allowed", req.method()),
        }),
    }
}

/// Extract query parameters from query string
fn extract_query_params(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Some((urlencoding::decode(key).unwrap_or_default().to_string(), urlencoding::decode(value).unwrap_or_default().to_string())),
                _ => None,
            }
        })
        .collect()
}

/// Handle health check with version information
pub async fn handle_versioned_health(_req: Request<Incoming>, version_registry: &VersionRegistry) -> ApiResult<Response<hyper::body::Bytes>> {
    let health_info = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version_info": {
            "api_versions": {
                "rest": {
                    "vm": version_registry.get_latest_version(&ProtocolType::Rest, &ServiceType::Vm)
                        .map(|v| v.to_string()).unwrap_or_else(|| "1.0.0".to_string()),
                    "database": version_registry.get_latest_version(&ProtocolType::Rest, &ServiceType::Database)
                        .map(|v| v.to_string()).unwrap_or_else(|| "1.0.0".to_string()),
                },
                "grpc": {
                    "vm": version_registry.get_latest_version(&ProtocolType::Grpc, &ServiceType::Vm)
                        .map(|v| v.to_string()).unwrap_or_else(|| "1.0.0".to_string()),
                    "database": version_registry.get_latest_version(&ProtocolType::Grpc, &ServiceType::Database)
                        .map(|v| v.to_string()).unwrap_or_else(|| "1.0.0".to_string()),
                }
            }
        }
    });

    let response_body = serde_json::to_vec(&health_info).map_err(|e| ApiError::InternalServerError {
        message: format!("Failed to serialize health info: {}", e),
    })?;

    Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(hyper::body::Bytes::from(response_body))
        .map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to build response: {}", e),
        })
}

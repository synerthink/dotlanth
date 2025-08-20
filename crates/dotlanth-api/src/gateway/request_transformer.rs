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

//! Request transformation between HTTP and gRPC protocols

use super::transcoder::TranscodingContext;
use crate::error::{ApiError, ApiResult};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{HeaderMap, Method, Request, Uri};
use serde_json::Value;
use std::collections::HashMap;
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing::{debug, error};

/// Transforms requests between HTTP and gRPC formats
pub struct RequestTransformer;

impl RequestTransformer {
    pub fn new() -> Self {
        Self
    }

    /// Transform HTTP request to gRPC request
    pub async fn transform_to_grpc(&self, http_request: &Request<Incoming>, context: &TranscodingContext) -> ApiResult<tonic::Request<Value>> {
        debug!("Transforming HTTP request to gRPC for method: {}", context.service_method.method_name);

        // Extract and parse the request body
        let body_bytes = self.extract_body_bytes(http_request).await?;
        let request_data = self.parse_request_body(&body_bytes, &context.content_type, context)?;

        // Create gRPC metadata from HTTP headers
        let metadata = self.create_grpc_metadata(&context.headers)?;

        // Create the gRPC request
        let mut grpc_request = tonic::Request::new(request_data);
        *grpc_request.metadata_mut() = metadata;

        // Set timeout if specified
        if let Some(timeout_header) = context.headers.get("x-timeout") {
            if let Ok(timeout_str) = timeout_header.to_str() {
                if let Ok(timeout_ms) = timeout_str.parse::<u64>() {
                    grpc_request.set_timeout(std::time::Duration::from_millis(timeout_ms));
                }
            }
        }

        Ok(grpc_request)
    }

    /// Transform gRPC request to HTTP request
    pub async fn transform_to_http(&self, grpc_request: tonic::Request<Value>, target_url: &str) -> ApiResult<Request<Full<Bytes>>> {
        debug!("Transforming gRPC request to HTTP for URL: {}", target_url);

        let (metadata, extensions, message) = grpc_request.into_parts();

        // Convert gRPC message to JSON
        let json_body = serde_json::to_vec(&message).map_err(|e| ApiError::SerdeJsonError(e))?;

        // Parse target URL
        let uri: Uri = target_url.parse().map_err(|e| ApiError::BadRequest {
            message: format!("Invalid target URL: {}", e),
        })?;

        // Create HTTP headers from gRPC metadata
        let headers = self.create_http_headers(&metadata)?;

        // Build HTTP request
        let mut request_builder = Request::builder().method(Method::POST).uri(uri);

        // Add headers
        for (key, value) in headers.iter() {
            request_builder = request_builder.header(key, value);
        }

        // Set content type
        request_builder = request_builder.header("content-type", "application/json");

        // Build the request
        let request = request_builder.body(Full::new(Bytes::from(json_body))).map_err(|e| ApiError::BadRequest {
            message: format!("Failed to build HTTP request: {}", e),
        })?;

        Ok(request)
    }

    /// Extract body bytes from HTTP request
    async fn extract_body_bytes(&self, request: &Request<Incoming>) -> ApiResult<Vec<u8>> {
        // Note: This is a simplified approach. In practice, you'd need to handle
        // the body more carefully, especially for streaming requests.

        // For this implementation, we'll create a placeholder since we can't
        // actually consume the body from a reference
        Ok(Vec::new())
    }

    /// Parse request body based on content type
    fn parse_request_body(&self, body_bytes: &[u8], content_type: &str, context: &TranscodingContext) -> ApiResult<Value> {
        if body_bytes.is_empty() {
            // For GET requests or empty bodies, create request from query parameters
            return self.create_request_from_query_params(context);
        }

        match content_type {
            "application/json" => serde_json::from_slice(body_bytes).map_err(|e| ApiError::BadRequest {
                message: format!("Invalid JSON: {}", e),
            }),
            "application/x-www-form-urlencoded" => {
                let form_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid UTF-8 in form data: {}", e),
                })?;

                let form_data: HashMap<String, String> = url::form_urlencoded::parse(form_str.as_bytes()).into_owned().collect();

                Ok(serde_json::to_value(form_data).map_err(|e| ApiError::SerdeJsonError(e))?)
            }
            "application/xml" => {
                // Simplified XML parsing - in practice, you'd use a proper XML parser
                Err(ApiError::BadRequest {
                    message: "XML parsing not yet implemented".to_string(),
                })
            }
            _ => Err(ApiError::BadRequest {
                message: format!("Unsupported content type for request body: {}", content_type),
            }),
        }
    }

    /// Create request data from query parameters (for GET requests)
    fn create_request_from_query_params(&self, context: &TranscodingContext) -> ApiResult<Value> {
        let mut request_data = serde_json::Map::new();

        // Add query parameters to request
        for (key, value) in &context.query_params {
            request_data.insert(key.clone(), Value::String(value.clone()));
        }

        // Add path parameters if any
        if let Some(path_params) = self.extract_path_parameters(&context.path, &context.service_method.method_name) {
            for (key, value) in path_params {
                request_data.insert(key, Value::String(value));
            }
        }

        Ok(Value::Object(request_data))
    }

    /// Extract path parameters from URL path
    fn extract_path_parameters(&self, path: &str, method_name: &str) -> Option<HashMap<String, String>> {
        let mut params = HashMap::new();

        // Handle common path parameter patterns
        match method_name {
            "GetDot" | "UpdateDot" | "DeleteDot" => {
                // Pattern: /api/v1/vm/dots/{dot_id}
                if let Some(captures) = regex::Regex::new(r"/api/v1/vm/dots/([^/]+)").ok()?.captures(path) {
                    if let Some(dot_id) = captures.get(1) {
                        params.insert("dot_id".to_string(), dot_id.as_str().to_string());
                    }
                }
            }
            "GetDocument" | "UpdateDocument" | "DeleteDocument" => {
                // Pattern: /api/v1/db/documents/{document_id}
                if let Some(captures) = regex::Regex::new(r"/api/v1/db/documents/([^/]+)").ok()?.captures(path) {
                    if let Some(doc_id) = captures.get(1) {
                        params.insert("document_id".to_string(), doc_id.as_str().to_string());
                    }
                }
            }
            _ => {}
        }

        if params.is_empty() { None } else { Some(params) }
    }

    /// Create gRPC metadata from HTTP headers
    fn create_grpc_metadata(&self, headers: &HeaderMap) -> ApiResult<MetadataMap> {
        let mut metadata = MetadataMap::new();

        for (name, value) in headers.iter() {
            let header_name = name.as_str();

            // Skip certain HTTP-specific headers
            if self.should_skip_header(header_name) {
                continue;
            }

            // Convert header name to gRPC metadata key format
            let metadata_key = self.convert_header_to_metadata_key(header_name);

            // Convert header value
            if let Ok(metadata_value) = MetadataValue::try_from(value.as_bytes()) {
                if let Ok(key) = metadata_key.parse::<tonic::metadata::MetadataKey<tonic::metadata::Ascii>>() {
                    metadata.insert(key, metadata_value);
                }
            }
        }

        // Add standard gRPC metadata
        metadata.insert("content-type", MetadataValue::from_static("application/grpc"));
        metadata.insert("user-agent", MetadataValue::from_static("dotlanth-gateway/1.0"));

        Ok(metadata)
    }

    /// Create HTTP headers from gRPC metadata
    fn create_http_headers(&self, metadata: &MetadataMap) -> ApiResult<HeaderMap> {
        let mut headers = HeaderMap::new();

        for key_and_value in metadata.iter() {
            match key_and_value {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    let header_name = self.convert_metadata_key_to_header(key.as_str());

                    if !self.should_skip_metadata(key.as_str()) {
                        if let Ok(header_value) = value.to_str() {
                            if let Ok(parsed_value) = header_value.parse() {
                                if let Ok(name) = header_name.parse::<hyper::header::HeaderName>() {
                                    headers.insert(name, parsed_value);
                                }
                            }
                        }
                    }
                }
                tonic::metadata::KeyAndValueRef::Binary(key, value) => {
                    // Handle binary metadata if needed
                    debug!("Skipping binary metadata: {}", key);
                }
            }
        }

        // Add standard HTTP headers
        headers.insert("user-agent", "dotlanth-gateway/1.0".parse().unwrap());

        Ok(headers)
    }

    /// Check if HTTP header should be skipped when converting to gRPC metadata
    fn should_skip_header(&self, header_name: &str) -> bool {
        matches!(
            header_name.to_lowercase().as_str(),
            "host" | "connection" | "upgrade" | "transfer-encoding" | "content-length" | "te" | "trailer" | "proxy-connection"
        )
    }

    /// Check if gRPC metadata should be skipped when converting to HTTP headers
    fn should_skip_metadata(&self, metadata_key: &str) -> bool {
        matches!(
            metadata_key.to_lowercase().as_str(),
            "content-type" | "grpc-encoding" | "grpc-accept-encoding" | "grpc-timeout" | "grpc-status" | "grpc-message"
        )
    }

    /// Convert HTTP header name to gRPC metadata key format
    fn convert_header_to_metadata_key(&self, header_name: &str) -> String {
        // gRPC metadata keys should be lowercase
        let mut key = header_name.to_lowercase();

        // Add grpc- prefix for custom headers to avoid conflicts
        if !key.starts_with("grpc-") && !self.is_standard_header(&key) {
            key = format!("x-{}", key);
        }

        key
    }

    /// Convert gRPC metadata key to HTTP header name format
    fn convert_metadata_key_to_header(&self, metadata_key: &str) -> String {
        let mut header = metadata_key.to_string();

        // Remove x- prefix if it was added during conversion
        if header.starts_with("x-") && !metadata_key.starts_with("x-") {
            header = header[2..].to_string();
        }

        header
    }

    /// Check if header is a standard HTTP header
    fn is_standard_header(&self, header_name: &str) -> bool {
        matches!(
            header_name,
            "authorization"
                | "accept"
                | "accept-encoding"
                | "accept-language"
                | "cache-control"
                | "cookie"
                | "origin"
                | "referer"
                | "user-agent"
                | "x-forwarded-for"
                | "x-forwarded-proto"
                | "x-real-ip"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{AUTHORIZATION, CONTENT_TYPE};

    #[test]
    fn test_header_to_metadata_conversion() {
        let transformer = RequestTransformer::new();

        assert_eq!(transformer.convert_header_to_metadata_key("Authorization"), "authorization");

        assert_eq!(transformer.convert_header_to_metadata_key("Custom-Header"), "x-custom-header");
    }

    #[test]
    fn test_metadata_to_header_conversion() {
        let transformer = RequestTransformer::new();

        assert_eq!(transformer.convert_metadata_key_to_header("authorization"), "authorization");

        assert_eq!(transformer.convert_metadata_key_to_header("x-custom-header"), "x-custom-header");
    }

    #[test]
    fn test_should_skip_header() {
        let transformer = RequestTransformer::new();

        assert!(transformer.should_skip_header("Host"));
        assert!(transformer.should_skip_header("Connection"));
        assert!(!transformer.should_skip_header("Authorization"));
    }

    #[test]
    fn test_path_parameter_extraction() {
        let transformer = RequestTransformer::new();

        let params = transformer.extract_path_parameters("/api/v1/vm/dots/test-dot-123", "GetDot");
        assert!(params.is_some());

        let params = params.unwrap();
        assert_eq!(params.get("dot_id"), Some(&"test-dot-123".to_string()));
    }
}

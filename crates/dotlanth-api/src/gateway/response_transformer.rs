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

//! Response transformation between gRPC and HTTP protocols

use super::error_mapping::ErrorMapper;
use super::protocol_negotiation::ProtocolNegotiator;
use super::transcoder::TranscodingContext;
use crate::error::{ApiError, ApiResult};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::{HeaderMap, Response, StatusCode};
use serde_json::Value;
use tonic::metadata::MetadataMap;
use tracing::{debug, error};

/// Transforms responses between gRPC and HTTP formats
pub struct ResponseTransformer {
    error_mapper: ErrorMapper,
    protocol_negotiator: ProtocolNegotiator,
}

impl ResponseTransformer {
    pub fn new() -> Self {
        Self {
            error_mapper: ErrorMapper::new(),
            protocol_negotiator: ProtocolNegotiator::new(),
        }
    }

    /// Transform gRPC response to HTTP response
    pub async fn transform_to_http(&self, grpc_response: tonic::Response<Value>, context: &TranscodingContext) -> ApiResult<Response<Full<Bytes>>> {
        debug!("Transforming gRPC response to HTTP for method: {}", context.service_method.method_name);

        let metadata = grpc_response.metadata().clone();
        let message = grpc_response.into_inner();

        // Convert the response message based on accept type
        let response_body = self.serialize_response_body(&message, &context.accept_type)?;

        // Create HTTP headers from gRPC metadata
        let mut headers = self.create_http_headers_from_metadata(&metadata)?;

        // Add content type header
        headers.insert("content-type", context.accept_type.parse().unwrap());

        // Add CORS headers
        self.add_cors_headers(&mut headers);

        // Build the HTTP response
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(response_body)))
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to build HTTP response: {}", e),
            })?;

        Ok(response)
    }

    /// Transform HTTP response to gRPC response
    pub async fn transform_to_grpc(&self, http_response: Response<Incoming>) -> ApiResult<tonic::Response<Value>> {
        debug!("Transforming HTTP response to gRPC");

        let (parts, body) = http_response.into_parts();

        // Check for HTTP errors
        if !parts.status.is_success() {
            let error_message = format!("HTTP error: {}", parts.status);
            let grpc_status = self.error_mapper.http_to_grpc_status(parts.status, Some(error_message));
            return Err(self.error_mapper.grpc_status_to_api_error(&grpc_status));
        }

        // Read the response body
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to read HTTP response body: {}", e),
            })?
            .to_bytes();

        // Parse the response body
        let response_data = self.parse_http_response_body(&body_bytes, &parts.headers)?;

        // Create gRPC metadata from HTTP headers
        let metadata = self.create_grpc_metadata_from_headers(&parts.headers)?;

        // Create gRPC response
        let mut grpc_response = tonic::Response::new(response_data);
        *grpc_response.metadata_mut() = metadata;

        Ok(grpc_response)
    }

    /// Serialize response body based on content type
    fn serialize_response_body(&self, data: &Value, content_type: &str) -> ApiResult<Vec<u8>> {
        match content_type {
            "application/json" => serde_json::to_vec_pretty(data).map_err(|e| ApiError::SerdeJsonError(e)),
            "application/xml" => {
                // Simplified XML serialization
                let xml_string = self.json_to_xml(data)?;
                Ok(xml_string.into_bytes())
            }
            "text/plain" => {
                let text = match data {
                    Value::String(s) => s.clone(),
                    _ => data.to_string(),
                };
                Ok(text.into_bytes())
            }
            _ => {
                // Default to JSON
                serde_json::to_vec_pretty(data).map_err(|e| ApiError::SerdeJsonError(e))
            }
        }
    }

    /// Parse HTTP response body
    fn parse_http_response_body(&self, body_bytes: &[u8], headers: &HeaderMap) -> ApiResult<Value> {
        let content_type = headers.get("content-type").and_then(|ct| ct.to_str().ok()).unwrap_or("application/json");

        match content_type {
            ct if ct.starts_with("application/json") => {
                if body_bytes.is_empty() {
                    Ok(Value::Null)
                } else {
                    serde_json::from_slice(body_bytes).map_err(|e| ApiError::SerdeJsonError(e))
                }
            }
            ct if ct.starts_with("text/") => {
                let text = String::from_utf8(body_bytes.to_vec()).map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid UTF-8 in response: {}", e),
                })?;
                Ok(Value::String(text))
            }
            ct if ct.starts_with("application/xml") => {
                // Simplified XML parsing
                let xml_text = String::from_utf8(body_bytes.to_vec()).map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid UTF-8 in XML response: {}", e),
                })?;
                self.xml_to_json(&xml_text)
            }
            _ => {
                // Treat as binary data
                let base64_data = base64::encode(body_bytes);
                Ok(Value::String(base64_data))
            }
        }
    }

    /// Create HTTP headers from gRPC metadata
    fn create_http_headers_from_metadata(&self, metadata: &MetadataMap) -> ApiResult<HeaderMap> {
        let mut headers = HeaderMap::new();

        for key_and_value in metadata.iter() {
            match key_and_value {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    let header_name = key.as_str();

                    // Skip gRPC-specific metadata
                    if self.should_skip_grpc_metadata(header_name) {
                        continue;
                    }

                    // Convert metadata key to HTTP header name
                    let http_header_name = self.convert_metadata_to_header_name(header_name);

                    if let Ok(header_value) = value.to_str() {
                        if let Ok(parsed_value) = header_value.parse() {
                            if let Ok(name) = http_header_name.parse::<hyper::header::HeaderName>() {
                                headers.insert(name, parsed_value);
                            }
                        }
                    }
                }
                tonic::metadata::KeyAndValueRef::Binary(key, _value) => {
                    debug!("Skipping binary metadata in HTTP response: {}", key);
                }
            }
        }

        Ok(headers)
    }

    /// Create gRPC metadata from HTTP headers
    fn create_grpc_metadata_from_headers(&self, headers: &HeaderMap) -> ApiResult<MetadataMap> {
        let mut metadata = MetadataMap::new();

        for (name, value) in headers.iter() {
            let header_name = name.as_str();

            // Skip HTTP-specific headers
            if self.should_skip_http_header(header_name) {
                continue;
            }

            // Convert header name to metadata key
            let metadata_key = self.convert_header_to_metadata_key(header_name);

            if let Ok(metadata_value) = tonic::metadata::MetadataValue::try_from(value.as_bytes()) {
                if let Ok(key) = metadata_key.parse::<tonic::metadata::MetadataKey<tonic::metadata::Ascii>>() {
                    metadata.insert(key, metadata_value);
                }
            }
        }

        Ok(metadata)
    }

    /// Add CORS headers to HTTP response
    fn add_cors_headers(&self, headers: &mut HeaderMap) {
        if let Ok(value) = "*".parse() {
            headers.insert("access-control-allow-origin", value);
        }

        if let Ok(value) = "GET, POST, PUT, DELETE, OPTIONS".parse() {
            headers.insert("access-control-allow-methods", value);
        }

        if let Ok(value) = "Content-Type, Authorization, Accept".parse() {
            headers.insert("access-control-allow-headers", value);
        }

        if let Ok(value) = "true".parse() {
            headers.insert("access-control-allow-credentials", value);
        }
    }

    /// Convert JSON to XML (simplified)
    fn json_to_xml(&self, data: &Value) -> ApiResult<String> {
        match data {
            Value::Object(map) => {
                let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<response>\n");
                for (key, value) in map {
                    xml.push_str(&format!("  <{}>{}</{}>\n", key, self.value_to_xml_content(value)?, key));
                }
                xml.push_str("</response>");
                Ok(xml)
            }
            _ => Ok(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<response>{}</response>", self.value_to_xml_content(data)?)),
        }
    }

    /// Convert JSON value to XML content
    fn value_to_xml_content(&self, value: &Value) -> ApiResult<String> {
        match value {
            Value::String(s) => Ok(self.escape_xml(s)),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok(String::new()),
            Value::Array(arr) => {
                let mut content = String::new();
                for item in arr {
                    content.push_str(&format!("<item>{}</item>", self.value_to_xml_content(item)?));
                }
                Ok(content)
            }
            Value::Object(map) => {
                let mut content = String::new();
                for (key, val) in map {
                    content.push_str(&format!("<{}>{}</{}>\n", key, self.value_to_xml_content(val)?, key));
                }
                Ok(content)
            }
        }
    }

    /// Escape XML special characters
    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
    }

    /// Convert XML to JSON (simplified)
    fn xml_to_json(&self, xml: &str) -> ApiResult<Value> {
        // This is a very simplified XML to JSON conversion
        // In practice, you'd use a proper XML parser like quick-xml

        // For now, just wrap the XML content as a string
        Ok(Value::String(xml.to_string()))
    }

    /// Check if gRPC metadata should be skipped in HTTP response
    fn should_skip_grpc_metadata(&self, key: &str) -> bool {
        matches!(
            key.to_lowercase().as_str(),
            "content-type" | "grpc-encoding" | "grpc-accept-encoding" | "grpc-timeout" | "grpc-status" | "grpc-message" | "grpc-status-details-bin"
        )
    }

    /// Check if HTTP header should be skipped in gRPC metadata
    fn should_skip_http_header(&self, header: &str) -> bool {
        matches!(
            header.to_lowercase().as_str(),
            "content-length" | "transfer-encoding" | "connection" | "upgrade" | "host" | "te" | "trailer"
        )
    }

    /// Convert metadata key to HTTP header name
    fn convert_metadata_to_header_name(&self, metadata_key: &str) -> String {
        // Remove x- prefix if it was added during HTTP to gRPC conversion
        if metadata_key.starts_with("x-") {
            metadata_key[2..].to_string()
        } else {
            metadata_key.to_string()
        }
    }

    /// Convert HTTP header name to metadata key
    fn convert_header_to_metadata_key(&self, header_name: &str) -> String {
        let key = header_name.to_lowercase();

        // Add x- prefix for custom headers
        if !self.is_standard_header(&key) { format!("x-{}", key) } else { key }
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
                | "content-type"
                | "content-encoding"
                | "content-language"
        )
    }
}

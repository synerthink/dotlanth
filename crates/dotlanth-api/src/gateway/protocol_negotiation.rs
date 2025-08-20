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

//! Protocol negotiation for content types and encoding

use crate::error::{ApiError, ApiResult};
use hyper::HeaderMap;
use mime::Mime;
use std::str::FromStr;
use tracing::{debug, warn};

/// Supported content types for the gateway
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Json,
    Protobuf,
    MessagePack,
    Xml,
    FormUrlEncoded,
    Multipart,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Json => "application/json",
            ContentType::Protobuf => "application/x-protobuf",
            ContentType::MessagePack => "application/msgpack",
            ContentType::Xml => "application/xml",
            ContentType::FormUrlEncoded => "application/x-www-form-urlencoded",
            ContentType::Multipart => "multipart/form-data",
        }
    }

    pub fn from_mime(mime: &Mime) -> Option<Self> {
        match (mime.type_(), mime.subtype()) {
            (mime::APPLICATION, mime::JSON) => Some(ContentType::Json),
            (mime::APPLICATION, name) if name == "x-protobuf" => Some(ContentType::Protobuf),
            (mime::APPLICATION, name) if name == "msgpack" => Some(ContentType::MessagePack),
            (mime::APPLICATION, mime::XML) => Some(ContentType::Xml),
            (mime::APPLICATION, mime::WWW_FORM_URLENCODED) => Some(ContentType::FormUrlEncoded),
            (mime::MULTIPART, mime::FORM_DATA) => Some(ContentType::Multipart),
            _ => None,
        }
    }
}

/// Supported encoding types
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    Identity,
    Gzip,
    Deflate,
    Brotli,
}

impl Encoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Encoding::Identity => "identity",
            Encoding::Gzip => "gzip",
            Encoding::Deflate => "deflate",
            Encoding::Brotli => "br",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "identity" => Some(Encoding::Identity),
            "gzip" => Some(Encoding::Gzip),
            "deflate" => Some(Encoding::Deflate),
            "br" | "brotli" => Some(Encoding::Brotli),
            _ => None,
        }
    }
}

/// Protocol negotiator for handling content type and encoding negotiation
pub struct ProtocolNegotiator {
    supported_content_types: Vec<ContentType>,
    supported_encodings: Vec<Encoding>,
    default_content_type: ContentType,
    default_encoding: Encoding,
}

impl ProtocolNegotiator {
    pub fn new() -> Self {
        Self {
            supported_content_types: vec![ContentType::Json, ContentType::Protobuf, ContentType::MessagePack, ContentType::Xml, ContentType::FormUrlEncoded],
            supported_encodings: vec![Encoding::Identity, Encoding::Gzip, Encoding::Deflate, Encoding::Brotli],
            default_content_type: ContentType::Json,
            default_encoding: Encoding::Identity,
        }
    }

    /// Negotiate content types from HTTP headers
    pub fn negotiate_content_types(&self, headers: &HeaderMap) -> ApiResult<(String, String)> {
        let content_type = self.negotiate_content_type(headers)?;
        let accept_type = self.negotiate_accept_type(headers)?;

        Ok((content_type, accept_type))
    }

    /// Negotiate content type from Content-Type header
    pub fn negotiate_content_type(&self, headers: &HeaderMap) -> ApiResult<String> {
        if let Some(content_type_header) = headers.get("content-type") {
            let content_type_str = content_type_header.to_str().map_err(|_| ApiError::BadRequest {
                message: "Invalid Content-Type header".to_string(),
            })?;

            // Parse the MIME type
            let mime = Mime::from_str(content_type_str).map_err(|_| ApiError::BadRequest {
                message: format!("Invalid Content-Type: {}", content_type_str),
            })?;

            // Check if we support this content type
            if let Some(content_type) = ContentType::from_mime(&mime) {
                if self.supported_content_types.contains(&content_type) {
                    return Ok(content_type.as_str().to_string());
                }
            }

            warn!("Unsupported content type: {}", content_type_str);
            return Err(ApiError::BadRequest {
                message: format!("Unsupported Content-Type: {}", content_type_str),
            });
        }

        // Default to JSON if no Content-Type header
        Ok(self.default_content_type.as_str().to_string())
    }

    /// Negotiate accept type from Accept header
    pub fn negotiate_accept_type(&self, headers: &HeaderMap) -> ApiResult<String> {
        if let Some(accept_header) = headers.get("accept") {
            let accept_str = accept_header.to_str().map_err(|_| ApiError::BadRequest {
                message: "Invalid Accept header".to_string(),
            })?;

            // Parse accept header (simplified - doesn't handle q-values)
            for accept_part in accept_str.split(',') {
                let accept_part = accept_part.trim();

                // Handle wildcard
                if accept_part == "*/*" || accept_part == "application/*" {
                    return Ok(self.default_content_type.as_str().to_string());
                }

                // Try to parse as MIME type
                if let Ok(mime) = Mime::from_str(accept_part) {
                    if let Some(content_type) = ContentType::from_mime(&mime) {
                        if self.supported_content_types.contains(&content_type) {
                            return Ok(content_type.as_str().to_string());
                        }
                    }
                }
            }

            debug!("No supported accept type found in: {}", accept_str);
        }

        // Default to JSON
        Ok(self.default_content_type.as_str().to_string())
    }

    /// Negotiate encoding from Accept-Encoding header
    pub fn negotiate_encoding(&self, headers: &HeaderMap) -> Encoding {
        if let Some(encoding_header) = headers.get("accept-encoding") {
            if let Ok(encoding_str) = encoding_header.to_str() {
                // Parse accept-encoding header (simplified)
                for encoding_part in encoding_str.split(',') {
                    let encoding_part = encoding_part.trim();

                    if let Some(encoding) = Encoding::from_str(encoding_part) {
                        if self.supported_encodings.contains(&encoding) {
                            return encoding;
                        }
                    }
                }
            }
        }

        self.default_encoding.clone()
    }

    /// Check if a content type is supported
    pub fn is_supported_content_type(&self, content_type: &str) -> bool {
        if let Ok(mime) = Mime::from_str(content_type) {
            if let Some(ct) = ContentType::from_mime(&mime) {
                return self.supported_content_types.contains(&ct);
            }
        }
        false
    }

    /// Check if an encoding is supported
    pub fn is_supported_encoding(&self, encoding: &str) -> bool {
        if let Some(enc) = Encoding::from_str(encoding) {
            return self.supported_encodings.contains(&enc);
        }
        false
    }

    /// Get the best content type for gRPC communication
    pub fn get_grpc_content_type(&self) -> String {
        ContentType::Protobuf.as_str().to_string()
    }

    /// Get the best content type for HTTP communication
    pub fn get_http_content_type(&self, preferred: Option<&str>) -> String {
        if let Some(pref) = preferred {
            if self.is_supported_content_type(pref) {
                return pref.to_string();
            }
        }
        self.default_content_type.as_str().to_string()
    }

    /// Create appropriate headers for response
    pub fn create_response_headers(&self, content_type: &str, encoding: &Encoding) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Ok(ct_value) = content_type.parse() {
            headers.insert("content-type", ct_value);
        }

        if *encoding != Encoding::Identity {
            if let Ok(enc_value) = encoding.as_str().parse() {
                headers.insert("content-encoding", enc_value);
            }
        }

        // Add CORS headers
        if let Ok(cors_value) = "*".parse() {
            headers.insert("access-control-allow-origin", cors_value);
        }

        if let Ok(methods_value) = "GET, POST, PUT, DELETE, OPTIONS".parse() {
            headers.insert("access-control-allow-methods", methods_value);
        }

        if let Ok(headers_value) = "Content-Type, Authorization, Accept, Accept-Encoding".parse() {
            headers.insert("access-control-allow-headers", headers_value);
        }

        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{ACCEPT, CONTENT_TYPE};

    #[test]
    fn test_content_type_negotiation() {
        let negotiator = ProtocolNegotiator::new();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let result = negotiator.negotiate_content_type(&headers).unwrap();
        assert_eq!(result, "application/json");
    }

    #[test]
    fn test_accept_type_negotiation() {
        let negotiator = ProtocolNegotiator::new();
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, "application/json, application/xml".parse().unwrap());

        let result = negotiator.negotiate_accept_type(&headers).unwrap();
        assert_eq!(result, "application/json");
    }

    #[test]
    fn test_encoding_negotiation() {
        let negotiator = ProtocolNegotiator::new();
        let mut headers = HeaderMap::new();
        headers.insert("accept-encoding", "gzip, deflate".parse().unwrap());

        let result = negotiator.negotiate_encoding(&headers);
        assert_eq!(result, Encoding::Gzip);
    }

    #[test]
    fn test_unsupported_content_type() {
        let negotiator = ProtocolNegotiator::new();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());

        let result = negotiator.negotiate_content_type(&headers);
        assert!(result.is_err());
    }
}

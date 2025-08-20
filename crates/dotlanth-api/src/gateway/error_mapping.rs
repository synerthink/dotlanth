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

//! Error code mapping between gRPC and HTTP protocols

use crate::error::ApiError;
use hyper::StatusCode;
use serde_json::json;
use tonic::{Code, Status};
use tracing::warn;

/// Maps errors between gRPC and HTTP protocols
pub struct ErrorMapper;

impl ErrorMapper {
    pub fn new() -> Self {
        Self
    }

    /// Map gRPC status to HTTP status code
    pub fn grpc_to_http_status(&self, grpc_status: &Status) -> StatusCode {
        match grpc_status.code() {
            Code::Ok => StatusCode::OK,
            Code::Cancelled => StatusCode::REQUEST_TIMEOUT,
            Code::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
            Code::InvalidArgument => StatusCode::BAD_REQUEST,
            Code::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
            Code::NotFound => StatusCode::NOT_FOUND,
            Code::AlreadyExists => StatusCode::CONFLICT,
            Code::PermissionDenied => StatusCode::FORBIDDEN,
            Code::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
            Code::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
            Code::Aborted => StatusCode::CONFLICT,
            Code::OutOfRange => StatusCode::BAD_REQUEST,
            Code::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Code::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Code::DataLoss => StatusCode::INTERNAL_SERVER_ERROR,
            Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        }
    }

    /// Map HTTP status code to gRPC status
    pub fn http_to_grpc_status(&self, http_status: StatusCode, message: Option<String>) -> Status {
        let msg = message.unwrap_or_else(|| "HTTP error".to_string());

        match http_status {
            StatusCode::OK => Status::ok(msg),
            StatusCode::BAD_REQUEST => Status::invalid_argument(msg),
            StatusCode::UNAUTHORIZED => Status::unauthenticated(msg),
            StatusCode::FORBIDDEN => Status::permission_denied(msg),
            StatusCode::NOT_FOUND => Status::not_found(msg),
            StatusCode::CONFLICT => Status::already_exists(msg),
            StatusCode::PRECONDITION_FAILED => Status::failed_precondition(msg),
            StatusCode::TOO_MANY_REQUESTS => Status::resource_exhausted(msg),
            StatusCode::REQUEST_TIMEOUT => Status::deadline_exceeded(msg),
            StatusCode::GATEWAY_TIMEOUT => Status::deadline_exceeded(msg),
            StatusCode::NOT_IMPLEMENTED => Status::unimplemented(msg),
            StatusCode::SERVICE_UNAVAILABLE => Status::unavailable(msg),
            StatusCode::INTERNAL_SERVER_ERROR => Status::internal(msg),
            _ => {
                warn!("Unmapped HTTP status code: {}", http_status);
                Status::unknown(format!("HTTP {}: {}", http_status.as_u16(), msg))
            }
        }
    }

    /// Map ApiError to gRPC status
    pub fn api_error_to_grpc_status(&self, error: &ApiError) -> Status {
        match error {
            ApiError::BadRequest { message } => Status::invalid_argument(message),
            ApiError::Unauthorized { message } => Status::unauthenticated(message),
            ApiError::Forbidden { message } => Status::permission_denied(message),
            ApiError::NotFound { message } => Status::not_found(message),
            ApiError::Conflict { message } => Status::already_exists(message),
            ApiError::MethodNotAllowed { message } => Status::invalid_argument(message),
            ApiError::UnprocessableEntity { message } => Status::invalid_argument(message),
            ApiError::TooManyRequests { message } => Status::resource_exhausted(message),
            ApiError::ServiceUnavailable { message } => Status::unavailable(message),
            ApiError::GatewayTimeout { message } => Status::deadline_exceeded(message),
            ApiError::InternalServerError { message } => Status::internal(message),
            ApiError::GrpcError(status) => status.clone(),
            ApiError::JwtError(e) => Status::unauthenticated(format!("JWT error: {}", e)),
            ApiError::SerdeJsonError(e) => Status::internal(format!("JSON error: {}", e)),
            ApiError::HyperError(e) => Status::internal(format!("HTTP error: {}", e)),
            ApiError::IoError(e) => Status::internal(format!("IO error: {}", e)),
            ApiError::HttpError(e) => Status::internal(format!("HTTP error: {}", e)),
            ApiError::RouterError(e) => Status::internal(format!("Router error: {}", e)),
        }
    }

    /// Map gRPC status to ApiError
    pub fn grpc_status_to_api_error(&self, status: &Status) -> ApiError {
        let message = status.message().to_string();

        match status.code() {
            Code::Ok => ApiError::InternalServerError {
                message: "Unexpected OK status in error context".to_string(),
            },
            Code::InvalidArgument => ApiError::BadRequest { message },
            Code::Unauthenticated => ApiError::Unauthorized { message },
            Code::PermissionDenied => ApiError::Forbidden { message },
            Code::NotFound => ApiError::NotFound { message },
            Code::AlreadyExists => ApiError::Conflict { message },
            Code::ResourceExhausted => ApiError::TooManyRequests { message },
            Code::DeadlineExceeded => ApiError::GatewayTimeout { message },
            Code::Unavailable => ApiError::ServiceUnavailable { message },
            Code::Cancelled => ApiError::GatewayTimeout {
                message: format!("Request cancelled: {}", message),
            },
            Code::Unimplemented => ApiError::BadRequest {
                message: format!("Method not implemented: {}", message),
            },
            _ => ApiError::InternalServerError {
                message: format!("gRPC error ({}): {}", status.code(), message),
            },
        }
    }

    /// Create error response body for HTTP
    pub fn create_http_error_body(&self, status: StatusCode, message: &str, details: Option<serde_json::Value>) -> serde_json::Value {
        let mut error_body = json!({
            "error": {
                "code": status.as_u16(),
                "status": status.canonical_reason().unwrap_or("Unknown"),
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        if let Some(details) = details {
            error_body["error"]["details"] = details;
        }

        error_body
    }

    /// Create error metadata for gRPC
    pub fn create_grpc_error_metadata(&self, original_error: &ApiError) -> tonic::metadata::MetadataMap {
        let mut metadata = tonic::metadata::MetadataMap::new();

        // Add error type
        let error_type = match original_error {
            ApiError::BadRequest { .. } => "bad_request",
            ApiError::Unauthorized { .. } => "unauthorized",
            ApiError::Forbidden { .. } => "forbidden",
            ApiError::NotFound { .. } => "not_found",
            ApiError::MethodNotAllowed { .. } => "method_not_allowed",
            ApiError::Conflict { .. } => "conflict",
            ApiError::UnprocessableEntity { .. } => "unprocessable_entity",
            ApiError::TooManyRequests { .. } => "too_many_requests",
            ApiError::ServiceUnavailable { .. } => "service_unavailable",
            ApiError::GatewayTimeout { .. } => "gateway_timeout",
            ApiError::InternalServerError { .. } => "internal_server_error",
            ApiError::GrpcError(_) => "grpc_error",
            ApiError::JwtError(_) => "jwt_error",
            ApiError::SerdeJsonError(_) => "json_error",
            ApiError::HyperError(_) => "http_error",
            ApiError::IoError(_) => "io_error",
            ApiError::HttpError(_) => "http_error",
            ApiError::RouterError(_) => "router_error",
        };

        if let Ok(value) = error_type.parse() {
            metadata.insert("error-type", value);
        }

        // Add timestamp
        if let Ok(timestamp) = chrono::Utc::now().to_rfc3339().parse() {
            metadata.insert("error-timestamp", timestamp);
        }

        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_to_http_status_mapping() {
        let mapper = ErrorMapper::new();

        assert_eq!(mapper.grpc_to_http_status(&Status::ok("test")), StatusCode::OK);
        assert_eq!(mapper.grpc_to_http_status(&Status::invalid_argument("test")), StatusCode::BAD_REQUEST);
        assert_eq!(mapper.grpc_to_http_status(&Status::not_found("test")), StatusCode::NOT_FOUND);
        assert_eq!(mapper.grpc_to_http_status(&Status::internal("test")), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_http_to_grpc_status_mapping() {
        let mapper = ErrorMapper::new();

        assert_eq!(mapper.http_to_grpc_status(StatusCode::OK, None).code(), Code::Ok);
        assert_eq!(mapper.http_to_grpc_status(StatusCode::BAD_REQUEST, None).code(), Code::InvalidArgument);
        assert_eq!(mapper.http_to_grpc_status(StatusCode::NOT_FOUND, None).code(), Code::NotFound);
        assert_eq!(mapper.http_to_grpc_status(StatusCode::INTERNAL_SERVER_ERROR, None).code(), Code::Internal);
    }

    #[test]
    fn test_error_body_creation() {
        let mapper = ErrorMapper::new();
        let body = mapper.create_http_error_body(StatusCode::BAD_REQUEST, "Invalid input", Some(json!({"field": "name"})));

        assert_eq!(body["error"]["code"], 400);
        assert_eq!(body["error"]["message"], "Invalid input");
        assert_eq!(body["error"]["details"]["field"], "name");
    }
}

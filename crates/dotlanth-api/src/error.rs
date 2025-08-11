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

//! Error handling for the REST API gateway
//! Implements RFC 7807 Problem Details format

use http_body_util::Full;
use hyper::{Response, StatusCode, body::Bytes};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::error;

/// API error types following REST conventions
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Bad request: {message}")]
    BadRequest { message: String },

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("Forbidden: {message}")]
    Forbidden { message: String },

    #[error("Not found: {message}")]
    NotFound { message: String },

    #[error("Method not allowed: {message}")]
    MethodNotAllowed { message: String },

    #[error("Conflict: {message}")]
    Conflict { message: String },

    #[error("Unprocessable entity: {message}")]
    UnprocessableEntity { message: String },

    #[error("Too many requests: {message}")]
    TooManyRequests { message: String },

    #[error("Internal server error: {message}")]
    InternalServerError { message: String },

    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    #[error("Gateway timeout: {message}")]
    GatewayTimeout { message: String },

    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Serde JSON error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Hyper error: {0}")]
    HyperError(#[from] hyper::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Router error: {0}")]
    RouterError(String),
}

impl ApiError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::GatewayTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            ApiError::GrpcError(status) => match status.code() {
                tonic::Code::InvalidArgument => StatusCode::BAD_REQUEST,
                tonic::Code::Unauthenticated => StatusCode::UNAUTHORIZED,
                tonic::Code::PermissionDenied => StatusCode::FORBIDDEN,
                tonic::Code::NotFound => StatusCode::NOT_FOUND,
                tonic::Code::AlreadyExists => StatusCode::CONFLICT,
                tonic::Code::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
                tonic::Code::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
                tonic::Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                tonic::Code::DeadlineExceeded => StatusCode::GATEWAY_TIMEOUT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiError::JwtError(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error type identifier
    pub fn error_type(&self) -> &'static str {
        match self {
            ApiError::BadRequest { .. } => "bad_request",
            ApiError::Unauthorized { .. } => "unauthorized",
            ApiError::Forbidden { .. } => "forbidden",
            ApiError::NotFound { .. } => "not_found",
            ApiError::MethodNotAllowed { .. } => "method_not_allowed",
            ApiError::Conflict { .. } => "conflict",
            ApiError::UnprocessableEntity { .. } => "unprocessable_entity",
            ApiError::TooManyRequests { .. } => "too_many_requests",
            ApiError::InternalServerError { .. } => "internal_server_error",
            ApiError::ServiceUnavailable { .. } => "service_unavailable",
            ApiError::GatewayTimeout { .. } => "gateway_timeout",
            ApiError::GrpcError(_) => "grpc_error",
            ApiError::JwtError(_) => "jwt_error",
            ApiError::SerdeJsonError(_) => "json_error",
            ApiError::HyperError(_) => "http_error",
            ApiError::IoError(_) => "io_error",
            ApiError::HttpError(_) => "http_error",
            ApiError::RouterError(_) => "router_error",
        }
    }
}

/// RFC 7807 Problem Details response format
#[derive(Debug, Serialize, Deserialize)]
pub struct ProblemDetails {
    /// A URI reference that identifies the problem type
    #[serde(rename = "type")]
    pub problem_type: String,

    /// A short, human-readable summary of the problem type
    pub title: String,

    /// The HTTP status code generated by the origin server
    pub status: u16,

    /// A human-readable explanation specific to this occurrence
    pub detail: String,

    /// A URI reference that identifies the specific occurrence
    pub instance: String,

    /// Additional extension members
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ProblemDetails {
    /// Create a new problem details response
    pub fn new(error: &ApiError, instance: String) -> Self {
        let status_code = error.status_code();
        let error_type = error.error_type();

        Self {
            problem_type: format!("https://api.dotlanth.com/problems/{}", error_type),
            title: Self::status_to_title(status_code),
            status: status_code.as_u16(),
            detail: error.to_string(),
            instance,
            extensions: HashMap::new(),
        }
    }

    /// Add extension data to the problem details
    pub fn with_extension(mut self, key: String, value: serde_json::Value) -> Self {
        self.extensions.insert(key, value);
        self
    }

    /// Convert status code to human-readable title
    fn status_to_title(status: StatusCode) -> String {
        match status {
            StatusCode::BAD_REQUEST => "Bad Request".to_string(),
            StatusCode::UNAUTHORIZED => "Unauthorized".to_string(),
            StatusCode::FORBIDDEN => "Forbidden".to_string(),
            StatusCode::NOT_FOUND => "Not Found".to_string(),
            StatusCode::METHOD_NOT_ALLOWED => "Method Not Allowed".to_string(),
            StatusCode::CONFLICT => "Conflict".to_string(),
            StatusCode::UNPROCESSABLE_ENTITY => "Unprocessable Entity".to_string(),
            StatusCode::TOO_MANY_REQUESTS => "Too Many Requests".to_string(),
            StatusCode::INTERNAL_SERVER_ERROR => "Internal Server Error".to_string(),
            StatusCode::SERVICE_UNAVAILABLE => "Service Unavailable".to_string(),
            StatusCode::GATEWAY_TIMEOUT => "Gateway Timeout".to_string(),
            _ => "Unknown Error".to_string(),
        }
    }
}

/// Convert ApiError to HTTP response
impl From<ApiError> for Response<Full<Bytes>> {
    fn from(error: ApiError) -> Self {
        let status_code = error.status_code();
        let problem_details = ProblemDetails::new(&error, "/".to_string());

        // Log the error
        error!("API Error: {} - {}", status_code, error);

        // Serialize problem details
        let json = match serde_json::to_string(&problem_details) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize error response: {}", e);
                r#"{"type":"https://api.dotlanth.com/problems/internal_server_error","title":"Internal Server Error","status":500,"detail":"An internal error occurred","instance":"/"}"#.to_string()
            }
        };

        Response::builder()
            .status(status_code)
            .header("content-type", "application/problem+json")
            .header("cache-control", "no-cache")
            .body(Full::new(Bytes::from(json)))
            .unwrap_or_else(|e| {
                error!("Failed to build error response: {}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Internal Server Error")))
                    .unwrap()
            })
    }
}

/// Result type for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// From implementations for common errors
impl From<hyper::http::Error> for ApiError {
    fn from(err: hyper::http::Error) -> Self {
        ApiError::HttpError(err.to_string())
    }
}

impl From<matchit::InsertError> for ApiError {
    fn from(err: matchit::InsertError) -> Self {
        ApiError::RouterError(err.to_string())
    }
}

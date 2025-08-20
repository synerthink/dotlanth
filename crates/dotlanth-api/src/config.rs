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

//! Configuration management for the REST API gateway

use std::env;

/// Configuration for the REST API gateway
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind the HTTP server to
    pub bind_address: String,

    /// Address of the gRPC VM service
    pub vm_service_address: String,

    /// Address of the gRPC Database service (via VM service)
    pub db_service_address: String,

    /// JWT secret key for authentication
    pub jwt_secret: String,

    /// Enable CORS for web clients
    pub cors_enabled: bool,

    /// Allowed CORS origins
    pub cors_origins: Vec<String>,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Maximum request body size in bytes
    pub max_body_size: usize,

    /// Enable OpenAPI documentation
    pub openapi_enabled: bool,

    /// OpenAPI documentation path
    pub openapi_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".to_string(),
            vm_service_address: "http://127.0.0.1:50051".to_string(),
            db_service_address: "http://127.0.0.1:50051".to_string(), // VM service handles DB operations
            jwt_secret: "default-secret-change-in-production".to_string(),
            cors_enabled: true,
            cors_origins: vec!["http://localhost:3000".to_string()],
            request_timeout_secs: 30,
            max_body_size: 10 * 1024 * 1024, // 10MB
            openapi_enabled: true,
            openapi_path: "/docs".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            bind_address: env::var("DOTLANTH_API_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),

            vm_service_address: env::var("DOTLANTH_VM_SERVICE_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string()),

            db_service_address: env::var("DOTLANTH_DB_SERVICE_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string()),

            jwt_secret: env::var("DOTLANTH_JWT_SECRET").unwrap_or_else(|_| "default-secret-change-in-production".to_string()),

            cors_enabled: env::var("DOTLANTH_CORS_ENABLED").map(|v| v.parse().unwrap_or(true)).unwrap_or(true),

            cors_origins: env::var("DOTLANTH_CORS_ORIGINS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["http://localhost:3000".to_string()]),

            request_timeout_secs: env::var("DOTLANTH_REQUEST_TIMEOUT_SECS").map(|v| v.parse().unwrap_or(30)).unwrap_or(30),

            max_body_size: env::var("DOTLANTH_MAX_BODY_SIZE").map(|v| v.parse().unwrap_or(10 * 1024 * 1024)).unwrap_or(10 * 1024 * 1024),

            openapi_enabled: env::var("DOTLANTH_OPENAPI_ENABLED").map(|v| v.parse().unwrap_or(true)).unwrap_or(true),

            openapi_path: env::var("DOTLANTH_OPENAPI_PATH").unwrap_or_else(|_| "/docs".to_string()),
        }
    }
}

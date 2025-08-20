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

//! Health check handlers

use crate::db::DatabaseClient;
use crate::error::ApiError;
use crate::models::{ApiVersion, HealthResponse, ServiceStatus};
use crate::vm::VmClient;
use chrono::Utc;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode, body::Bytes};
use std::collections::HashMap;
use std::time::Instant;
use tracing::info;

/// Health check handler
/// GET /api/v1/health
#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy")
    ),
    tag = "Health"
)]
pub async fn health_check(_req: Request<hyper::body::Incoming>, db_client: DatabaseClient, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing health check request");

    let mut services = HashMap::new();
    let mut overall_healthy = true;

    // Check database service
    let db_start = Instant::now();
    let db_healthy = db_client.health_check().await.unwrap_or(false);
    let db_response_time = db_start.elapsed().as_millis() as u64;

    services.insert(
        "database".to_string(),
        ServiceStatus {
            status: if db_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
            response_time_ms: db_response_time,
            last_checked: Utc::now(),
        },
    );

    if !db_healthy {
        overall_healthy = false;
    }

    // Check VM service
    let vm_start = Instant::now();
    let vm_healthy = vm_client.health_check().await.unwrap_or(false);
    let vm_response_time = vm_start.elapsed().as_millis() as u64;

    services.insert(
        "vm".to_string(),
        ServiceStatus {
            status: if vm_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
            response_time_ms: vm_response_time,
            last_checked: Utc::now(),
        },
    );

    if !vm_healthy {
        overall_healthy = false;
    }

    let health_response = HealthResponse {
        status: if overall_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services,
    };

    let status_code = if overall_healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    let response_json = serde_json::to_string(&health_response)?;

    Ok(Response::builder()
        .status(status_code)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Version information handler
/// GET /api/v1/version
#[utoipa::path(
    get,
    path = "/api/v1/version",
    responses(
        (status = 200, description = "API version information", body = ApiVersion)
    ),
    tag = "Health"
)]
pub async fn version_info(_req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing version info request");

    let version_info = ApiVersion {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build: format!("{}+{}", env!("CARGO_PKG_VERSION"), option_env!("GIT_HASH").unwrap_or("unknown")),
        features: vec![
            "database_collections".to_string(),
            "document_management".to_string(),
            "vm_dot_deployment".to_string(),
            "vm_dot_execution".to_string(),
            "jwt_authentication".to_string(),
            "rbac_authorization".to_string(),
            "openapi_docs".to_string(),
            "cors_support".to_string(),
            "request_validation".to_string(),
            "error_handling".to_string(),
        ],
    };

    let response_json = serde_json::to_string(&version_info)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

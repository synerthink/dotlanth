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

//! VM handlers

use crate::error::ApiError;
use crate::middleware::{check_permissions, extract_claims};
use crate::models::{DeployDotRequest, DeployDotResponse, DotState, ExecuteDotRequest, ExecuteDotResponse};
use crate::vm::VmClient;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Bytes};
use percent_encoding::percent_decode_str;
use tracing::{error, info};

/// Deploy a new dot
/// POST /api/v1/vm/dots/deploy
#[utoipa::path(
    post,
    path = "/api/v1/vm/dots/deploy",
    request_body = DeployDotRequest,
    responses(
        (status = 201, description = "Dot deployed successfully", body = DeployDotResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 422, description = "Bytecode validation failed")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn deploy_dot(req: Request<hyper::body::Incoming>, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing deploy dot request");

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["deploy:dots"])?;

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let deploy_request: DeployDotRequest = serde_json::from_slice(&body)?;

    // Validate request
    if deploy_request.name.is_empty() {
        return Err(ApiError::BadRequest {
            message: "Dot name cannot be empty".to_string(),
        });
    }

    if deploy_request.name.len() > 64 {
        return Err(ApiError::BadRequest {
            message: "Dot name cannot exceed 64 characters".to_string(),
        });
    }

    if deploy_request.bytecode.is_empty() {
        return Err(ApiError::BadRequest {
            message: "Bytecode cannot be empty".to_string(),
        });
    }

    // Deploy the dot
    let response = vm_client.deploy_dot(deploy_request).await?;

    info!("Deployed dot successfully: {}", response.dot_id);

    let response_json = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Get dot state
/// GET /api/v1/vm/dots/{id}/state
#[utoipa::path(
    get,
    path = "/api/v1/vm/dots/{id}/state",
    params(
        ("id" = String, Path, description = "Dot ID")
    ),
    responses(
        (status = 200, description = "Dot state", body = DotState),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Dot not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn get_dot_state(req: Request<hyper::body::Incoming>, dot_id: String, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get dot state request: {}", dot_id);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["execute:dots"])?;

    // Decode dot ID
    let dot_id = percent_decode_str(&dot_id)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid dot ID encoding".to_string(),
        })?
        .to_string();

    // Get dot state
    let dot_state = vm_client.get_dot_state(&dot_id).await?;

    info!("Retrieved dot state: {}", dot_id);

    let response_json = serde_json::to_string(&dot_state)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Execute a dot function
/// POST /api/v1/vm/dots/{id}/execute
#[utoipa::path(
    post,
    path = "/api/v1/vm/dots/{id}/execute",
    params(
        ("id" = String, Path, description = "Dot ID")
    ),
    request_body = ExecuteDotRequest,
    responses(
        (status = 200, description = "Execution completed", body = ExecuteDotResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Dot not found"),
        (status = 408, description = "Execution timeout")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn execute_dot(req: Request<hyper::body::Incoming>, dot_id: String, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing execute dot request: {}", dot_id);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["execute:dots"])?;

    // Decode dot ID
    let dot_id = percent_decode_str(&dot_id)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid dot ID encoding".to_string(),
        })?
        .to_string();

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let execute_request: ExecuteDotRequest = serde_json::from_slice(&body)?;

    // Validate request
    if execute_request.function.is_empty() {
        return Err(ApiError::BadRequest {
            message: "Function name cannot be empty".to_string(),
        });
    }

    // Execute the dot function
    let response = vm_client.execute_dot(&dot_id, execute_request).await?;

    info!("Executed dot function successfully: {}", dot_id);

    let response_json = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// List all deployed dots
/// GET /api/v1/vm/dots
#[utoipa::path(
    get,
    path = "/api/v1/vm/dots",
    responses(
        (status = 200, description = "List of deployed dots", body = [DotState]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn list_dots(req: Request<hyper::body::Incoming>, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing list dots request");

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["execute:dots"])?;

    // List dots
    let dots = vm_client.list_dots().await?;

    info!("Retrieved {} deployed dots", dots.len());

    let response_json = serde_json::to_string(&dots)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Delete a deployed dot
/// DELETE /api/v1/vm/dots/{id}
#[utoipa::path(
    delete,
    path = "/api/v1/vm/dots/{id}",
    params(
        ("id" = String, Path, description = "Dot ID")
    ),
    responses(
        (status = 204, description = "Dot deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Dot not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn delete_dot(req: Request<hyper::body::Incoming>, dot_id: String, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing delete dot request: {}", dot_id);

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["deploy:dots"])?; // Assuming deploy permission covers delete

    // Decode dot ID
    let dot_id = percent_decode_str(&dot_id)
        .decode_utf8()
        .map_err(|_| ApiError::BadRequest {
            message: "Invalid dot ID encoding".to_string(),
        })?
        .to_string();

    // Delete the dot
    vm_client.delete_dot(&dot_id).await?;

    info!("Deleted dot: {}", dot_id);

    Ok(Response::builder().status(StatusCode::NO_CONTENT).body(Full::new(Bytes::new()))?)
}

/// Get VM status
/// GET /api/v1/vm/status
#[utoipa::path(
    get,
    path = "/api/v1/vm/status",
    responses(
        (status = 200, description = "VM status information"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn get_vm_status(req: Request<hyper::body::Incoming>, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get VM status request");

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["execute:dots"])?;

    // Get VM status
    let status = vm_client.get_vm_status().await?;

    info!("Retrieved VM status");

    let response_json = serde_json::to_string(&status)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Get supported architectures
/// GET /api/v1/vm/architectures
#[utoipa::path(
    get,
    path = "/api/v1/vm/architectures",
    responses(
        (status = 200, description = "List of supported architectures", body = [String]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Virtual Machine"
)]
pub async fn get_architectures(req: Request<hyper::body::Incoming>, vm_client: VmClient) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get architectures request");

    // Check authentication and permissions
    let claims = extract_claims(&req)?;
    check_permissions(claims, &["execute:dots"])?;

    // Get supported architectures
    let architectures = vm_client.get_architectures().await?;

    info!("Retrieved {} supported architectures", architectures.len());

    let response_json = serde_json::to_string(&architectures)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

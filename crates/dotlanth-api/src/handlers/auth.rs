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

//! Authentication handlers

use crate::auth::AuthService;
use crate::error::{ApiError, ApiResult};
use crate::middleware::extract_claims;
use crate::models::{LoginRequest, TokenResponse, UserProfile};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Bytes};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Login handler
/// POST /api/v1/auth/login
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 400, description = "Bad request")
    ),
    tag = "Authentication"
)]
pub async fn login(req: Request<hyper::body::Incoming>, auth_service: Arc<Mutex<AuthService>>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing login request");

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let login_request: LoginRequest = serde_json::from_slice(&body)?;

    // Authenticate user
    let mut auth_service = auth_service.lock().await;
    let token_response = auth_service.login(login_request).await?;

    info!("User authenticated successfully");

    let response_json = serde_json::to_string(&token_response)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Get user profile handler
/// GET /api/v1/auth/profile
#[utoipa::path(
    get,
    path = "/api/v1/auth/profile",
    responses(
        (status = 200, description = "User profile", body = UserProfile),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Authentication"
)]
pub async fn get_profile(req: Request<hyper::body::Incoming>, auth_service: Arc<Mutex<AuthService>>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get profile request");

    // Extract user claims from authentication middleware
    let claims = extract_claims(&req)?;

    // Get user profile
    let auth_service = auth_service.lock().await;
    let user_profile = auth_service.get_user_profile(&claims.sub)?;

    info!("Retrieved user profile for user: {}", claims.sub);

    let response_json = serde_json::to_string(&user_profile)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

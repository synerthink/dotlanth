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

use crate::auth::{AuthService, extract_token_from_header};
use crate::error::{ApiError, ApiResult};
use crate::middleware::JwtClaimsExt;
use crate::models::{LoginRequest, RegisterRequest, TokenPair, UserProfile};
use http_body_util::{BodyExt, Full};
use hyper::{HeaderMap, Request, Response, StatusCode, body::Bytes};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

/// Refresh token request
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Logout request
#[derive(Debug, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

/// Register handler
/// POST /api/v1/auth/register
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = UserProfile),
        (status = 409, description = "User already exists"),
        (status = 400, description = "Bad request")
    ),
    tag = "Authentication"
)]
pub async fn register(req: Request<hyper::body::Incoming>, auth_service: Arc<Mutex<AuthService>>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing user registration request");

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let register_request: RegisterRequest = serde_json::from_slice(&body)?;

    // Register user
    let mut auth_service = auth_service.lock().await;
    let user_profile = auth_service.register(register_request).await?;

    info!("User registered successfully: {}", user_profile.username);

    let response_json = serde_json::to_string(&user_profile)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Login handler
/// POST /api/v1/auth/login
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenPair),
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
    let token_pair = auth_service.login(login_request).await?;

    info!("User authenticated successfully");

    let response_json = serde_json::to_string(&token_pair)?;

    // Set secure cookies for web clients
    let mut response = Response::builder().status(StatusCode::OK).header("content-type", "application/json");

    // Add secure HTTP-only cookie for refresh token
    let cookie_value = format!(
        "refresh_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        token_pair.refresh_token,
        30 * 24 * 60 * 60 // 30 days
    );
    response = response.header("set-cookie", cookie_value);

    Ok(response.body(Full::new(Bytes::from(response_json)))?)
}

/// Refresh token handler
/// POST /api/v1/auth/refresh
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = TokenPair),
        (status = 401, description = "Invalid refresh token"),
        (status = 400, description = "Bad request")
    ),
    tag = "Authentication"
)]
pub async fn refresh_token(req: Request<hyper::body::Incoming>, auth_service: Arc<Mutex<AuthService>>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing token refresh request");

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let refresh_request: RefreshTokenRequest = serde_json::from_slice(&body)?;

    // Refresh token
    let auth_service = auth_service.lock().await;
    let token_pair = auth_service.refresh_token(&refresh_request.refresh_token).await?;

    info!("Token refreshed successfully");

    let response_json = serde_json::to_string(&token_pair)?;

    // Set secure cookies for web clients
    let mut response = Response::builder().status(StatusCode::OK).header("content-type", "application/json");

    // Add secure HTTP-only cookie for new refresh token
    let cookie_value = format!(
        "refresh_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        token_pair.refresh_token,
        30 * 24 * 60 * 60 // 30 days
    );
    response = response.header("set-cookie", cookie_value);

    Ok(response.body(Full::new(Bytes::from(response_json)))?)
}

/// Logout handler
/// POST /api/v1/auth/logout
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful"),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Bad request")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Authentication"
)]
pub async fn logout(req: Request<hyper::body::Incoming>, auth_service: Arc<Mutex<AuthService>>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing logout request");

    // Extract access token from Authorization header before consuming the request
    let auth_header_value = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized {
            message: "Missing authorization header".to_string(),
        })?
        .to_string();

    let access_token = extract_token_from_header(&auth_header_value)?;

    // Read request body for refresh token
    let body = req.into_body().collect().await?.to_bytes();
    let logout_request: LogoutRequest = serde_json::from_slice(&body)?;

    // Blacklist access token and revoke refresh token
    let auth_service = auth_service.lock().await;
    auth_service.logout(access_token).await?;

    if let Some(refresh_token) = logout_request.refresh_token {
        // In a full implementation, we would revoke the refresh token here
        info!("Refresh token provided for logout: {}", refresh_token);
    }

    info!("User logged out successfully");

    // Clear refresh token cookie
    let mut response = Response::builder().status(StatusCode::OK).header("content-type", "application/json");

    // Clear the refresh token cookie
    let clear_cookie = "refresh_token=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";
    response = response.header("set-cookie", clear_cookie);

    let response_body = serde_json::json!({
        "message": "Logout successful"
    });

    Ok(response.body(Full::new(Bytes::from(response_body.to_string())))?)
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

    // Extract user claims from JWT middleware
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

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

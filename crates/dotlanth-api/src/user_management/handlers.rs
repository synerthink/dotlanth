//! HTTP handlers for user management

use crate::auth::Claims;
use crate::error::{ApiError, ApiResult};
use crate::middleware::JwtClaimsExt;
use crate::user_management::export::UserDataExportService;
use crate::user_management::manager::UserManager;
use crate::user_management::models::{User, UserDataExportRequest, UserRegistration, UserSearchQuery, UserStatus, UserUpdates};
use crate::user_management::preferences::PreferencesManager;
use crate::user_management::search::UserSearchService;
use http_body_util::{BodyExt, Full};
use hyper::{HeaderMap, Request, Response, StatusCode, body::Bytes};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

/// Create user request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub registration: UserRegistration,
}

/// Update user request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub updates: UserUpdates,
}

/// Assign role request
#[derive(Debug, Serialize, Deserialize)]
pub struct AssignRoleRequest {
    pub role_id: String,
}

/// Suspend user request
#[derive(Debug, Serialize, Deserialize)]
pub struct SuspendUserRequest {
    pub reason: String,
}

/// User management handlers
pub struct UserHandlers {
    user_manager: Arc<UserManager>,
    preferences_manager: Arc<PreferencesManager>,
    search_service: Arc<UserSearchService>,
    export_service: Arc<UserDataExportService>,
}

impl UserHandlers {
    /// Create new user handlers
    pub fn new(user_manager: Arc<UserManager>, preferences_manager: Arc<PreferencesManager>, search_service: Arc<UserSearchService>, export_service: Arc<UserDataExportService>) -> Self {
        Self {
            user_manager,
            preferences_manager,
            search_service,
            export_service,
        }
    }
}

/// Create user handler
/// POST /api/v1/users
#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = User),
        (status = 400, description = "Bad request"),
        (status = 409, description = "User already exists"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "User Management"
)]
pub async fn create_user(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing create user request");

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to create users".to_string(),
        });
    }

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let create_request: CreateUserRequest = serde_json::from_slice(&body)?;

    // Create user
    let user = user_handlers.user_manager.create_user(create_request.registration).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to create user: {}", e),
    })?;

    info!("User created successfully: {}", user.id);

    let response_json = serde_json::to_string(&user)?;

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Get user handler
/// GET /api/v1/users/{user_id}
#[utoipa::path(
        get,
        path = "/api/v1/users/{user_id}",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        responses(
            (status = 200, description = "User details", body = User),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn get_user(req: Request<hyper::body::Incoming>, user_id: String, user_handlers: Arc<UserHandlers>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing get user request for user: {}", user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    // Users can view their own profile, admins can view any profile
    if claims.sub != user_id && !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to view this user".to_string(),
        });
    }

    // Get user
    let user = user_handlers
        .user_manager
        .get_user(&user_id)
        .await
        .map_err(|e| ApiError::InternalServerError {
            message: format!("Failed to get user: {}", e),
        })?
        .ok_or_else(|| ApiError::NotFound {
            message: "User not found".to_string(),
        })?;

    let response_json = serde_json::to_string(&user)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Update user handler
/// PUT /api/v1/users/{user_id}
#[utoipa::path(
        put,
        path = "/api/v1/users/{user_id}",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        request_body = UpdateUserRequest,
        responses(
            (status = 200, description = "User updated successfully", body = User),
            (status = 400, description = "Bad request"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn update_user(req: Request<hyper::body::Incoming>, user_id: String, user_handlers: Arc<UserHandlers>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing update user request for user: {}", user_id);

    // Extract claims before consuming request
    let claims = req
        .jwt_claims()
        .ok_or_else(|| ApiError::Unauthorized {
            message: "Missing JWT claims".to_string(),
        })?
        .clone();

    // Read request body first
    let body = req.into_body().collect().await?.to_bytes();

    // Users can update their own profile, admins can update any profile
    if claims.sub != user_id && !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to update this user".to_string(),
        });
    }
    let update_request: UpdateUserRequest = serde_json::from_slice(&body)?;

    // Non-admin users cannot change status
    if claims.sub == user_id && !claims.has_permission("admin:users") {
        if update_request.updates.status.is_some() {
            return Err(ApiError::Forbidden {
                message: "Users cannot change their own status".to_string(),
            });
        }
    }

    // Update user
    let user = user_handlers.user_manager.update_user(&user_id, update_request.updates).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to update user: {}", e),
    })?;

    info!("User updated successfully: {}", user_id);

    let response_json = serde_json::to_string(&user)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Delete user handler
/// DELETE /api/v1/users/{user_id}
#[utoipa::path(
        delete,
        path = "/api/v1/users/{user_id}",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        responses(
            (status = 204, description = "User deleted successfully"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn delete_user(req: Request<hyper::body::Incoming>, user_id: String, user_handlers: Arc<UserHandlers>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing delete user request for user: {}", user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to delete users".to_string(),
        });
    }

    // Delete user
    user_handlers.user_manager.delete_user(&user_id).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to delete user: {}", e),
    })?;

    info!("User deleted successfully: {}", user_id);

    Ok(Response::builder().status(StatusCode::NO_CONTENT).body(Full::new(Bytes::new()))?)
}

/// Search users handler
/// GET /api/v1/users/search
#[utoipa::path(
        get,
        path = "/api/v1/users/search",
        params(
            ("q" = Option<String>, Query, description = "Search query"),
            ("status" = Option<String>, Query, description = "Filter by status"),
            ("roles" = Option<String>, Query, description = "Filter by roles (comma-separated)"),
            ("page" = Option<u32>, Query, description = "Page number"),
            ("page_size" = Option<u32>, Query, description = "Page size"),
            ("sort_by" = Option<String>, Query, description = "Sort field"),
            ("sort_direction" = Option<String>, Query, description = "Sort direction")
        ),
        responses(
            (status = 200, description = "Search results", body = UserSearchResults),
            (status = 400, description = "Bad request"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn search_users(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing search users request");

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") && !claims.has_permission("read:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to search users".to_string(),
        });
    }

    // Parse query parameters
    let uri = req.uri();
    let query_params = uri.query().unwrap_or("");
    let search_query = parse_search_query(query_params)?;

    // Search users
    let results = user_handlers.search_service.search(&search_query).await.map_err(|e| ApiError::BadRequest {
        message: format!("Search failed: {}", e),
    })?;

    let response_json = serde_json::to_string(&results)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Assign role handler
/// POST /api/v1/users/{user_id}/roles
#[utoipa::path(
        post,
        path = "/api/v1/users/{user_id}/roles",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        request_body = AssignRoleRequest,
        responses(
            (status = 200, description = "Role assigned successfully"),
            (status = 400, description = "Bad request"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn assign_role(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>, user_id: String) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing assign role request for user: {}", user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to assign roles".to_string(),
        });
    }

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let assign_request: AssignRoleRequest = serde_json::from_slice(&body)?;

    // Assign role
    user_handlers.user_manager.assign_role(&user_id, &assign_request.role_id).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to assign role: {}", e),
    })?;

    info!("Role assigned successfully: {} to user: {}", assign_request.role_id, user_id);

    let response = serde_json::json!({
            "message": "Role assigned successfully"
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response.to_string())))?)
}

/// Revoke role handler
/// DELETE /api/v1/users/{user_id}/roles/{role_id}
#[utoipa::path(
        delete,
        path = "/api/v1/users/{user_id}/roles/{role_id}",
        params(
            ("user_id" = String, Path, description = "User ID"),
            ("role_id" = String, Path, description = "Role ID")
        ),
        responses(
            (status = 200, description = "Role revoked successfully"),
            (status = 400, description = "Bad request"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn revoke_role(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>, user_id: String, role_id: String) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing revoke role request: {} from user: {}", role_id, user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to revoke roles".to_string(),
        });
    }

    // Revoke role
    user_handlers.user_manager.revoke_role(&user_id, &role_id).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to revoke role: {}", e),
    })?;

    info!("Role revoked successfully: {} from user: {}", role_id, user_id);

    let response = serde_json::json!({
            "message": "Role revoked successfully"
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response.to_string())))?)
}

/// Suspend user handler
/// POST /api/v1/users/{user_id}/suspend
#[utoipa::path(
        post,
        path = "/api/v1/users/{user_id}/suspend",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        request_body = SuspendUserRequest,
        responses(
            (status = 200, description = "User suspended successfully"),
            (status = 400, description = "Bad request"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn suspend_user(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>, user_id: String) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing suspend user request for user: {}", user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to suspend users".to_string(),
        });
    }

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let suspend_request: SuspendUserRequest = serde_json::from_slice(&body)?;

    // Suspend user
    user_handlers.user_manager.suspend_user(&user_id, &suspend_request.reason).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to suspend user: {}", e),
    })?;

    info!("User suspended successfully: {}", user_id);

    let response = serde_json::json!({
            "message": "User suspended successfully"
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response.to_string())))?)
}

/// Reactivate user handler
/// POST /api/v1/users/{user_id}/reactivate
#[utoipa::path(
        post,
        path = "/api/v1/users/{user_id}/reactivate",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        responses(
            (status = 200, description = "User reactivated successfully"),
            (status = 400, description = "Bad request"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn reactivate_user(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>, user_id: String) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing reactivate user request for user: {}", user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    if !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to reactivate users".to_string(),
        });
    }

    // Reactivate user
    user_handlers.user_manager.reactivate_user(&user_id).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to reactivate user: {}", e),
    })?;

    info!("User reactivated successfully: {}", user_id);

    let response = serde_json::json!({
            "message": "User reactivated successfully"
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response.to_string())))?)
}

/// Request data export handler
/// POST /api/v1/users/{user_id}/export
#[utoipa::path(
        post,
        path = "/api/v1/users/{user_id}/export",
        params(
            ("user_id" = String, Path, description = "User ID")
        ),
        request_body = UserDataExportRequest,
        responses(
            (status = 202, description = "Export request accepted", body = UserDataExport),
            (status = 400, description = "Bad request"),
            (status = 404, description = "User not found"),
            (status = 403, description = "Forbidden")
        ),
        security(
            ("bearer_auth" = [])
        ),
        tag = "User Management"
    )]
pub async fn request_data_export(req: Request<hyper::body::Incoming>, user_handlers: Arc<UserHandlers>, user_id: String) -> Result<Response<Full<Bytes>>, ApiError> {
    info!("Processing data export request for user: {}", user_id);

    // Check permissions
    let claims = req.jwt_claims().ok_or_else(|| ApiError::Unauthorized {
        message: "Missing JWT claims".to_string(),
    })?;

    // Users can export their own data, admins can export any user's data
    if claims.sub != user_id && !claims.has_permission("admin:users") {
        return Err(ApiError::Forbidden {
            message: "Insufficient permissions to export this user's data".to_string(),
        });
    }

    // Read request body
    let body = req.into_body().collect().await?.to_bytes();
    let export_request: UserDataExportRequest = serde_json::from_slice(&body)?;

    // Request export
    let export = user_handlers.export_service.request_export(&user_id, export_request).await.map_err(|e| ApiError::BadRequest {
        message: format!("Failed to request export: {}", e),
    })?;

    info!("Data export requested successfully for user: {}", user_id);

    let response_json = serde_json::to_string(&export)?;

    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_json)))?)
}

/// Parse search query parameters
fn parse_search_query(query_string: &str) -> Result<UserSearchQuery, ApiError> {
    let mut query = UserSearchQuery {
        query: None,
        status: None,
        roles: None,
        created_after: None,
        created_before: None,
        last_login_after: None,
        sort_by: None,
        sort_direction: None,
        page: None,
        page_size: None,
    };

    for param in query_string.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            let decoded_value = urlencoding::decode(value).map_err(|_| ApiError::BadRequest {
                message: "Invalid URL encoding".to_string(),
            })?;

            match key {
                "q" => query.query = Some(decoded_value.to_string()),
                "status" => {
                    query.status = Some(match decoded_value.as_ref() {
                        "active" => UserStatus::Active,
                        "suspended" => UserStatus::Suspended,
                        "pending_verification" => UserStatus::PendingVerification,
                        "deleted" => UserStatus::Deleted,
                        _ => {
                            return Err(ApiError::BadRequest {
                                message: "Invalid status value".to_string(),
                            });
                        }
                    });
                }
                "roles" => {
                    query.roles = Some(decoded_value.split(',').map(|s| s.to_string()).collect());
                }
                "page" => {
                    query.page = Some(decoded_value.parse().map_err(|_| ApiError::BadRequest {
                        message: "Invalid page number".to_string(),
                    })?);
                }
                "page_size" => {
                    query.page_size = Some(decoded_value.parse().map_err(|_| ApiError::BadRequest {
                        message: "Invalid page size".to_string(),
                    })?);
                }
                "sort_by" => query.sort_by = Some(decoded_value.to_string()),
                "sort_direction" => query.sort_direction = Some(decoded_value.to_string()),
                _ => {} // Ignore unknown parameters
            }
        }
    }

    Ok(query)
}

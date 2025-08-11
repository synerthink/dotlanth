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

//! HTTP routing for the REST API

use crate::auth::{AuthService, extract_token_from_header};
use crate::db::DatabaseClient;
use crate::error::{ApiError, ApiResult};
use crate::handlers::{auth, db, health, vm};
use crate::vm::VmClient;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode, body::Bytes};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

/// HTTP router for the REST API
pub struct Router {
    pub auth_service: Arc<Mutex<AuthService>>,
    db_client: DatabaseClient,
    vm_client: VmClient,
    openapi_spec: String,
}

impl Router {
    /// Create a new router
    pub fn new(auth_service: Arc<Mutex<AuthService>>, db_client: DatabaseClient, vm_client: VmClient) -> ApiResult<Self> {
        // Generate OpenAPI specification
        let openapi_spec = generate_openapi_spec();

        Ok(Self {
            auth_service,
            db_client,
            vm_client,
            openapi_spec,
        })
    }

    /// Route a request to the appropriate handler
    pub async fn route(&self, mut req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        info!("Routing request: {} {}", method, path);

        // Public paths that don't require authentication
        let public_paths = ["/api/v1/health", "/api/v1/version", "/api/v1/auth/login", "/docs", "/docs/", "/api-docs", "/openapi.json"];

        // Check if authentication is required
        let requires_auth = !public_paths.iter().any(|public_path| path.as_str() == *public_path || path.starts_with(&format!("{}/", public_path)));

        if requires_auth {
            // Extract and validate JWT token
            if let Some(auth_header) = req.headers().get("authorization") {
                match auth_header.to_str() {
                    Ok(auth_str) => {
                        match extract_token_from_header(auth_str) {
                            Ok(token) => {
                                let auth_service = self.auth_service.lock().await;
                                match auth_service.validate_token(token) {
                                    Ok(claims) => {
                                        // Add claims to request extensions
                                        req.extensions_mut().insert(claims);
                                    }
                                    Err(e) => {
                                        error!("Token validation failed: {}", e);
                                        return Err(ApiError::Unauthorized {
                                            message: "Invalid or expired token".to_string(),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid authorization header format: {}", e);
                                return Err(ApiError::Unauthorized {
                                    message: "Invalid authorization header format".to_string(),
                                });
                            }
                        }
                    }
                    Err(_) => {
                        warn!("Authorization header contains invalid UTF-8");
                        return Err(ApiError::Unauthorized {
                            message: "Invalid authorization header encoding".to_string(),
                        });
                    }
                }
            } else {
                warn!("Missing authorization header for protected path: {}", path);
                return Err(ApiError::Unauthorized {
                    message: "No authentication information found".to_string(),
                });
            }
        }

        // Simple path matching
        match (&method, path.as_str()) {
            // Health endpoints
            (&Method::GET, "/api/v1/health") => health::health_check(req, self.db_client.clone(), self.vm_client.clone()).await,
            (&Method::GET, "/api/v1/version") => health::version_info(req).await,

            // Auth endpoints
            (&Method::POST, "/api/v1/auth/login") => auth::login(req, self.auth_service.clone()).await,
            (&Method::GET, "/api/v1/auth/profile") => auth::get_profile(req, self.auth_service.clone()).await,

            // Collections
            (&Method::GET, "/api/v1/collections") => db::list_collections(req, self.db_client.clone()).await,

            // VM endpoints
            (&Method::POST, "/api/v1/vm/dots/deploy") => vm::deploy_dot(req, self.vm_client.clone()).await,
            (&Method::GET, "/api/v1/vm/dots") => vm::list_dots(req, self.vm_client.clone()).await,
            (&Method::GET, "/api/v1/vm/status") => vm::get_vm_status(req, self.vm_client.clone()).await,
            (&Method::GET, "/api/v1/vm/architectures") => vm::get_architectures(req, self.vm_client.clone()).await,

            // Documentation
            (&Method::GET, "/docs") | (&Method::GET, "/docs/") => self.serve_docs().await,
            (&Method::GET, "/openapi.json") => self.serve_openapi_spec().await,

            // Dynamic routes with path parameters
            _ => self.handle_dynamic_routes(req).await,
        }
    }

    /// Handle dynamic routes with path parameters
    async fn handle_dynamic_routes(&self, req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();
        let query = req.uri().query().unwrap_or("").to_string();
        let path_segments: Vec<&str> = path.split('/').collect();

        match (&method, path_segments.as_slice()) {
            // Collections with dynamic collection name
            (&Method::POST, ["", "api", "v1", "collections", collection]) => db::create_collection(req, collection.to_string(), self.db_client.clone()).await,
            (&Method::DELETE, ["", "api", "v1", "collections", collection]) => db::delete_collection(req, collection.to_string(), self.db_client.clone()).await,

            // Documents
            (&Method::GET, ["", "api", "v1", "collections", collection, "documents"]) => {
                let query_params = parse_query_params(&query);
                db::get_documents(req, collection.to_string(), query_params, self.db_client.clone()).await
            }
            (&Method::POST, ["", "api", "v1", "collections", collection, "documents"]) => db::create_document(req, collection.to_string(), self.db_client.clone()).await,

            // Individual documents
            (&Method::GET, ["", "api", "v1", "collections", collection, "documents", id]) => db::get_document(req, collection.to_string(), id.to_string(), self.db_client.clone()).await,
            (&Method::PUT, ["", "api", "v1", "collections", collection, "documents", id]) => db::update_document(req, collection.to_string(), id.to_string(), self.db_client.clone()).await,
            (&Method::DELETE, ["", "api", "v1", "collections", collection, "documents", id]) => db::delete_document(req, collection.to_string(), id.to_string(), self.db_client.clone()).await,

            // Search
            (&Method::GET, ["", "api", "v1", "collections", collection, "search"]) => {
                let query_params = parse_query_params(&query);
                db::search_documents(req, collection.to_string(), query_params, self.db_client.clone()).await
            }

            // VM dots
            (&Method::GET, ["", "api", "v1", "vm", "dots", id, "state"]) => vm::get_dot_state(req, id.to_string(), self.vm_client.clone()).await,
            (&Method::POST, ["", "api", "v1", "vm", "dots", id, "execute"]) => vm::execute_dot(req, id.to_string(), self.vm_client.clone()).await,
            (&Method::DELETE, ["", "api", "v1", "vm", "dots", id]) => vm::delete_dot(req, id.to_string(), self.vm_client.clone()).await,

            _ => {
                warn!("Route not found: {} {}", method, path);
                Err(ApiError::NotFound {
                    message: format!("Route not found: {} {}", method, path),
                })
            }
        }
    }

    /// Serve OpenAPI documentation
    async fn serve_docs(&self) -> Result<Response<Full<Bytes>>, ApiError> {
        let swagger_ui_html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Dotlanth API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui.css" />
    <style>
        html { box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin:0; background: #fafafa; }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            const ui = SwaggerUIBundle({
                url: '/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout"
            });
        };
    </script>
</body>
</html>
        "#;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html")
            .body(Full::new(Bytes::from(swagger_ui_html)))?)
    }

    /// Serve OpenAPI specification
    async fn serve_openapi_spec(&self) -> Result<Response<Full<Bytes>>, ApiError> {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(self.openapi_spec.clone())))?)
    }

    /// Get the OpenAPI specification
    pub fn openapi_spec(&self) -> &str {
        &self.openapi_spec
    }
}

/// Parse query parameters from a query string
fn parse_query_params(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = percent_encoding::percent_decode_str(key).decode_utf8().unwrap_or_default().to_string();
            let value = percent_encoding::percent_decode_str(value).decode_utf8().unwrap_or_default().to_string();
            params.insert(key, value);
        }
    }

    params
}

/// Generate OpenAPI specification
fn generate_openapi_spec() -> String {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            // Health endpoints
            health::health_check,
            health::version_info,

            // Auth endpoints
            auth::login,
            auth::get_profile,

            // Database endpoints
            db::list_collections,
            db::create_collection,
            db::delete_collection,
            db::get_documents,
            db::create_document,
            db::get_document,
            db::update_document,
            db::delete_document,
            db::search_documents,

            // VM endpoints
            vm::deploy_dot,
            vm::get_dot_state,
            vm::execute_dot,
            vm::list_dots,
            vm::delete_dot,
            vm::get_vm_status,
            vm::get_architectures,
        ),
        components(
            schemas(
                crate::models::TokenResponse,
                crate::models::LoginRequest,
                crate::models::UserProfile,
                crate::models::Document,
                crate::models::CreateDocumentRequest,
                crate::models::UpdateDocumentRequest,
                crate::models::CreateDocumentResponse,
                crate::models::Collection,
                crate::models::DocumentList,
                crate::models::PaginationInfo,
                crate::models::SearchResults,
                crate::models::DeployDotRequest,
                crate::models::DeployDotResponse,
                crate::models::DotState,
                crate::models::ExecuteDotRequest,
                crate::models::ExecuteDotResponse,
                crate::models::DotStatus,
                crate::models::ExecutionStatus,
                crate::models::ValidationResult,
                crate::models::HealthResponse,
                crate::models::ServiceStatus,
                crate::models::ApiVersion,
                crate::models::DotConfig,
                crate::models::ExecutionContext,
            )
        ),
        tags(
            (name = "Health", description = "Health check and version endpoints"),
            (name = "Authentication", description = "Authentication and authorization endpoints"),
            (name = "Database", description = "Database collection and document management"),
            (name = "Virtual Machine", description = "VM dot deployment and execution")
        ),
        modifiers(&SecurityAddon)
    )]
    struct ApiDoc;

    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme("bearer_auth", SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).bearer_format("JWT").build()))
            }
        }
    }

    ApiDoc::openapi().to_pretty_json().unwrap_or_else(|_| "{}".to_string())
}

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

use crate::auth::{AuthService, Claims, extract_token_from_header};
use crate::db::DatabaseClient;
use crate::error::{ApiError, ApiResult};
use crate::gateway::{GatewayBridge, GatewayConfig};
use crate::graphql::{AppSchema, build_schema};
use crate::handlers::{auth, db, health, vm};
use crate::vm::VmClient;
use crate::websocket::WebSocketManager;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Method, Request, Response, StatusCode};
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
    websocket_manager: Arc<WebSocketManager>,
    graphql_schema: AppSchema,
    openapi_spec: String,
    gateway_bridge: Arc<GatewayBridge>,
}

impl Router {
    /// Create a new router
    pub async fn new(auth_service: Arc<Mutex<AuthService>>, db_client: DatabaseClient, vm_client: VmClient) -> ApiResult<Self> {
        // Generate OpenAPI specification
        let openapi_spec = generate_openapi_spec();

        // Create WebSocket manager
        let websocket_manager = Arc::new(WebSocketManager::new(vm_client.clone(), auth_service.clone()));

        // Build GraphQL schema
        let graphql_schema = build_schema(auth_service.clone(), db_client.clone(), vm_client.clone(), websocket_manager.clone());

        // Create gateway bridge
        let gateway_config = GatewayConfig::default();
        let gateway_bridge = Arc::new(GatewayBridge::new(gateway_config, auth_service.clone()).await?);

        Ok(Self {
            auth_service,
            db_client,
            vm_client,
            websocket_manager,
            graphql_schema,
            openapi_spec,
            gateway_bridge,
        })
    }

    /// Route a request to the appropriate handler
    pub async fn route(&self, mut req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
        let path = req.uri().path().to_string();
        let method = req.method().clone();

        info!("Routing request: {} {}", method, path);

        // Public paths that don't require authentication
        let public_paths = [
            "/api/v1/health",
            "/api/v1/version",
            "/api/v1/auth/login",
            "/docs",
            "/docs/",
            "/api-docs",
            "/openapi.json",
            "/graphql",
            "/playground",
        ];

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

        // Check for WebSocket upgrade request
        if method == Method::GET && path.as_str() == "/api/v1/ws" {
            // Simple check for WebSocket upgrade request
            if req.headers().get("upgrade").and_then(|h| h.to_str().ok()).map(|h| h.to_lowercase() == "websocket").unwrap_or(false) {
                return crate::handlers::websocket::websocket_upgrade(self.websocket_manager.clone(), req).await;
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

            // GraphQL
            (&Method::GET, "/playground") => self.serve_graphiql().await,
            (&Method::POST, "/graphql") => self.handle_graphql(req).await,

            // Documentation
            (&Method::GET, "/docs") | (&Method::GET, "/docs/") => self.serve_docs().await,
            (&Method::GET, "/openapi.json") => self.serve_openapi_spec().await,

            // Gateway bridge endpoints
            (&Method::GET, "/api/v1/gateway/health") => self.gateway_health_check().await,
            (&Method::GET, "/api/v1/gateway/metrics") => self.gateway_metrics().await,

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

    /*async fn handle_graphql_ws(&self, mut req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
        // Verify WebSocket upgrade headers
        let key = match req.headers().get("sec-websocket-key").and_then(|v| v.to_str().ok()) {
            Some(k) => k.to_string(),
            None => {
                return Err(ApiError::BadRequest {
                    message: "Missing Sec-WebSocket-Key".to_string(),
                });
            }
        };
        let upgrade = req.headers().get("upgrade").and_then(|v| v.to_str().ok()).map(|v| v.eq_ignore_ascii_case("websocket")).unwrap_or(false);
        let connection_upgrade = req
            .headers()
            .get("connection")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_ascii_lowercase().contains("upgrade"))
            .unwrap_or(false);
        if !upgrade || !connection_upgrade {
            return Err(ApiError::BadRequest {
                message: "Expected WebSocket upgrade".to_string(),
            });
        }

        // Compute Sec-WebSocket-Accept
        use base64::{Engine as _, engine::general_purpose};
        use sha1::{Digest, Sha1};
        const WS_MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
        let mut hasher = Sha1::new();
        hasher.update(format!("{}{}", key, WS_MAGIC).as_bytes());
        let accept = general_purpose::STANDARD.encode(hasher.finalize());

        // Build 101 Switching Protocols response
        let response = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Accept", accept)
            .body(Full::new(Bytes::new()))?;

        // Spawn task to handle upgraded connection
        let schema = self.graphql_schema.clone();
        tokio::spawn(async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(upgraded) => {
                    use futures::{SinkExt, StreamExt};
                    use tokio_tungstenite::WebSocketStream;
                    use tokio_tungstenite::tungstenite::protocol::Role;

                    let io = hyper_util::rt::TokioIo::new(upgraded);
                    let mut ws = WebSocketStream::from_raw_socket(io, Role::Server, None).await;

                    // Reader adapter: map Text/Binary messages to Vec<u8>
                    let (mut sink, mut stream) = ws.split();
                    let mapped = stream.filter_map(|msg| async move {
                        match msg {
                            Ok(m) if m.is_text() => Some(Ok(m.into_data())),
                            Ok(m) if m.is_binary() => Some(Ok(m.into_data())),
                            Ok(m) if m.is_close() => None,
                            Ok(_) => None, // ignore ping/pong
                            Err(e) => {
                                tracing::error!("WS read error: {}", e);
                                None
                            }
                        }
                    });

                    // Build GraphQL WS server
                    use async_graphql::http::WebSocketProtocols;
                    let server = async_graphql::http::WebSocket::new(schema, mapped, WebSocketProtocols::GraphQLWS)
                        .on_connection_init(|_payload| async move { Ok(async_graphql::Data::default()) })
                        .on_send(|msg| async move {
                            // Send as Text (GraphQL protocol frames are JSON)
                            if let Err(e) = sink.send(tokio_tungstenite::tungstenite::Message::Text(String::from_utf8_lossy(&msg).to_string())).await {
                                tracing::error!("WS write error: {}", e);
                            }
                        });

                    if let Err(e) = server.serve().await {
                        tracing::error!("GraphQL WS serve error: {}", e);
                    }
                }
                Err(e) => tracing::error!("Upgrade error: {}", e),
            }
        });

        Ok(response)
    }*/

    /// Serve GraphiQL
    async fn serve_graphiql(&self) -> Result<Response<Full<Bytes>>, ApiError> {
        let html = async_graphql::http::GraphiQLSource::build().endpoint("/graphql").subscription_endpoint("/graphql").finish();
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Full::new(Bytes::from(html)))?)
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

    async fn handle_graphql(&self, req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
        use async_graphql::{
            Request as GqlRequest,
            http::{MultipartOptions, receive_body},
        };
        use http_body_util::BodyExt;
        let claims_opt = req.extensions().get::<Claims>().cloned();
        let body = req.into_body().collect().await?.to_bytes();
        let content_type: Option<&str> = None;
        let gql_req: GqlRequest = receive_body(content_type, body.as_ref(), MultipartOptions::default()).await.map_err(|e| ApiError::BadRequest {
            message: format!("Invalid GraphQL request: {}", e),
        })?;
        // Inject claims into GraphQL Data if present
        let mut gql_req = gql_req;
        if let Some(claims) = claims_opt {
            gql_req = gql_req.data(claims);
        }
        let resp = self.graphql_schema.execute(gql_req).await;
        let text = serde_json::to_string(&resp)?;
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(text)))?)
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

    /// Gateway health check endpoint
    async fn gateway_health_check(&self) -> Result<Response<Full<Bytes>>, ApiError> {
        match self.gateway_bridge.health_check().await {
            Ok(_) => {
                let response = serde_json::json!({
                    "status": "healthy",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "service": "gateway_bridge"
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(serde_json::to_string(&response)?)))?)
            }
            Err(e) => {
                let response = serde_json::json!({
                    "status": "unhealthy",
                    "error": e.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "service": "gateway_bridge"
                });

                Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .header("content-type", "application/json")
                    .body(Full::new(Bytes::from(serde_json::to_string(&response)?)))?)
            }
        }
    }

    /// Gateway metrics endpoint
    async fn gateway_metrics(&self) -> Result<Response<Full<Bytes>>, ApiError> {
        let metrics = self.gateway_bridge.get_metrics().await;

        let response = serde_json::json!({
            "metrics": {
                "total_requests": metrics.total_requests,
                "successful_requests": metrics.successful_requests,
                "failed_requests": metrics.failed_requests,
                "avg_latency_ms": metrics.avg_latency_ms,
                "active_streaming_connections": metrics.active_streaming_connections,
                "protocol_conversions": metrics.protocol_conversions,
                "error_rate": if metrics.total_requests > 0 {
                    metrics.failed_requests as f64 / metrics.total_requests as f64
                } else {
                    0.0
                },
                "success_rate": if metrics.total_requests > 0 {
                    metrics.successful_requests as f64 / metrics.total_requests as f64
                } else {
                    0.0
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "service": "gateway_bridge"
        });

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(serde_json::to_string(&response)?)))?)
    }

    /// Get the gateway bridge instance
    pub fn gateway_bridge(&self) -> Arc<GatewayBridge> {
        self.gateway_bridge.clone()
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
                crate::models::DotConfig,
                crate::models::ExecuteDotRequest,
                crate::models::ExecuteDotResponse,
                crate::models::DotState,
                crate::models::ExecutionContext,
                crate::models::DotStatus,
                crate::models::ExecutionStatus,
                crate::models::ValidationResult,
                crate::models::HealthResponse,
                crate::models::ServiceStatus,
                crate::models::ApiVersion,
                crate::models::WebSocketMessage,
                crate::models::DotEvent,
            )
        ),
        tags(
            (name = "Health", description = "Health check and version endpoints"),
            (name = "Authentication", description = "Authentication and authorization endpoints"),
            (name = "Database", description = "Database collection and document management"),
            (name = "Virtual Machine", description = "VM dot deployment and execution"),
            (name = "WebSocket", description = "WebSocket streaming for real-time events")
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

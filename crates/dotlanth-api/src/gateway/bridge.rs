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

//! Main bridge implementation that coordinates all gateway components

use super::error_mapping::ErrorMapper;
use super::protocol_negotiation::ProtocolNegotiator;
use super::streaming_bridge::StreamingBridge;
use super::transcoder::GrpcHttpTranscoder;
use super::websocket_grpc_bridge::WebSocketGrpcBridge;
use super::{GatewayConfig, GatewayMetrics};
use crate::auth::AuthService;
use crate::error::{ApiError, ApiResult};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Method, Request, Response, StatusCode, Uri};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, warn};

/// Authentication context for bridge operations
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub is_authenticated: bool,
}

impl Default for AuthContext {
    fn default() -> Self {
        Self {
            user_id: None,
            roles: vec![],
            permissions: vec![],
            is_authenticated: false,
        }
    }
}

/// Request context for bridge operations
#[derive(Debug)]
pub struct BridgeContext {
    pub request_id: String,
    pub client_ip: String,
    pub user_agent: String,
    pub auth: AuthContext,
    pub started_at: Instant,
}

/// Main bridge implementation
pub struct Bridge {
    config: GatewayConfig,
    metrics: Arc<RwLock<GatewayMetrics>>,
    transcoder: Arc<GrpcHttpTranscoder>,
    streaming_bridge: Arc<StreamingBridge>,
    websocket_bridge: Arc<WebSocketGrpcBridge>,
    error_mapper: ErrorMapper,
    protocol_negotiator: ProtocolNegotiator,
    grpc_channels: Arc<RwLock<std::collections::HashMap<String, Channel>>>,
    auth_service: Arc<Mutex<AuthService>>,
}

impl Bridge {
    /// Create a new bridge instance
    pub async fn new(config: GatewayConfig, auth_service: Arc<Mutex<AuthService>>) -> ApiResult<Self> {
        info!("Initializing gRPC-HTTP Bridge with config: {:?}", config);

        let metrics = Arc::new(RwLock::new(GatewayMetrics::default()));
        let transcoder = Arc::new(GrpcHttpTranscoder::new(config.clone())?);
        let streaming_bridge = Arc::new(StreamingBridge::new(config.clone())?);
        let websocket_bridge = Arc::new(WebSocketGrpcBridge::new(config.clone())?);
        let error_mapper = ErrorMapper::new();
        let protocol_negotiator = ProtocolNegotiator::new();
        let grpc_channels = Arc::new(RwLock::new(std::collections::HashMap::new()));

        Ok(Self {
            config,
            metrics,
            transcoder,
            streaming_bridge,
            websocket_bridge,
            error_mapper,
            protocol_negotiator,
            grpc_channels,
            auth_service,
        })
    }

    /// Handle HTTP request and bridge to gRPC
    pub async fn handle_http_request(&self, req: Request<Incoming>, target_service: &str) -> ApiResult<Response<Full<Bytes>>> {
        let start_time = Instant::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        // Create bridge context
        let context = self.create_bridge_context(&req, request_id.clone()).await?;

        debug!("Handling HTTP request {} to service: {}", request_id, target_service);

        // Authenticate the request
        if self.config.enable_validation {
            self.authenticate_request(&req, &context).await?;
        }

        // Get or create gRPC channel
        let grpc_channel = self.get_grpc_channel(target_service).await?;

        // Check if this is a streaming request
        if self.is_streaming_request(&req) {
            return self.handle_streaming_request(req, context, grpc_channel).await;
        }

        // Handle regular HTTP to gRPC transcoding
        let result = self.transcoder.transcode_http_to_grpc(req, grpc_channel).await;

        // Update metrics
        let latency = start_time.elapsed().as_millis() as f64;
        self.update_metrics(result.is_ok(), latency).await;

        match result {
            Ok(response) => {
                info!("Successfully handled HTTP request {} in {:.2}ms", request_id, latency);
                Ok(response)
            }
            Err(e) => {
                error!("Failed to handle HTTP request {}: {}", request_id, e);
                Ok(self.create_error_response(&e))
            }
        }
    }

    /// Handle gRPC request and bridge to HTTP
    pub async fn handle_grpc_request(&self, grpc_request: tonic::Request<serde_json::Value>, target_url: &str) -> ApiResult<tonic::Response<serde_json::Value>> {
        let start_time = Instant::now();
        let request_id = uuid::Uuid::new_v4().to_string();

        debug!("Handling gRPC request {} to URL: {}", request_id, target_url);

        // Create HTTP client
        let http_client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build(hyper_util::client::legacy::connect::HttpConnector::new());

        // Transcode gRPC to HTTP
        let result = self.transcoder.transcode_grpc_to_http(grpc_request, &http_client, target_url).await;

        // Update metrics
        let latency = start_time.elapsed().as_millis() as f64;
        self.update_metrics(result.is_ok(), latency).await;

        match result {
            Ok(response) => {
                info!("Successfully handled gRPC request {} in {:.2}ms", request_id, latency);
                Ok(response)
            }
            Err(e) => {
                error!("Failed to handle gRPC request {}: {}", request_id, e);
                Err(e)
            }
        }
    }

    /// Handle WebSocket connection for gRPC streaming
    pub async fn handle_websocket_connection<S>(&self, websocket: tokio_tungstenite::WebSocketStream<S>, client_address: String, target_service: &str) -> ApiResult<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        info!("Handling WebSocket connection from {} to service: {}", client_address, target_service);

        // Get gRPC channel for the target service
        let grpc_channel = self.get_grpc_channel(target_service).await?;

        // Delegate to WebSocket bridge
        self.websocket_bridge.handle_websocket_connection(websocket, client_address, grpc_channel).await
    }

    /// Create bridge context from HTTP request
    async fn create_bridge_context(&self, req: &Request<Incoming>, request_id: String) -> ApiResult<BridgeContext> {
        let client_ip = req
            .headers()
            .get("x-forwarded-for")
            .or_else(|| req.headers().get("x-real-ip"))
            .and_then(|h| h.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let user_agent = req.headers().get("user-agent").and_then(|h| h.to_str().ok()).unwrap_or("unknown").to_string();

        // Extract authentication context
        let auth = self.extract_auth_context(req).await?;

        Ok(BridgeContext {
            request_id,
            client_ip,
            user_agent,
            auth,
            started_at: Instant::now(),
        })
    }

    /// Extract authentication context from request
    async fn extract_auth_context(&self, req: &Request<Incoming>) -> ApiResult<AuthContext> {
        // Check for Authorization header
        if let Some(auth_header) = req.headers().get("authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];

                    // Validate token with auth service
                    let auth_service = self.auth_service.lock().await;
                    match auth_service.validate_token(token) {
                        Ok(claims) => {
                            return Ok(AuthContext {
                                user_id: Some(claims.sub),
                                roles: claims.roles.clone(),
                                permissions: claims.permissions.clone(), // Use actual permissions
                                is_authenticated: true,
                            });
                        }
                        Err(_) => {
                            return Err(ApiError::Unauthorized { message: "Invalid token".to_string() });
                        }
                    }
                }
            }
        }

        // Check for API key
        if let Some(api_key) = req.headers().get("x-api-key") {
            if let Ok(key_str) = api_key.to_str() {
                // Validate API key (simplified implementation)
                if !key_str.is_empty() {
                    return Ok(AuthContext {
                        user_id: Some(format!("api_key_{}", &key_str[..std::cmp::min(8, key_str.len())])),
                        roles: vec!["api_user".to_string()],
                        permissions: vec!["read".to_string(), "write".to_string()],
                        is_authenticated: true,
                    });
                }
            }
        }

        // No authentication found
        Ok(AuthContext::default())
    }

    /// Authenticate the request
    async fn authenticate_request(&self, req: &Request<Incoming>, context: &BridgeContext) -> ApiResult<()> {
        // Check if authentication is required for this endpoint
        let path = req.uri().path();
        if self.requires_authentication(path) && !context.auth.is_authenticated {
            return Err(ApiError::Unauthorized {
                message: "Authentication required".to_string(),
            });
        }

        // Check permissions
        if context.auth.is_authenticated {
            let required_permission = self.get_required_permission(req.method(), path);
            if let Some(perm) = required_permission {
                if !context.auth.permissions.contains(&perm) {
                    return Err(ApiError::Forbidden {
                        message: format!("Missing required permission: {}", perm),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if endpoint requires authentication
    fn requires_authentication(&self, path: &str) -> bool {
        // Public endpoints that don't require authentication
        matches!(path, "/health" | "/metrics" | "/docs" | "/api/v1/auth/login" | "/api/v1/auth/register")
    }

    /// Get required permission for endpoint
    fn get_required_permission(&self, method: &Method, path: &str) -> Option<String> {
        match method {
            &Method::GET => Some("read".to_string()),
            &Method::POST | &Method::PUT | &Method::PATCH => Some("write".to_string()),
            &Method::DELETE => Some("delete".to_string()),
            _ => None,
        }
    }

    /// Check if request is for streaming
    fn is_streaming_request(&self, req: &Request<Incoming>) -> bool {
        let path = req.uri().path();
        path.contains("/stream") || req.headers().get("accept").map_or(false, |h| h.to_str().map_or(false, |s| s.contains("text/event-stream")))
    }

    /// Handle streaming HTTP request
    async fn handle_streaming_request(&self, req: Request<Incoming>, context: BridgeContext, grpc_channel: Channel) -> ApiResult<Response<Full<Bytes>>> {
        info!("Handling streaming request: {}", context.request_id);

        // This is a simplified implementation
        // In practice, you'd create a proper streaming response
        let response_body = "data: {\"message\": \"Streaming started\"}\n\n";

        let response = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .header("access-control-allow-origin", "*")
            .body(Full::new(Bytes::from(response_body)))
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to create streaming response: {}", e),
            })?;

        Ok(response)
    }

    /// Get or create gRPC channel for service
    async fn get_grpc_channel(&self, service: &str) -> ApiResult<Channel> {
        let channels = self.grpc_channels.read().await;

        if let Some(channel) = channels.get(service) {
            return Ok(channel.clone());
        }

        drop(channels);

        // Create new channel
        let endpoint_url = self.get_service_endpoint(service)?;
        let endpoint = Endpoint::from_shared(endpoint_url)
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Invalid gRPC endpoint: {}", e),
            })?
            .timeout(std::time::Duration::from_millis(self.config.max_timeout_ms));

        let channel = endpoint.connect().await.map_err(|e| ApiError::ServiceUnavailable {
            message: format!("Failed to connect to gRPC service '{}': {}", service, e),
        })?;

        // Cache the channel
        let mut channels = self.grpc_channels.write().await;
        channels.insert(service.to_string(), channel.clone());

        info!("Created new gRPC channel for service: {}", service);
        Ok(channel)
    }

    /// Get service endpoint URL
    fn get_service_endpoint(&self, service: &str) -> ApiResult<String> {
        // In practice, this would use service discovery
        match service {
            "vm" => Ok("http://127.0.0.1:50051".to_string()),
            "db" => Ok("http://127.0.0.1:50052".to_string()),
            "cluster" => Ok("http://127.0.0.1:50053".to_string()),
            _ => Err(ApiError::NotFound {
                message: format!("Service endpoint for: {}", service),
            }),
        }
    }

    /// Create error response
    fn create_error_response(&self, error: &ApiError) -> Response<Full<Bytes>> {
        let status = match error {
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApiError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::GatewayTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error_body = self.error_mapper.create_http_error_body(status, &error.to_string(), None);
        let body_bytes = serde_json::to_vec(&error_body).unwrap_or_default();

        Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(body_bytes)))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(Bytes::from("Internal server error")))
                    .unwrap()
            })
    }

    /// Update bridge metrics
    async fn update_metrics(&self, success: bool, latency_ms: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.protocol_conversions += 1;

        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update rolling average latency
        let total_successful = metrics.successful_requests.max(1);
        metrics.avg_latency_ms = (metrics.avg_latency_ms * (total_successful - 1) as f64 + latency_ms) / total_successful as f64;
    }

    /// Get current bridge metrics
    pub async fn get_metrics(&self) -> GatewayMetrics {
        self.metrics.read().await.clone()
    }

    /// Health check for the bridge
    pub async fn health_check(&self) -> ApiResult<()> {
        // Check individual components
        self.streaming_bridge.health_check().await?;
        self.websocket_bridge.health_check().await?;

        // Check overall metrics
        let metrics = self.get_metrics().await;

        if metrics.total_requests > 100 {
            let error_rate = metrics.failed_requests as f64 / metrics.total_requests as f64;
            if error_rate > 0.1 {
                return Err(ApiError::ServiceUnavailable {
                    message: format!("Bridge error rate too high: {:.2}%", error_rate * 100.0),
                });
            }
        }

        if metrics.avg_latency_ms > self.config.max_timeout_ms as f64 {
            return Err(ApiError::ServiceUnavailable {
                message: format!("Bridge latency too high: {:.2}ms", metrics.avg_latency_ms),
            });
        }

        Ok(())
    }
}

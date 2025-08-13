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

//! WebSocket streaming for real-time events and bidirectional communication

use crate::auth::{AuthService, Claims, extract_token_from_header};
use crate::error::{ApiError, ApiResult};
use crate::models::{DotEvent, WebSocketMessage};
use crate::vm::VmClient;
use base64::{Engine as _, engine::general_purpose};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use metrics::{counter, gauge};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::time::interval;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{accept_async, tungstenite::Message as TungsteniteMessage};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

/// WebSocket connection manager
pub struct WebSocketManager {
    /// Active connections
    connections: Arc<DashMap<String, Arc<WebSocketConnection>>>,

    /// Event broadcasters for different event types
    event_broadcasters: Arc<DashMap<String, broadcast::Sender<WebSocketMessage>>>,

    /// VM client for interacting with the VM service
    vm_client: VmClient,

    /// Authentication service for validating tokens
    auth_service: Arc<Mutex<AuthService>>,

    /// Connection metrics
    metrics: Arc<WebSocketMetrics>,
}

/// WebSocket connection
pub struct WebSocketConnection {
    /// Connection ID
    id: String,

    /// User claims
    claims: Claims,

    /// Subscribed event types
    subscriptions: Arc<RwLock<HashSet<String>>>,

    /// Connection sender for sending messages
    sender: mpsc::UnboundedSender<WebSocketMessage>,

    /// Connection metrics
    metrics: Arc<WebSocketMetrics>,
}

/// WebSocket metrics
#[derive(Default)]
pub struct WebSocketMetrics {
    /// Active connections count
    active_connections: RwLock<u64>,

    /// Total connections
    total_connections: RwLock<u64>,

    /// Total messages sent
    messages_sent: RwLock<u64>,

    /// Total messages received
    messages_received: RwLock<u64>,

    /// Total connection errors
    connection_errors: RwLock<u64>,
}

/// WebSocket subscription request
#[derive(Debug, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    /// Event types to subscribe to
    pub subscribe: Vec<String>,

    /// Event types to unsubscribe from
    pub unsubscribe: Vec<String>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(vm_client: VmClient, auth_service: Arc<Mutex<AuthService>>) -> Self {
        let metrics = Arc::new(WebSocketMetrics::default());

        // Spawn metrics updater task
        let metrics_clone = metrics.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                metrics_clone.update_metrics();
            }
        });

        Self {
            connections: Arc::new(DashMap::new()),
            event_broadcasters: Arc::new(DashMap::new()),
            vm_client,
            auth_service,
            metrics,
        }
    }

    /// Handle WebSocket upgrade request
    pub async fn handle_websocket_upgrade(&self, mut req: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
        debug!("Handling WebSocket upgrade request");

        // Extract and validate JWT token from Authorization header
        let claims = if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                match extract_token_from_header(auth_str) {
                    Ok(token) => {
                        let auth_service = self.auth_service.lock().await;
                        match auth_service.validate_token(token) {
                            Ok(claims) => claims,
                            Err(e) => {
                                warn!("Invalid token during WebSocket handshake: {}", e);
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
            } else {
                warn!("Authorization header contains invalid UTF-8");
                return Err(ApiError::Unauthorized {
                    message: "Invalid authorization header encoding".to_string(),
                });
            }
        } else {
            warn!("Missing authorization header for WebSocket connection");
            return Err(ApiError::Unauthorized {
                message: "No authentication information found".to_string(),
            });
        };

        // Upgrade to WebSocket connection using hyper's upgrade mechanism
        match hyper::upgrade::on(&mut req).await {
            Ok(upgraded) => {
                // Create a new connection
                let connection_id = Uuid::new_v4().to_string();
                let (sender, receiver) = mpsc::unbounded_channel();

                let connection = Arc::new(WebSocketConnection {
                    id: connection_id.clone(),
                    claims,
                    subscriptions: Arc::new(RwLock::new(HashSet::new())),
                    sender,
                    metrics: self.metrics.clone(),
                });

                // Add connection to manager
                self.connections.insert(connection_id.clone(), connection.clone());

                // Increment active connections
                self.metrics.increment_active_connections();
                self.metrics.increment_total_connections();

                info!("New WebSocket connection established: {}", connection_id);

                // Spawn task for handling the connection
                let manager = self.clone();
                tokio::spawn(async move {
                    // Wrap the upgraded connection in TokioIo to make it compatible with tokio-tungstenite
                    let io = TokioIo::new(upgraded);

                    // Convert the upgraded connection to a WebSocket
                    match accept_async(io).await {
                        Ok(ws_stream) => {
                            if let Err(e) = manager.handle_websocket_connection(ws_stream, connection, receiver).await {
                                error!("WebSocket connection error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to accept WebSocket connection: {}", e);
                        }
                    }
                });

                // Return a successful upgrade response with proper WebSocket headers
                let key = if let Some(key) = req.headers().get("sec-websocket-key") {
                    key.to_str().unwrap_or_default()
                } else {
                    ""
                };

                // Calculate the Sec-WebSocket-Accept header value according to RFC 6455
                // This is required for proper WebSocket handshake completion
                let accept_key = if !key.is_empty() {
                    const WEBSOCKET_HANDSHAKE_MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
                    let concat = format!("{}{}", key, WEBSOCKET_HANDSHAKE_MAGIC);
                    let mut hasher = Sha1::new();
                    hasher.update(concat.as_bytes());
                    let result = hasher.finalize();
                    general_purpose::STANDARD.encode(&result)
                } else {
                    String::new()
                };

                Ok(Response::builder()
                    .status(StatusCode::SWITCHING_PROTOCOLS)
                    .header("Upgrade", "websocket")
                    .header("Connection", "Upgrade")
                    .header("Sec-WebSocket-Accept", accept_key)
                    .body(Full::new(Bytes::new()))
                    .unwrap())
            }
            Err(e) => {
                error!("WebSocket upgrade failed: {}", e);
                Err(ApiError::InternalServerError {
                    message: format!("WebSocket upgrade failed: {}", e),
                })
            }
        }
    }

    /// Handle WebSocket connection
    async fn handle_websocket_connection(
        &self,
        ws_stream: tokio_tungstenite::WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>,
        connection: Arc<WebSocketConnection>,
        receiver: mpsc::UnboundedReceiver<WebSocketMessage>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection_id = connection.id.clone();

        let (ws_sink, ws_stream) = ws_stream.split();

        // Handle incoming messages from the WebSocket
        let manager = self.clone();
        let connection_clone = connection.clone();
        let mut ws_stream = ws_stream;
        tokio::spawn(async move {
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(msg) => {
                        if msg.is_text() || msg.is_binary() {
                            manager.metrics.increment_messages_received();

                            // Handle the message
                            if let Err(e) = manager.handle_message(&connection_clone, msg).await {
                                error!("Error handling WebSocket message: {}", e);
                            }
                        } else if msg.is_close() {
                            info!("WebSocket connection closed: {}", connection_clone.id);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("WebSocket error for connection {}: {}", connection_clone.id, e);
                        manager.metrics.increment_connection_errors();
                        break;
                    }
                }
            }

            // Clean up connection
            manager.remove_connection(&connection_clone.id).await;
        });

        // Handle outgoing messages to the WebSocket
        let mut ws_sink = ws_sink;
        let mut receiver_stream = UnboundedReceiverStream::new(receiver);
        while let Some(message) = receiver_stream.next().await {
            let json_msg = serde_json::to_string(&message)?;
            let ws_msg = TungsteniteMessage::Text(json_msg);

            if let Err(e) = ws_sink.send(ws_msg).await {
                error!("Failed to send message to WebSocket connection {}: {}", connection_id, e);
                break;
            }

            self.metrics.increment_messages_sent();
        }

        Ok(())
    }

    /// Handle incoming WebSocket message
    async fn handle_message(&self, connection: &WebSocketConnection, msg: TungsteniteMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        trace!("Handling WebSocket message from connection {}", connection.id);

        // Parse the message as JSON
        let text = msg.to_text()?;

        // Try to parse as subscription request first
        if let Ok(subscription_req) = serde_json::from_str::<SubscriptionRequest>(text) {
            self.handle_subscription_request(connection, subscription_req).await?;
            return Ok(());
        }

        // Try to parse as generic WebSocket message
        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(text) {
            self.handle_websocket_message(connection, ws_message).await?;
            return Ok(());
        }

        // If we can't parse it, send an error response
        let error_msg = WebSocketMessage {
            event_type: "error".to_string(),
            payload: serde_json::json!({
                "message": "Invalid message format",
                "details": "Message must be a valid WebSocketMessage or SubscriptionRequest"
            }),
            timestamp: chrono::Utc::now(),
        };

        connection.sender.send(error_msg)?;

        Ok(())
    }

    /// Handle subscription request
    async fn handle_subscription_request(&self, connection: &WebSocketConnection, req: SubscriptionRequest) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Handling subscription request for connection {}", connection.id);

        // Add new subscriptions
        {
            let mut subscriptions = connection.subscriptions.write();
            for event_type in req.subscribe {
                subscriptions.insert(event_type);
            }

            // Remove subscriptions
            for event_type in req.unsubscribe {
                subscriptions.remove(&event_type);
            }
        }

        // Send confirmation message
        let response = WebSocketMessage {
            event_type: "subscription_update".to_string(),
            payload: serde_json::json!({
                "message": "Subscriptions updated successfully"
            }),
            timestamp: chrono::Utc::now(),
        };

        connection.sender.send(response)?;

        Ok(())
    }

    /// Handle WebSocket message
    async fn handle_websocket_message(&self, connection: &WebSocketConnection, msg: WebSocketMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Handling WebSocket message of type '{}' from connection {}", msg.event_type, connection.id);

        match msg.event_type.as_str() {
            "ping" => {
                // Respond with pong
                let pong_msg = WebSocketMessage {
                    event_type: "pong".to_string(),
                    payload: serde_json::json!({}),
                    timestamp: chrono::Utc::now(),
                };

                connection.sender.send(pong_msg)?;
            }
            "interactive_execution_request" => {
                // Handle interactive execution request
                self.handle_interactive_execution_request(connection, msg).await?;
            }
            "debug_request" => {
                // Handle debug request
                self.handle_debug_request(connection, msg).await?;
            }
            _ => {
                // Unknown message type, send error response
                let error_msg = WebSocketMessage {
                    event_type: "error".to_string(),
                    payload: serde_json::json!({
                        "message": format!("Unknown message type: {}", msg.event_type),
                        "details": "Supported message types: ping, interactive_execution_request, debug_request"
                    }),
                    timestamp: chrono::Utc::now(),
                };

                connection.sender.send(error_msg)?;
            }
        }

        Ok(())
    }

    /// Handle interactive execution request
    async fn handle_interactive_execution_request(&self, connection: &WebSocketConnection, msg: WebSocketMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Handling interactive execution request from connection {}", connection.id);

        // Extract dot_id from payload
        let dot_id = msg.payload.get("dot_id").and_then(|v| v.as_str()).ok_or_else(|| "Missing dot_id in interactive execution request")?;

        // Check if user has permission to execute this dot
        // For now, we'll assume the user has permission if they can connect via WebSocket
        // In a production environment, you'd want more granular permissions

        // Forward the request to the VM service
        // This would involve using the gRPC streaming interfaces defined in the proto
        // For now, we'll send a placeholder response

        let response = WebSocketMessage {
            event_type: "interactive_execution_response".to_string(),
            payload: serde_json::json!({
                "dot_id": dot_id,
                "status": "started",
                "session_id": Uuid::new_v4().to_string(),
                "message": "Interactive execution session started"
            }),
            timestamp: chrono::Utc::now(),
        };

        connection.sender.send(response)?;

        Ok(())
    }

    /// Handle debug request
    async fn handle_debug_request(&self, connection: &WebSocketConnection, msg: WebSocketMessage) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Handling debug request from connection {}", connection.id);

        // Extract dot_id from payload
        let dot_id = msg.payload.get("dot_id").and_then(|v| v.as_str()).ok_or_else(|| "Missing dot_id in debug request")?;

        // Check if user has permission to debug this dot
        // For now, we'll assume the user has permission if they can connect via WebSocket
        // In a production environment, you'd want more granular permissions

        // Forward the request to the VM service
        // This would involve using the gRPC streaming interfaces defined in the proto
        // For now, we'll send a placeholder response

        let response = WebSocketMessage {
            event_type: "debug_response".to_string(),
            payload: serde_json::json!({
                "dot_id": dot_id,
                "status": "debug_session_started",
                "session_id": Uuid::new_v4().to_string(),
                "message": "Debug session started"
            }),
            timestamp: chrono::Utc::now(),
        };

        connection.sender.send(response)?;

        Ok(())
    }

    /// Broadcast an event to all subscribed connections
    pub async fn broadcast_event(&self, event: WebSocketMessage) {
        debug!("Broadcasting event of type: {}", event.event_type);

        // Get or create broadcaster for this event type
        let sender = self
            .event_broadcasters
            .entry(event.event_type.clone())
            .or_insert_with(|| {
                broadcast::channel(100).0 // Buffer size of 100
            })
            .clone();

        // Send the event to all subscribers
        if let Err(e) = sender.send(event) {
            warn!("Failed to broadcast event: {}", e);
        }
    }

    /// Remove a connection
    async fn remove_connection(&self, connection_id: &str) {
        info!("Removing WebSocket connection: {}", connection_id);

        // Remove from connections map
        self.connections.remove(connection_id);

        // Decrement active connections
        self.metrics.decrement_active_connections();
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get metrics
    pub fn get_metrics(&self) -> WebSocketMetricsSnapshot {
        WebSocketMetricsSnapshot {
            active_connections: *self.metrics.active_connections.read(),
            total_connections: *self.metrics.total_connections.read(),
            messages_sent: *self.metrics.messages_sent.read(),
            messages_received: *self.metrics.messages_received.read(),
            connection_errors: *self.metrics.connection_errors.read(),
        }
    }
}

impl WebSocketConnection {
    /// Check if connection is subscribed to a specific event type
    pub fn is_subscribed(&self, event_type: &str) -> bool {
        self.subscriptions.read().contains(event_type)
    }
}

impl WebSocketMetrics {
    /// Increment active connections
    fn increment_active_connections(&self) {
        *self.active_connections.write() += 1;
        counter!("websocket_active_connections", 1);
    }

    /// Decrement active connections
    fn decrement_active_connections(&self) {
        let mut active = self.active_connections.write();
        if *active > 0 {
            *active -= 1;
        }
        gauge!("websocket_active_connections", *active as f64);
    }

    /// Increment total connections
    fn increment_total_connections(&self) {
        *self.total_connections.write() += 1;
        counter!("websocket_total_connections", 1);
    }

    /// Increment messages sent
    fn increment_messages_sent(&self) {
        *self.messages_sent.write() += 1;
        counter!("websocket_messages_sent", 1);
    }

    /// Increment messages received
    fn increment_messages_received(&self) {
        *self.messages_received.write() += 1;
        counter!("websocket_messages_received", 1);
    }

    /// Increment connection errors
    fn increment_connection_errors(&self) {
        *self.connection_errors.write() += 1;
        counter!("websocket_connection_errors", 1);
    }

    /// Update metrics gauges
    fn update_metrics(&self) {
        gauge!("websocket_active_connections", *self.active_connections.read() as f64);
        gauge!("websocket_total_connections", *self.total_connections.read() as f64);
        gauge!("websocket_messages_sent", *self.messages_sent.read() as f64);
        gauge!("websocket_messages_received", *self.messages_received.read() as f64);
        gauge!("websocket_connection_errors", *self.connection_errors.read() as f64);
    }
}

/// WebSocket metrics snapshot
#[derive(Debug, Clone)]
pub struct WebSocketMetricsSnapshot {
    pub active_connections: u64,
    pub total_connections: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connection_errors: u64,
}

impl Clone for WebSocketManager {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            event_broadcasters: self.event_broadcasters.clone(),
            vm_client: self.vm_client.clone(),
            auth_service: self.auth_service.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

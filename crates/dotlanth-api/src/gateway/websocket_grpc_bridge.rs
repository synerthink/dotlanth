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

//! WebSocket to gRPC streaming bridge

use super::GatewayConfig;
use crate::error::{ApiError, ApiResult};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use tonic::Streaming;
use tonic::transport::Channel;
use tracing::{debug, error, info, warn};

/// WebSocket connection metadata
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub id: String,
    pub service_method: String,
    pub client_address: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// WebSocket message types for gRPC communication
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketGrpcMessage {
    /// Start a gRPC streaming call
    StartStream {
        stream_id: String,
        service: String,
        method: String,
        request: Value,
    },
    /// Send data to an active gRPC stream
    StreamData {
        stream_id: String,
        data: Value,
    },
    /// Close a gRPC stream
    CloseStream {
        stream_id: String,
    },
    /// Response from gRPC stream
    StreamResponse {
        stream_id: String,
        data: Value,
    },
    /// Error from gRPC stream
    StreamError {
        stream_id: String,
        error: String,
        code: i32,
    },
    /// Stream closed notification
    StreamClosed {
        stream_id: String,
        reason: String,
    },
    /// Ping/Pong for connection health
    Ping {
        timestamp: i64,
    },
    Pong {
        timestamp: i64,
    },
}

/// Active gRPC stream information
#[derive(Debug)]
struct ActiveStream {
    stream_id: String,
    service_method: String,
    sender: mpsc::Sender<Value>,
    started_at: chrono::DateTime<chrono::Utc>,
}

/// WebSocket to gRPC bridge
pub struct WebSocketGrpcBridge {
    config: GatewayConfig,
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    active_streams: Arc<RwLock<HashMap<String, ActiveStream>>>,
}

impl WebSocketGrpcBridge {
    pub fn new(config: GatewayConfig) -> ApiResult<Self> {
        Ok(Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Handle a new WebSocket connection
    pub async fn handle_websocket_connection<S>(&self, websocket: WebSocketStream<S>, client_address: String, grpc_channel: Channel) -> ApiResult<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let connection_id = uuid::Uuid::new_v4().to_string();
        let connection = WebSocketConnection {
            id: connection_id.clone(),
            service_method: "unknown".to_string(),
            client_address: client_address.clone(),
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
        };

        // Register the connection
        self.connections.write().await.insert(connection_id.clone(), connection);

        info!("New WebSocket connection established: {} from {}", connection_id, client_address);

        // Handle the WebSocket connection
        let result = self.handle_websocket_messages(websocket, connection_id.clone(), grpc_channel).await;

        // Clean up connection
        self.cleanup_connection(&connection_id).await;

        result
    }

    /// Handle WebSocket messages and bridge to gRPC
    async fn handle_websocket_messages<S>(&self, websocket: WebSocketStream<S>, connection_id: String, grpc_channel: Channel) -> ApiResult<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (mut ws_sender, mut ws_receiver) = websocket.split();

        // Create channels for bidirectional communication
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Message>(self.config.streaming_buffer_size);

        // Spawn task to handle outbound messages (gRPC -> WebSocket)
        let outbound_connection_id = connection_id.clone();
        tokio::spawn(async move {
            while let Some(message) = outbound_rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    error!("Failed to send WebSocket message for connection {}: {}", outbound_connection_id, e);
                    break;
                }
            }
        });

        // Handle inbound messages (WebSocket -> gRPC)
        while let Some(message_result) = ws_receiver.next().await {
            match message_result {
                Ok(message) => {
                    // Update last activity
                    if let Some(conn) = self.connections.write().await.get_mut(&connection_id) {
                        conn.last_activity = chrono::Utc::now();
                    }

                    match self.handle_websocket_message(message, &connection_id, &grpc_channel, &outbound_tx).await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("Error handling WebSocket message for connection {}: {}", connection_id, e);

                            // Send error back to client
                            let error_msg = WebSocketGrpcMessage::StreamError {
                                stream_id: "unknown".to_string(),
                                error: e.to_string(),
                                code: -1,
                            };

                            if let Ok(error_json) = serde_json::to_string(&error_msg) {
                                let _ = outbound_tx.send(Message::Text(error_json)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket error for connection {}: {}", connection_id, e);
                    break;
                }
            }
        }

        info!("WebSocket connection closed: {}", connection_id);
        Ok(())
    }

    /// Handle a single WebSocket message
    async fn handle_websocket_message(&self, message: Message, connection_id: &str, grpc_channel: &Channel, outbound_tx: &mpsc::Sender<Message>) -> ApiResult<()> {
        match message {
            Message::Text(text) => {
                let ws_message: WebSocketGrpcMessage = serde_json::from_str(&text).map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid WebSocket message format: {}", e),
                })?;

                self.handle_grpc_message(ws_message, connection_id, grpc_channel, outbound_tx).await
            }
            Message::Binary(data) => {
                // Try to parse binary data as JSON
                let text = String::from_utf8(data).map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid UTF-8 in binary message: {}", e),
                })?;

                let ws_message: WebSocketGrpcMessage = serde_json::from_str(&text).map_err(|e| ApiError::BadRequest {
                    message: format!("Invalid WebSocket message format: {}", e),
                })?;

                self.handle_grpc_message(ws_message, connection_id, grpc_channel, outbound_tx).await
            }
            Message::Ping(data) => {
                // Respond with pong
                let _ = outbound_tx.send(Message::Pong(data)).await;
                Ok(())
            }
            Message::Pong(_) => {
                // Handle pong (connection is alive)
                debug!("Received pong from connection: {}", connection_id);
                Ok(())
            }
            Message::Close(_) => {
                info!("WebSocket close message received for connection: {}", connection_id);
                Ok(())
            }
            Message::Frame(_) => {
                warn!("Received raw frame message for connection: {}", connection_id);
                Ok(())
            }
        }
    }

    /// Handle gRPC-related WebSocket messages
    async fn handle_grpc_message(&self, message: WebSocketGrpcMessage, connection_id: &str, grpc_channel: &Channel, outbound_tx: &mpsc::Sender<Message>) -> ApiResult<()> {
        match message {
            WebSocketGrpcMessage::StartStream { stream_id, service, method, request } => self.start_grpc_stream(stream_id, service, method, request, connection_id, grpc_channel, outbound_tx).await,
            WebSocketGrpcMessage::StreamData { stream_id, data } => self.send_stream_data(stream_id, data).await,
            WebSocketGrpcMessage::CloseStream { stream_id } => self.close_grpc_stream(stream_id, outbound_tx).await,
            WebSocketGrpcMessage::Ping { timestamp } => {
                let pong = WebSocketGrpcMessage::Pong { timestamp };
                let pong_json = serde_json::to_string(&pong).map_err(|e| ApiError::SerdeJsonError(e))?;
                let _ = outbound_tx.send(Message::Text(pong_json)).await;
                Ok(())
            }
            WebSocketGrpcMessage::Pong { .. } => {
                // Handle pong
                Ok(())
            }
            _ => Err(ApiError::BadRequest {
                message: "Invalid message type for client-to-server communication".to_string(),
            }),
        }
    }

    /// Start a new gRPC streaming call
    async fn start_grpc_stream(
        &self,
        stream_id: String,
        service: String,
        method: String,
        request: Value,
        connection_id: &str,
        grpc_channel: &Channel,
        outbound_tx: &mpsc::Sender<Message>,
    ) -> ApiResult<()> {
        info!("Starting gRPC stream {} for service {}/{}", stream_id, service, method);

        // Create a channel for sending data to the gRPC stream
        let (stream_tx, stream_rx) = mpsc::channel(self.config.streaming_buffer_size);

        // Register the active stream
        let active_stream = ActiveStream {
            stream_id: stream_id.clone(),
            service_method: format!("{}/{}", service, method),
            sender: stream_tx,
            started_at: chrono::Utc::now(),
        };

        self.active_streams.write().await.insert(stream_id.clone(), active_stream);

        // Start the gRPC streaming call
        let outbound_tx_clone = outbound_tx.clone();
        let stream_id_clone = stream_id.clone();
        let active_streams = self.active_streams.clone();

        tokio::spawn(async move {
            // This is a simplified implementation
            // In practice, you'd use the actual gRPC client based on service/method

            // Simulate gRPC streaming response
            let mut counter = 0;
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                let response_data = serde_json::json!({
                    "message": format!("Stream response {}", counter),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "counter": counter
                });

                let response_msg = WebSocketGrpcMessage::StreamResponse {
                    stream_id: stream_id_clone.clone(),
                    data: response_data,
                };

                if let Ok(response_json) = serde_json::to_string(&response_msg) {
                    if outbound_tx_clone.send(Message::Text(response_json)).await.is_err() {
                        break;
                    }
                } else {
                    break;
                }

                counter += 1;
                if counter >= 10 {
                    // End the stream after 10 messages
                    let close_msg = WebSocketGrpcMessage::StreamClosed {
                        stream_id: stream_id_clone.clone(),
                        reason: "Stream completed".to_string(),
                    };

                    if let Ok(close_json) = serde_json::to_string(&close_msg) {
                        let _ = outbound_tx_clone.send(Message::Text(close_json)).await;
                    }
                    break;
                }
            }

            // Clean up the stream
            active_streams.write().await.remove(&stream_id_clone);
        });

        Ok(())
    }

    /// Send data to an active gRPC stream
    async fn send_stream_data(&self, stream_id: String, data: Value) -> ApiResult<()> {
        let active_streams = self.active_streams.read().await;

        if let Some(stream) = active_streams.get(&stream_id) {
            stream.sender.send(data).await.map_err(|_| ApiError::InternalServerError {
                message: format!("Failed to send data to stream: {}", stream_id),
            })?;
            Ok(())
        } else {
            Err(ApiError::NotFound {
                message: format!("Active stream: {}", stream_id),
            })
        }
    }

    /// Close a gRPC stream
    async fn close_grpc_stream(&self, stream_id: String, outbound_tx: &mpsc::Sender<Message>) -> ApiResult<()> {
        let mut active_streams = self.active_streams.write().await;

        if active_streams.remove(&stream_id).is_some() {
            let close_msg = WebSocketGrpcMessage::StreamClosed {
                stream_id: stream_id.clone(),
                reason: "Client requested close".to_string(),
            };

            if let Ok(close_json) = serde_json::to_string(&close_msg) {
                let _ = outbound_tx.send(Message::Text(close_json)).await;
            }

            info!("Closed gRPC stream: {}", stream_id);
            Ok(())
        } else {
            Err(ApiError::NotFound {
                message: format!("Active stream: {}", stream_id),
            })
        }
    }

    /// Clean up a WebSocket connection
    async fn cleanup_connection(&self, connection_id: &str) {
        // Remove the connection
        self.connections.write().await.remove(connection_id);

        // Close all streams for this connection
        let mut active_streams = self.active_streams.write().await;
        let streams_to_remove: Vec<String> = active_streams
            .values()
            .filter(|stream| stream.stream_id.starts_with(connection_id))
            .map(|stream| stream.stream_id.clone())
            .collect();

        for stream_id in streams_to_remove {
            active_streams.remove(&stream_id);
        }

        info!("Cleaned up WebSocket connection: {}", connection_id);
    }

    /// Get active WebSocket connections
    pub async fn get_active_connections(&self) -> Vec<WebSocketConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    /// Get active gRPC streams
    pub async fn get_active_streams(&self) -> Vec<String> {
        self.active_streams.read().await.keys().cloned().collect()
    }

    /// Health check for WebSocket bridge
    pub async fn health_check(&self) -> ApiResult<()> {
        let connections = self.connections.read().await;
        let streams = self.active_streams.read().await;

        // Check if we're at capacity
        if connections.len() >= self.config.max_streaming_connections {
            return Err(ApiError::ServiceUnavailable {
                message: "WebSocket bridge at capacity".to_string(),
            });
        }

        // Check for stale connections
        let now = chrono::Utc::now();
        let stale_count = connections
            .values()
            .filter(|conn| (now - conn.last_activity).num_seconds() > 300) // 5 minutes
            .count();

        if stale_count > connections.len() / 2 {
            warn!("High number of stale WebSocket connections: {}/{}", stale_count, connections.len());
        }

        info!("WebSocket bridge health: {} connections, {} streams", connections.len(), streams.len());
        Ok(())
    }
}

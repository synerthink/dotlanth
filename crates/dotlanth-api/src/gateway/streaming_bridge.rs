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

//! Streaming bridge for gRPC streaming to HTTP streaming conversion

use super::GatewayConfig;
use crate::error::{ApiError, ApiResult};
use futures::{Stream, StreamExt, TryStreamExt};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::Streaming;
use tracing::{debug, error, info, warn};

/// Streaming connection metadata
#[derive(Debug, Clone)]
pub struct StreamingConnection {
    pub id: String,
    pub service_method: String,
    pub client_info: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Streaming bridge metrics
#[derive(Debug, Default, Clone)]
pub struct StreamingMetrics {
    pub active_connections: u64,
    pub total_connections: u64,
    pub messages_streamed: u64,
    pub errors: u64,
    pub avg_connection_duration_secs: f64,
}

/// HTTP Server-Sent Events stream wrapper
pub struct SseStream {
    receiver: mpsc::Receiver<Result<Bytes, ApiError>>,
}

impl SseStream {
    pub fn new(receiver: mpsc::Receiver<Result<Bytes, ApiError>>) -> Self {
        Self { receiver }
    }
}

impl Stream for SseStream {
    type Item = Result<Bytes, ApiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

/// Streaming bridge for converting gRPC streams to HTTP streams
pub struct StreamingBridge {
    config: GatewayConfig,
    connections: Arc<RwLock<std::collections::HashMap<String, StreamingConnection>>>,
    metrics: Arc<RwLock<StreamingMetrics>>,
}

impl StreamingBridge {
    pub fn new(config: GatewayConfig) -> ApiResult<Self> {
        Ok(Self {
            config,
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
            metrics: Arc::new(RwLock::new(StreamingMetrics::default())),
        })
    }

    /// Convert gRPC server streaming to HTTP Server-Sent Events
    pub async fn grpc_stream_to_sse(&self, mut grpc_stream: Streaming<Value>, service_method: String, client_info: String) -> ApiResult<Response<Full<Bytes>>> {
        let connection_id = uuid::Uuid::new_v4().to_string();
        let connection = StreamingConnection {
            id: connection_id.clone(),
            service_method: service_method.clone(),
            client_info: client_info.clone(),
            started_at: chrono::Utc::now(),
        };

        // Register the connection
        self.register_connection(connection.clone()).await;

        info!("Starting gRPC to SSE streaming for connection: {}", connection_id);

        // Create a channel for the SSE stream
        let (sender, receiver) = mpsc::channel(self.config.streaming_buffer_size);

        // Spawn a task to handle the gRPC stream
        let connections = self.connections.clone();
        let metrics = self.metrics.clone();
        let connection_id_clone = connection_id.clone();

        tokio::spawn(async move {
            let mut message_count = 0u64;
            let start_time = std::time::Instant::now();

            while let Some(result) = grpc_stream.next().await {
                match result {
                    Ok(message) => {
                        // Convert gRPC message to SSE format
                        let sse_data = match Self::format_sse_message(&message) {
                            Ok(data) => data,
                            Err(e) => {
                                error!("Failed to format SSE message: {}", e);
                                let _ = sender.send(Err(e)).await;
                                break;
                            }
                        };

                        // Send the message
                        if sender.send(Ok(sse_data)).await.is_err() {
                            debug!("SSE client disconnected for connection: {}", connection_id_clone);
                            break;
                        }

                        message_count += 1;
                    }
                    Err(status) => {
                        error!("gRPC stream error for connection {}: {}", connection_id_clone, status);

                        // Send error as SSE event
                        let error_sse = Self::format_sse_error(&status);
                        let _ = sender.send(Ok(error_sse)).await;

                        // Update metrics
                        let mut metrics_guard = metrics.write().await;
                        metrics_guard.errors += 1;
                        break;
                    }
                }
            }

            // Clean up connection
            connections.write().await.remove(&connection_id_clone);

            // Update metrics
            let mut metrics_guard = metrics.write().await;
            metrics_guard.active_connections = metrics_guard.active_connections.saturating_sub(1);
            metrics_guard.messages_streamed += message_count;

            let duration = start_time.elapsed().as_secs_f64();
            let total_connections = metrics_guard.total_connections.max(1);
            metrics_guard.avg_connection_duration_secs = (metrics_guard.avg_connection_duration_secs * (total_connections - 1) as f64 + duration) / total_connections as f64;

            info!("Streaming connection {} ended after {} messages in {:.2}s", connection_id_clone, message_count, duration);
        });

        // Create SSE response
        let sse_stream = SseStream::new(receiver);
        self.create_sse_response(sse_stream).await
    }

    /// Convert HTTP stream to gRPC client streaming
    pub async fn http_stream_to_grpc(
        &self,
        http_stream: impl Stream<Item = Result<Bytes, ApiError>> + Send + 'static,
        service_method: String,
    ) -> ApiResult<impl Stream<Item = Result<Value, tonic::Status>>> {
        let connection_id = uuid::Uuid::new_v4().to_string();

        info!("Starting HTTP to gRPC streaming for connection: {}", connection_id);

        // Convert HTTP stream to gRPC stream
        let grpc_stream = http_stream.map(move |result| {
            match result {
                Ok(bytes) => {
                    // Parse HTTP chunk as JSON
                    match serde_json::from_slice::<Value>(&bytes) {
                        Ok(value) => Ok(value),
                        Err(e) => {
                            error!("Failed to parse HTTP stream chunk as JSON: {}", e);
                            Err(tonic::Status::invalid_argument(format!("Invalid JSON in stream: {}", e)))
                        }
                    }
                }
                Err(api_error) => {
                    error!("HTTP stream error: {}", api_error);
                    Err(tonic::Status::internal(format!("HTTP stream error: {}", api_error)))
                }
            }
        });

        Ok(grpc_stream)
    }

    /// Register a new streaming connection
    async fn register_connection(&self, connection: StreamingConnection) {
        let mut connections = self.connections.write().await;
        connections.insert(connection.id.clone(), connection);

        let mut metrics = self.metrics.write().await;
        metrics.active_connections += 1;
        metrics.total_connections += 1;
    }

    /// Create Server-Sent Events HTTP response
    async fn create_sse_response(&self, sse_stream: SseStream) -> ApiResult<Response<Full<Bytes>>> {
        // For this implementation, we'll create a simple response
        // In practice, you'd want to use a proper streaming response body

        let response_body = "data: {\"message\": \"SSE stream started\"}\n\n".to_string();

        let response = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("cache-control", "no-cache")
            .header("connection", "keep-alive")
            .header("access-control-allow-origin", "*")
            .header("access-control-allow-headers", "Cache-Control")
            .body(Full::new(Bytes::from(response_body)))
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to create SSE response: {}", e),
            })?;

        Ok(response)
    }

    /// Format a message as Server-Sent Events data
    fn format_sse_message(message: &Value) -> ApiResult<Bytes> {
        let json_str = serde_json::to_string(message).map_err(|e| ApiError::SerdeJsonError(e))?;

        let sse_data = format!("data: {}\n\n", json_str);
        Ok(Bytes::from(sse_data))
    }

    /// Format an error as Server-Sent Events data
    fn format_sse_error(status: &tonic::Status) -> Bytes {
        let error_data = serde_json::json!({
            "error": {
                "code": status.code() as i32,
                "message": status.message(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        let sse_data = format!("event: error\ndata: {}\n\n", error_data);
        Bytes::from(sse_data)
    }

    /// Get current streaming metrics
    pub async fn get_metrics(&self) -> StreamingMetrics {
        self.metrics.read().await.clone()
    }

    /// Get active connections
    pub async fn get_active_connections(&self) -> Vec<StreamingConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    /// Close a specific streaming connection
    pub async fn close_connection(&self, connection_id: &str) -> ApiResult<()> {
        let mut connections = self.connections.write().await;

        if connections.remove(connection_id).is_some() {
            let mut metrics = self.metrics.write().await;
            metrics.active_connections = metrics.active_connections.saturating_sub(1);

            info!("Closed streaming connection: {}", connection_id);
            Ok(())
        } else {
            Err(ApiError::NotFound {
                message: format!("Streaming connection: {}", connection_id),
            })
        }
    }

    /// Close all streaming connections
    pub async fn close_all_connections(&self) {
        let mut connections = self.connections.write().await;
        let count = connections.len();
        connections.clear();

        let mut metrics = self.metrics.write().await;
        metrics.active_connections = 0;

        info!("Closed {} streaming connections", count);
    }

    /// Health check for streaming bridge
    pub async fn health_check(&self) -> ApiResult<()> {
        let metrics = self.get_metrics().await;

        // Check if we're at capacity
        if metrics.active_connections >= self.config.max_streaming_connections as u64 {
            return Err(ApiError::ServiceUnavailable {
                message: "Streaming bridge at capacity".to_string(),
            });
        }

        // Check error rate
        if metrics.total_connections > 100 {
            let error_rate = metrics.errors as f64 / metrics.total_connections as f64;
            if error_rate > 0.1 {
                return Err(ApiError::ServiceUnavailable {
                    message: format!("Streaming bridge error rate too high: {:.2}%", error_rate * 100.0),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_message_formatting() {
        let message = serde_json::json!({
            "type": "update",
            "data": {"key": "value"}
        });

        let sse_data = StreamingBridge::format_sse_message(&message).unwrap();
        let sse_str = String::from_utf8(sse_data.to_vec()).unwrap();

        assert!(sse_str.starts_with("data: "));
        assert!(sse_str.ends_with("\n\n"));
        assert!(sse_str.contains("\"type\":\"update\""));
    }

    #[test]
    fn test_sse_error_formatting() {
        let status = tonic::Status::internal("Test error");
        let sse_data = StreamingBridge::format_sse_error(&status);
        let sse_str = String::from_utf8(sse_data.to_vec()).unwrap();

        assert!(sse_str.starts_with("event: error\n"));
        assert!(sse_str.contains("Test error"));
    }

    #[tokio::test]
    async fn test_connection_management() {
        let config = GatewayConfig::default();
        let bridge = StreamingBridge::new(config).unwrap();

        let connection = StreamingConnection {
            id: "test-123".to_string(),
            service_method: "TestMethod".to_string(),
            client_info: "test-client".to_string(),
            started_at: chrono::Utc::now(),
        };

        bridge.register_connection(connection).await;

        let metrics = bridge.get_metrics().await;
        assert_eq!(metrics.active_connections, 1);
        assert_eq!(metrics.total_connections, 1);

        let connections = bridge.get_active_connections().await;
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].id, "test-123");
    }
}

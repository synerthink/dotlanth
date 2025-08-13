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

//! WebSocket handlers for real-time events and bidirectional communication

use crate::error::{ApiError, ApiResult};
use crate::models::WebSocketMessage;
use crate::websocket::WebSocketManager;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::{Request, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Handle WebSocket connection upgrade request
pub async fn websocket_upgrade(websocket_manager: Arc<WebSocketManager>, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiError> {
    debug!("Handling WebSocket connection upgrade request");

    // Delegate the actual WebSocket upgrade handling to the WebSocketManager
    websocket_manager.handle_websocket_upgrade(req).await
}

/// Handle WebSocket connection events (for real-time VM execution events)
pub async fn handle_websocket_event(websocket_manager: Arc<WebSocketManager>, event: WebSocketMessage) -> ApiResult<()> {
    debug!("Handling WebSocket event: {}", event.event_type);

    // Broadcast the event to all subscribed connections
    websocket_manager.broadcast_event(event).await;

    Ok(())
}

/// Get WebSocket connection metrics
pub async fn get_websocket_metrics(websocket_manager: Arc<WebSocketManager>) -> ApiResult<WebSocketMessage> {
    debug!("Getting WebSocket connection metrics");

    let metrics = websocket_manager.get_metrics();

    Ok(WebSocketMessage {
        event_type: "websocket_metrics".to_string(),
        payload: serde_json::json!({
            "active_connections": metrics.active_connections,
            "total_connections": metrics.total_connections,
            "messages_sent": metrics.messages_sent,
            "messages_received": metrics.messages_received,
            "connection_errors": metrics.connection_errors,
        }),
        timestamp: chrono::Utc::now(),
    })
}

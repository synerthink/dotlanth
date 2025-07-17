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

//! Dot event streaming implementation

use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use futures::Stream;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tonic::Status;
use tracing::{debug, error, info};
use dashmap::DashMap;

use crate::proto::vm_service::{DotEvent, StreamDotEventsRequest};

/// Dot event broadcaster for managing event subscriptions
pub struct DotEventBroadcaster {
    sender: broadcast::Sender<DotEvent>,
    subscribers: Arc<RwLock<DashMap<String, broadcast::Receiver<DotEvent>>>>,
    max_subscribers: usize,
}

impl DotEventBroadcaster {
    pub fn new(buffer_size: usize, max_subscribers: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        
        Self {
            sender,
            subscribers: Arc::new(RwLock::new(DashMap::new())),
            max_subscribers,
        }
    }

    /// Broadcast an event to all subscribers
    pub async fn broadcast_event(&self, event: DotEvent) -> Result<usize, String> {
        match self.sender.send(event.clone()) {
            Ok(subscriber_count) => {
                debug!("Broadcasted event to {} subscribers: {:?}", subscriber_count, event.event_type);
                Ok(subscriber_count)
            }
            Err(_) => {
                error!("Failed to broadcast event - no active subscribers");
                Err("No active subscribers".to_string())
            }
        }
    }

    /// Subscribe to events with optional filtering
    pub async fn subscribe(&self, subscriber_id: String, filter: Option<DotEventFilter>) -> Result<DotEventStream, String> {
        let subscribers = self.subscribers.read().await;
        
        if subscribers.len() >= self.max_subscribers {
            return Err("Maximum subscribers reached".to_string());
        }
        
        drop(subscribers); // Release read lock
        
        let receiver = self.sender.subscribe();
        
        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(subscriber_id.clone(), receiver);
        
        info!("New subscriber: {} (total: {})", subscriber_id, subscribers.len());
        
        let stream = DotEventStream::new(self.sender.subscribe(), filter);
        Ok(stream)
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscriber_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        if subscribers.remove(subscriber_id).is_some() {
            info!("Unsubscribed: {} (remaining: {})", subscriber_id, subscribers.len());
        }
    }

    /// Get current subscriber count
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }

    /// Broadcast dot execution started event
    pub async fn dot_execution_started(&self, dot_id: String, user_id: String) {
        let event = DotEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: 1, // ExecutionStarted
            dot_id: dot_id.clone(),
            event_data: format!("Dot {} execution started", dot_id).into_bytes(),
            metadata: std::collections::HashMap::new(),
        };
        
        let _ = self.broadcast_event(event).await;
    }

    /// Broadcast dot execution completed event
    pub async fn dot_execution_completed(&self, dot_id: String, user_id: String, success: bool, duration_ms: u64) {
        let event = DotEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: if success { 2 } else { 3 }, // ExecutionCompleted or ExecutionFailed
            dot_id: dot_id.clone(),
            event_data: format!("Dot {} execution {} in {}ms", dot_id, if success { "completed" } else { "failed" }, duration_ms).into_bytes(),
            metadata: {
                let mut map = std::collections::HashMap::new();
                map.insert("duration_ms".to_string(), duration_ms.to_string());
                map.insert("success".to_string(), success.to_string());
                map
            },
        };
        
        let _ = self.broadcast_event(event).await;
    }

    /// Broadcast dot deployed event
    pub async fn dot_deployed(&self, dot_id: String, user_id: String, version: String) {
        let event = DotEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            event_type: DotEventType::DotDeployed as i32,
            dot_id: dot_id.clone(),
            user_id: user_id.clone(),
            message: format!("Dot {} version {} deployed by {}", dot_id, version, user_id),
            metadata: {
                let mut map = std::collections::HashMap::new();
                map.insert("version".to_string(), version);
                map
            },
        };
        
        let _ = self.broadcast_event(event).await;
    }
}

/// Event filter for selective subscription
#[derive(Debug, Clone)]
pub struct DotEventFilter {
    pub dot_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub event_types: Option<Vec<DotEventType>>,
}

impl DotEventFilter {
    pub fn matches(&self, event: &DotEvent) -> bool {
        // Check dot_id filter
        if let Some(dot_ids) = &self.dot_ids {
            if !dot_ids.contains(&event.dot_id) {
                return false;
            }
        }

        // Check user_id filter
        if let Some(_user_ids) = &self.user_ids {
            // Note: user_id filtering would be implemented here if needed
            // For now, we'll skip this filter since user_id is not in the current proto
        }

        // Check event_type filter
        if let Some(_event_types) = &self.event_types {
            // Note: event_type filtering would be implemented here
            // For now, we'll skip this filter since event types are not in the current proto
        }

        true
    }
}

/// Streaming implementation for dot events
pub struct DotEventStream {
    receiver: broadcast::Receiver<DotEvent>,
    filter: Option<DotEventFilter>,
}

impl DotEventStream {
    pub fn new(receiver: broadcast::Receiver<DotEvent>, filter: Option<DotEventFilter>) -> Self {
        Self { receiver, filter }
    }
}

impl Stream for DotEventStream {
    type Item = Result<DotEvent, Status>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(event) => {
                // Apply filter if present
                if let Some(filter) = &self.filter {
                    if !filter.matches(&event) {
                        // Event doesn't match filter, continue polling
                        cx.waker().wake_by_ref();
                        return std::task::Poll::Pending;
                    }
                }
                
                std::task::Poll::Ready(Some(Ok(event)))
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                // No events available, register waker and return pending
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                // Channel closed, end stream
                std::task::Poll::Ready(None)
            }
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // We've missed some events, but continue
                debug!("Event stream lagged, some events may have been missed");
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        }
    }
}

/// Helper to create event filter from request
pub fn create_filter_from_request(request: &StreamDotEventsRequest) -> Option<DotEventFilter> {
    let has_filters = !request.dot_ids.is_empty() || 
                     // User ID filtering would be checked here
                     !request.event_types.is_empty();

    if !has_filters {
        return None;
    }

    Some(DotEventFilter {
        dot_ids: if request.dot_ids.is_empty() { None } else { Some(request.dot_ids.clone()) },
        user_ids: None, // User ID filtering not implemented in current proto
        event_types: if request.event_types.is_empty() { 
            None 
        } else { 
            Some(request.event_types.iter()
                .filter_map(|&t| DotEventType::try_from(t).ok())
                .collect())
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_dot_event_broadcaster() {
        let broadcaster = DotEventBroadcaster::new(100, 10);
        
        // Subscribe to events
        let stream = broadcaster.subscribe("test_subscriber".to_string(), None).await.unwrap();
        
        // Broadcast an event
        broadcaster.dot_execution_started("test_dot".to_string(), "test_user".to_string()).await;
        
        // Check subscriber count
        assert_eq!(broadcaster.subscriber_count().await, 1);
        
        // Unsubscribe
        broadcaster.unsubscribe("test_subscriber").await;
        assert_eq!(broadcaster.subscriber_count().await, 0);
    }

    #[tokio::test]
    async fn test_event_filter() {
        let filter = DotEventFilter {
            dot_ids: Some(vec!["dot1".to_string(), "dot2".to_string()]),
            user_ids: None,
            event_types: Some(vec![DotEventType::ExecutionStarted]),
        };

        let event1 = DotEvent {
            event_id: "1".to_string(),
            timestamp: 0,
            event_type: DotEventType::ExecutionStarted as i32,
            dot_id: "dot1".to_string(),
            user_id: "user1".to_string(),
            message: "test".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        let event2 = DotEvent {
            event_id: "2".to_string(),
            timestamp: 0,
            event_type: DotEventType::ExecutionCompleted as i32,
            dot_id: "dot1".to_string(),
            user_id: "user1".to_string(),
            message: "test".to_string(),
            metadata: std::collections::HashMap::new(),
        };

        assert!(filter.matches(&event1)); // Matches dot_id and event_type
        assert!(!filter.matches(&event2)); // Wrong event_type
    }
}
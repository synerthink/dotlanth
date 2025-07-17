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

//! Advanced gRPC Streaming Service
//! 
//! Implements sophisticated streaming patterns with backpressure, flow control,
//! and multiplexing capabilities for high-performance real-time communication.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use futures::{Stream, StreamExt};
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{interval, sleep};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Stream configuration for advanced features
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Maximum number of concurrent streams per client
    pub max_concurrent_streams: usize,
    /// Buffer size for each stream
    pub buffer_size: usize,
    /// Backpressure threshold (percentage of buffer)
    pub backpressure_threshold: f32,
    /// Flow control window size
    pub flow_control_window: usize,
    /// Stream timeout duration
    pub stream_timeout: Duration,
    /// Enable compression for streams
    pub compression_enabled: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 100,
            buffer_size: 1000,
            backpressure_threshold: 0.8,
            flow_control_window: 65536,
            stream_timeout: Duration::from_secs(300), // 5 minutes
            compression_enabled: true,
        }
    }
}

/// Stream metadata for tracking and management
#[derive(Debug, Clone)]
pub struct StreamMetadata {
    pub stream_id: String,
    pub client_id: String,
    pub created_at: Instant,
    pub last_activity: Instant,
    pub message_count: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub is_active: bool,
}

/// Backpressure signal for flow control
#[derive(Debug, Clone)]
pub enum BackpressureSignal {
    /// Normal flow - no backpressure
    Normal,
    /// Slow down - reduce sending rate
    SlowDown { factor: f32 },
    /// Pause - stop sending temporarily
    Pause { duration: Duration },
    /// Resume - continue normal flow
    Resume,
}

/// Flow control message
#[derive(Debug, Clone)]
pub struct FlowControlMessage {
    pub stream_id: String,
    pub signal: BackpressureSignal,
    pub buffer_usage: f32,
    pub timestamp: Instant,
}

/// Advanced streaming service with sophisticated flow control
#[derive(Debug)]
pub struct AdvancedStreamingService {
    /// Configuration
    config: StreamConfig,
    /// Active streams metadata
    streams: Arc<RwLock<HashMap<String, StreamMetadata>>>,
    /// Flow control channels per stream
    flow_control_senders: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<FlowControlMessage>>>>,
    /// Semaphore for limiting concurrent streams
    stream_semaphore: Arc<Semaphore>,
    /// Global metrics
    metrics: Arc<RwLock<StreamingMetrics>>,
}

/// Streaming metrics for monitoring
#[derive(Debug, Default)]
pub struct StreamingMetrics {
    pub total_streams_created: u64,
    pub active_streams: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub backpressure_events: u64,
    pub stream_timeouts: u64,
    pub stream_errors: u64,
}

impl AdvancedStreamingService {
    /// Create a new advanced streaming service
    pub fn new(config: StreamConfig) -> Self {
        let stream_semaphore = Arc::new(Semaphore::new(config.max_concurrent_streams));
        
        Self {
            config,
            streams: Arc::new(RwLock::new(HashMap::new())),
            flow_control_senders: Arc::new(RwLock::new(HashMap::new())),
            stream_semaphore,
            metrics: Arc::new(RwLock::new(StreamingMetrics::default())),
        }
    }

    /// Create a new managed stream with flow control
    pub async fn create_managed_stream<T>(&self, client_id: String) -> Result<ManagedStream<T>, Status> 
    where
        T: Send + 'static,
    {
        // Acquire semaphore permit for concurrent stream limiting
        let permit = self.stream_semaphore.clone()
            .acquire_owned()
            .await
            .map_err(|_| Status::resource_exhausted("Too many concurrent streams"))?;

        let stream_id = Uuid::new_v4().to_string();
        let now = Instant::now();

        // Create stream metadata
        let metadata = StreamMetadata {
            stream_id: stream_id.clone(),
            client_id: client_id.clone(),
            created_at: now,
            last_activity: now,
            message_count: 0,
            bytes_sent: 0,
            bytes_received: 0,
            is_active: true,
        };

        // Create flow control channel
        let (flow_tx, flow_rx) = mpsc::unbounded_channel();
        
        // Create message channel with backpressure
        let (msg_tx, msg_rx) = mpsc::channel(self.config.buffer_size);

        // Register stream
        {
            let mut streams = self.streams.write().await;
            streams.insert(stream_id.clone(), metadata);
            
            let mut flow_senders = self.flow_control_senders.write().await;
            flow_senders.insert(stream_id.clone(), flow_tx);
            
            let mut metrics = self.metrics.write().await;
            metrics.total_streams_created += 1;
            metrics.active_streams += 1;
        }

        info!("Created managed stream {} for client {}", stream_id, client_id);

        Ok(ManagedStream::new(
            stream_id,
            client_id,
            msg_tx,
            msg_rx,
            flow_rx,
            permit,
            self.config.clone(),
            Arc::clone(&self.streams),
            Arc::clone(&self.metrics),
        ))
    }

    /// Get stream metrics
    pub async fn get_metrics(&self) -> StreamingMetrics {
        let metrics = self.metrics.read().await;
        StreamingMetrics {
            total_streams_created: metrics.total_streams_created,
            active_streams: metrics.active_streams,
            total_messages_sent: metrics.total_messages_sent,
            total_messages_received: metrics.total_messages_received,
            total_bytes_sent: metrics.total_bytes_sent,
            total_bytes_received: metrics.total_bytes_received,
            backpressure_events: metrics.backpressure_events,
            stream_timeouts: metrics.stream_timeouts,
            stream_errors: metrics.stream_errors,
        }
    }

    /// Get active streams
    pub async fn get_active_streams(&self) -> Vec<StreamMetadata> {
        self.streams.read().await
            .values()
            .filter(|stream| stream.is_active)
            .cloned()
            .collect()
    }

    /// Cleanup inactive streams
    pub async fn cleanup_inactive_streams(&self) {
        let mut streams_to_remove = Vec::new();
        let timeout = self.config.stream_timeout;
        let now = Instant::now();

        {
            let streams = self.streams.read().await;
            for (stream_id, metadata) in streams.iter() {
                if now.duration_since(metadata.last_activity) > timeout {
                    streams_to_remove.push(stream_id.clone());
                }
            }
        }

        if !streams_to_remove.is_empty() {
            let mut streams = self.streams.write().await;
            let mut flow_senders = self.flow_control_senders.write().await;
            let mut metrics = self.metrics.write().await;

            for stream_id in streams_to_remove {
                streams.remove(&stream_id);
                flow_senders.remove(&stream_id);
                metrics.active_streams = metrics.active_streams.saturating_sub(1);
                metrics.stream_timeouts += 1;
                
                warn!("Cleaned up inactive stream: {}", stream_id);
            }
        }
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        let service = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Cleanup every minute
            
            loop {
                interval.tick().await;
                service.cleanup_inactive_streams().await;
            }
        });
    }
}

/// Managed stream with advanced flow control
pub struct ManagedStream<T> {
    stream_id: String,
    client_id: String,
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
    flow_control_rx: mpsc::UnboundedReceiver<FlowControlMessage>,
    _permit: tokio::sync::OwnedSemaphorePermit,
    config: StreamConfig,
    streams: Arc<RwLock<HashMap<String, StreamMetadata>>>,
    metrics: Arc<RwLock<StreamingMetrics>>,
    buffer_usage: f32,
    last_backpressure_check: Instant,
}

impl<T> ManagedStream<T> 
where
    T: Send + 'static,
{
    fn new(
        stream_id: String,
        client_id: String,
        sender: mpsc::Sender<T>,
        receiver: mpsc::Receiver<T>,
        flow_control_rx: mpsc::UnboundedReceiver<FlowControlMessage>,
        permit: tokio::sync::OwnedSemaphorePermit,
        config: StreamConfig,
        streams: Arc<RwLock<HashMap<String, StreamMetadata>>>,
        metrics: Arc<RwLock<StreamingMetrics>>,
    ) -> Self {
        Self {
            stream_id,
            client_id,
            sender,
            receiver,
            flow_control_rx,
            _permit: permit,
            config,
            streams,
            metrics,
            buffer_usage: 0.0,
            last_backpressure_check: Instant::now(),
        }
    }

    /// Send message with backpressure handling
    pub async fn send(&mut self, message: T) -> Result<(), Status> {
        // Check buffer usage for backpressure
        self.check_backpressure().await;

        // Apply flow control if needed
        if self.buffer_usage > self.config.backpressure_threshold {
            self.apply_flow_control().await?;
        }

        // Send message
        self.sender.send(message).await
            .map_err(|_| Status::internal("Stream closed"))?;

        // Update metrics
        self.update_send_metrics().await;

        Ok(())
    }

    /// Receive message
    pub async fn recv(&mut self) -> Option<T> {
        let message = self.receiver.recv().await;
        
        if message.is_some() {
            self.update_recv_metrics().await;
        }
        
        message
    }

    /// Check and handle backpressure
    async fn check_backpressure(&mut self) {
        let now = Instant::now();
        
        // Only check backpressure periodically to avoid overhead
        if now.duration_since(self.last_backpressure_check) < Duration::from_millis(100) {
            return;
        }
        
        self.last_backpressure_check = now;
        
        // Calculate buffer usage
        let capacity = self.config.buffer_size as f32;
        let used = (capacity - self.sender.capacity() as f32).max(0.0);
        self.buffer_usage = used / capacity;

        debug!(
            "Stream {} buffer usage: {:.2}%", 
            self.stream_id, 
            self.buffer_usage * 100.0
        );
    }

    /// Apply flow control based on buffer usage
    async fn apply_flow_control(&mut self) -> Result<(), Status> {
        if self.buffer_usage > 0.95 {
            // Critical backpressure - pause briefly
            warn!("Critical backpressure on stream {}, pausing", self.stream_id);
            sleep(Duration::from_millis(10)).await;
            
            let mut metrics = self.metrics.write().await;
            metrics.backpressure_events += 1;
        } else if self.buffer_usage > self.config.backpressure_threshold {
            // Moderate backpressure - small delay
            sleep(Duration::from_millis(1)).await;
        }

        Ok(())
    }

    /// Update send metrics
    async fn update_send_metrics(&self) {
        let mut streams = self.streams.write().await;
        if let Some(metadata) = streams.get_mut(&self.stream_id) {
            metadata.message_count += 1;
            metadata.last_activity = Instant::now();
            // Note: bytes_sent would be updated with actual message size
        }

        let mut metrics = self.metrics.write().await;
        metrics.total_messages_sent += 1;
    }

    /// Update receive metrics
    async fn update_recv_metrics(&self) {
        let mut streams = self.streams.write().await;
        if let Some(metadata) = streams.get_mut(&self.stream_id) {
            metadata.last_activity = Instant::now();
            // Note: bytes_received would be updated with actual message size
        }

        let mut metrics = self.metrics.write().await;
        metrics.total_messages_received += 1;
    }

    /// Get stream ID
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    /// Get client ID
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Get current buffer usage
    pub fn buffer_usage(&self) -> f32 {
        self.buffer_usage
    }
}

impl<T> Drop for ManagedStream<T> {
    fn drop(&mut self) {
        info!("Dropping managed stream: {}", self.stream_id);
        
        // Mark stream as inactive
        let streams = Arc::clone(&self.streams);
        let stream_id = self.stream_id.clone();
        let metrics = Arc::clone(&self.metrics);
        
        tokio::spawn(async move {
            let mut streams = streams.write().await;
            if let Some(metadata) = streams.get_mut(&stream_id) {
                metadata.is_active = false;
            }
            
            let mut metrics = metrics.write().await;
            metrics.active_streams = metrics.active_streams.saturating_sub(1);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_managed_stream_creation() {
        let config = StreamConfig::default();
        let service = AdvancedStreamingService::new(config);
        
        let stream = service.create_managed_stream::<String>("test_client".to_string()).await;
        assert!(stream.is_ok());
        
        let metrics = service.get_metrics().await;
        assert_eq!(metrics.total_streams_created, 1);
        assert_eq!(metrics.active_streams, 1);
    }

    #[tokio::test]
    async fn test_stream_send_receive() {
        let config = StreamConfig::default();
        let service = AdvancedStreamingService::new(config);
        
        let mut stream = service.create_managed_stream::<String>("test_client".to_string()).await.unwrap();
        
        // Send a message
        let result = stream.send("test message".to_string()).await;
        assert!(result.is_ok());
        
        // Receive the message
        let received = stream.recv().await;
        assert_eq!(received, Some("test message".to_string()));
    }
}
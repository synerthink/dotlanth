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

//! VM metrics streaming implementation

use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use futures::Stream;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Interval};
use tonic::Status;
use tracing::{debug, error, info};

use crate::proto::vm_service::{VmMetric, StreamVmMetricsRequest};

/// VM metrics collector for gathering system metrics
pub struct VmMetricsCollector {
    sender: broadcast::Sender<VmMetric>,
    collection_interval: Duration,
    is_running: Arc<RwLock<bool>>,
}

impl VmMetricsCollector {
    pub fn new(buffer_size: usize, collection_interval: Duration) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        
        let collector = Self {
            sender,
            collection_interval,
            is_running: Arc::new(RwLock::new(false)),
        };

        // Start metrics collection task
        let collector_clone = Arc::new(collector);
        let task_collector = collector_clone.clone();
        tokio::spawn(async move {
            task_collector.collection_task().await;
        });

        Arc::try_unwrap(collector_clone).unwrap_or_else(|_| unreachable!())
    }

    /// Start metrics collection
    pub async fn start(&self) {
        let mut running = self.is_running.write().await;
        *running = true;
        info!("VM metrics collection started");
    }

    /// Stop metrics collection
    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;
        info!("VM metrics collection stopped");
    }

    /// Subscribe to metrics stream
    pub fn subscribe(&self) -> VmMetricsStream {
        VmMetricsStream::new(self.sender.subscribe(), self.collection_interval)
    }

    /// Get current subscriber count
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Manually emit a metric
    pub async fn emit_metric(&self, metric: VmMetric) -> Result<usize, String> {
        match self.sender.send(metric.clone()) {
            Ok(subscriber_count) => {
                debug!("Emitted metric to {} subscribers: {:?}", subscriber_count, metric.metric_type);
                Ok(subscriber_count)
            }
            Err(_) => {
                error!("Failed to emit metric - no active subscribers");
                Err("No active subscribers".to_string())
            }
        }
    }

    /// Background task for collecting metrics
    async fn collection_task(&self) {
        let mut interval = interval(self.collection_interval);
        
        loop {
            interval.tick().await;
            
            let is_running = *self.is_running.read().await;
            if !is_running {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            // Collect various metrics
            self.collect_cpu_metrics().await;
            self.collect_memory_metrics().await;
            self.collect_execution_metrics().await;
            self.collect_connection_metrics().await;
        }
    }

    async fn collect_cpu_metrics(&self) {
        // Mock CPU usage - in real implementation, use system APIs
        let cpu_usage = rand::random::<f64>() * 100.0;
        
        let metric = VmMetric {
            metric_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            metric_type: VmMetricType::CpuUsage as i32,
            name: "cpu_usage_percent".to_string(),
            value: cpu_usage,
            unit: "percent".to_string(),
            labels: {
                let mut labels = std::collections::HashMap::new();
                labels.insert("component".to_string(), "vm".to_string());
                labels
            },
        };

        let _ = self.emit_metric(metric).await;
    }

    async fn collect_memory_metrics(&self) {
        // Mock memory usage
        let memory_used = (rand::random::<f64>() * 1024.0 * 1024.0 * 1024.0) as u64; // GB
        let memory_total = 8 * 1024 * 1024 * 1024; // 8GB
        let memory_percent = (memory_used as f64 / memory_total as f64) * 100.0;

        let metrics = vec![
            VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::MemoryUsage as i32,
                name: "memory_used_bytes".to_string(),
                value: memory_used as f64,
                unit: "bytes".to_string(),
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("component".to_string(), "vm".to_string());
                    labels
                },
            },
            VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::MemoryUsage as i32,
                name: "memory_usage_percent".to_string(),
                value: memory_percent,
                unit: "percent".to_string(),
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("component".to_string(), "vm".to_string());
                    labels
                },
            },
        ];

        for metric in metrics {
            let _ = self.emit_metric(metric).await;
        }
    }

    async fn collect_execution_metrics(&self) {
        // Mock execution metrics
        let dots_executed = rand::random::<u32>() % 100;
        let avg_execution_time = rand::random::<f64>() * 1000.0; // ms

        let metrics = vec![
            VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::ExecutionCount as i32,
                name: "dots_executed_total".to_string(),
                value: dots_executed as f64,
                unit: "count".to_string(),
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("component".to_string(), "executor".to_string());
                    labels
                },
            },
            VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::ExecutionTime as i32,
                name: "avg_execution_time_ms".to_string(),
                value: avg_execution_time,
                unit: "milliseconds".to_string(),
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("component".to_string(), "executor".to_string());
                    labels
                },
            },
        ];

        for metric in metrics {
            let _ = self.emit_metric(metric).await;
        }
    }

    async fn collect_connection_metrics(&self) {
        // Mock connection metrics
        let active_connections = rand::random::<u32>() % 1000;
        let requests_per_second = rand::random::<f64>() * 100.0;

        let metrics = vec![
            VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::ConnectionCount as i32,
                name: "active_connections".to_string(),
                value: active_connections as f64,
                unit: "count".to_string(),
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("component".to_string(), "grpc_server".to_string());
                    labels
                },
            },
            VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::RequestRate as i32,
                name: "requests_per_second".to_string(),
                value: requests_per_second,
                unit: "requests/second".to_string(),
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("component".to_string(), "grpc_server".to_string());
                    labels
                },
            },
        ];

        for metric in metrics {
            let _ = self.emit_metric(metric).await;
        }
    }
}

/// Streaming implementation for VM metrics
pub struct VmMetricsStream {
    receiver: broadcast::Receiver<VmMetric>,
    _interval: Duration,
}

impl VmMetricsStream {
    pub fn new(receiver: broadcast::Receiver<VmMetric>, interval: Duration) -> Self {
        Self {
            receiver,
            _interval: interval,
        }
    }
}

impl Stream for VmMetricsStream {
    type Item = Result<VmMetric, Status>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.receiver.try_recv() {
            Ok(metric) => {
                std::task::Poll::Ready(Some(Ok(metric)))
            }
            Err(broadcast::error::TryRecvError::Empty) => {
                // No metrics available, register waker and return pending
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            Err(broadcast::error::TryRecvError::Closed) => {
                // Channel closed, end stream
                std::task::Poll::Ready(None)
            }
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // We've missed some metrics, but continue
                debug!("Metrics stream lagged, some metrics may have been missed");
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        }
    }
}

/// Metrics aggregator for computing statistics
pub struct MetricsAggregator {
    window_size: Duration,
    metrics_history: Arc<RwLock<Vec<VmMetric>>>,
}

impl MetricsAggregator {
    pub fn new(window_size: Duration) -> Self {
        Self {
            window_size,
            metrics_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add metric to history
    pub async fn add_metric(&self, metric: VmMetric) {
        let mut history = self.metrics_history.write().await;
        history.push(metric);

        // Clean old metrics outside window
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_sub(self.window_size.as_secs());

        history.retain(|m| m.timestamp >= cutoff);
    }

    /// Get aggregated statistics for a metric type
    pub async fn get_stats(&self, metric_type: VmMetricType) -> MetricStats {
        let history = self.metrics_history.read().await;
        let filtered: Vec<&VmMetric> = history
            .iter()
            .filter(|m| VmMetricType::try_from(m.metric_type).unwrap_or(VmMetricType::Unknown) == metric_type)
            .collect();

        if filtered.is_empty() {
            return MetricStats::default();
        }

        let values: Vec<f64> = filtered.iter().map(|m| m.value).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len() as f64;
        let avg = sum / count;

        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted_values[0];
        let max = sorted_values[sorted_values.len() - 1];
        let median = if sorted_values.len() % 2 == 0 {
            (sorted_values[sorted_values.len() / 2 - 1] + sorted_values[sorted_values.len() / 2]) / 2.0
        } else {
            sorted_values[sorted_values.len() / 2]
        };

        MetricStats {
            count: count as u64,
            sum,
            avg,
            min,
            max,
            median,
        }
    }
}

/// Metric statistics
#[derive(Debug, Clone, Default)]
pub struct MetricStats {
    pub count: u64,
    pub sum: f64,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_vm_metrics_collector() {
        let collector = VmMetricsCollector::new(100, Duration::from_millis(100));
        
        // Start collection
        collector.start().await;
        
        // Subscribe to metrics
        let mut stream = collector.subscribe();
        
        // Wait for some metrics
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Should have received some metrics
        let metric = tokio::time::timeout(Duration::from_millis(500), stream.next()).await;
        assert!(metric.is_ok());
        
        collector.stop().await;
    }

    #[tokio::test]
    async fn test_metrics_aggregator() {
        let aggregator = MetricsAggregator::new(Duration::from_secs(60));
        
        // Add some test metrics
        for i in 0..10 {
            let metric = VmMetric {
                metric_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metric_type: VmMetricType::CpuUsage as i32,
                name: "cpu_usage".to_string(),
                value: (i * 10) as f64,
                unit: "percent".to_string(),
                labels: std::collections::HashMap::new(),
            };
            aggregator.add_metric(metric).await;
        }
        
        let stats = aggregator.get_stats(VmMetricType::CpuUsage).await;
        assert_eq!(stats.count, 10);
        assert_eq!(stats.avg, 45.0); // (0+10+20+...+90)/10
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 90.0);
    }
}
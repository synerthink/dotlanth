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

use dashmap::DashMap;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::RwLock, time::interval};

#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    pub timestamp: u64,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub metric_type: String,
    pub data_points: Vec<MetricDataPoint>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug)]
pub struct MetricCollector {
    metrics: Arc<DashMap<String, Vec<MetricDataPoint>>>,
    counters: Arc<DashMap<String, AtomicU64>>,
    gauges: Arc<DashMap<String, Arc<RwLock<f64>>>>,
    histograms: Arc<DashMap<String, Arc<RwLock<Vec<f64>>>>>,
    labels: Arc<DashMap<String, HashMap<String, String>>>,
    retention_duration: Duration,
}

impl MetricCollector {
    pub fn new(retention_duration: Duration) -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
            counters: Arc::new(DashMap::new()),
            gauges: Arc::new(DashMap::new()),
            histograms: Arc::new(DashMap::new()),
            labels: Arc::new(DashMap::new()),
            retention_duration,
        }
    }

    pub async fn record_counter(&self, name: &str, value: u64, labels: Option<HashMap<String, String>>) {
        let key = self.create_metric_key(name, &labels);

        // Update counter
        self.counters.entry(key.clone()).or_insert_with(|| AtomicU64::new(0)).fetch_add(value, Ordering::Relaxed);

        // Store labels if provided
        if let Some(labels) = labels {
            self.labels.insert(key.clone(), labels);
        }

        // Add data point
        let timestamp = current_timestamp();
        let current_value = self.counters.get(&key).unwrap().load(Ordering::Relaxed) as f64;

        self.add_data_point(&key, timestamp, current_value).await;
    }

    pub async fn set_gauge(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let key = self.create_metric_key(name, &labels);

        // Update gauge
        {
            let gauge = self.gauges.entry(key.clone()).or_insert_with(|| Arc::new(RwLock::new(0.0)));
            let mut gauge_value = gauge.write().await;
            *gauge_value = value;
        }

        // Store labels if provided
        if let Some(labels) = labels {
            self.labels.insert(key.clone(), labels);
        }

        // Add data point
        let timestamp = current_timestamp();
        self.add_data_point(&key, timestamp, value).await;
    }

    pub async fn record_histogram(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let key = self.create_metric_key(name, &labels);

        // Update histogram
        {
            let histogram = self.histograms.entry(key.clone()).or_insert_with(|| Arc::new(RwLock::new(Vec::new())));
            let mut histogram_values = histogram.write().await;
            histogram_values.push(value);

            // Keep only recent values to manage memory
            if histogram_values.len() > 10000 {
                histogram_values.drain(0..5000);
            }
        }

        // Store labels if provided
        if let Some(labels) = labels {
            self.labels.insert(key.clone(), labels);
        }

        // Add data point
        let timestamp = current_timestamp();
        self.add_data_point(&key, timestamp, value).await;
    }

    async fn add_data_point(&self, key: &str, timestamp: u64, value: f64) {
        let data_point = MetricDataPoint { timestamp, value };

        self.metrics.entry(key.to_string()).or_insert_with(Vec::new).push(data_point);

        // Cleanup old data points
        if let Some(mut data_points) = self.metrics.get_mut(key) {
            let cutoff_time = timestamp.saturating_sub(self.retention_duration.as_secs());
            data_points.retain(|dp| dp.timestamp >= cutoff_time);
        }
    }

    pub async fn get_metric(&self, name: &str, labels: Option<&HashMap<String, String>>) -> Option<Metric> {
        let key = if let Some(labels) = labels {
            self.create_metric_key(name, &Some(labels.clone()))
        } else {
            self.create_metric_key(name, &None)
        };

        if let Some(data_points) = self.metrics.get(&key) {
            let metric_labels = self.labels.get(&key).map(|entry| entry.value().clone()).unwrap_or_default();

            Some(Metric {
                name: name.to_string(),
                metric_type: self.infer_metric_type(&key),
                data_points: data_points.value().clone(),
                labels: metric_labels,
            })
        } else {
            None
        }
    }

    pub async fn get_metrics_by_pattern(&self, pattern: &str) -> Vec<Metric> {
        let mut results = Vec::new();

        for entry in self.metrics.iter() {
            let key = entry.key();
            if key.contains(pattern) {
                if let Some(name) = self.extract_metric_name(key) {
                    let metric_labels = self.labels.get(key).map(|entry| entry.value().clone()).unwrap_or_default();

                    results.push(Metric {
                        name,
                        metric_type: self.infer_metric_type(key),
                        data_points: entry.value().clone(),
                        labels: metric_labels,
                    });
                }
            }
        }

        results
    }

    pub async fn get_all_metrics(&self) -> Vec<Metric> {
        let mut results = Vec::new();

        for entry in self.metrics.iter() {
            let key = entry.key();
            if let Some(name) = self.extract_metric_name(key) {
                let metric_labels = self.labels.get(key).map(|entry| entry.value().clone()).unwrap_or_default();

                results.push(Metric {
                    name,
                    metric_type: self.infer_metric_type(key),
                    data_points: entry.value().clone(),
                    labels: metric_labels,
                });
            }
        }

        results
    }

    pub async fn get_histogram_stats(&self, name: &str, labels: Option<&HashMap<String, String>>) -> Option<HistogramStats> {
        let key = if let Some(labels) = labels {
            self.create_metric_key(name, &Some(labels.clone()))
        } else {
            self.create_metric_key(name, &None)
        };

        if let Some(histogram) = self.histograms.get(&key) {
            let values = histogram.read().await;
            if values.is_empty() {
                return None;
            }

            let mut sorted_values = values.clone();
            sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let count = sorted_values.len();
            let sum = sorted_values.iter().sum::<f64>();
            let mean = sum / count as f64;

            let p50 = percentile(&sorted_values, 0.5);
            let p90 = percentile(&sorted_values, 0.9);
            let p95 = percentile(&sorted_values, 0.95);
            let p99 = percentile(&sorted_values, 0.99);

            Some(HistogramStats {
                count,
                sum,
                mean,
                min: sorted_values[0],
                max: sorted_values[count - 1],
                p50,
                p90,
                p95,
                p99,
            })
        } else {
            None
        }
    }

    fn create_metric_key(&self, name: &str, labels: &Option<HashMap<String, String>>) -> String {
        if let Some(labels) = labels {
            if labels.is_empty() {
                name.to_string()
            } else {
                let mut label_pairs: Vec<_> = labels.iter().collect();
                label_pairs.sort_by_key(|(k, _)| *k);
                let label_string = label_pairs.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(",");
                format!("{}:{}", name, label_string)
            }
        } else {
            name.to_string()
        }
    }

    fn extract_metric_name(&self, key: &str) -> Option<String> {
        if let Some(colon_pos) = key.find(':') {
            Some(key[..colon_pos].to_string())
        } else {
            Some(key.to_string())
        }
    }

    fn infer_metric_type(&self, key: &str) -> String {
        if self.counters.contains_key(key) {
            "counter".to_string()
        } else if self.gauges.contains_key(key) {
            "gauge".to_string()
        } else if self.histograms.contains_key(key) {
            "histogram".to_string()
        } else {
            "unknown".to_string()
        }
    }

    pub async fn cleanup_old_metrics(&self) {
        let cutoff_time = current_timestamp().saturating_sub(self.retention_duration.as_secs());

        // Clean up metrics
        self.metrics.retain(|_, data_points| {
            data_points.retain(|dp| dp.timestamp >= cutoff_time);
            !data_points.is_empty()
        });

        // Clean up labels for metrics that no longer exist
        let existing_keys: std::collections::HashSet<_> = self.metrics.iter().map(|entry| entry.key().clone()).collect();

        self.labels.retain(|key, _| existing_keys.contains(key));
        self.counters.retain(|key, _| existing_keys.contains(key));
        self.gauges.retain(|key, _| existing_keys.contains(key));
        self.histograms.retain(|key, _| existing_keys.contains(key));
    }

    pub async fn start_cleanup_task(self: Arc<Self>) {
        let collector_clone = Arc::clone(&self);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Cleanup every 5 minutes

            loop {
                interval.tick().await;
                collector_clone.cleanup_old_metrics().await;
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct HistogramStats {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let index = (p * (sorted_values.len() - 1) as f64).round() as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[derive(Debug)]
pub struct ClusterMetrics {
    collector: Arc<MetricCollector>,
}

impl ClusterMetrics {
    pub fn new() -> Self {
        Self {
            collector: Arc::new(MetricCollector::new(Duration::from_secs(3600))), // 1 hour retention
        }
    }

    pub async fn record_request(&self, service_name: &str, success: bool, response_time_ms: f64) {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), service_name.to_string());
        labels.insert("status".to_string(), if success { "success" } else { "error" }.to_string());

        // Record request count
        self.collector.record_counter("requests_total", 1, Some(labels.clone())).await;

        // Record response time
        self.collector.record_histogram("response_time_ms", response_time_ms, Some(labels.clone())).await;

        // Record error rate
        if !success {
            self.collector.record_counter("errors_total", 1, Some(labels)).await;
        }
    }

    pub async fn record_connection(&self, service_name: &str, active_connections: u32) {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), service_name.to_string());

        self.collector.set_gauge("active_connections", active_connections as f64, Some(labels)).await;
    }

    pub async fn record_load_balancing(&self, algorithm: &str, backend_id: &str, selection_time_ms: f64) {
        let mut labels = HashMap::new();
        labels.insert("algorithm".to_string(), algorithm.to_string());
        labels.insert("backend".to_string(), backend_id.to_string());

        self.collector.record_counter("load_balancer_selections", 1, Some(labels.clone())).await;
        self.collector.record_histogram("selection_time_ms", selection_time_ms, Some(labels)).await;
    }

    pub async fn record_circuit_breaker(&self, service_name: &str, state: &str) {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), service_name.to_string());
        labels.insert("state".to_string(), state.to_string());

        self.collector.record_counter("circuit_breaker_state_changes", 1, Some(labels)).await;
    }

    pub async fn record_service_discovery(&self, action: &str, service_count: usize) {
        let mut labels = HashMap::new();
        labels.insert("action".to_string(), action.to_string());

        self.collector.record_counter("service_discovery_actions", 1, Some(labels.clone())).await;
        self.collector.set_gauge("registered_services", service_count as f64, None).await;
    }

    pub async fn get_metrics(&self, names: Option<Vec<String>>) -> Vec<Metric> {
        if let Some(names) = names {
            let mut results = Vec::new();
            for name in names {
                if let Some(metric) = self.collector.get_metric(&name, None).await {
                    results.push(metric);
                }
            }
            results
        } else {
            self.collector.get_all_metrics().await
        }
    }

    pub async fn get_service_metrics(&self, service_name: &str) -> Vec<Metric> {
        self.collector.get_metrics_by_pattern(service_name).await
    }

    pub async fn get_response_time_stats(&self, service_name: &str) -> Option<HistogramStats> {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), service_name.to_string());

        self.collector.get_histogram_stats("response_time_ms", Some(&labels)).await
    }

    pub fn get_collector(&self) -> Arc<MetricCollector> {
        Arc::clone(&self.collector)
    }

    pub async fn start_background_tasks(&self) {
        Arc::clone(&self.collector).start_cleanup_task().await;
    }
}

impl Default for ClusterMetrics {
    fn default() -> Self {
        Self::new()
    }
}

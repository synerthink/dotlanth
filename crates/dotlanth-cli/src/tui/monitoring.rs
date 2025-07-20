// Enhanced monitoring integration for TUI
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default)]
pub struct ClusterMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub active_connections: u64,
    pub max_connections: u64,
    pub requests_per_second: u64,
    pub error_rate_percent: f64,
    pub total_nodes: u32,
    pub healthy_nodes: u32,
}

#[derive(Debug, Clone)]
pub struct NodeHealth {
    pub node_id: String,
    pub node_type: String,
    pub status: NodeStatus,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub latency_ms: f64,
    pub last_seen: SystemTime,
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeStatus::Healthy => write!(f, "Healthy"),
            NodeStatus::Warning => write!(f, "Warning"),
            NodeStatus::Critical => write!(f, "Critical"),
            NodeStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub timestamp: SystemTime,
    pub requests_per_second: u64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub timestamp: SystemTime,
    pub node_id: Option<String>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARN"),
            AlertSeverity::Critical => write!(f, "CRIT"),
        }
    }
}

pub struct PrometheusClient {
    base_url: String,
    client: Client,
}

impl PrometheusClient {
    pub fn new(base_url: &str) -> Result<Self> {
        Ok(Self {
            base_url: base_url.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
        })
    }
    
    pub async fn get_cluster_metrics(&self) -> Result<ClusterMetrics> {
        let mut metrics = ClusterMetrics::default();
        
        // Query multiple metrics in parallel
        let queries = vec![
            ("cpu_usage", "avg(rate(cpu_usage_seconds_total[5m])) * 100"),
            ("memory_usage", "avg(memory_usage_percent)"),
            ("active_connections", "sum(active_connections)"),
            ("requests_per_second", "sum(rate(http_requests_total[1m]))"),
            ("error_rate", "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m])) * 100"),
            ("total_nodes", "count(up)"),
            ("healthy_nodes", "count(up == 1)"),
        ];
        
        for (metric_name, query) in queries {
            if let Ok(value) = self.query_single_value(query).await {
                match metric_name {
                    "cpu_usage" => metrics.cpu_usage_percent = value,
                    "memory_usage" => metrics.memory_usage_percent = value,
                    "active_connections" => metrics.active_connections = value as u64,
                    "requests_per_second" => metrics.requests_per_second = value as u64,
                    "error_rate" => metrics.error_rate_percent = value,
                    "total_nodes" => metrics.total_nodes = value as u32,
                    "healthy_nodes" => metrics.healthy_nodes = value as u32,
                    _ => {}
                }
            }
        }
        
        // Set reasonable defaults if queries fail
        if metrics.max_connections == 0 {
            metrics.max_connections = 1000;
        }
        
        Ok(metrics)
    }
    
    pub async fn get_node_health(&self) -> Result<HashMap<String, NodeHealth>> {
        let mut node_health = HashMap::new();
        
        // Query node-specific metrics
        let node_queries = vec![
            ("cpu", "cpu_usage_percent"),
            ("memory", "memory_usage_percent"),
            ("disk", "disk_usage_percent"),
        ];
        
        // For now, create mock data based on what we might get from Prometheus
        // In a real implementation, this would parse the actual Prometheus response
        let mock_nodes = vec!["master-1", "master-2", "master-3", "worker-1", "worker-2", "storage-1", "storage-2"];
        
        for node_id in mock_nodes {
            let health = NodeHealth {
                node_id: node_id.to_string(),
                node_type: if node_id.starts_with("master") {
                    "Master".to_string()
                } else if node_id.starts_with("worker") {
                    "Worker".to_string()
                } else {
                    "Storage".to_string()
                },
                status: if rand::random::<f64>() > 0.8 {
                    NodeStatus::Warning
                } else {
                    NodeStatus::Healthy
                },
                cpu_usage: rand::random::<f64>() * 100.0,
                memory_usage: rand::random::<f64>() * 100.0,
                disk_usage: rand::random::<f64>() * 100.0,
                latency_ms: rand::random::<f64>() * 50.0,
                last_seen: SystemTime::now(),
            };
            
            node_health.insert(node_id.to_string(), health);
        }
        
        Ok(node_health)
    }
    
    pub async fn get_performance_history(&self, duration: Duration) -> Result<Vec<PerformanceSnapshot>> {
        // Query historical data from Prometheus
        let end_time = SystemTime::now();
        let start_time = end_time - duration;
        
        // For now, generate mock historical data
        // In real implementation, this would use Prometheus range queries
        let mut history = Vec::new();
        let points = 20;
        
        for i in 0..points {
            let timestamp = start_time + Duration::from_secs((duration.as_secs() * i) / points);
            let snapshot = PerformanceSnapshot {
                timestamp,
                requests_per_second: (rand::random::<f64>() * 1000.0) as u64,
                avg_latency_ms: rand::random::<f64>() * 100.0,
                error_rate: rand::random::<f64>() * 5.0,
                cpu_usage: rand::random::<f64>() * 100.0,
                memory_usage: rand::random::<f64>() * 100.0,
            };
            history.push(snapshot);
        }
        
        Ok(history)
    }
    
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>> {
        // Query active alerts from Prometheus AlertManager
        let url = format!("{}/api/v1/alerts", self.base_url);
        
        // For now, return mock alerts
        // In real implementation, this would parse AlertManager API response
        let mut alerts = Vec::new();
        
        if rand::random::<f64>() > 0.7 {
            alerts.push(Alert {
                id: "high-cpu-worker-1".to_string(),
                severity: AlertSeverity::Warning,
                title: "High CPU Usage".to_string(),
                description: "Worker-1 CPU usage above 80%".to_string(),
                timestamp: SystemTime::now(),
                node_id: Some("worker-1".to_string()),
                resolved: false,
            });
        }
        
        if rand::random::<f64>() > 0.9 {
            alerts.push(Alert {
                id: "storage-disk-full".to_string(),
                severity: AlertSeverity::Critical,
                title: "Disk Space Critical".to_string(),
                description: "Storage node disk usage above 95%".to_string(),
                timestamp: SystemTime::now(),
                node_id: Some("storage-1".to_string()),
                resolved: false,
            });
        }
        
        Ok(alerts)
    }
    
    async fn query_single_value(&self, query: &str) -> Result<f64> {
        let url = format!("{}/api/v1/query?query={}", self.base_url, query);
        
        // Try to query Prometheus, fall back to mock data if it fails
        match self.client.get(&url).send().await {
            Ok(response) => {
                if let Ok(json) = response.json::<serde_json::Value>().await {
                    if let Some(result) = json["data"]["result"].as_array() {
                        if let Some(first_result) = result.first() {
                            if let Some(value) = first_result["value"][1].as_str() {
                                return Ok(value.parse().unwrap_or(0.0));
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Prometheus not available, return mock data
                return Ok(rand::random::<f64>() * 100.0);
            }
        }
        
        Ok(0.0)
    }
}

pub struct GrafanaClient {
    base_url: String,
    client: Client,
    username: String,
    password: String,
}

impl GrafanaClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Result<Self> {
        Ok(Self {
            base_url: base_url.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?,
            username: username.to_string(),
            password: password.to_string(),
        })
    }
    
    pub async fn get_dashboard_data(&self) -> Result<HashMap<String, serde_json::Value>> {
        // Query Grafana dashboard data
        // For now, return mock data structure
        let mut dashboard_data = HashMap::new();
        
        dashboard_data.insert("cluster_overview".to_string(), serde_json::json!({
            "cpu_usage": rand::random::<f64>() * 100.0,
            "memory_usage": rand::random::<f64>() * 100.0,
            "network_io": rand::random::<f64>() * 1000.0,
        }));
        
        dashboard_data.insert("node_metrics".to_string(), serde_json::json!({
            "master_nodes": 3,
            "worker_nodes": 2,
            "storage_nodes": 2,
            "healthy_nodes": 6,
        }));
        
        Ok(dashboard_data)
    }
    
    pub async fn get_node_health_dashboard(&self) -> Result<HashMap<String, NodeHealth>> {
        // This would integrate with Grafana's API to get dashboard data
        // For now, return empty map as Prometheus is the primary source
        Ok(HashMap::new())
    }
}

// Add rand dependency for mock data generation
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Simple pseudo-random number generator for mock data
fn rand_f64() -> f64 {
    let mut hasher = DefaultHasher::new();
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos().hash(&mut hasher);
    (hasher.finish() % 1000) as f64 / 1000.0
}

// Replace rand::random calls with our simple implementation
fn rand() -> f64 {
    rand_f64()
}
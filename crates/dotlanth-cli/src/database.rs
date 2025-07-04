use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub address: String,
    pub status: NodeStatus,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub capabilities: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Offline,
    Maintenance,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub id: String,
    pub dot_name: String,
    pub dot_version: String,
    pub node_id: String,
    pub status: DeploymentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    Running,
    Stopped,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub node_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricEntry {
    pub id: String,
    pub node_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_in: u64,
    pub network_out: u64,
}

pub struct DotLanthDatabase {
    nodes: Arc<Mutex<HashMap<String, NodeInfo>>>,
    deployments: Arc<Mutex<HashMap<String, DeploymentInfo>>>,
    metrics: Arc<Mutex<Vec<MetricEntry>>>,
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl DotLanthDatabase {
    pub fn new(_storage_path: impl AsRef<std::path::Path>) -> Result<Self> {
        let db = Self {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            deployments: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(Vec::new())),
            logs: Arc::new(Mutex::new(Vec::new())),
        };
        println!("Mock Database initialized (placeholder for dotdb integration)");
        Ok(db)
    }

    pub fn register_node(&self, node: NodeInfo) -> Result<()> {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.insert(node.id.clone(), node);
        Ok(())
    }

    pub fn get_node(&self, node_id: &str) -> Result<Option<NodeInfo>> {
        let nodes = self.nodes.lock().unwrap();
        Ok(nodes.get(node_id).cloned())
    }

    pub fn list_nodes(&self) -> Result<Vec<NodeInfo>> {
        let nodes = self.nodes.lock().unwrap();
        Ok(nodes.values().cloned().collect())
    }

    pub fn remove_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.remove(node_id);
        Ok(())
    }

    pub fn create_deployment(&self, deployment: DeploymentInfo) -> Result<()> {
        let mut deployments = self.deployments.lock().unwrap();
        deployments.insert(deployment.id.clone(), deployment);
        Ok(())
    }

    pub fn get_deployment(&self, deployment_id: &str) -> Result<Option<DeploymentInfo>> {
        let deployments = self.deployments.lock().unwrap();
        Ok(deployments.get(deployment_id).cloned())
    }

    pub fn list_deployments(&self) -> Result<Vec<DeploymentInfo>> {
        let deployments = self.deployments.lock().unwrap();
        Ok(deployments.values().cloned().collect())
    }

    pub fn update_deployment_status(&self, deployment_id: &str, status: DeploymentStatus) -> Result<()> {
        let mut deployments = self.deployments.lock().unwrap();
        if let Some(deployment) = deployments.get_mut(deployment_id) {
            deployment.status = status;
            deployment.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    pub fn store_metrics(&self, metric: MetricEntry) -> Result<()> {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.push(metric);
        let excess = metrics.len().saturating_sub(1000);
        if excess > 0 {
            metrics.drain(0..excess);
        }
        Ok(())
    }

    pub fn get_recent_metrics(&self, node_id: Option<&str>, limit: usize) -> Result<Vec<MetricEntry>> {
        let metrics = self.metrics.lock().unwrap();
        let filtered: Vec<MetricEntry> = if let Some(node_id) = node_id {
            metrics.iter().filter(|m| m.node_id == node_id).cloned().collect()
        } else {
            metrics.clone()
        };
        Ok(filtered.into_iter().rev().take(limit).collect())
    }

    pub fn store_log(&self, log: LogEntry) -> Result<()> {
        let mut logs = self.logs.lock().unwrap();
        logs.push(log);
        let excess = logs.len().saturating_sub(1000);
        if excess > 0 {
            logs.drain(0..excess);
        }
        Ok(())
    }

    pub fn get_recent_logs(&self, node_id: Option<&str>, limit: usize) -> Result<Vec<LogEntry>> {
        let logs = self.logs.lock().unwrap();
        let filtered: Vec<LogEntry> = if let Some(node_id) = node_id {
            logs.iter().filter(|l| l.node_id == node_id).cloned().collect()
        } else {
            logs.clone()
        };
        Ok(filtered.into_iter().rev().take(limit).collect())
    }

    pub fn generate_sample_data(&self) -> Result<()> {
        self.generate_sample_nodes()?;
        self.generate_sample_deployments()?;
        self.generate_sample_metrics()?;
        self.generate_sample_logs()?;
        Ok(())
    }

    fn generate_sample_nodes(&self) -> Result<()> {
        Ok(())
    }

    fn generate_sample_deployments(&self) -> Result<()> {
        Ok(())
    }

    fn generate_sample_metrics(&self) -> Result<()> {
        Ok(())
    }

    fn generate_sample_logs(&self) -> Result<()> {
        Ok(())
    }
}

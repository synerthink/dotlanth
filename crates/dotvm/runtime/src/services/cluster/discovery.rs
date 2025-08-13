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
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::RwLock, time::interval};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub version: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
    pub last_heartbeat: Instant,
    pub registered_at: SystemTime,
    pub capabilities: ServiceCapabilities,
    pub load_metrics: LoadMetrics,
}

#[derive(Debug, Clone)]
pub struct ServiceCapabilities {
    pub max_connections: u32,
    pub supported_protocols: Vec<String>,
    pub features: Vec<String>,
    pub security_level: String,
}

#[derive(Debug, Clone)]
pub struct LoadMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_connections: u32,
    pub request_rate: f64,
    pub response_time_avg: f64,
    pub error_rate: f64,
}

impl Default for LoadMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_connections: 0,
            request_rate: 0.0,
            response_time_avg: 0.0,
            error_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unhealthy,
    Unknown,
}

impl From<i32> for HealthStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => HealthStatus::Healthy,
            2 => HealthStatus::Warning,
            3 => HealthStatus::Critical,
            4 => HealthStatus::Unhealthy,
            _ => HealthStatus::Unknown,
        }
    }
}

impl From<HealthStatus> for i32 {
    fn from(status: HealthStatus) -> Self {
        match status {
            HealthStatus::Unknown => 0,
            HealthStatus::Healthy => 1,
            HealthStatus::Warning => 2,
            HealthStatus::Critical => 3,
            HealthStatus::Unhealthy => 4,
        }
    }
}

#[derive(Debug)]
pub struct ServiceRegistry {
    services: Arc<DashMap<String, ServiceInstance>>,
    service_dependencies: Arc<DashMap<String, Vec<String>>>,
    heartbeat_interval: Duration,
    health_check_timeout: Duration,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(DashMap::new()),
            service_dependencies: Arc::new(DashMap::new()),
            heartbeat_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(120),
        }
    }

    pub async fn register_service(&self, mut instance: ServiceInstance) -> Result<String, String> {
        // Generate unique ID if not provided
        if instance.id.is_empty() {
            instance.id = Uuid::new_v4().to_string();
        }

        // Set registration time
        instance.registered_at = SystemTime::now();
        instance.last_heartbeat = Instant::now();

        // Validate instance
        self.validate_service_instance(&instance)?;

        // Store the service
        self.services.insert(instance.id.clone(), instance.clone());

        tracing::info!(
            service_id = %instance.id,
            service_name = %instance.name,
            address = %instance.address,
            port = instance.port,
            "Service registered successfully"
        );

        Ok(instance.id)
    }

    pub async fn unregister_service(&self, service_id: &str) -> Result<(), String> {
        match self.services.remove(service_id) {
            Some((_, instance)) => {
                tracing::info!(
                    service_id = %service_id,
                    service_name = %instance.name,
                    "Service unregistered successfully"
                );
                Ok(())
            }
            None => Err(format!("Service with ID {} not found", service_id)),
        }
    }

    pub async fn update_service_health(&self, service_id: &str, health_status: HealthStatus) -> Result<(), String> {
        match self.services.get_mut(service_id) {
            Some(mut service) => {
                service.health_status = health_status.clone();
                service.last_heartbeat = Instant::now();

                tracing::debug!(
                    service_id = %service_id,
                    health_status = ?health_status,
                    "Service health updated"
                );

                Ok(())
            }
            None => Err(format!("Service with ID {} not found", service_id)),
        }
    }

    pub async fn update_service_metrics(&self, service_id: &str, metrics: LoadMetrics) -> Result<(), String> {
        match self.services.get_mut(service_id) {
            Some(mut service) => {
                service.load_metrics = metrics;
                service.last_heartbeat = Instant::now();
                Ok(())
            }
            None => Err(format!("Service with ID {} not found", service_id)),
        }
    }

    pub async fn get_service(&self, service_id: &str) -> Option<ServiceInstance> {
        self.services.get(service_id).map(|entry| entry.value().clone())
    }

    pub async fn get_services_by_name(&self, service_name: &str) -> Vec<ServiceInstance> {
        self.services.iter().filter(|entry| entry.value().name == service_name).map(|entry| entry.value().clone()).collect()
    }

    pub async fn get_healthy_services(&self, service_name: &str) -> Vec<ServiceInstance> {
        self.services
            .iter()
            .filter(|entry| {
                let service = entry.value();
                service.name == service_name && service.health_status == HealthStatus::Healthy && service.last_heartbeat.elapsed() < self.health_check_timeout
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub async fn get_all_services(&self) -> Vec<ServiceInstance> {
        self.services.iter().map(|entry| entry.value().clone()).collect()
    }

    pub async fn add_service_dependency(&self, service_id: &str, dependency_id: &str) -> Result<(), String> {
        // Validate that both services exist
        if !self.services.contains_key(service_id) {
            return Err(format!("Service with ID {} not found", service_id));
        }
        if !self.services.contains_key(dependency_id) {
            return Err(format!("Dependency service with ID {} not found", dependency_id));
        }

        // Check for circular dependencies
        if self.would_create_circular_dependency(service_id, dependency_id).await {
            return Err("Adding this dependency would create a circular dependency".to_string());
        }

        // Add the dependency
        self.service_dependencies.entry(service_id.to_string()).or_insert_with(Vec::new).push(dependency_id.to_string());

        tracing::info!(
            service_id = %service_id,
            dependency_id = %dependency_id,
            "Service dependency added"
        );

        Ok(())
    }

    pub async fn get_service_dependencies(&self, service_id: &str) -> Vec<String> {
        self.service_dependencies.get(service_id).map(|deps| deps.value().clone()).unwrap_or_default()
    }

    async fn would_create_circular_dependency(&self, service_id: &str, dependency_id: &str) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.has_path_to(dependency_id, service_id, &mut visited)
    }

    fn has_path_to(&self, from: &str, to: &str, visited: &mut std::collections::HashSet<String>) -> bool {
        if from == to {
            return true;
        }

        if visited.contains(from) {
            return false;
        }

        visited.insert(from.to_string());

        if let Some(deps) = self.service_dependencies.get(from) {
            for dep in deps.value() {
                if self.has_path_to(dep, to, visited) {
                    return true;
                }
            }
        }

        false
    }

    pub async fn start_health_monitoring(&self) {
        let services = Arc::clone(&self.services);
        let timeout = self.health_check_timeout;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let mut unhealthy_services = Vec::new();

                for entry in services.iter() {
                    let service = entry.value();
                    if service.last_heartbeat.elapsed() > timeout {
                        unhealthy_services.push(service.id.clone());
                    }
                }

                for service_id in unhealthy_services {
                    if let Some(mut service) = services.get_mut(&service_id) {
                        if service.health_status != HealthStatus::Unhealthy {
                            service.health_status = HealthStatus::Unhealthy;
                            tracing::warn!(
                                service_id = %service_id,
                                service_name = %service.name,
                                "Service marked as unhealthy due to missed heartbeats"
                            );
                        }
                    }
                }
            }
        });
    }

    fn validate_service_instance(&self, instance: &ServiceInstance) -> Result<(), String> {
        if instance.name.is_empty() {
            return Err("Service name cannot be empty".to_string());
        }

        if instance.address.is_empty() {
            return Err("Service address cannot be empty".to_string());
        }

        if instance.port == 0 {
            return Err("Service port must be greater than 0".to_string());
        }

        Ok(())
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ServiceDiscovery {
    registry: Arc<ServiceRegistry>,
    watchers: Arc<RwLock<HashMap<String, Vec<tokio::sync::broadcast::Sender<ServiceEvent>>>>>,
}

#[derive(Debug, Clone)]
pub enum ServiceEvent {
    ServiceRegistered(ServiceInstance),
    ServiceUnregistered(String),
    ServiceHealthChanged { service_id: String, health: HealthStatus },
    ServiceMetricsUpdated { service_id: String, metrics: LoadMetrics },
}

impl ServiceDiscovery {
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        Self {
            registry,
            watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn discover_services(&self, service_name: &str) -> Result<Vec<ServiceInstance>, String> {
        Ok(self.registry.get_healthy_services(service_name).await)
    }

    pub async fn watch_service_changes(&self, service_name: &str) -> tokio::sync::broadcast::Receiver<ServiceEvent> {
        let (tx, rx) = tokio::sync::broadcast::channel(100);

        {
            let mut watchers = self.watchers.write().await;
            watchers.entry(service_name.to_string()).or_insert_with(Vec::new).push(tx);
        }

        rx
    }

    pub async fn notify_service_event(&self, event: ServiceEvent) {
        let watchers = self.watchers.read().await;

        let service_name = match &event {
            ServiceEvent::ServiceRegistered(instance) => instance.name.clone(),
            ServiceEvent::ServiceUnregistered(id) => {
                if let Some(instance) = self.registry.get_service(id).await {
                    instance.name.clone()
                } else {
                    return;
                }
            }
            ServiceEvent::ServiceHealthChanged { service_id, .. } | ServiceEvent::ServiceMetricsUpdated { service_id, .. } => {
                if let Some(instance) = self.registry.get_service(service_id).await {
                    instance.name.clone()
                } else {
                    return;
                }
            }
        };

        if let Some(service_watchers) = watchers.get(&service_name) {
            for sender in service_watchers {
                let _ = sender.send(event.clone());
            }
        }
    }

    pub async fn auto_register_service(&self, instance: ServiceInstance) -> Result<String, String> {
        let service_id = self.registry.register_service(instance.clone()).await?;

        self.notify_service_event(ServiceEvent::ServiceRegistered(instance)).await;

        Ok(service_id)
    }

    pub async fn auto_unregister_service(&self, service_id: &str) -> Result<(), String> {
        self.registry.unregister_service(service_id).await?;

        self.notify_service_event(ServiceEvent::ServiceUnregistered(service_id.to_string())).await;

        Ok(())
    }

    pub fn get_registry(&self) -> Arc<ServiceRegistry> {
        Arc::clone(&self.registry)
    }
}

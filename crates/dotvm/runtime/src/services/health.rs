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

//! gRPC Health Checking Service Implementation
//! 
//! Implements the standard gRPC Health Checking Protocol for service discovery
//! and load balancer integration.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};

/// Health check status for services
#[derive(Debug, Clone, PartialEq)]
pub enum ServingStatus {
    Unknown,
    Serving,
    NotServing,
    ServiceUnknown,
}

impl From<ServingStatus> for i32 {
    fn from(status: ServingStatus) -> i32 {
        match status {
            ServingStatus::Unknown => 0,
            ServingStatus::Serving => 1,
            ServingStatus::NotServing => 2,
            ServingStatus::ServiceUnknown => 3,
        }
    }
}

/// Health check response
#[derive(Debug, Clone)]
pub struct HealthCheckResponse {
    pub status: ServingStatus,
}

/// Health check request
#[derive(Debug, Clone)]
pub struct HealthCheckRequest {
    pub service: String,
}

/// Health service implementation
#[derive(Debug)]
pub struct HealthService {
    /// Service status map
    service_status: Arc<RwLock<HashMap<String, ServingStatus>>>,
    /// Global health status
    global_status: Arc<RwLock<ServingStatus>>,
}

impl HealthService {
    /// Create a new health service
    pub fn new() -> Self {
        let mut initial_services = HashMap::new();
        
        // Register core services
        initial_services.insert("".to_string(), ServingStatus::Serving); // Overall health
        initial_services.insert("vm_service.VmService".to_string(), ServingStatus::Serving);
        initial_services.insert("runtime.Runtime".to_string(), ServingStatus::Serving);
        
        Self {
            service_status: Arc::new(RwLock::new(initial_services)),
            global_status: Arc::new(RwLock::new(ServingStatus::Serving)),
        }
    }

    /// Set the status of a specific service
    pub async fn set_service_status(&self, service: String, status: ServingStatus) {
        let mut services = self.service_status.write().await;
        services.insert(service.clone(), status.clone());
        
        info!("Health status updated: {} -> {:?}", service, status);
        
        // Update global status based on all services
        self.update_global_status(&services).await;
    }

    /// Get the status of a specific service
    pub async fn get_service_status(&self, service: &str) -> ServingStatus {
        let services = self.service_status.read().await;
        
        if service.is_empty() {
            // Return global status for empty service name
            let global_status = self.global_status.read().await;
            return global_status.clone();
        }
        
        services.get(service)
            .cloned()
            .unwrap_or(ServingStatus::ServiceUnknown)
    }

    /// Update global status based on individual service statuses
    async fn update_global_status(&self, services: &HashMap<String, ServingStatus>) {
        let mut global_status = self.global_status.write().await;
        
        // Global status is serving only if all services are serving
        let all_serving = services.values()
            .filter(|status| **status != ServingStatus::ServiceUnknown)
            .all(|status| *status == ServingStatus::Serving);
            
        *global_status = if all_serving {
            ServingStatus::Serving
        } else {
            ServingStatus::NotServing
        };
    }

    /// Perform health check
    pub async fn check(&self, request: HealthCheckRequest) -> Result<HealthCheckResponse, Status> {
        debug!("Health check requested for service: {}", request.service);
        
        let status = self.get_service_status(&request.service).await;
        
        match status {
            ServingStatus::ServiceUnknown => {
                warn!("Health check for unknown service: {}", request.service);
                Err(Status::not_found(format!("Service '{}' not found", request.service)))
            }
            _ => {
                debug!("Health check result for {}: {:?}", request.service, status);
                Ok(HealthCheckResponse { status })
            }
        }
    }

    /// Register a new service for health checking
    pub async fn register_service(&self, service: String, initial_status: ServingStatus) {
        let mut services = self.service_status.write().await;
        services.insert(service.clone(), initial_status.clone());
        
        info!("Registered service for health checking: {} -> {:?}", service, initial_status);
        
        self.update_global_status(&services).await;
    }

    /// Unregister a service from health checking
    pub async fn unregister_service(&self, service: &str) {
        let mut services = self.service_status.write().await;
        if services.remove(service).is_some() {
            info!("Unregistered service from health checking: {}", service);
            self.update_global_status(&services).await;
        }
    }

    /// Get all registered services and their statuses
    pub async fn get_all_services(&self) -> HashMap<String, ServingStatus> {
        self.service_status.read().await.clone()
    }

    /// Set the service to not serving (for graceful shutdown)
    pub async fn shutdown(&self) {
        let mut services = self.service_status.write().await;
        for status in services.values_mut() {
            *status = ServingStatus::NotServing;
        }
        
        *self.global_status.write().await = ServingStatus::NotServing;
        
        info!("Health service marked all services as not serving for shutdown");
    }
}

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check utilities
pub mod utils {
    use super::*;
    use std::time::Duration;
    use tokio::time::interval;

    /// Periodic health checker that monitors service health
    pub struct PeriodicHealthChecker {
        health_service: Arc<HealthService>,
        check_interval: Duration,
    }

    impl PeriodicHealthChecker {
        pub fn new(health_service: Arc<HealthService>, check_interval: Duration) -> Self {
            Self {
                health_service,
                check_interval,
            }
        }

        /// Start periodic health checking
        pub async fn start(&self) {
            let mut interval = interval(self.check_interval);
            
            loop {
                interval.tick().await;
                self.perform_health_checks().await;
            }
        }

        /// Perform health checks on all registered services
        async fn perform_health_checks(&self) {
            let services = self.health_service.get_all_services().await;
            
            for (service_name, _) in services {
                if service_name.is_empty() {
                    continue; // Skip global service
                }
                
                // Here you would implement actual health checks
                // For now, we'll just log that we're checking
                debug!("Performing health check for service: {}", service_name);
                
                // Example: Check if service is responsive
                // let is_healthy = self.check_service_health(&service_name).await;
                // let new_status = if is_healthy { 
                //     ServingStatus::Serving 
                // } else { 
                //     ServingStatus::NotServing 
                // };
                // 
                // self.health_service.set_service_status(service_name, new_status).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_service_basic() {
        let health_service = HealthService::new();
        
        // Test global health
        let status = health_service.get_service_status("").await;
        assert_eq!(status, ServingStatus::Serving);
        
        // Test specific service
        let status = health_service.get_service_status("vm_service.VmService").await;
        assert_eq!(status, ServingStatus::Serving);
        
        // Test unknown service
        let status = health_service.get_service_status("unknown.Service").await;
        assert_eq!(status, ServingStatus::ServiceUnknown);
    }

    #[tokio::test]
    async fn test_service_registration() {
        let health_service = HealthService::new();
        
        // Register new service
        health_service.register_service(
            "test.Service".to_string(), 
            ServingStatus::Serving
        ).await;
        
        let status = health_service.get_service_status("test.Service").await;
        assert_eq!(status, ServingStatus::Serving);
        
        // Update service status
        health_service.set_service_status(
            "test.Service".to_string(), 
            ServingStatus::NotServing
        ).await;
        
        let status = health_service.get_service_status("test.Service").await;
        assert_eq!(status, ServingStatus::NotServing);
    }
}
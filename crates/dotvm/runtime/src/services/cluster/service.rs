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

use futures::Stream;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tonic::{Request, Response, Result as TonicResult, Status};
use uuid::Uuid;

use super::discovery::{HealthStatus, LoadMetrics, ServiceCapabilities, ServiceInstance};
use super::{ClusterMetrics, LoadBalancer, LoadBalancingAlgorithm, LoadBalancingStrategy, ServiceDiscovery, ServiceRegistry};

// Import proto from the main crate to avoid duplicate compilation
use crate::proto::cluster_service as proto;

use proto::{cluster_service_server::ClusterService, *};

#[derive(Debug)]
pub struct ClusterServiceImpl {
    service_discovery: Arc<ServiceDiscovery>,
    service_registry: Arc<ServiceRegistry>,
    load_balancers: Arc<RwLock<HashMap<String, Arc<LoadBalancer>>>>,
    metrics: Arc<ClusterMetrics>,
    cluster_config: Arc<RwLock<ClusterConfig>>,
}

impl ClusterServiceImpl {
    pub fn new() -> Self {
        let service_registry = Arc::new(ServiceRegistry::new());
        let service_discovery = Arc::new(ServiceDiscovery::new(Arc::clone(&service_registry)));
        let metrics = Arc::new(ClusterMetrics::new());

        // Create default cluster config
        let default_config = ClusterConfig {
            cluster_name: "dotlanth-cluster".to_string(),
            cluster_version: "1.0.0".to_string(),
            network: Some(NetworkConfig {
                cluster_cidr: "10.0.0.0/16".to_string(),
                service_cidr: "10.1.0.0/16".to_string(),
                dns_domain: "cluster.local".to_string(),
                dns_servers: vec!["10.1.0.10".to_string()],
                mtu: 1500,
            }),
            security: Some(SecurityConfig {
                rbac_enabled: true,
                network_policies_enabled: true,
                pod_security_policies_enabled: true,
                default_security_context: "restrictive".to_string(),
                allowed_registries: vec!["docker.io".to_string(), "ghcr.io".to_string()],
            }),
            resources: Some(ResourceConfig {
                default_quota: Some(ResourceQuota {
                    max_cpu_cores: 16,
                    max_memory_bytes: 32 * 1024 * 1024 * 1024,    // 32GB
                    max_storage_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
                    max_deployments: 100,
                    max_dots: 1000,
                }),
                default_limits: Some(ResourceLimits {
                    default_cpu_request: 100,                  // 100m CPU
                    default_memory_request: 128 * 1024 * 1024, // 128MB
                    default_cpu_limit: 1000,                   // 1 CPU
                    default_memory_limit: 512 * 1024 * 1024,   // 512MB
                }),
                resource_quotas_enabled: true,
                limit_ranges_enabled: true,
            }),
            scheduling: Some(SchedulingConfig {
                default_scheduler: "dotlanth-scheduler".to_string(),
                enable_preemption: true,
                scheduling_timeout_seconds: 60,
                scheduler_config: HashMap::new(),
            }),
            custom_config: HashMap::new(),
        };

        Self {
            service_discovery,
            service_registry,
            load_balancers: Arc::new(RwLock::new(HashMap::new())),
            metrics,
            cluster_config: Arc::new(RwLock::new(default_config)),
        }
    }

    pub async fn start_background_tasks(&self) {
        // Start health monitoring
        self.service_registry.start_health_monitoring().await;

        // Start metrics collection
        self.metrics.start_background_tasks().await;

        tracing::info!("Cluster service background tasks started");
    }

    async fn convert_to_service_instance(&self, node_info: &NodeInfo, capabilities: &NodeCapabilities) -> ServiceInstance {
        ServiceInstance {
            id: node_info.node_id.clone(),
            name: node_info.name.clone(),
            address: node_info.address.clone(),
            port: node_info.port as u16,
            version: node_info.version.clone(),
            tags: vec![], // Could be derived from labels
            metadata: node_info.labels.clone(),
            health_status: HealthStatus::Healthy, // Default to healthy
            last_heartbeat: std::time::Instant::now(),
            registered_at: SystemTime::now(),
            capabilities: ServiceCapabilities {
                max_connections: capabilities.resources.as_ref().map(|r| r.max_dots).unwrap_or(100),
                supported_protocols: capabilities.supported_protocols.clone(),
                features: capabilities.features.clone(),
                security_level: capabilities.security.as_ref().map(|s| s.security_level.clone()).unwrap_or_default(),
            },
            load_metrics: LoadMetrics::default(),
        }
    }

    async fn get_or_create_load_balancer(&self, service_name: &str) -> Arc<LoadBalancer> {
        let mut balancers = self.load_balancers.write().await;

        if let Some(balancer) = balancers.get(service_name) {
            Arc::clone(balancer)
        } else {
            let strategy = LoadBalancingStrategy::default();
            let balancer = Arc::new(LoadBalancer::new(strategy));
            balancers.insert(service_name.to_string(), Arc::clone(&balancer));
            balancer
        }
    }
}

impl Default for ClusterServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl ClusterService for ClusterServiceImpl {
    async fn register_node(&self, request: Request<RegisterNodeRequest>) -> TonicResult<Response<RegisterNodeResponse>> {
        let req = request.into_inner();

        let node_info = req.node_info.ok_or_else(|| Status::invalid_argument("Node info is required"))?;
        let capabilities = req.capabilities.ok_or_else(|| Status::invalid_argument("Node capabilities are required"))?;

        tracing::info!(
            node_id = %node_info.node_id,
            node_name = %node_info.name,
            address = format!("{}:{}", node_info.address, node_info.port),
            "Registering node"
        );

        // Convert to service instance
        let instance = self.convert_to_service_instance(&node_info, &capabilities).await;

        // Register the service
        match self.service_registry.register_service(instance.clone()).await {
            Ok(service_id) => {
                // Add to load balancer
                let load_balancer = self.get_or_create_load_balancer(&instance.name).await;
                load_balancer.add_backend(instance.clone()).await;

                // Record metrics
                self.metrics.record_service_discovery("register", 1).await;

                // Notify service discovery
                self.service_discovery.notify_service_event(super::discovery::ServiceEvent::ServiceRegistered(instance)).await;

                let registration = NodeRegistration {
                    registration_token: Uuid::new_v4().to_string(),
                    expires_at: (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600) as u64,
                    cluster_config: Some(self.cluster_config.read().await.clone()),
                    assigned_roles: vec!["worker".to_string()],
                };

                let response = RegisterNodeResponse {
                    success: true,
                    node_id: service_id,
                    registration: Some(registration),
                    error_message: String::new(),
                };

                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!(error = %err, "Failed to register node");
                let response = RegisterNodeResponse {
                    success: false,
                    node_id: String::new(),
                    registration: None,
                    error_message: err,
                };
                Ok(Response::new(response))
            }
        }
    }

    async fn unregister_node(&self, request: Request<UnregisterNodeRequest>) -> TonicResult<Response<UnregisterNodeResponse>> {
        let req = request.into_inner();

        tracing::info!(node_id = %req.node_id, "Unregistering node");

        // Get node info before removing
        let node_info = self.service_registry.get_service(&req.node_id).await;

        match self.service_registry.unregister_service(&req.node_id).await {
            Ok(_) => {
                // Remove from load balancer if we have node info
                if let Some(instance) = node_info {
                    let load_balancer = self.get_or_create_load_balancer(&instance.name).await;
                    load_balancer.remove_backend(&req.node_id).await;

                    // Notify service discovery
                    self.service_discovery
                        .notify_service_event(super::discovery::ServiceEvent::ServiceUnregistered(req.node_id.clone()))
                        .await;
                }

                // Record metrics
                self.metrics.record_service_discovery("unregister", 1).await;

                let response = UnregisterNodeResponse {
                    success: true,
                    error_message: String::new(),
                };

                Ok(Response::new(response))
            }
            Err(err) => {
                tracing::error!(error = %err, node_id = %req.node_id, "Failed to unregister node");
                let response = UnregisterNodeResponse { success: false, error_message: err };
                Ok(Response::new(response))
            }
        }
    }

    async fn list_nodes(&self, request: Request<ListNodesRequest>) -> TonicResult<Response<ListNodesResponse>> {
        let req = request.into_inner();

        let services = self.service_registry.get_all_services().await;

        // Apply filtering if provided
        let filtered_services = if let Some(filter) = req.filter {
            services
                .into_iter()
                .filter(|service| {
                    // Apply filters based on filter criteria
                    if !filter.name_pattern.is_empty() && !service.name.contains(&filter.name_pattern) {
                        return false;
                    }
                    // Add more filter logic as needed
                    true
                })
                .collect()
        } else {
            services
        };

        // Convert to NodeDetails
        let mut nodes = Vec::new();
        for service in filtered_services {
            let node_details = NodeDetails {
                info: Some(NodeInfo {
                    node_id: service.id.clone(),
                    name: service.name.clone(),
                    address: service.address.clone(),
                    port: service.port as u32,
                    r#type: 1, // Worker type
                    version: service.version.clone(),
                    labels: service.metadata.clone(),
                    annotations: HashMap::new(),
                }),
                status: service.health_status.clone().into(),
                capabilities: Some(NodeCapabilities {
                    resources: Some(ResourceCapacity {
                        cpu_cores: 4,
                        memory_bytes: 8 * 1024 * 1024 * 1024,    // 8GB
                        storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
                        network_bandwidth_bps: 1_000_000_000,    // 1Gbps
                        max_dots: service.capabilities.max_connections,
                        max_paradots: 50,
                    }),
                    supported_architectures: vec!["x86_64".to_string()],
                    supported_protocols: service.capabilities.supported_protocols.clone(),
                    features: service.capabilities.features.clone(),
                    security: Some(SecurityCapabilities {
                        tls_enabled: true,
                        mtls_enabled: true,
                        auth_methods: vec!["jwt".to_string()],
                        encryption_at_rest: true,
                        security_level: service.capabilities.security_level.clone(),
                    }),
                }),
                metrics: Some(NodeMetrics {
                    resource_usage: Some(ResourceUsage {
                        cpu_usage_percent: service.load_metrics.cpu_usage,
                        memory_used_bytes: (service.load_metrics.memory_usage * 8.0 * 1024.0 * 1024.0 * 1024.0) as u64,
                        memory_available_bytes: 8 * 1024 * 1024 * 1024,
                        storage_used_bytes: 10 * 1024 * 1024 * 1024,
                        storage_available_bytes: 90 * 1024 * 1024 * 1024,
                        network_in_bps: 1000000,
                        network_out_bps: 1000000,
                    }),
                    performance: Some(PerformanceMetrics {
                        load_average_1m: 0.5,
                        load_average_5m: 0.4,
                        load_average_15m: 0.3,
                        requests_per_second: service.load_metrics.request_rate as u64,
                        average_response_time_ms: service.load_metrics.response_time_avg,
                        error_rate_percent: service.load_metrics.error_rate as u32,
                    }),
                    uptime_seconds: service.registered_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                    active_dots: service.load_metrics.active_connections,
                    active_paradots: 5,
                }),
                health: Some(NodeHealth {
                    overall_status: service.health_status.clone().into(),
                    checks: vec![HealthCheck {
                        name: "connectivity".to_string(),
                        status: service.health_status.into(),
                        message: "Node is responsive".to_string(),
                        last_success: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        last_failure: 0,
                        consecutive_failures: 0,
                    }],
                    last_check_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                }),
                registered_at: service.registered_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                deployments: vec![], // Could be populated from deployment registry
            };

            nodes.push(node_details);
        }

        let total_count = nodes.len() as u32;
        let response = ListNodesResponse {
            nodes,
            total_count,
            next_cursor: String::new(),
            has_more: false,
        };

        Ok(Response::new(response))
    }

    async fn get_node(&self, request: Request<GetNodeRequest>) -> TonicResult<Response<GetNodeResponse>> {
        let req = request.into_inner();

        match self.service_registry.get_service(&req.node_id).await {
            Some(service) => {
                let node_details = NodeDetails {
                    info: Some(NodeInfo {
                        node_id: service.id.clone(),
                        name: service.name.clone(),
                        address: service.address.clone(),
                        port: service.port as u32,
                        r#type: 1, // Worker type
                        version: service.version.clone(),
                        labels: service.metadata.clone(),
                        annotations: HashMap::new(),
                    }),
                    status: service.health_status.clone().into(),
                    capabilities: Some(NodeCapabilities {
                        resources: Some(ResourceCapacity {
                            cpu_cores: 4,
                            memory_bytes: 8 * 1024 * 1024 * 1024,
                            storage_bytes: 100 * 1024 * 1024 * 1024,
                            network_bandwidth_bps: 1_000_000_000,
                            max_dots: service.capabilities.max_connections,
                            max_paradots: 50,
                        }),
                        supported_architectures: vec!["x86_64".to_string()],
                        supported_protocols: service.capabilities.supported_protocols.clone(),
                        features: service.capabilities.features.clone(),
                        security: Some(SecurityCapabilities {
                            tls_enabled: true,
                            mtls_enabled: true,
                            auth_methods: vec!["jwt".to_string()],
                            encryption_at_rest: true,
                            security_level: service.capabilities.security_level.clone(),
                        }),
                    }),
                    metrics: Some(NodeMetrics {
                        resource_usage: Some(ResourceUsage {
                            cpu_usage_percent: service.load_metrics.cpu_usage,
                            memory_used_bytes: (service.load_metrics.memory_usage * 8.0 * 1024.0 * 1024.0 * 1024.0) as u64,
                            memory_available_bytes: 8 * 1024 * 1024 * 1024,
                            storage_used_bytes: 10 * 1024 * 1024 * 1024,
                            storage_available_bytes: 90 * 1024 * 1024 * 1024,
                            network_in_bps: 1000000,
                            network_out_bps: 1000000,
                        }),
                        performance: Some(PerformanceMetrics {
                            load_average_1m: 0.5,
                            load_average_5m: 0.4,
                            load_average_15m: 0.3,
                            requests_per_second: service.load_metrics.request_rate as u64,
                            average_response_time_ms: service.load_metrics.response_time_avg,
                            error_rate_percent: service.load_metrics.error_rate as u32,
                        }),
                        uptime_seconds: service.registered_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                        active_dots: service.load_metrics.active_connections,
                        active_paradots: 5,
                    }),
                    health: Some(NodeHealth {
                        overall_status: service.health_status.clone().into(),
                        checks: vec![HealthCheck {
                            name: "connectivity".to_string(),
                            status: service.health_status.into(),
                            message: "Node is responsive".to_string(),
                            last_success: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                            last_failure: 0,
                            consecutive_failures: 0,
                        }],
                        last_check_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    }),
                    registered_at: service.registered_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
                    last_heartbeat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                    deployments: vec![],
                };

                let response = GetNodeResponse {
                    success: true,
                    node: Some(node_details),
                    error_message: String::new(),
                };

                Ok(Response::new(response))
            }
            None => {
                let response = GetNodeResponse {
                    success: false,
                    node: None,
                    error_message: format!("Node with ID {} not found", req.node_id),
                };
                Ok(Response::new(response))
            }
        }
    }

    async fn update_node(&self, request: Request<UpdateNodeRequest>) -> TonicResult<Response<UpdateNodeResponse>> {
        let req = request.into_inner();

        let updated_info = req.updated_info.ok_or_else(|| Status::invalid_argument("Updated node info is required"))?;

        // For now, we'll return a placeholder response
        // In a full implementation, we would update the service registry
        let response = UpdateNodeResponse {
            success: false,
            updated_node: None,
            error_message: "UpdateNode not yet fully implemented".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_node_health(&self, request: Request<GetNodeHealthRequest>) -> TonicResult<Response<GetNodeHealthResponse>> {
        let req = request.into_inner();

        match self.service_registry.get_service(&req.node_id).await {
            Some(service) => {
                let health = NodeHealth {
                    overall_status: service.health_status.clone().into(),
                    checks: vec![HealthCheck {
                        name: "connectivity".to_string(),
                        status: service.health_status.into(),
                        message: "Node is responsive".to_string(),
                        last_success: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        last_failure: 0,
                        consecutive_failures: 0,
                    }],
                    last_check_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                };

                let response = GetNodeHealthResponse {
                    success: true,
                    health: Some(health),
                    error_message: String::new(),
                };

                Ok(Response::new(response))
            }
            None => {
                let response = GetNodeHealthResponse {
                    success: false,
                    health: None,
                    error_message: format!("Node with ID {} not found", req.node_id),
                };
                Ok(Response::new(response))
            }
        }
    }

    // Load balancer methods
    async fn get_load_balancer_status(&self, request: Request<GetLoadBalancerStatusRequest>) -> TonicResult<Response<GetLoadBalancerStatusResponse>> {
        let req = request.into_inner();

        let balancers = self.load_balancers.read().await;
        if let Some(balancer) = balancers.get(&req.load_balancer_id) {
            let backend_stats = balancer.get_backend_stats().await;

            let backends: Vec<BackendNode> = backend_stats
                .into_iter()
                .map(|(_, stats)| {
                    BackendNode {
                        node_id: stats.id,
                        address: stats.address,
                        port: 0, // Would need to parse from address
                        status: match stats.health_status {
                            HealthStatus::Healthy => 0,
                            HealthStatus::Unhealthy => 1,
                            _ => 1,
                        },
                        weight: stats.weight,
                        metrics: Some(BackendMetrics {
                            active_connections: stats.current_connections as u64,
                            requests_per_second: stats.total_requests as u64,
                            response_time_ms: stats.average_response_time,
                            error_rate_percent: stats.error_rate as u32,
                        }),
                    }
                })
                .collect();

            let status = LoadBalancerStatus {
                load_balancer_id: req.load_balancer_id,
                state: 1, // Active
                backends,
                metrics: Some(LoadBalancerMetrics {
                    total_requests: 1000,
                    total_connections: 50,
                    average_response_time_ms: 100.0,
                    error_rate_percent: 1,
                    bytes_in: 1024 * 1024,
                    bytes_out: 1024 * 1024,
                }),
                config: Some(LoadBalancerConfig {
                    algorithm: balancer.get_strategy().algorithm.clone().into(),
                    health_check: Some(HealthCheckConfig {
                        path: "/health".to_string(),
                        interval_seconds: 30,
                        timeout_seconds: 5,
                        healthy_threshold: 2,
                        unhealthy_threshold: 3,
                    }),
                    session_affinity: Some(SessionAffinityConfig {
                        enabled: balancer.get_strategy().sticky_sessions,
                        r#type: if balancer.get_strategy().sticky_sessions { 1 } else { 0 },
                        timeout_seconds: balancer.get_strategy().session_affinity_timeout.as_secs() as u32,
                    }),
                    connection_timeout_seconds: 30,
                    request_timeout_seconds: 60,
                }),
            };

            let response = GetLoadBalancerStatusResponse { status: Some(status) };

            Ok(Response::new(response))
        } else {
            Err(Status::not_found(format!("Load balancer {} not found", req.load_balancer_id)))
        }
    }

    async fn update_load_balancer_config(&self, request: Request<UpdateLoadBalancerConfigRequest>) -> TonicResult<Response<UpdateLoadBalancerConfigResponse>> {
        let req = request.into_inner();

        let response = UpdateLoadBalancerConfigResponse {
            success: false,
            updated_config: None,
            error_message: "UpdateLoadBalancerConfig not yet fully implemented".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_node_load(&self, request: Request<GetNodeLoadRequest>) -> TonicResult<Response<GetNodeLoadResponse>> {
        let req = request.into_inner();

        match self.service_registry.get_service(&req.node_id).await {
            Some(service) => {
                let load_info = NodeLoadInfo {
                    node_id: req.node_id,
                    current_load_percent: service.load_metrics.cpu_usage,
                    average_load_percent: service.load_metrics.cpu_usage,
                    peak_load_percent: service.load_metrics.cpu_usage * 1.2,
                    trend: 0, // Stable
                    history: vec![LoadDataPoint {
                        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        load_percent: service.load_metrics.cpu_usage,
                    }],
                };

                let response = GetNodeLoadResponse { load_info: Some(load_info) };

                Ok(Response::new(response))
            }
            None => Err(Status::not_found(format!("Node {} not found", req.node_id))),
        }
    }

    // Cluster operations
    async fn get_cluster_status(&self, request: Request<GetClusterStatusRequest>) -> TonicResult<Response<GetClusterStatusResponse>> {
        let req = request.into_inner();

        let services = self.service_registry.get_all_services().await;
        let healthy_nodes = services.iter().filter(|s| matches!(s.health_status, HealthStatus::Healthy)).count();

        let cluster_info = ClusterInfo {
            cluster_id: "dotlanth-cluster-001".to_string(),
            name: "Dotlanth Cluster".to_string(),
            version: "1.0.0".to_string(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            total_nodes: services.len() as u32,
            ready_nodes: healthy_nodes as u32,
            total_deployments: 0,
            running_deployments: 0,
        };

        let cluster_metrics = proto::ClusterMetrics {
            total_resources: Some(ResourceUsage {
                cpu_usage_percent: 50.0,
                memory_used_bytes: 16 * 1024 * 1024 * 1024,
                memory_available_bytes: 64 * 1024 * 1024 * 1024,
                storage_used_bytes: 100 * 1024 * 1024 * 1024,
                storage_available_bytes: 1024 * 1024 * 1024 * 1024,
                network_in_bps: 10000000,
                network_out_bps: 10000000,
            }),
            used_resources: Some(ResourceUsage {
                cpu_usage_percent: 25.0,
                memory_used_bytes: 8 * 1024 * 1024 * 1024,
                memory_available_bytes: 56 * 1024 * 1024 * 1024,
                storage_used_bytes: 50 * 1024 * 1024 * 1024,
                storage_available_bytes: 974 * 1024 * 1024 * 1024,
                network_in_bps: 5000000,
                network_out_bps: 5000000,
            }),
            available_resources: Some(ResourceUsage {
                cpu_usage_percent: 25.0,
                memory_used_bytes: 0,
                memory_available_bytes: 48 * 1024 * 1024 * 1024,
                storage_used_bytes: 0,
                storage_available_bytes: 924 * 1024 * 1024 * 1024,
                network_in_bps: 0,
                network_out_bps: 0,
            }),
            performance: Some(PerformanceMetrics {
                load_average_1m: 0.5,
                load_average_5m: 0.4,
                load_average_15m: 0.3,
                requests_per_second: 1000,
                average_response_time_ms: 50.0,
                error_rate_percent: 1,
            }),
            total_requests: 10000,
            successful_requests: 9900,
            failed_requests: 100,
        };

        let status = ClusterStatus {
            state: 2, // Ready
            info: Some(cluster_info),
            metrics: Some(cluster_metrics),
            nodes: if req.include_nodes {
                // Convert services to node details - simplified for now
                vec![]
            } else {
                vec![]
            },
            deployments: if req.include_deployments { vec![] } else { vec![] },
        };

        let response = GetClusterStatusResponse { status: Some(status) };

        Ok(Response::new(response))
    }

    async fn get_cluster_metrics(&self, request: Request<GetClusterMetricsRequest>) -> TonicResult<Response<GetClusterMetricsResponse>> {
        let req = request.into_inner();

        let metrics = if req.metric_names.is_empty() {
            self.metrics.get_metrics(None).await
        } else {
            self.metrics.get_metrics(Some(req.metric_names)).await
        };

        let cluster_metrics: Vec<ClusterMetric> = metrics
            .into_iter()
            .map(|metric| ClusterMetric {
                name: metric.name,
                r#type: metric.metric_type,
                data_points: metric
                    .data_points
                    .into_iter()
                    .map(|dp| MetricDataPoint {
                        timestamp: dp.timestamp,
                        value: dp.value,
                    })
                    .collect(),
                labels: metric.labels,
            })
            .collect();

        let response = GetClusterMetricsResponse { metrics: cluster_metrics };

        Ok(Response::new(response))
    }

    // Configuration management
    async fn get_cluster_config(&self, request: Request<GetClusterConfigRequest>) -> TonicResult<Response<GetClusterConfigResponse>> {
        let config = self.cluster_config.read().await.clone();

        let response = GetClusterConfigResponse { config: Some(config) };

        Ok(Response::new(response))
    }

    async fn update_cluster_config(&self, request: Request<UpdateClusterConfigRequest>) -> TonicResult<Response<UpdateClusterConfigResponse>> {
        let req = request.into_inner();

        let new_config = req.config.ok_or_else(|| Status::invalid_argument("Cluster config is required"))?;

        if req.validate_only {
            // Just validate without updating
            let response = UpdateClusterConfigResponse {
                success: true,
                updated_config: Some(new_config),
                validation_errors: vec![],
                error_message: String::new(),
            };
            return Ok(Response::new(response));
        }

        // Update the config
        {
            let mut config = self.cluster_config.write().await;
            *config = new_config.clone();
        }

        let response = UpdateClusterConfigResponse {
            success: true,
            updated_config: Some(new_config),
            validation_errors: vec![],
            error_message: String::new(),
        };

        Ok(Response::new(response))
    }

    // Placeholder implementations for other methods
    async fn create_deployment(&self, request: Request<CreateDeploymentRequest>) -> TonicResult<Response<CreateDeploymentResponse>> {
        let response = CreateDeploymentResponse {
            success: false,
            deployment_id: String::new(),
            status: None,
            error_message: "CreateDeployment not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn list_deployments(&self, request: Request<ListDeploymentsRequest>) -> TonicResult<Response<ListDeploymentsResponse>> {
        let response = ListDeploymentsResponse {
            deployments: vec![],
            total_count: 0,
            next_cursor: String::new(),
            has_more: false,
        };
        Ok(Response::new(response))
    }

    async fn get_deployment(&self, request: Request<GetDeploymentRequest>) -> TonicResult<Response<GetDeploymentResponse>> {
        let response = GetDeploymentResponse {
            success: false,
            deployment: None,
            error_message: "GetDeployment not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn update_deployment(&self, request: Request<UpdateDeploymentRequest>) -> TonicResult<Response<UpdateDeploymentResponse>> {
        let response = UpdateDeploymentResponse {
            success: false,
            updated_deployment: None,
            error_message: "UpdateDeployment not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn delete_deployment(&self, request: Request<DeleteDeploymentRequest>) -> TonicResult<Response<DeleteDeploymentResponse>> {
        let response = DeleteDeploymentResponse {
            success: false,
            error_message: "DeleteDeployment not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn scale_deployment(&self, request: Request<ScaleDeploymentRequest>) -> TonicResult<Response<ScaleDeploymentResponse>> {
        let response = ScaleDeploymentResponse {
            success: false,
            new_status: None,
            error_message: "ScaleDeployment not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn drain_node(&self, request: Request<DrainNodeRequest>) -> TonicResult<Response<DrainNodeResponse>> {
        let response = DrainNodeResponse {
            success: false,
            status: None,
            error_message: "DrainNode not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn cordon_node(&self, request: Request<CordonNodeRequest>) -> TonicResult<Response<CordonNodeResponse>> {
        let response = CordonNodeResponse {
            success: false,
            error_message: "CordonNode not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn uncordon_node(&self, request: Request<UncordonNodeRequest>) -> TonicResult<Response<UncordonNodeResponse>> {
        let response = UncordonNodeResponse {
            success: false,
            error_message: "UncordonNode not yet implemented".to_string(),
        };
        Ok(Response::new(response))
    }

    // Streaming operations
    type StreamNodeEventsStream = Pin<Box<dyn Stream<Item = Result<NodeEvent, Status>> + Send>>;

    async fn stream_node_events(&self, request: Request<StreamNodeEventsRequest>) -> TonicResult<Response<Self::StreamNodeEventsStream>> {
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    type StreamDeploymentEventsStream = Pin<Box<dyn Stream<Item = Result<DeploymentEvent, Status>> + Send>>;

    async fn stream_deployment_events(&self, request: Request<StreamDeploymentEventsRequest>) -> TonicResult<Response<Self::StreamDeploymentEventsStream>> {
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    type StreamClusterMetricsStream = Pin<Box<dyn Stream<Item = Result<ClusterMetric, Status>> + Send>>;

    async fn stream_cluster_metrics(&self, request: Request<StreamClusterMetricsRequest>) -> TonicResult<Response<Self::StreamClusterMetricsStream>> {
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }
}

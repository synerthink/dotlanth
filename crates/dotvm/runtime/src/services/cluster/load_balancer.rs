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

use super::CircuitBreaker;
use super::discovery::{HealthStatus, ServiceInstance};
use dashmap::DashMap;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    IpHash,
    LeastResponseTime,
    Random,
    WeightedLeastConnections,
}

impl From<i32> for LoadBalancingAlgorithm {
    fn from(value: i32) -> Self {
        match value {
            0 => LoadBalancingAlgorithm::RoundRobin,
            1 => LoadBalancingAlgorithm::LeastConnections,
            2 => LoadBalancingAlgorithm::WeightedRoundRobin,
            3 => LoadBalancingAlgorithm::IpHash,
            4 => LoadBalancingAlgorithm::LeastResponseTime,
            5 => LoadBalancingAlgorithm::Random,
            6 => LoadBalancingAlgorithm::WeightedLeastConnections,
            _ => LoadBalancingAlgorithm::RoundRobin,
        }
    }
}

impl From<LoadBalancingAlgorithm> for i32 {
    fn from(algorithm: LoadBalancingAlgorithm) -> Self {
        match algorithm {
            LoadBalancingAlgorithm::RoundRobin => 0,
            LoadBalancingAlgorithm::LeastConnections => 1,
            LoadBalancingAlgorithm::WeightedRoundRobin => 2,
            LoadBalancingAlgorithm::IpHash => 3,
            LoadBalancingAlgorithm::LeastResponseTime => 4,
            LoadBalancingAlgorithm::Random => 5,
            LoadBalancingAlgorithm::WeightedLeastConnections => 6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadBalancingStrategy {
    pub algorithm: LoadBalancingAlgorithm,
    pub weights: HashMap<String, u32>,
    pub health_check_enabled: bool,
    pub circuit_breaker_enabled: bool,
    pub sticky_sessions: bool,
    pub session_affinity_timeout: Duration,
}

impl Default for LoadBalancingStrategy {
    fn default() -> Self {
        Self {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            weights: HashMap::new(),
            health_check_enabled: true,
            circuit_breaker_enabled: true,
            sticky_sessions: false,
            session_affinity_timeout: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug)]
pub struct BackendNode {
    pub instance: ServiceInstance,
    pub weight: u32,
    pub current_connections: AtomicUsize,
    pub total_requests: AtomicUsize,
    pub failed_requests: AtomicUsize,
    pub response_time_sum: Arc<RwLock<f64>>,
    pub last_request_time: Arc<RwLock<Instant>>,
    pub circuit_breaker: Option<Arc<CircuitBreaker>>,
}

impl BackendNode {
    pub fn new(instance: ServiceInstance, weight: u32, enable_circuit_breaker: bool) -> Self {
        let circuit_breaker = if enable_circuit_breaker {
            Some(Arc::new(CircuitBreaker::new(
                5,                       // failure_threshold
                Duration::from_secs(60), // timeout
                Duration::from_secs(10), // retry_timeout
            )))
        } else {
            None
        };

        Self {
            instance,
            weight,
            current_connections: AtomicUsize::new(0),
            total_requests: AtomicUsize::new(0),
            failed_requests: AtomicUsize::new(0),
            response_time_sum: Arc::new(RwLock::new(0.0)),
            last_request_time: Arc::new(RwLock::new(Instant::now())),
            circuit_breaker,
        }
    }

    pub async fn record_request(&self, success: bool, response_time: Duration) {
        self.total_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if !success {
            self.failed_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if let Some(cb) = &self.circuit_breaker {
                cb.record_failure().await;
            }
        } else if let Some(cb) = &self.circuit_breaker {
            cb.record_success().await;
        }

        {
            let mut sum = self.response_time_sum.write().await;
            *sum += response_time.as_millis() as f64;
        }

        {
            let mut last_request = self.last_request_time.write().await;
            *last_request = Instant::now();
        }
    }

    pub async fn is_available(&self) -> bool {
        // Check basic health
        if self.instance.health_status != HealthStatus::Healthy {
            return false;
        }

        // Check circuit breaker
        if let Some(cb) = &self.circuit_breaker {
            return cb.can_execute().await;
        }

        true
    }

    pub async fn get_average_response_time(&self) -> f64 {
        let total_requests = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        if total_requests == 0 {
            return 0.0;
        }

        let sum = *self.response_time_sum.read().await;
        sum / total_requests as f64
    }

    pub fn get_current_connections(&self) -> usize {
        self.current_connections.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn increment_connections(&self) {
        self.current_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn decrement_connections(&self) {
        self.current_connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_error_rate(&self) -> f64 {
        let total = self.total_requests.load(std::sync::atomic::Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }

        let failed = self.failed_requests.load(std::sync::atomic::Ordering::Relaxed);
        (failed as f64 / total as f64) * 100.0
    }
}

#[derive(Debug)]
pub struct LoadBalancer {
    backends: Arc<DashMap<String, Arc<BackendNode>>>,
    strategy: LoadBalancingStrategy,
    round_robin_counter: AtomicUsize,
    session_affinity: Arc<DashMap<String, String>>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            backends: Arc::new(DashMap::new()),
            strategy,
            round_robin_counter: AtomicUsize::new(0),
            session_affinity: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_backend(&self, instance: ServiceInstance) -> String {
        let weight = self.strategy.weights.get(&instance.id).cloned().unwrap_or(1);
        let backend = Arc::new(BackendNode::new(instance.clone(), weight, self.strategy.circuit_breaker_enabled));

        let backend_id = instance.id.clone();
        self.backends.insert(backend_id.clone(), backend);

        tracing::info!(
            backend_id = %backend_id,
            instance_name = %instance.name,
            weight = weight,
            "Backend added to load balancer"
        );

        backend_id
    }

    pub async fn remove_backend(&self, backend_id: &str) -> bool {
        match self.backends.remove(backend_id) {
            Some(_) => {
                tracing::info!(backend_id = %backend_id, "Backend removed from load balancer");
                true
            }
            None => false,
        }
    }

    pub async fn select_backend(&self, client_id: Option<&str>) -> Option<Arc<BackendNode>> {
        // Check for session affinity
        if self.strategy.sticky_sessions {
            if let Some(client_id) = client_id {
                if let Some(backend_id) = self.session_affinity.get(client_id) {
                    if let Some(backend) = self.backends.get(backend_id.value()) {
                        if backend.is_available().await {
                            return Some(backend.clone());
                        } else {
                            // Remove invalid session affinity
                            self.session_affinity.remove(client_id);
                        }
                    }
                }
            }
        }

        // Get available backends
        let available_backends: Vec<_> = {
            let mut backends = Vec::new();
            for entry in self.backends.iter() {
                if entry.value().is_available().await {
                    backends.push(entry.value().clone());
                }
            }
            backends
        };

        if available_backends.is_empty() {
            return None;
        }

        let selected = match self.strategy.algorithm {
            LoadBalancingAlgorithm::RoundRobin => self.round_robin_select(&available_backends),
            LoadBalancingAlgorithm::LeastConnections => self.least_connections_select(&available_backends).await,
            LoadBalancingAlgorithm::WeightedRoundRobin => self.weighted_round_robin_select(&available_backends),
            LoadBalancingAlgorithm::IpHash => self.ip_hash_select(&available_backends, client_id),
            LoadBalancingAlgorithm::LeastResponseTime => self.least_response_time_select(&available_backends).await,
            LoadBalancingAlgorithm::Random => self.random_select(&available_backends),
            LoadBalancingAlgorithm::WeightedLeastConnections => self.weighted_least_connections_select(&available_backends).await,
        };

        // Set session affinity if enabled
        if let (Some(backend), Some(client_id)) = (&selected, client_id) {
            if self.strategy.sticky_sessions {
                self.session_affinity.insert(client_id.to_string(), backend.instance.id.clone());
            }
        }

        selected
    }

    fn round_robin_select(&self, backends: &[Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        let index = self.round_robin_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % backends.len();
        Some(backends[index].clone())
    }

    async fn least_connections_select(&self, backends: &[Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        let mut min_connections = usize::MAX;
        let mut selected = None;

        for backend in backends {
            let connections = backend.get_current_connections();
            if connections < min_connections {
                min_connections = connections;
                selected = Some(backend.clone());
            }
        }

        selected
    }

    fn weighted_round_robin_select(&self, backends: &[Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: u32 = backends.iter().map(|b| b.weight).sum();
        if total_weight == 0 {
            return self.round_robin_select(backends);
        }

        // Get weighted position
        let position = self.round_robin_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) as u32 % total_weight;
        let mut current_weight = 0;

        for backend in backends {
            current_weight += backend.weight;
            if position < current_weight {
                return Some(backend.clone());
            }
        }

        backends.first().cloned()
    }

    fn ip_hash_select(&self, backends: &[Arc<BackendNode>], client_id: Option<&str>) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        let hash = if let Some(client_id) = client_id { self.simple_hash(client_id.as_bytes()) } else { 0 };

        let index = hash % backends.len();
        Some(backends[index].clone())
    }

    async fn least_response_time_select(&self, backends: &[Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        let mut min_response_time = f64::MAX;
        let mut selected = None;

        for backend in backends {
            let response_time = backend.get_average_response_time().await;
            if response_time < min_response_time {
                min_response_time = response_time;
                selected = Some(backend.clone());
            }
        }

        selected.or_else(|| backends.first().cloned())
    }

    fn random_select(&self, backends: &[Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..backends.len());
        Some(backends[index].clone())
    }

    async fn weighted_least_connections_select(&self, backends: &[Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        if backends.is_empty() {
            return None;
        }

        let mut min_ratio = f64::MAX;
        let mut selected = None;

        for backend in backends {
            let connections = backend.get_current_connections() as f64;
            let weight = backend.weight as f64;
            let ratio = if weight > 0.0 { connections / weight } else { f64::MAX };

            if ratio < min_ratio {
                min_ratio = ratio;
                selected = Some(backend.clone());
            }
        }

        selected.or_else(|| backends.first().cloned())
    }

    fn simple_hash(&self, data: &[u8]) -> usize {
        let mut hash = 0usize;
        for &byte in data {
            hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
        }
        hash
    }

    pub async fn get_backend_stats(&self) -> HashMap<String, BackendStats> {
        let mut stats = HashMap::new();

        for entry in self.backends.iter() {
            let backend = entry.value();
            let backend_stats = BackendStats {
                id: backend.instance.id.clone(),
                name: backend.instance.name.clone(),
                address: format!("{}:{}", backend.instance.address, backend.instance.port),
                weight: backend.weight,
                current_connections: backend.get_current_connections(),
                total_requests: backend.total_requests.load(std::sync::atomic::Ordering::Relaxed),
                failed_requests: backend.failed_requests.load(std::sync::atomic::Ordering::Relaxed),
                average_response_time: backend.get_average_response_time().await,
                error_rate: backend.get_error_rate(),
                health_status: backend.instance.health_status.clone(),
                is_circuit_breaker_open: backend.circuit_breaker.as_ref().map(|cb| cb.is_open()).unwrap_or(false),
            };
            stats.insert(backend.instance.id.clone(), backend_stats);
        }

        stats
    }

    pub fn get_strategy(&self) -> &LoadBalancingStrategy {
        &self.strategy
    }

    pub async fn update_strategy(&mut self, strategy: LoadBalancingStrategy) {
        let old_strategy = std::mem::replace(&mut self.strategy, strategy);

        // Update backend weights by recreating backends with new weights
        let mut updated_backends = DashMap::new();

        for entry in self.backends.iter() {
            let backend_id = entry.key().clone();
            let old_backend = entry.value();

            // Get new weight or keep existing weight
            let new_weight = self.strategy.weights.get(&backend_id).copied().unwrap_or(old_backend.weight);

            // Only recreate if weight actually changed
            if new_weight != old_backend.weight {
                let updated_backend = Arc::new(BackendNode {
                    instance: old_backend.instance.clone(),
                    weight: new_weight, // New weight
                    current_connections: AtomicUsize::new(old_backend.current_connections.load(Ordering::Relaxed)),
                    total_requests: AtomicUsize::new(old_backend.total_requests.load(Ordering::Relaxed)),
                    failed_requests: AtomicUsize::new(old_backend.failed_requests.load(Ordering::Relaxed)),
                    response_time_sum: old_backend.response_time_sum.clone(),
                    last_request_time: old_backend.last_request_time.clone(),
                    circuit_breaker: old_backend.circuit_breaker.clone(),
                });

                updated_backends.insert(backend_id.clone(), updated_backend);

                tracing::info!(
                    backend_id = %backend_id,
                    old_weight = old_backend.weight,
                    new_weight = new_weight,
                    "Backend recreated with updated weight"
                );
            } else {
                // Weight unchanged, keep existing backend
                updated_backends.insert(backend_id, Arc::clone(&old_backend));
            }
        }

        // Replace backends with updated versions
        self.backends = Arc::new(updated_backends);
    }

    pub fn cleanup_expired_sessions(&self) {
        let now = Instant::now();
        let timeout = self.strategy.session_affinity_timeout;

        self.session_affinity.retain(|_, backend_id| {
            if let Some(backend) = self.backends.get(backend_id) {
                let last_request = backend.last_request_time.try_read().map(|time| *time).unwrap_or(now);

                now.duration_since(last_request) < timeout
            } else {
                false
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct BackendStats {
    pub id: String,
    pub name: String,
    pub address: String,
    pub weight: u32,
    pub current_connections: usize,
    pub total_requests: usize,
    pub failed_requests: usize,
    pub average_response_time: f64,
    pub error_rate: f64,
    pub health_status: HealthStatus,
    pub is_circuit_breaker_open: bool,
}

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

//! Connection pooling for gRPC clients and load balancing

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use dashmap::DashMap;
use tonic::transport::{Channel, Endpoint};
use tracing::{error, info, warn};
use std::collections::VecDeque;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub max_connections_per_endpoint: usize,
    pub max_idle_time: Duration,
    pub connection_timeout: Duration,
    pub health_check_interval: Duration,
    pub max_retries: usize,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_endpoint: 10,
            max_idle_time: Duration::from_secs(300), // 5 minutes
            connection_timeout: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

/// Connection wrapper with metadata
#[derive(Debug)]
struct PooledConnection {
    channel: Channel,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
    is_healthy: bool,
}

impl PooledConnection {
    fn new(channel: Channel) -> Self {
        let now = Instant::now();
        Self {
            channel,
            created_at: now,
            last_used: now,
            use_count: 0,
            is_healthy: true,
        }
    }

    fn is_expired(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() > max_idle_time
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }
}

/// Connection pool for managing gRPC channels
pub struct ConnectionPool {
    config: ConnectionPoolConfig,
    pools: DashMap<String, Arc<RwLock<VecDeque<PooledConnection>>>>,
    semaphores: DashMap<String, Arc<Semaphore>>,
}

impl ConnectionPool {
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let pool = Self {
            config,
            pools: DashMap::new(),
            semaphores: DashMap::new(),
        };

        // Start background cleanup task
        let pool_clone = Arc::new(pool);
        let cleanup_pool = pool_clone.clone();
        tokio::spawn(async move {
            cleanup_pool.cleanup_task().await;
        });

        // Extract the pool from Arc for return
        Arc::try_unwrap(pool_clone).unwrap_or_else(|_| unreachable!())
    }

    /// Get a connection from the pool or create a new one
    pub async fn get_connection(&self, endpoint: &str) -> Result<Channel, tonic::transport::Error> {
        let semaphore = self.get_or_create_semaphore(endpoint);
        
        // Acquire permit to limit concurrent connections
        let _permit = semaphore.acquire().await.unwrap();

        // Try to get an existing connection
        if let Some(connection) = self.try_get_existing_connection(endpoint).await {
            return Ok(connection);
        }

        // Create new connection
        self.create_new_connection(endpoint).await
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, endpoint: &str, channel: Channel) {
        let pool = self.get_or_create_pool(endpoint);
        let mut pool_guard = pool.write().await;
        
        let mut conn = PooledConnection::new(channel);
        conn.mark_used();
        
        // Only keep if under limit
        if pool_guard.len() < self.config.max_connections_per_endpoint {
            pool_guard.push_back(conn);
        }
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> ConnectionPoolStats {
        let mut total_connections = 0;
        let mut total_endpoints = 0;
        let mut healthy_connections = 0;

        for entry in self.pools.iter() {
            total_endpoints += 1;
            let pool = entry.value().read().await;
            total_connections += pool.len();
            healthy_connections += pool.iter().filter(|c| c.is_healthy).count();
        }

        ConnectionPoolStats {
            total_endpoints,
            total_connections,
            healthy_connections,
        }
    }

    fn get_or_create_semaphore(&self, endpoint: &str) -> Arc<Semaphore> {
        self.semaphores
            .entry(endpoint.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(self.config.max_connections_per_endpoint)))
            .clone()
    }

    fn get_or_create_pool(&self, endpoint: &str) -> Arc<RwLock<VecDeque<PooledConnection>>> {
        self.pools
            .entry(endpoint.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(VecDeque::new())))
            .clone()
    }

    async fn try_get_existing_connection(&self, endpoint: &str) -> Option<Channel> {
        let pool = self.get_or_create_pool(endpoint);
        let mut pool_guard = pool.write().await;

        while let Some(mut conn) = pool_guard.pop_front() {
            if !conn.is_expired(self.config.max_idle_time) && conn.is_healthy {
                conn.mark_used();
                let channel = conn.channel.clone();
                pool_guard.push_back(conn);
                return Some(channel);
            }
            // Connection expired or unhealthy, don't return it to pool
        }

        None
    }

    async fn create_new_connection(&self, endpoint: &str) -> Result<Channel, tonic::transport::Error> {
        info!("Creating new connection to: {}", endpoint);
        
        let endpoint = Endpoint::from_shared(endpoint.to_string())?
            .timeout(self.config.connection_timeout)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .keep_alive_while_idle(true);

        let channel = endpoint.connect().await?;
        
        info!("Successfully created connection to: {}", endpoint.uri());
        Ok(channel)
    }

    async fn cleanup_task(&self) {
        let mut interval = tokio::time::interval(self.config.health_check_interval);
        
        loop {
            interval.tick().await;
            self.cleanup_expired_connections().await;
        }
    }

    async fn cleanup_expired_connections(&self) {
        let mut total_cleaned = 0;
        
        for entry in self.pools.iter() {
            let endpoint = entry.key();
            let pool = entry.value();
            let mut pool_guard = pool.write().await;
            
            let original_len = pool_guard.len();
            pool_guard.retain(|conn| {
                !conn.is_expired(self.config.max_idle_time)
            });
            
            let cleaned = original_len - pool_guard.len();
            total_cleaned += cleaned;
            
            if cleaned > 0 {
                info!("Cleaned {} expired connections for endpoint: {}", cleaned, endpoint);
            }
        }
        
        if total_cleaned > 0 {
            info!("Total expired connections cleaned: {}", total_cleaned);
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub total_endpoints: usize,
    pub total_connections: usize,
    pub healthy_connections: usize,
}

/// Load balancer for distributing requests across multiple endpoints
pub struct LoadBalancer {
    endpoints: Vec<String>,
    current_index: Arc<RwLock<usize>>,
    pool: Arc<ConnectionPool>,
}

impl LoadBalancer {
    pub fn new(endpoints: Vec<String>, pool: Arc<ConnectionPool>) -> Self {
        Self {
            endpoints,
            current_index: Arc::new(RwLock::new(0)),
            pool,
        }
    }

    /// Get next endpoint using round-robin
    pub async fn get_next_endpoint(&self) -> Option<String> {
        if self.endpoints.is_empty() {
            return None;
        }

        let mut index = self.current_index.write().await;
        let endpoint = self.endpoints[*index].clone();
        *index = (*index + 1) % self.endpoints.len();
        
        Some(endpoint)
    }

    /// Get a connection with load balancing
    pub async fn get_connection(&self) -> Result<(String, Channel), tonic::transport::Error> {
        let mut last_error = None;
        
        // Try each endpoint once
        for _ in 0..self.endpoints.len() {
            if let Some(endpoint) = self.get_next_endpoint().await {
                match self.pool.get_connection(&endpoint).await {
                    Ok(channel) => return Ok((endpoint, channel)),
                    Err(e) => {
                        warn!("Failed to connect to {}: {}", endpoint, e);
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            tonic::transport::Error::new_io_error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No endpoints available",
            ))
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_endpoints, 0);
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_load_balancer_round_robin() {
        let endpoints = vec![
            "http://localhost:50051".to_string(),
            "http://localhost:50052".to_string(),
            "http://localhost:50053".to_string(),
        ];
        
        let pool = Arc::new(ConnectionPool::new(ConnectionPoolConfig::default()));
        let lb = LoadBalancer::new(endpoints.clone(), pool);
        
        // Test round-robin behavior
        for i in 0..6 {
            let endpoint = lb.get_next_endpoint().await.unwrap();
            assert_eq!(endpoint, endpoints[i % 3]);
        }
    }
}
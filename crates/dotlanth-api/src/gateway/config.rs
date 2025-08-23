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

//! Gateway bridge configuration and main coordinator

use super::bridge::Bridge;
use crate::error::{ApiError, ApiResult};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Gateway bridge configuration
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Maximum request timeout for transcoding operations
    pub max_timeout_ms: u64,
    /// Enable request/response validation
    pub enable_validation: bool,
    /// Enable performance metrics collection
    pub enable_metrics: bool,
    /// Maximum concurrent streaming connections
    pub max_streaming_connections: usize,
    /// Buffer size for streaming operations
    pub streaming_buffer_size: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            max_timeout_ms: 5000, // 5 seconds max overhead
            enable_validation: true,
            enable_metrics: true,
            max_streaming_connections: 1000,
            streaming_buffer_size: 8192,
        }
    }
}

/// Gateway bridge metrics
#[derive(Debug, Default, Clone)]
pub struct GatewayMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub active_streaming_connections: u64,
    pub protocol_conversions: u64,
}

/// Main gateway bridge coordinator
pub struct GatewayBridge {
    config: GatewayConfig,
    metrics: Arc<RwLock<GatewayMetrics>>,
    bridge: Arc<Bridge>,
}

impl GatewayBridge {
    /// Create a new gateway bridge
    pub async fn new(config: GatewayConfig, auth_service: Arc<tokio::sync::Mutex<crate::auth::AuthService>>) -> ApiResult<Self> {
        info!("Initializing gRPC-HTTP Gateway Bridge");

        let metrics = Arc::new(RwLock::new(GatewayMetrics::default()));
        let bridge = Arc::new(Bridge::new(config.clone(), auth_service).await?);

        Ok(Self { config, metrics, bridge })
    }

    /// Get the bridge instance
    pub fn bridge(&self) -> Arc<Bridge> {
        self.bridge.clone()
    }

    /// Get current gateway metrics
    pub async fn get_metrics(&self) -> GatewayMetrics {
        self.bridge.get_metrics().await
    }

    /// Update metrics with request result
    async fn update_metrics(&self, success: bool, latency_ms: f64) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }

        // Update rolling average latency
        let total_successful = metrics.successful_requests.max(1);
        metrics.avg_latency_ms = (metrics.avg_latency_ms * (total_successful - 1) as f64 + latency_ms) / total_successful as f64;
    }

    /// Check if gateway is healthy
    pub async fn health_check(&self) -> ApiResult<()> {
        self.bridge.health_check().await
    }

    /// Get gateway configuration
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_config_default() {
        let config = GatewayConfig::default();
        assert_eq!(config.max_timeout_ms, 5000);
        assert!(config.enable_validation);
        assert!(config.enable_metrics);
        assert_eq!(config.max_streaming_connections, 1000);
        assert_eq!(config.streaming_buffer_size, 8192);
    }

    #[test]
    fn test_gateway_metrics_default() {
        let metrics = GatewayMetrics::default();
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.avg_latency_ms, 0.0);
        assert_eq!(metrics.active_streaming_connections, 0);
        assert_eq!(metrics.protocol_conversions, 0);
    }

    #[tokio::test]
    async fn test_gateway_bridge_creation() {
        let config = GatewayConfig::default();
        let auth_service = Arc::new(tokio::sync::Mutex::new(crate::auth::AuthService::new("test-secret").unwrap()));

        let bridge = GatewayBridge::new(config, auth_service).await;
        assert!(bridge.is_ok());
    }
}

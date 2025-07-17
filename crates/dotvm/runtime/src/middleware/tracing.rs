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

//! Enhanced tracing middleware for gRPC services

use std::time::Instant;
use tonic::{Request, Status};
use tracing::{info, error, warn, debug, Span};
use uuid::Uuid;

/// Tracing interceptor for gRPC requests
#[derive(Clone)]
pub struct TracingInterceptor {
    service_name: String,
    include_request_body: bool,
    include_response_body: bool,
}

impl TracingInterceptor {
    pub fn new(service_name: String) -> Self {
        Self {
            service_name,
            include_request_body: false,
            include_response_body: false,
        }
    }

    pub fn with_body_logging(mut self, include_request: bool, include_response: bool) -> Self {
        self.include_request_body = include_request;
        self.include_response_body = include_response;
        self
    }

    pub fn intercept<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        let start_time = Instant::now();
        let request_id = Uuid::new_v4().to_string();
        let method = "grpc_method".to_string(); // Simplified for now
        
        // Extract client information
        let client_ip = request
            .metadata()
            .get("x-forwarded-for")
            .or_else(|| request.metadata().get("x-real-ip"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let user_agent = request
            .metadata()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let user_id = request
            .metadata()
            .get("user-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous");

        // Create tracing span
        let span = tracing::info_span!(
            "grpc_request",
            service = %self.service_name,
            method = %method,
            request_id = %request_id,
            client_ip = %client_ip,
            user_agent = %user_agent,
            user_id = %user_id,
        );

        // Add request metadata to extensions
        let request_metadata = RequestMetadata {
            request_id: request_id.clone(),
            method: method.clone(),
            start_time,
            client_ip: client_ip.to_string(),
            user_agent: user_agent.to_string(),
            user_id: user_id.to_string(),
        };

        // Add metadata to request extensions
        {
            let extensions = request.extensions_mut();
            extensions.insert(request_metadata);
            extensions.insert(span.clone());
        }

        // Log request start
        let _enter = span.enter();
        info!(
            "gRPC request started: {} from {}",
            method, client_ip
        );

        if self.include_request_body {
            debug!("Request metadata: {:?}", request.metadata());
        }

        Ok(request)
    }

    pub fn log_response<T>(&self, request: &Request<T>, result: &Result<tonic::Response<T>, Status>) {
        if let Some(metadata) = request.extensions().get::<RequestMetadata>() {
            let duration = metadata.start_time.elapsed();
            
            if let Some(span) = request.extensions().get::<Span>() {
                let _enter = span.enter();
                
                match result {
                    Ok(_response) => {
                        info!(
                            "gRPC request completed successfully: {} in {:?}",
                            metadata.method, duration
                        );
                        
                        if self.include_response_body {
                            debug!("Response sent successfully");
                        }
                    }
                    Err(status) => {
                        match status.code() {
                            tonic::Code::InvalidArgument | 
                            tonic::Code::NotFound | 
                            tonic::Code::AlreadyExists => {
                                warn!(
                                    "gRPC request failed with client error: {} - {} in {:?}",
                                    metadata.method, status.message(), duration
                                );
                            }
                            _ => {
                                error!(
                                    "gRPC request failed with server error: {} - {} in {:?}",
                                    metadata.method, status.message(), duration
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Request metadata for tracing
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub request_id: String,
    pub method: String,
    pub start_time: Instant,
    pub client_ip: String,
    pub user_agent: String,
    pub user_id: String,
}

/// Helper to extract request metadata
pub fn extract_request_metadata<T>(request: &Request<T>) -> Option<&RequestMetadata> {
    request.extensions().get::<RequestMetadata>()
}

/// Performance metrics collector
pub struct PerformanceMetrics {
    request_count: std::sync::atomic::AtomicU64,
    total_duration: std::sync::atomic::AtomicU64,
    error_count: std::sync::atomic::AtomicU64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            request_count: std::sync::atomic::AtomicU64::new(0),
            total_duration: std::sync::atomic::AtomicU64::new(0),
            error_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn record_request(&self, duration: std::time::Duration, is_error: bool) {
        use std::sync::atomic::Ordering;
        
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.total_duration.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        
        if is_error {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get_stats(&self) -> PerformanceStats {
        use std::sync::atomic::Ordering;
        
        let request_count = self.request_count.load(Ordering::Relaxed);
        let total_duration = self.total_duration.load(Ordering::Relaxed);
        let error_count = self.error_count.load(Ordering::Relaxed);
        
        let avg_duration = if request_count > 0 {
            total_duration as f64 / request_count as f64
        } else {
            0.0
        };
        
        let error_rate = if request_count > 0 {
            error_count as f64 / request_count as f64
        } else {
            0.0
        };

        PerformanceStats {
            request_count,
            total_duration_ms: total_duration,
            error_count,
            avg_duration_ms: avg_duration,
            error_rate,
        }
    }

    pub fn reset(&self) {
        use std::sync::atomic::Ordering;
        
        self.request_count.store(0, Ordering::Relaxed);
        self.total_duration.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
    }
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub request_count: u64,
    pub total_duration_ms: u64,
    pub error_count: u64,
    pub avg_duration_ms: f64,
    pub error_rate: f64,
}

/// Structured logging for gRPC events
pub struct StructuredLogger;

impl StructuredLogger {
    pub fn log_dot_execution(
        dot_id: &str,
        user_id: &str,
        duration: std::time::Duration,
        success: bool,
        paradots_used: &[String],
    ) {
        if success {
            info!(
                dot_id = %dot_id,
                user_id = %user_id,
                duration_ms = duration.as_millis(),
                paradots_count = paradots_used.len(),
                paradots = ?paradots_used,
                "Dot execution completed successfully"
            );
        } else {
            error!(
                dot_id = %dot_id,
                user_id = %user_id,
                duration_ms = duration.as_millis(),
                "Dot execution failed"
            );
        }
    }

    pub fn log_abi_operation(
        operation: &str,
        dot_id: &str,
        user_id: &str,
        success: bool,
        details: Option<&str>,
    ) {
        if success {
            info!(
                operation = %operation,
                dot_id = %dot_id,
                user_id = %user_id,
                details = ?details,
                "ABI operation completed successfully"
            );
        } else {
            error!(
                operation = %operation,
                dot_id = %dot_id,
                user_id = %user_id,
                details = ?details,
                "ABI operation failed"
            );
        }
    }

    pub fn log_security_event(
        event_type: &str,
        user_id: &str,
        client_ip: &str,
        details: &str,
        severity: SecuritySeverity,
    ) {
        match severity {
            SecuritySeverity::Low => {
                info!(
                    event_type = %event_type,
                    user_id = %user_id,
                    client_ip = %client_ip,
                    details = %details,
                    severity = "low",
                    "Security event"
                );
            }
            SecuritySeverity::Medium => {
                warn!(
                    event_type = %event_type,
                    user_id = %user_id,
                    client_ip = %client_ip,
                    details = %details,
                    severity = "medium",
                    "Security event"
                );
            }
            SecuritySeverity::High => {
                error!(
                    event_type = %event_type,
                    user_id = %user_id,
                    client_ip = %client_ip,
                    details = %details,
                    severity = "high",
                    "Security event"
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        
        // Record some requests
        metrics.record_request(std::time::Duration::from_millis(100), false);
        metrics.record_request(std::time::Duration::from_millis(200), false);
        metrics.record_request(std::time::Duration::from_millis(150), true);
        
        let stats = metrics.get_stats();
        assert_eq!(stats.request_count, 3);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.error_rate, 1.0 / 3.0);
        assert_eq!(stats.avg_duration_ms, 150.0);
    }

    #[test]
    fn test_tracing_interceptor() {
        let interceptor = TracingInterceptor::new("test_service".to_string());
        
        let request = Request::new(());
        let result = interceptor.intercept(request);
        
        assert!(result.is_ok());
        let request = result.unwrap();
        assert!(request.extensions().get::<RequestMetadata>().is_some());
    }
}
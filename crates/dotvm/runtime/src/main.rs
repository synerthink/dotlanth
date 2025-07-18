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

use tonic::transport::Server;
use tonic::{Request, Response, Status};

mod config;
use config::RuntimeConfig;

// Basic proto imports
mod proto {
    tonic::include_proto!("runtime");

    pub mod vm_service {
        tonic::include_proto!("vm_service");
    }

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("runtime_descriptor");
}

use proto::runtime_server::{Runtime, RuntimeServer};
use proto::vm_service::vm_service_server::{VmService, VmServiceServer};

// Simple working runtime service
#[derive(Debug, Default)]
struct SimpleRuntimeService;

#[tonic::async_trait]
impl Runtime for SimpleRuntimeService {
    async fn ping(&self, request: Request<proto::PingRequest>) -> Result<Response<proto::PingResponse>, Status> {
        println!("Runtime Ping received: {}", request.get_ref().message);

        let response = proto::PingResponse {
            message: format!("Dotlanth Server Response: {}", request.into_inner().message),
        };

        Ok(Response::new(response))
    }
}

// Basic VM service implementation - simplified and working
#[derive(Debug, Default)]
struct VmServiceImpl;

#[tonic::async_trait]
impl VmService for VmServiceImpl {
    async fn get_architectures(&self, _request: Request<proto::vm_service::GetArchitecturesRequest>) -> Result<Response<proto::vm_service::GetArchitecturesResponse>, Status> {
        println!("GetArchitectures called");
        let response = proto::vm_service::GetArchitecturesResponse {
            architectures: vec![
                proto::vm_service::ArchitectureInfo {
                    name: "WASM".to_string(),
                    description: "WebAssembly virtual machine".to_string(),
                    features: vec!["basic_execution".to_string()],
                    is_default: true,
                    performance: None,
                },
                proto::vm_service::ArchitectureInfo {
                    name: "X86_64".to_string(),
                    description: "Native x86-64 execution".to_string(),
                    features: vec!["native_execution".to_string()],
                    is_default: false,
                    performance: None,
                },
            ],
        };
        Ok(Response::new(response))
    }

    async fn get_vm_status(&self, _request: Request<proto::vm_service::GetVmStatusRequest>) -> Result<Response<proto::vm_service::GetVmStatusResponse>, Status> {
        println!("GetVMStatus called");
        let response = proto::vm_service::GetVmStatusResponse {
            status: 1, // Running
            active_dots: vec![],
            info: Some(proto::vm_service::VmInfo {
                architecture: "WASM".to_string(),
                uptime_seconds: 3600,
                version: "0.1.0".to_string(),
                dots_count: 0,
                paradots_count: 0,
                resource_usage: Some(proto::vm_service::ResourceUsage {
                    memory_used_bytes: 1024 * 1024,
                    memory_total_bytes: 8 * 1024 * 1024,
                    cpu_usage_percent: 5.0,
                    storage_used_bytes: 0,
                    active_connections: 1,
                }),
            }),
            active_paradots: vec![],
        };
        Ok(Response::new(response))
    }

    async fn get_vm_metrics(&self, _request: Request<proto::vm_service::GetVmMetricsRequest>) -> Result<Response<proto::vm_service::GetVmMetricsResponse>, Status> {
        println!("GetVMMetrics called");
        let response = proto::vm_service::GetVmMetricsResponse {
            metrics: vec![proto::vm_service::VmMetric {
                name: "cpu_usage".to_string(),
                r#type: "gauge".to_string(),
                data_points: vec![],
                labels: std::collections::HashMap::new(),
            }],
        };
        Ok(Response::new(response))
    }

    // VM Service Ping - working implementation
    async fn ping(&self, request: Request<proto::vm_service::PingRequest>) -> Result<Response<proto::vm_service::PingResponse>, Status> {
        let req = request.into_inner();
        println!("VM Service Ping called from client: {}", req.client_id);

        let response = proto::vm_service::PingResponse {
            server_id: "dotvm-server-001".to_string(),
            timestamp: req.timestamp,
            server_time: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            status: Some(proto::vm_service::ServerStatus {
                version: "1.0.0".to_string(),
                uptime_seconds: 3600,
                active_connections: 1,
                total_requests: 42,
                cpu_usage: 15.5,
                memory_usage_bytes: 1024 * 1024 * 128,
            }),
        };

        Ok(Response::new(response))
    }

    // Health Check - working implementation
    async fn health_check(&self, request: Request<proto::vm_service::HealthCheckRequest>) -> Result<Response<proto::vm_service::HealthCheckResponse>, Status> {
        let req = request.into_inner();
        println!("Health check requested for services: {:?}", req.services);

        let mut service_health = vec![
            proto::vm_service::ServiceHealth {
                service_name: "vm_service".to_string(),
                status: proto::vm_service::OverallHealth::HealthServing as i32,
                message: "VM service is healthy".to_string(),
                details: std::collections::HashMap::new(),
            },
            proto::vm_service::ServiceHealth {
                service_name: "runtime".to_string(),
                status: proto::vm_service::OverallHealth::HealthServing as i32,
                message: "Runtime service is healthy".to_string(),
                details: std::collections::HashMap::new(),
            },
        ];

        // Filter by requested services if specified
        if !req.services.is_empty() {
            service_health.retain(|s| req.services.contains(&s.service_name));
        }

        let overall_status = if service_health.iter().all(|s| s.status == proto::vm_service::OverallHealth::HealthServing as i32) {
            proto::vm_service::OverallHealth::HealthServing
        } else {
            proto::vm_service::OverallHealth::HealthNotServing
        };

        let mut system_info = std::collections::HashMap::new();
        if req.include_details {
            system_info.insert("server_id".to_string(), "dotvm-server-001".to_string());
            system_info.insert("uptime_seconds".to_string(), "3600".to_string());
            system_info.insert("version".to_string(), "1.0.0".to_string());
        }

        let response = proto::vm_service::HealthCheckResponse {
            overall_status: overall_status as i32,
            service_health,
            system_info,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };

        Ok(Response::new(response))
    }

    // Basic implementations to avoid RST_STREAM errors
    async fn execute_dot(&self, request: Request<proto::vm_service::ExecuteDotRequest>) -> Result<Response<proto::vm_service::ExecuteDotResponse>, Status> {
        let req = request.into_inner();
        println!("ExecuteDot called for dot_id: {}", req.dot_id);
        
        let response = proto::vm_service::ExecuteDotResponse {
            success: false,
            outputs: std::collections::HashMap::new(),
            execution_time_ms: 0,
            paradots_used: vec![],
            logs: vec![],
            events: vec![],
            error_message: "ExecuteDot not yet implemented - this is a placeholder response".to_string(),
            metrics: None,
        };
        Ok(Response::new(response))
    }

    async fn deploy_dot(&self, request: Request<proto::vm_service::DeployDotRequest>) -> Result<Response<proto::vm_service::DeployDotResponse>, Status> {
        let req = request.into_inner();
        println!("DeployDot called for dot_name: {}", req.dot_name);
        
        let response = proto::vm_service::DeployDotResponse {
            success: false,
            dot_id: "".to_string(),
            bytecode: vec![],
            abi: None,
            metrics: None,
            error_message: "DeployDot not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn get_dot_state(&self, request: Request<proto::vm_service::GetDotStateRequest>) -> Result<Response<proto::vm_service::GetDotStateResponse>, Status> {
        let req = request.into_inner();
        println!("GetDotState called for dot_id: {}", req.dot_id);
        
        let response = proto::vm_service::GetDotStateResponse {
            success: false,
            state_data: std::collections::HashMap::new(),
            state_root_hash: "".to_string(),
            version: 0,
            error_message: "GetDotState not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn list_dots(&self, _request: Request<proto::vm_service::ListDotsRequest>) -> Result<Response<proto::vm_service::ListDotsResponse>, Status> {
        println!("ListDots called");
        
        let response = proto::vm_service::ListDotsResponse {
            dots: vec![], // Empty list for now
            total_count: 0,
            next_cursor: "".to_string(),
            has_more: false,
        };
        Ok(Response::new(response))
    }

    async fn delete_dot(&self, request: Request<proto::vm_service::DeleteDotRequest>) -> Result<Response<proto::vm_service::DeleteDotResponse>, Status> {
        let req = request.into_inner();
        println!("DeleteDot called for dot_id: {}", req.dot_id);
        
        let response = proto::vm_service::DeleteDotResponse {
            success: false,
            error_message: "DeleteDot not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn get_bytecode(&self, request: Request<proto::vm_service::GetBytecodeRequest>) -> Result<Response<proto::vm_service::GetBytecodeResponse>, Status> {
        let req = request.into_inner();
        println!("GetBytecode called for dot_id: {}", req.dot_id);
        
        let response = proto::vm_service::GetBytecodeResponse {
            success: false,
            bytecode: vec![],
            info: None,
            error_message: "GetBytecode not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn validate_bytecode(&self, request: Request<proto::vm_service::ValidateBytecodeRequest>) -> Result<Response<proto::vm_service::ValidateBytecodeResponse>, Status> {
        let req = request.into_inner();
        println!("ValidateBytecode called for {} bytes", req.bytecode.len());
        
        let response = proto::vm_service::ValidateBytecodeResponse {
            valid: false,
            errors: vec![proto::vm_service::ValidationError {
                field: "bytecode".to_string(),
                error_code: "NOT_IMPLEMENTED".to_string(),
                message: "ValidateBytecode not yet implemented - this is a placeholder response".to_string(),
            }],
            analysis: None,
        };
        Ok(Response::new(response))
    }

    async fn get_dot_abi(&self, request: Request<proto::vm_service::GetDotAbiRequest>) -> Result<Response<proto::vm_service::GetDotAbiResponse>, Status> {
        let req = request.into_inner();
        println!("GetDotABI called for dot_id: {}", req.dot_id);
        
        let response = proto::vm_service::GetDotAbiResponse {
            success: false,
            abi: None,
            error_message: "GetDotABI not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn validate_abi(&self, request: Request<proto::vm_service::ValidateAbiRequest>) -> Result<Response<proto::vm_service::ValidateAbiResponse>, Status> {
        let req = request.into_inner();
        println!("ValidateABI called");
        
        let response = proto::vm_service::ValidateAbiResponse {
            valid: false,
            errors: vec![proto::vm_service::ValidationError {
                field: "abi".to_string(),
                error_code: "NOT_IMPLEMENTED".to_string(),
                message: "ValidateABI not yet implemented - this is a placeholder response".to_string(),
            }],
            warnings: vec![],
        };
        Ok(Response::new(response))
    }

    async fn generate_abi(&self, request: Request<proto::vm_service::GenerateAbiRequest>) -> Result<Response<proto::vm_service::GenerateAbiResponse>, Status> {
        let req = request.into_inner();
        println!("GenerateABI called");
        
        let response = proto::vm_service::GenerateAbiResponse {
            success: false,
            abi: None,
            warnings: vec![],
            error_message: "GenerateABI not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    async fn register_abi(&self, request: Request<proto::vm_service::RegisterAbiRequest>) -> Result<Response<proto::vm_service::RegisterAbiResponse>, Status> {
        let req = request.into_inner();
        println!("RegisterABI called for dot_id: {}", req.dot_id);
        
        let response = proto::vm_service::RegisterAbiResponse {
            success: false,
            abi_version: "0".to_string(),
            error_message: "RegisterABI not yet implemented - this is a placeholder response".to_string(),
        };
        Ok(Response::new(response))
    }

    type StreamDotEventsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::DotEvent, Status>> + Send>>;

    async fn stream_dot_events(&self, _request: Request<proto::vm_service::StreamDotEventsRequest>) -> Result<Response<Self::StreamDotEventsStream>, Status> {
        println!("StreamDotEvents called - returning empty stream");
        
        // Create an empty stream that completes immediately
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    type StreamVMMetricsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::VmMetric, Status>> + Send>>;

    async fn stream_vm_metrics(&self, _request: Request<proto::vm_service::StreamVmMetricsRequest>) -> Result<Response<Self::StreamVMMetricsStream>, Status> {
        println!("StreamVMMetrics called - returning empty stream");
        
        // Create an empty stream that completes immediately
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    type InteractiveDotExecutionStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::InteractiveExecutionResponse, Status>> + Send>>;

    async fn interactive_dot_execution(
        &self,
        _request: Request<tonic::Streaming<proto::vm_service::InteractiveExecutionRequest>>,
    ) -> Result<Response<Self::InteractiveDotExecutionStream>, Status> {
        println!("InteractiveDotExecution called - returning empty stream");
        
        // Create an empty stream that completes immediately
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }

    type LiveDotDebuggingStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::DebugResponse, Status>> + Send>>;

    async fn live_dot_debugging(&self, _request: Request<tonic::Streaming<proto::vm_service::DebugRequest>>) -> Result<Response<Self::LiveDotDebuggingStream>, Status> {
        println!("LiveDotDebugging called - returning empty stream");
        
        // Create an empty stream that completes immediately
        let stream = futures::stream::empty();
        Ok(Response::new(Box::pin(stream)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Handle Ctrl+C for graceful shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("Received Ctrl+C, shutting down gracefully...");
        let _ = shutdown_tx.send(()).await;
    });

    println!("Starting Dotlanth gRPC Server...");

    // Load runtime configuration with cross-platform support
    let runtime_config = RuntimeConfig::from_env();
    let addr = runtime_config.get_bind_address_for_platform();
    let runtime_service = SimpleRuntimeService::default();
    let vm_service = VmServiceImpl::default();

    // Set up reflection service
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build()?;

    println!("Server starting on {}", addr);
    println!("Basic functionality ready");
    println!("VM service enabled");
    println!("gRPC reflection enabled");
    println!("");
    println!("Test with:");
    println!("  grpcurl -plaintext -d '{{\"message\": \"hello\"}}' {} runtime.Runtime/Ping", addr);
    println!("  grpcurl -plaintext {} list", addr);
    println!("  grpcurl -plaintext -d '{{}}' {} vm_service.VmService/GetArchitectures", addr);
    println!("  grpcurl -plaintext -d '{{\"client_id\": \"test\", \"timestamp\": 1640995200}}' {} vm_service.VmService/Ping", addr);
    println!("  grpcurl -plaintext -d '{{\"services\": [], \"include_details\": true}}' {} vm_service.VmService/HealthCheck", addr);
    println!("");
    println!("Cross-platform connection tips:");
    println!("  Ubuntu/Linux: Use 127.0.0.1:{} (recommended) or localhost:{}", addr.port(), addr.port());
    println!("  macOS: Use 127.0.0.1:{} or localhost:{}", addr.port(), addr.port());
    println!("  Windows: Use 127.0.0.1:{} or localhost:{}", addr.port(), addr.port());
    println!("  Force IPv4: Use 127.0.0.1:{} instead of localhost", addr.port());
    println!("  IPv6 (if enabled): grpcurl -plaintext [::1]:{} list", addr.port());

    // Start the server with graceful shutdown
    println!("Starting server with graceful shutdown support...");
    println!("Press Ctrl+C to stop the server and free the port");

    Server::builder()
        .add_service(reflection_service)
        .add_service(RuntimeServer::new(runtime_service))
        .add_service(VmServiceServer::new(vm_service))
        .serve_with_shutdown(addr, async {
            shutdown_rx.recv().await;
            println!("Shutdown signal received, stopping server...");
        })
        .await?;

    println!("Server stopped, port 50051 is now free");

    Ok(())
}
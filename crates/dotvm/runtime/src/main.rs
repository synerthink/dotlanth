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

use std::sync::Arc;
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

// Import our advanced services (simplified for now)
// mod services {
//     pub mod health;
// }
// use services::health::{HealthService, ServingStatus};

use proto::runtime_server::{Runtime, RuntimeServer};
use proto::vm_service::vm_service_server::{VmService, VmServiceServer};

// Health service proto (commented out for now)
// pub mod health_proto {
//     tonic::include_proto!("grpc.health.v1");
// }
// use health_proto::health_server::{Health, HealthServer};

// Simple working runtime service
#[derive(Debug, Default)]
struct SimpleRuntimeService;

// Health service implementation (simplified for now)
// Will be added back once proto issues are resolved

#[tonic::async_trait]
impl Runtime for SimpleRuntimeService {
    async fn ping(&self, request: Request<proto::PingRequest>) -> Result<Response<proto::PingResponse>, Status> {
        println!("Received ping: {}", request.get_ref().message);

        let response = proto::PingResponse {
            message: format!("Dotlanth Server Response: {}", request.into_inner().message),
        };

        Ok(Response::new(response))
    }
}

// Basic VM service implementation
#[derive(Debug, Default)]
struct VmServiceImpl;

#[tonic::async_trait]
impl VmService for VmServiceImpl {
    async fn get_architectures(&self, _request: tonic::Request<proto::vm_service::GetArchitecturesRequest>) -> Result<tonic::Response<proto::vm_service::GetArchitecturesResponse>, tonic::Status> {
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
        Ok(tonic::Response::new(response))
    }

    async fn get_vm_status(&self, _request: tonic::Request<proto::vm_service::GetVmStatusRequest>) -> Result<tonic::Response<proto::vm_service::GetVmStatusResponse>, tonic::Status> {
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
        Ok(tonic::Response::new(response))
    }

    async fn get_vm_metrics(&self, _request: tonic::Request<proto::vm_service::GetVmMetricsRequest>) -> Result<tonic::Response<proto::vm_service::GetVmMetricsResponse>, tonic::Status> {
        let response = proto::vm_service::GetVmMetricsResponse {
            metrics: vec![proto::vm_service::VmMetric {
                name: "cpu_usage".to_string(),
                r#type: "gauge".to_string(),
                data_points: vec![],
                labels: std::collections::HashMap::new(),
            }],
        };
        Ok(tonic::Response::new(response))
    }

    // Placeholder implementations for required methods
    async fn execute_dot(&self, _request: tonic::Request<proto::vm_service::ExecuteDotRequest>) -> Result<tonic::Response<proto::vm_service::ExecuteDotResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("ExecuteDot not yet implemented"))
    }

    async fn deploy_dot(&self, _request: tonic::Request<proto::vm_service::DeployDotRequest>) -> Result<tonic::Response<proto::vm_service::DeployDotResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("DeployDot not yet implemented"))
    }

    async fn get_dot_state(&self, _request: tonic::Request<proto::vm_service::GetDotStateRequest>) -> Result<tonic::Response<proto::vm_service::GetDotStateResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("GetDotState not yet implemented"))
    }

    async fn list_dots(&self, _request: tonic::Request<proto::vm_service::ListDotsRequest>) -> Result<tonic::Response<proto::vm_service::ListDotsResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("ListDots not yet implemented"))
    }

    async fn delete_dot(&self, _request: tonic::Request<proto::vm_service::DeleteDotRequest>) -> Result<tonic::Response<proto::vm_service::DeleteDotResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("DeleteDot not yet implemented"))
    }

    async fn get_bytecode(&self, _request: tonic::Request<proto::vm_service::GetBytecodeRequest>) -> Result<tonic::Response<proto::vm_service::GetBytecodeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("GetBytecode not yet implemented"))
    }

    async fn validate_bytecode(&self, _request: tonic::Request<proto::vm_service::ValidateBytecodeRequest>) -> Result<tonic::Response<proto::vm_service::ValidateBytecodeResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("ValidateBytecode not yet implemented"))
    }

    async fn get_dot_abi(&self, _request: tonic::Request<proto::vm_service::GetDotAbiRequest>) -> Result<tonic::Response<proto::vm_service::GetDotAbiResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("GetDotABI not yet implemented"))
    }

    async fn validate_abi(&self, _request: tonic::Request<proto::vm_service::ValidateAbiRequest>) -> Result<tonic::Response<proto::vm_service::ValidateAbiResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("ValidateABI not yet implemented"))
    }

    async fn generate_abi(&self, _request: tonic::Request<proto::vm_service::GenerateAbiRequest>) -> Result<tonic::Response<proto::vm_service::GenerateAbiResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("GenerateABI not yet implemented"))
    }

    async fn register_abi(&self, _request: tonic::Request<proto::vm_service::RegisterAbiRequest>) -> Result<tonic::Response<proto::vm_service::RegisterAbiResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("RegisterABI not yet implemented"))
    }

    type StreamDotEventsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::DotEvent, tonic::Status>> + Send>>;

    async fn stream_dot_events(&self, _request: tonic::Request<proto::vm_service::StreamDotEventsRequest>) -> Result<tonic::Response<Self::StreamDotEventsStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("StreamDotEvents not yet implemented"))
    }

    type StreamVMMetricsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::VmMetric, tonic::Status>> + Send>>;

    async fn stream_vm_metrics(&self, _request: tonic::Request<proto::vm_service::StreamVmMetricsRequest>) -> Result<tonic::Response<Self::StreamVMMetricsStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("StreamVMMetrics not yet implemented"))
    }

    // Week 3: Advanced gRPC Features - Bidirectional Streaming
    type InteractiveDotExecutionStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::InteractiveExecutionResponse, tonic::Status>> + Send>>;

    async fn interactive_dot_execution(
        &self,
        _request: tonic::Request<tonic::Streaming<proto::vm_service::InteractiveExecutionRequest>>,
    ) -> Result<tonic::Response<Self::InteractiveDotExecutionStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("InteractiveDotExecution not yet implemented"))
    }

    type LiveDotDebuggingStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<proto::vm_service::DebugResponse, tonic::Status>> + Send>>;

    async fn live_dot_debugging(&self, _request: tonic::Request<tonic::Streaming<proto::vm_service::DebugRequest>>) -> Result<tonic::Response<Self::LiveDotDebuggingStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("LiveDotDebugging not yet implemented"))
    }

    // Week 3: Connection Management
    async fn ping(&self, request: tonic::Request<proto::vm_service::PingRequest>) -> Result<tonic::Response<proto::vm_service::PingResponse>, tonic::Status> {
        println!("VM Service Ping called from client: {}", request.get_ref().client_id);

        let response = proto::vm_service::PingResponse {
            server_id: "dotvm-server-001".to_string(),
            timestamp: request.get_ref().timestamp,
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

        Ok(tonic::Response::new(response))
    }

    async fn health_check(&self, request: tonic::Request<proto::vm_service::HealthCheckRequest>) -> Result<tonic::Response<proto::vm_service::HealthCheckResponse>, tonic::Status> {
        println!("Health check requested for services: {:?}", request.get_ref().services);

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
        if !request.get_ref().services.is_empty() {
            service_health.retain(|s| request.get_ref().services.contains(&s.service_name));
        }

        let overall_status = if service_health.iter().all(|s| s.status == proto::vm_service::OverallHealth::HealthServing as i32) {
            proto::vm_service::OverallHealth::HealthServing
        } else {
            proto::vm_service::OverallHealth::HealthNotServing
        };

        let mut system_info = std::collections::HashMap::new();
        if request.get_ref().include_details {
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

        Ok(tonic::Response::new(response))
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
    // Simple logging
    println!("Starting Dotlanth gRPC Server...");

    // Load runtime configuration with cross-platform support
    let runtime_config = RuntimeConfig::from_env();
    let addr = runtime_config.get_bind_address_for_platform();
    let runtime_service = SimpleRuntimeService::default();
    let vm_service = VmServiceImpl::default();
    // let health_service = HealthServiceImpl::new();

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
    println!("  grpcurl -plaintext {} vm_service.VmService/GetArchitectures", addr);
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
        // .add_service(HealthServer::new(health_service))  // Will add back later
        .serve_with_shutdown(addr, async {
            shutdown_rx.recv().await;
            println!("Shutdown signal received, stopping server...");
        })
        .await?;

    println!("Server stopped, port 50051 is now free");

    Ok(())
}

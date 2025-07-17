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

    let addr = "[::1]:50051".parse()?;
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
    println!("  grpcurl -plaintext -d '{{\"message\": \"hello\"}}' localhost:50051 runtime.Runtime/Ping");
    println!("  grpcurl -plaintext localhost:50051 list");
    println!("  grpcurl -plaintext localhost:50051 vm_service.VmService/GetArchitectures");

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

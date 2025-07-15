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

//! VM Service implementation for gRPC

use std::sync::Arc;
use tonic::{Request, Response, Result as TonicResult, Status};
use tracing::{error, info, instrument};
// TODO: Import actual VM and StateStorage when available
// use dotvm_core::vm::VirtualMachine;
// use dotdb_core::state::StateStorage;

// Import generated protobuf types
use crate::proto::vm_service::{vm_service_server::VmService, *};

use super::{AbiService, DotsService, MetricsService, VmManagementService};

/// VM Service implementation - coordinates all sub-services
pub struct VmServiceImpl {
    dots_service: Arc<DotsService>,
    abi_service: Arc<AbiService>,
    metrics_service: Arc<MetricsService>,
    vm_management_service: Arc<VmManagementService>,
}

impl VmServiceImpl {
    pub fn new() -> Self {
        Self {
            dots_service: Arc::new(DotsService::new()),
            abi_service: Arc::new(AbiService::new()),
            metrics_service: Arc::new(MetricsService::new()),
            vm_management_service: Arc::new(VmManagementService::new()),
        }
    }
}

#[tonic::async_trait]
impl VmService for VmServiceImpl {
    #[instrument(skip(self, request))]
    async fn execute_dot(&self, request: Request<ExecuteDotRequest>) -> TonicResult<Response<ExecuteDotResponse>> {
        // Delegate to dots service
        self.dots_service.execute_dot(request).await
    }

    #[instrument(skip(self, request))]
    async fn deploy_dot(&self, request: Request<DeployDotRequest>) -> TonicResult<Response<DeployDotResponse>> {
        // Delegate to dots service
        self.dots_service.deploy_dot(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_dot_state(&self, request: Request<GetDotStateRequest>) -> TonicResult<Response<GetDotStateResponse>> {
        // Delegate to dots service
        self.dots_service.get_dot_state(request).await
    }

    #[instrument(skip(self, request))]
    async fn list_dots(&self, request: Request<ListDotsRequest>) -> TonicResult<Response<ListDotsResponse>> {
        // Delegate to dots service
        self.dots_service.list_dots(request).await
    }

    #[instrument(skip(self, request))]
    async fn delete_dot(&self, request: Request<DeleteDotRequest>) -> TonicResult<Response<DeleteDotResponse>> {
        // Delegate to dots service
        self.dots_service.delete_dot(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_bytecode(&self, request: Request<GetBytecodeRequest>) -> TonicResult<Response<GetBytecodeResponse>> {
        let req = request.into_inner();

        info!("Getting bytecode for dot: {}", req.dot_id);

        // TODO: Implement bytecode retrieval
        let response = GetBytecodeResponse {
            success: true,
            bytecode: vec![0x01, 0x02, 0x03, 0x04], // Mock bytecode
            info: Some(BytecodeInfo {
                size_bytes: 4,
                architecture: "arch64".to_string(),
                compilation_target: "dotvm".to_string(),
                has_debug_info: false,
                dependencies: vec![],
            }),
            error_message: String::new(),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request))]
    async fn validate_bytecode(&self, request: Request<ValidateBytecodeRequest>) -> TonicResult<Response<ValidateBytecodeResponse>> {
        let req = request.into_inner();

        info!("Validating bytecode ({} bytes)", req.bytecode.len());

        // TODO: Implement bytecode validation
        let response = ValidateBytecodeResponse {
            valid: true,
            errors: vec![],
            analysis: Some(BytecodeAnalysis {
                instruction_count: 10,
                used_opcodes: vec!["LOAD".to_string(), "STORE".to_string(), "ADD".to_string()],
                estimated_cpu_cycles: 1000,
                security: Some(SecurityAnalysis {
                    has_unsafe_operations: false,
                    security_warnings: vec![],
                    complexity_score: 5,
                }),
            }),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request))]
    async fn get_dot_abi(&self, request: Request<GetDotAbiRequest>) -> TonicResult<Response<GetDotAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.get_dot_abi(request).await
    }

    #[instrument(skip(self, request))]
    async fn validate_abi(&self, request: Request<ValidateAbiRequest>) -> TonicResult<Response<ValidateAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.validate_abi(request).await
    }

    #[instrument(skip(self, request))]
    async fn generate_abi(&self, request: Request<GenerateAbiRequest>) -> TonicResult<Response<GenerateAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.generate_abi(request).await
    }

    #[instrument(skip(self, request))]
    async fn register_abi(&self, request: Request<RegisterAbiRequest>) -> TonicResult<Response<RegisterAbiResponse>> {
        // Delegate to ABI service
        self.abi_service.register_abi(request).await
    }

    // ParaDot operations removed - they are automatically managed during dot execution
    // ParaDots are spawned and coordinated internally based on dot requirements
    // See dots/paradots/ module for ParaDot management implementation

    #[instrument(skip(self, request))]
    async fn get_vm_status(&self, request: Request<GetVmStatusRequest>) -> TonicResult<Response<GetVmStatusResponse>> {
        // Delegate to VM management service
        self.vm_management_service.get_vm_status(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_vm_metrics(&self, request: Request<GetVmMetricsRequest>) -> TonicResult<Response<GetVmMetricsResponse>> {
        // Delegate to metrics service
        self.metrics_service.get_vm_metrics(request).await
    }

    #[instrument(skip(self, request))]
    async fn get_architectures(&self, request: Request<GetArchitecturesRequest>) -> TonicResult<Response<GetArchitecturesResponse>> {
        // Delegate to VM management service
        self.vm_management_service.get_architectures(request).await
    }

    // Streaming methods
    type StreamDotEventsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<DotEvent, Status>> + Send>>;
    type StreamVMMetricsStream = std::pin::Pin<Box<dyn futures::Stream<Item = Result<VmMetric, Status>> + Send>>;

    async fn stream_dot_events(&self, _request: Request<StreamDotEventsRequest>) -> TonicResult<Response<Self::StreamDotEventsStream>> {
        // TODO: Implement streaming
        Err(Status::unimplemented("Streaming not yet implemented"))
    }

    async fn stream_vm_metrics(&self, _request: Request<StreamVmMetricsRequest>) -> TonicResult<Response<Self::StreamVMMetricsStream>> {
        // TODO: Implement streaming
        Err(Status::unimplemented("Streaming not yet implemented"))
    }
}

// Required associated types for streaming are defined in the trait implementation above

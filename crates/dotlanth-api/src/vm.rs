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

//! VM client for interacting with the DotVM runtime via gRPC

use crate::error::{ApiError, ApiResult};
use crate::models::{DeployDotRequest, DeployDotResponse, DotState, DotStatus, ExecuteDotRequest, ExecuteDotResponse, ExecutionStatus, ValidationResult};
use base64::Engine;
use chrono::Utc;
use std::collections::HashMap;
use tonic::transport::Channel;
use tracing::{error, info, warn};
use uuid::Uuid;

// Import generated gRPC client
mod proto {
    tonic::include_proto!("vm_service");
}

use proto::vm_service_client::VmServiceClient;

/// VM client for interacting with DotVM via gRPC
#[derive(Clone)]
pub struct VmClient {
    client: VmServiceClient<Channel>,
}

impl VmClient {
    /// Create a new VM client
    pub async fn new(vm_endpoint: &str) -> ApiResult<Self> {
        info!("Connecting to VM service at: {}", vm_endpoint);

        let channel = Channel::from_shared(vm_endpoint.to_string())
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Invalid VM endpoint: {}", e),
            })?
            .connect()
            .await
            .map_err(|e| ApiError::InternalServerError {
                message: format!("Failed to connect to VM service: {}", e),
            })?;

        let client = VmServiceClient::new(channel);

        info!("Successfully connected to VM service");

        Ok(Self { client })
    }

    /// Deploy a new dot
    pub async fn deploy_dot(&self, request: DeployDotRequest) -> ApiResult<DeployDotResponse> {
        info!("Deploying dot: {}", request.name);

        let grpc_request = proto::DeployDotRequest {
            dot_name: request.name.clone(),
            dot_source: request.bytecode.clone(),
            metadata: Some(proto::DotMetadata {
                version: "1.0.0".to_string(),
                description: "Deployed via API Gateway".to_string(),
                author: "api-gateway".to_string(),
                tags: vec![],
                license: "AGPL-3.0".to_string(),
                custom_fields: HashMap::new(),
            }),
            deployer_id: "api-gateway".to_string(),
            options: Some(proto::DeploymentOptions {
                validate_abi: true,
                generate_ui: false,
                target_architecture: "WASM".to_string(),
                enable_optimizations: true,
            }),
        };

        let mut client = self.client.clone();
        let response = client
            .deploy_dot(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC deploy_dot call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        if !response.success {
            return Err(ApiError::BadRequest {
                message: format!("Deployment failed: {}", response.error_message),
            });
        }

        // Create validation result based on success
        let validation = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        };

        info!("Successfully deployed dot: {}", response.dot_id);

        Ok(DeployDotResponse {
            dot_id: response.dot_id,
            status: DotStatus::Active,
            deployed_at: Utc::now(),
            validation,
        })
    }

    /// Get dot state
    pub async fn get_dot_state(&self, dot_id: &str) -> ApiResult<DotState> {
        info!("Getting dot state: {}", dot_id);

        let grpc_request = proto::GetDotStateRequest {
            dot_id: dot_id.to_string(),
            keys: vec![],           // Get all keys
            version: String::new(), // Latest version
        };

        let mut client = self.client.clone();
        let response = client
            .get_dot_state(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC get_dot_state call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        if !response.success {
            return Err(ApiError::NotFound {
                message: format!("Dot '{}' not found or error: {}", dot_id, response.error_message),
            });
        }

        // Convert state data from HashMap<String, Vec<u8>> to serde_json::Value
        let mut state_json = serde_json::Map::new();
        for (key, value) in response.state_data {
            if let Ok(json_value) = serde_json::from_slice::<serde_json::Value>(&value) {
                state_json.insert(key, json_value);
            } else {
                // If not valid JSON, store as string
                state_json.insert(key, serde_json::Value::String(String::from_utf8_lossy(&value).to_string()));
            }
        }

        info!("Retrieved state for dot: {}", dot_id);

        Ok(DotState {
            dot_id: dot_id.to_string(),
            status: DotStatus::Active, // Assume active if we can get state
            state: serde_json::Value::Object(state_json),
            updated_at: Utc::now(), // gRPC response doesn't include timestamps
            version: response.version,
        })
    }

    /// Execute a dot function
    pub async fn execute_dot(&self, dot_id: &str, request: ExecuteDotRequest) -> ApiResult<ExecuteDotResponse> {
        info!("Executing dot: {} function: {}", dot_id, request.function);
        let start_time = std::time::Instant::now();

        // Convert function arguments to gRPC format
        let mut inputs = HashMap::new();
        for (i, arg) in request.arguments.iter().enumerate() {
            let key = if i == 0 && request.function.is_empty() { "function".to_string() } else { format!("arg_{}", i) };

            let value = serde_json::to_vec(arg).map_err(|e| ApiError::BadRequest {
                message: format!("Failed to serialize argument: {}", e),
            })?;

            inputs.insert(key, value);
        }

        // Add function name to inputs
        if !request.function.is_empty() {
            inputs.insert("function_name".to_string(), request.function.as_bytes().to_vec());
        }

        let grpc_request = proto::ExecuteDotRequest {
            dot_id: dot_id.to_string(),
            inputs,
            paradots_enabled: false,
            caller_id: "api-gateway".to_string(),
            options: Some(proto::ExecutionOptions {
                debug_mode: false,
                trace_execution: false,
                timeout_seconds: 30,
                required_paradots: vec![],
            }),
        };

        let mut client = self.client.clone();
        let response = client
            .execute_dot(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC execute_dot call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        let execution_time = start_time.elapsed();

        if !response.success {
            return Err(ApiError::BadRequest {
                message: format!("Execution failed: {}", response.error_message),
            });
        }

        // Convert outputs back to JSON
        let result = if response.outputs.is_empty() {
            serde_json::Value::Null
        } else if let Some(output) = response.outputs.get("result") {
            serde_json::from_slice(output).unwrap_or(serde_json::Value::String(String::from_utf8_lossy(output).to_string()))
        } else {
            // Return first output if no "result" key
            let first_output = response.outputs.values().next().unwrap();
            serde_json::from_slice(first_output).unwrap_or(serde_json::Value::String(String::from_utf8_lossy(first_output).to_string()))
        };

        info!("Successfully executed dot: {}", dot_id);

        Ok(ExecuteDotResponse {
            result,
            status: ExecutionStatus::Success,
            gas_used: 1000, // gRPC doesn't return gas info yet
            execution_time_ms: execution_time.as_millis() as u64,
            transaction_id: request.context.and_then(|ctx| ctx.transaction_id).or_else(|| Some(Uuid::new_v4().to_string())),
        })
    }

    /// List all deployed dots
    pub async fn list_dots(&self) -> ApiResult<Vec<DotState>> {
        info!("Listing all deployed dots");

        let grpc_request = proto::ListDotsRequest {
            pagination: Some(proto::Pagination {
                page: 1,
                page_size: 100,
                cursor: String::new(),
            }),
            filter: None,
            include_abi: false,
            sort_by: String::new(),
        };

        let mut client = self.client.clone();
        let response = client
            .list_dots(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC list_dots call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        let dots: Vec<DotState> = response
            .dots
            .into_iter()
            .map(|dot_info| DotState {
                dot_id: dot_info.dot_id,
                status: match dot_info.status {
                    1 => DotStatus::Active,
                    2 => DotStatus::Paused,
                    3 => DotStatus::Error,
                    _ => DotStatus::Unknown,
                },
                state: serde_json::Value::Object(serde_json::Map::new()), // Empty state for list view
                version: 1,                                               // dot_info doesn't have version field, use default
                updated_at: Utc::now(),                                   // gRPC response doesn't include timestamps
            })
            .collect();

        info!("Retrieved {} deployed dots", dots.len());

        Ok(dots)
    }

    /// Delete a deployed dot
    pub async fn delete_dot(&self, dot_id: &str) -> ApiResult<()> {
        info!("Deleting dot: {}", dot_id);

        let grpc_request = proto::DeleteDotRequest {
            dot_id: dot_id.to_string(),
            force: false,
            requester_id: "api-gateway".to_string(),
        };

        let mut client = self.client.clone();
        let response = client
            .delete_dot(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC delete_dot call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        if !response.success {
            return Err(ApiError::NotFound {
                message: format!("Failed to delete dot '{}': {}", dot_id, response.error_message),
            });
        }

        info!("Successfully deleted dot: {}", dot_id);

        Ok(())
    }

    /// Get VM status
    pub async fn get_vm_status(&self) -> ApiResult<serde_json::Value> {
        info!("Getting VM status");

        let grpc_request = proto::GetVmStatusRequest { include_details: true };

        let mut client = self.client.clone();
        let response = client
            .get_vm_status(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC get_vm_status call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        let status_name = match response.status {
            0 => "unknown",
            1 => "running",
            2 => "stopped",
            3 => "error",
            _ => "unknown",
        };

        let mut status_json = serde_json::json!({
            "status": status_name,
            "active_dots": response.active_dots.len(),
        });

        if let Some(info) = response.info {
            status_json["version"] = serde_json::Value::String(info.version);
            status_json["architecture"] = serde_json::Value::String(info.architecture);
            status_json["uptime_seconds"] = serde_json::Value::Number(serde_json::Number::from(info.uptime_seconds));
            status_json["dots_count"] = serde_json::Value::Number(serde_json::Number::from(info.dots_count));

            if let Some(resource_usage) = info.resource_usage {
                status_json["memory_usage"] = serde_json::json!({
                    "used_bytes": resource_usage.memory_used_bytes,
                    "total_bytes": resource_usage.memory_total_bytes,
                    "cpu_usage_percent": resource_usage.cpu_usage_percent
                });
            }
        }

        info!("Retrieved VM status: {}", status_name);

        Ok(status_json)
    }

    /// Get supported architectures
    pub async fn get_architectures(&self) -> ApiResult<Vec<String>> {
        info!("Getting supported architectures");

        let grpc_request = proto::GetArchitecturesRequest {};

        let mut client = self.client.clone();
        let response = client
            .get_architectures(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC get_architectures call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        let architectures: Vec<String> = response.architectures.into_iter().map(|arch_info| arch_info.name).collect();

        info!("Retrieved {} supported architectures", architectures.len());

        Ok(architectures)
    }

    /// Health check for VM connection
    pub async fn health_check(&self) -> ApiResult<bool> {
        let grpc_request = proto::HealthCheckRequest {
            services: vec![],
            include_details: false,
        };

        let mut client = self.client.clone();
        let result = client.health_check(grpc_request).await;

        match result {
            Ok(response) => {
                let health_response = response.into_inner();
                Ok(health_response.overall_status == 1) // HealthServing = 1
            }
            Err(e) => {
                warn!("VM health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Validate bytecode using gRPC service
    async fn validate_bytecode(&self, bytecode: &str) -> ApiResult<ValidationResult> {
        info!("Validating bytecode ({} chars)", bytecode.len());

        // Convert base64 bytecode to bytes
        let bytecode_bytes = base64::engine::general_purpose::STANDARD.decode(bytecode).map_err(|_| ApiError::BadRequest {
            message: "Bytecode must be valid base64".to_string(),
        })?;

        let grpc_request = proto::ValidateBytecodeRequest {
            bytecode: bytecode_bytes,
            target_architecture: "WASM".to_string(),
            strict_validation: true,
        };

        let mut client = self.client.clone();
        let response = client
            .validate_bytecode(grpc_request)
            .await
            .map_err(|e| {
                error!("gRPC validate_bytecode call failed: {}", e);
                ApiError::InternalServerError {
                    message: format!("gRPC call failed: {}", e),
                }
            })?
            .into_inner();

        let errors = response.errors.into_iter().map(|err| format!("{}: {}", err.field, err.message)).collect();

        info!("Bytecode validation result: valid={}", response.valid);

        Ok(ValidationResult {
            valid: response.valid,
            errors,
            warnings: vec![],
        })
    }
}

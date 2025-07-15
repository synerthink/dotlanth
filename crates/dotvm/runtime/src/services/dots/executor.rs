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

//! Dot executor - handles dot execution and state management

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, instrument};

use crate::proto::vm_service::{ExecuteDotRequest, ExecuteDotResponse, ExecutionMetrics, GetDotStateRequest, GetDotStateResponse, LogEntry};

use super::paradots::ParaDotManager;
use super::registry::StoredDot;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Resource limit exceeded")]
    ResourceLimitExceeded,
    #[error("State error: {0}")]
    StateError(String),
}

/// Dot executor handles execution of deployed dots
pub struct DotExecutor {
    paradot_manager: Arc<ParaDotManager>,
    // TODO: Add VM instance, state storage, etc.
}

impl DotExecutor {
    pub fn new() -> Self {
        Self {
            paradot_manager: Arc::new(ParaDotManager::new()),
        }
    }

    #[instrument(skip(self, dot_info, request))]
    pub async fn execute(&self, dot_info: &StoredDot, request: ExecuteDotRequest) -> Result<ExecuteDotResponse, ExecutorError> {
        info!("Executing dot: {} with {} inputs", dot_info.info.dot_id, request.inputs.len());

        // Validate inputs against ABI
        if let Some(abi) = &dot_info.abi {
            self.validate_inputs(&request.inputs, abi)?;
        }

        // Execute bytecode in VM with automatic ParaDot coordination
        let execution_result = self.execute_bytecode(&dot_info.bytecode, &request).await?;

        // Validate outputs against ABI
        if let Some(abi) = &dot_info.abi {
            self.validate_outputs(&execution_result.outputs, abi)?;
        }

        Ok(execution_result)
    }

    #[instrument(skip(self, request))]
    pub async fn get_state(&self, request: GetDotStateRequest) -> Result<GetDotStateResponse, ExecutorError> {
        info!("Getting state for dot: {}", request.dot_id);

        // TODO: Implement state retrieval from storage
        Ok(GetDotStateResponse {
            success: true,
            state_data: HashMap::new(),
            state_root_hash: "mock_root_hash".to_string(),
            version: 1,
            error_message: String::new(),
        })
    }

    // Private methods
    async fn execute_bytecode(&self, bytecode: &[u8], request: &ExecuteDotRequest) -> Result<ExecuteDotResponse, ExecutorError> {
        info!("Executing bytecode ({} bytes)", bytecode.len());

        // TODO: Implement actual VM execution
        // For now, return mock execution result

        let start_time = std::time::Instant::now();

        // Mock execution - echo inputs as outputs
        let outputs = request.inputs.clone();

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(ExecuteDotResponse {
            success: true,
            outputs,
            execution_time_ms: execution_time,
            paradots_used: vec!["mock_paradot".to_string()],
            logs: vec![LogEntry {
                level: "info".to_string(),
                message: format!("Executed dot with {} inputs", request.inputs.len()),
                timestamp: chrono::Utc::now().timestamp() as u64,
                source: "dot_executor".to_string(),
                context: HashMap::new(),
            }],
            events: vec![],
            // Removed gas_used - not applicable for general VM
            error_message: String::new(),
            metrics: Some(ExecutionMetrics {
                instructions_executed: 100,
                memory_used_bytes: 1024,
                storage_reads: 5,
                storage_writes: 2,
                paradots_spawned: 1,
                cpu_time_ms: execution_time,
            }),
        })
    }

    fn validate_inputs(&self, inputs: &HashMap<String, Vec<u8>>, abi: &crate::proto::vm_service::DotAbi) -> Result<(), ExecutorError> {
        info!("Validating {} inputs against ABI", inputs.len());

        // TODO: Implement actual ABI validation
        // For now, just check that required inputs are present
        for input_field in &abi.inputs {
            if input_field.required && !inputs.contains_key(&input_field.name) {
                return Err(ExecutorError::InvalidInput(format!("Missing required input: {}", input_field.name)));
            }
        }

        Ok(())
    }

    fn validate_outputs(&self, outputs: &HashMap<String, Vec<u8>>, abi: &crate::proto::vm_service::DotAbi) -> Result<(), ExecutorError> {
        info!("Validating {} outputs against ABI", outputs.len());

        // TODO: Implement actual output validation
        // For now, just log the validation

        Ok(())
    }
}

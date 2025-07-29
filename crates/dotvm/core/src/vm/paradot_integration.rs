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

//! Parallel Integration with Main VM Executor
//!
//! This module provides the integration layer between the Parallel parallel
//! execution engine and the main DotVM executor.

use crate::opcode::parallel_opcodes::ParallelOpcode;
use crate::vm::paradot_executor::{Args, AsyncSyncPrimitive, AtomicOperation, BarrierId, DotId, Message, ParaDotResult, ParallelError, ParallelOpcodeExecutor};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("ParaDot executor not initialized")]
    ExecutorNotInitialized,
    #[error("Invalid opcode parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(#[from] ParallelError),
    #[error("Runtime bridge error: {0}")]
    RuntimeError(String),
}

/// Integration layer for ParaDot operations in the main VM
pub struct ParaDotVMIntegration {
    executor: Arc<ParallelOpcodeExecutor>,
    enabled: bool,
}

impl ParaDotVMIntegration {
    /// Create a new ParaDot VM integration using Tokio's runtime directly
    #[instrument]
    pub fn new() -> Result<Self, IntegrationError> {
        info!("Initializing ParaDot VM integration with Tokio runtime");

        let executor = Arc::new(ParallelOpcodeExecutor::new().map_err(|e| IntegrationError::ExecutionFailed(e))?);

        Ok(Self { executor, enabled: true })
    }

    /// Execute a parallel opcode with the given parameters
    #[instrument(skip(self, parameters))]
    pub async fn execute_parallel_opcode(&self, opcode: ParallelOpcode, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        if !self.enabled {
            return Err(IntegrationError::ExecutorNotInitialized);
        }

        debug!("Executing parallel opcode: {:?}", opcode);

        match opcode {
            ParallelOpcode::ParaDotSpawn => self.handle_paradot_spawn(parameters).await,
            ParallelOpcode::ParaDotSync => self.handle_paradot_sync(parameters).await,
            ParallelOpcode::ParaDotMessage => self.handle_paradot_message(parameters).await,
            ParallelOpcode::ParaDotJoin => self.handle_paradot_join(parameters).await,
            ParallelOpcode::Atomic => self.handle_atomic_operation(parameters).await,
            ParallelOpcode::Barrier => self.handle_barrier_operation(parameters).await,
            _ => {
                warn!("Unsupported ParaDot opcode: {:?}", opcode);
                Err(IntegrationError::InvalidParameters(format!("Opcode {:?} not implemented for ParaDot operations", opcode)))
            }
        }
    }

    /// Handle ParaDot spawn operation
    #[instrument(skip(self, parameters))]
    async fn handle_paradot_spawn(&self, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        let params = self.parse_spawn_parameters(parameters)?;

        let handle = self.executor.execute_paradot_spawn(params.dot_id.clone(), params.args).await?;

        // Store the handle for later joining (in a real implementation, this would be managed properly)
        debug!("ParaDot spawned successfully: {}", params.dot_id);

        // Return the dot_id as confirmation
        Ok(params.dot_id.into_bytes())
    }

    /// Handle ParaDot synchronization operation
    #[instrument(skip(self, parameters))]
    async fn handle_paradot_sync(&self, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        let sync_primitive = self.parse_sync_parameters(parameters)?;

        self.executor.execute_paradot_sync(sync_primitive).await?;

        debug!("ParaDot synchronization completed");
        Ok(b"sync_ok".to_vec())
    }

    /// Handle ParaDot message passing
    #[instrument(skip(self, parameters))]
    async fn handle_paradot_message(&self, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        let (target_dot, message) = self.parse_message_parameters(parameters)?;

        self.executor.execute_paradot_message(target_dot, message).await?;

        debug!("ParaDot message sent successfully");
        Ok(b"message_sent".to_vec())
    }

    /// Handle ParaDot join operation
    #[instrument(skip(self, parameters))]
    async fn handle_paradot_join(&self, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        let dot_id = self.parse_join_parameters(parameters)?;

        let result = self.executor.execute_paradot_join(dot_id).await?;

        debug!("ParaDot joined successfully: {}", result.dot_id);

        // Return the execution result
        Ok(result.output)
    }

    /// Handle atomic operations
    #[instrument(skip(self, parameters))]
    async fn handle_atomic_operation(&self, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        let atomic_op = self.parse_atomic_parameters(parameters)?;

        let result = self.executor.execute_atomic(atomic_op).await?;

        debug!("Atomic operation completed with result: {}", result);

        // Return the atomic operation result as bytes
        Ok(result.to_le_bytes().to_vec())
    }

    /// Handle barrier synchronization
    #[instrument(skip(self, parameters))]
    async fn handle_barrier_operation(&self, parameters: &[u8]) -> Result<Vec<u8>, IntegrationError> {
        let barrier_id = self.parse_barrier_parameters(parameters)?;

        self.executor.execute_barrier(barrier_id).await?;

        debug!("Barrier synchronization completed");
        Ok(b"barrier_ok".to_vec())
    }

    /// Parse spawn operation parameters
    fn parse_spawn_parameters(&self, parameters: &[u8]) -> Result<SpawnParams, IntegrationError> {
        if parameters.len() < 8 {
            return Err(IntegrationError::InvalidParameters("Insufficient parameters for spawn operation".to_string()));
        }

        // Simple parameter parsing (in a real implementation, this would use proper serialization)
        let dot_id_len = u32::from_le_bytes([parameters[0], parameters[1], parameters[2], parameters[3]]) as usize;
        let data_len = u32::from_le_bytes([parameters[4], parameters[5], parameters[6], parameters[7]]) as usize;

        if parameters.len() < 8 + dot_id_len + data_len {
            return Err(IntegrationError::InvalidParameters("Parameter buffer too small".to_string()));
        }

        let dot_id = String::from_utf8(parameters[8..8 + dot_id_len].to_vec()).map_err(|e| IntegrationError::InvalidParameters(format!("Invalid dot_id: {}", e)))?;

        let data = parameters[8 + dot_id_len..8 + dot_id_len + data_len].to_vec();

        Ok(SpawnParams {
            dot_id,
            args: Args {
                data,
                parameters: std::collections::HashMap::new(),
            },
        })
    }

    /// Parse synchronization parameters
    fn parse_sync_parameters(&self, parameters: &[u8]) -> Result<AsyncSyncPrimitive, IntegrationError> {
        if parameters.is_empty() {
            return Err(IntegrationError::InvalidParameters("No sync primitive specified".to_string()));
        }

        match parameters[0] {
            0 => Ok(AsyncSyncPrimitive::Mutex { id: "default_mutex".to_string() }),
            1 => Ok(AsyncSyncPrimitive::RwLock { id: "default_rwlock".to_string() }),
            2 => Ok(AsyncSyncPrimitive::Semaphore {
                id: "default_semaphore".to_string(),
                permits: 5,
            }),
            3 => Ok(AsyncSyncPrimitive::Oneshot { id: "default_oneshot".to_string() }),
            _ => Err(IntegrationError::InvalidParameters(format!("Unknown sync primitive type: {}", parameters[0]))),
        }
    }

    /// Parse message parameters
    fn parse_message_parameters(&self, parameters: &[u8]) -> Result<(DotId, Message), IntegrationError> {
        if parameters.len() < 8 {
            return Err(IntegrationError::InvalidParameters("Insufficient parameters for message operation".to_string()));
        }

        // Simple parsing - in practice, this would use proper serialization
        let target_len = u32::from_le_bytes([parameters[0], parameters[1], parameters[2], parameters[3]]) as usize;
        let content_len = u32::from_le_bytes([parameters[4], parameters[5], parameters[6], parameters[7]]) as usize;

        if parameters.len() < 8 + target_len + content_len {
            return Err(IntegrationError::InvalidParameters("Parameter buffer too small for message".to_string()));
        }

        let target_dot = String::from_utf8(parameters[8..8 + target_len].to_vec()).map_err(|e| IntegrationError::InvalidParameters(format!("Invalid target_dot: {}", e)))?;

        let content = parameters[8 + target_len..8 + target_len + content_len].to_vec();

        let message = Message {
            sender: "vm_executor".to_string(),
            content,
            message_type: "vm_message".to_string(),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        Ok((target_dot, message))
    }

    /// Parse join parameters
    fn parse_join_parameters(&self, parameters: &[u8]) -> Result<DotId, IntegrationError> {
        if parameters.is_empty() {
            return Err(IntegrationError::InvalidParameters("No dot_id specified for join".to_string()));
        }

        String::from_utf8(parameters.to_vec()).map_err(|e| IntegrationError::InvalidParameters(format!("Invalid dot_id: {}", e)))
    }

    /// Parse atomic operation parameters
    fn parse_atomic_parameters(&self, parameters: &[u8]) -> Result<AtomicOperation, IntegrationError> {
        if parameters.is_empty() {
            return Err(IntegrationError::InvalidParameters("No atomic operation specified".to_string()));
        }

        match parameters[0] {
            0 => Ok(AtomicOperation::Load { id: "default_atomic".to_string() }),
            1 => {
                if parameters.len() < 9 {
                    return Err(IntegrationError::InvalidParameters("Insufficient parameters for atomic store".to_string()));
                }
                let value = u64::from_le_bytes([parameters[1], parameters[2], parameters[3], parameters[4], parameters[5], parameters[6], parameters[7], parameters[8]]);
                Ok(AtomicOperation::Store {
                    id: "default_atomic".to_string(),
                    value,
                })
            }
            2 => Ok(AtomicOperation::FetchAdd {
                id: "default_atomic".to_string(),
                value: 1,
            }),
            3 => Ok(AtomicOperation::FetchSub {
                id: "default_atomic".to_string(),
                value: 1,
            }),
            _ => Err(IntegrationError::InvalidParameters(format!("Unknown atomic operation type: {}", parameters[0]))),
        }
    }

    /// Parse barrier parameters
    fn parse_barrier_parameters(&self, parameters: &[u8]) -> Result<BarrierId, IntegrationError> {
        if parameters.is_empty() {
            return Err(IntegrationError::InvalidParameters("No barrier_id specified".to_string()));
        }

        String::from_utf8(parameters.to_vec()).map_err(|e| IntegrationError::InvalidParameters(format!("Invalid barrier_id: {}", e)))
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> IntegrationStats {
        let executor_stats = self.executor.get_stats();

        IntegrationStats {
            enabled: self.enabled,
            active_paradots: executor_stats.active_paradots,
            total_spawned: executor_stats.total_spawned,
            active_barriers: executor_stats.active_barriers,
            active_channels: executor_stats.active_channels,
        }
    }

    /// Enable or disable ParaDot execution
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!("ParaDot execution {}", if enabled { "enabled" } else { "disabled" });
    }
}

/// Parameters for spawn operation
#[derive(Debug)]
struct SpawnParams {
    dot_id: DotId,
    args: Args,
}

/// Integration statistics
#[derive(Debug, Clone)]
pub struct IntegrationStats {
    pub enabled: bool,
    pub active_paradots: usize,
    pub total_spawned: usize,
    pub active_barriers: usize,
    pub active_channels: usize,
}

impl Default for ParaDotVMIntegration {
    fn default() -> Self {
        Self::new().expect("Failed to create default ParaDotVMIntegration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_integration_creation() {
        let integration = ParaDotVMIntegration::new().unwrap();
        let stats = integration.get_stats();

        assert!(stats.enabled);
        assert_eq!(stats.active_paradots, 0);
    }

    #[test]
    async fn test_spawn_operation() {
        let integration = ParaDotVMIntegration::new().unwrap();

        // Create spawn parameters
        let dot_id = "test_dot";
        let data = b"test_data";
        let mut parameters = Vec::new();

        // Add dot_id length and data length
        parameters.extend_from_slice(&(dot_id.len() as u32).to_le_bytes());
        parameters.extend_from_slice(&(data.len() as u32).to_le_bytes());
        parameters.extend_from_slice(dot_id.as_bytes());
        parameters.extend_from_slice(data);

        let result = integration.execute_parallel_opcode(ParallelOpcode::ParaDotSpawn, &parameters).await.unwrap();

        assert_eq!(result, dot_id.as_bytes());
    }

    #[test]
    async fn test_atomic_operation() {
        let integration = ParaDotVMIntegration::new().unwrap();

        // Test atomic store
        let mut parameters = vec![1]; // Store operation
        parameters.extend_from_slice(&42u64.to_le_bytes());

        let result = integration.execute_parallel_opcode(ParallelOpcode::Atomic, &parameters).await.unwrap();

        let stored_value = u64::from_le_bytes([result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7]]);
        assert_eq!(stored_value, 42);
    }

    #[test]
    async fn test_sync_operation() {
        let integration = ParaDotVMIntegration::new().unwrap();

        // Test mutex sync
        let parameters = vec![0]; // Mutex operation

        let result = integration.execute_parallel_opcode(ParallelOpcode::ParaDotSync, &parameters).await.unwrap();

        assert_eq!(result, b"sync_ok");
    }
}

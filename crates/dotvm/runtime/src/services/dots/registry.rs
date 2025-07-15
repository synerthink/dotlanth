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

//! Dot registry - manages dot storage, versioning, and metadata

use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;
use tracing::{error, info};

use crate::proto::vm_service::{
    DeleteDotRequest, DeleteDotResponse, DeployDotRequest, DeployDotResponse, DeploymentMetrics, DotAbi, DotInfo, DotMetadata, DotStats, DotStatus, ListDotsRequest, ListDotsResponse,
};

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Dot not found: {0}")]
    DotNotFound(String),
    #[error("Dot already exists: {0}")]
    DotAlreadyExists(String),
    #[error("Invalid dot source: {0}")]
    InvalidDotSource(String),
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
}

/// Dot registry manages all deployed dots
pub struct DotRegistry {
    dots: RwLock<HashMap<String, StoredDot>>,
}

#[derive(Clone, Debug)]
pub struct StoredDot {
    pub info: DotInfo,
    pub source: String,
    pub bytecode: Vec<u8>,
    pub abi: Option<DotAbi>,
}

impl DotRegistry {
    pub fn new() -> Self {
        Self { dots: RwLock::new(HashMap::new()) }
    }

    pub async fn deploy_dot(&self, request: DeployDotRequest) -> Result<DeployDotResponse, RegistryError> {
        info!("Deploying dot: {}", request.dot_name);

        // Generate dot ID
        let dot_id = self.generate_dot_id(&request.dot_name);

        // Check if dot already exists
        {
            let dots = self.dots.read().unwrap();
            if dots.contains_key(&dot_id) {
                return Err(RegistryError::DotAlreadyExists(dot_id));
            }
        }

        // TODO: Compile dot source to bytecode
        let bytecode = self.compile_dot_source(&request.dot_source)?;

        // TODO: Generate ABI from dot source
        let abi = self.generate_abi_from_source(&request.dot_source)?;

        // Create dot info
        let dot_info = DotInfo {
            dot_id: dot_id.clone(),
            name: request.dot_name.clone(),
            metadata: request.metadata.clone(),
            status: DotStatus::Active as i32,
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
            abi: Some(abi.clone()),
            stats: Some(DotStats {
                execution_count: 0,
                total_cpu_time_ms: 0,
                average_execution_time_ms: 0.0,
                error_count: 0,
                last_executed_at: 0,
            }),
        };

        // Store dot
        let stored_dot = StoredDot {
            info: dot_info.clone(),
            source: request.dot_source,
            bytecode: bytecode.clone(),
            abi: Some(abi.clone()),
        };

        {
            let mut dots = self.dots.write().unwrap();
            dots.insert(dot_id.clone(), stored_dot);
        }

        info!("Successfully deployed dot: {}", dot_id);

        Ok(DeployDotResponse {
            success: true,
            dot_id,
            bytecode: bytecode.clone(),
            abi: Some(abi),
            error_message: String::new(),
            metrics: Some(DeploymentMetrics {
                compilation_time_ms: 50, // Mock value
                bytecode_size_bytes: bytecode.len() as u64,
                optimization_passes: 2,
                ui_generated: false,
            }),
        })
    }

    pub async fn get_dot(&self, dot_id: &str) -> Result<StoredDot, RegistryError> {
        let dots = self.dots.read().unwrap();
        dots.get(dot_id).cloned().ok_or_else(|| RegistryError::DotNotFound(dot_id.to_string()))
    }

    pub async fn list_dots(&self, _request: ListDotsRequest) -> Result<ListDotsResponse, RegistryError> {
        let dots = self.dots.read().unwrap();

        let dot_infos: Vec<DotInfo> = dots.values().map(|stored_dot| stored_dot.info.clone()).collect();

        Ok(ListDotsResponse {
            dots: dot_infos.clone(),
            total_count: dot_infos.len() as u32,
            next_cursor: String::new(),
            has_more: false,
        })
    }

    pub async fn delete_dot(&self, request: DeleteDotRequest) -> Result<DeleteDotResponse, RegistryError> {
        let mut dots = self.dots.write().unwrap();

        if dots.remove(&request.dot_id).is_some() {
            info!("Successfully deleted dot: {}", request.dot_id);
            Ok(DeleteDotResponse {
                success: true,
                error_message: String::new(),
            })
        } else {
            Err(RegistryError::DotNotFound(request.dot_id))
        }
    }

    // Private helper methods
    fn generate_dot_id(&self, name: &str) -> String {
        format!("dot_{}_{}", name.to_lowercase().replace(" ", "_"), uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string())
    }

    fn compile_dot_source(&self, source: &str) -> Result<Vec<u8>, RegistryError> {
        // TODO: Implement actual dot compilation
        // For now, return mock bytecode
        info!("Compiling dot source ({} chars)", source.len());

        if source.trim().is_empty() {
            return Err(RegistryError::InvalidDotSource("Empty source".to_string()));
        }

        // Mock bytecode generation
        let mut bytecode = vec![0x01, 0x02, 0x03, 0x04]; // Header
        bytecode.extend_from_slice(source.as_bytes()); // Include source for now

        Ok(bytecode)
    }

    fn generate_abi_from_source(&self, source: &str) -> Result<DotAbi, RegistryError> {
        // TODO: Implement actual ABI generation
        info!("Generating ABI from source");

        Ok(DotAbi {
            dot_name: "GeneratedDot".to_string(),
            version: "1.0.0".to_string(),
            description: "Auto-generated ABI".to_string(),
            inputs: vec![],    // TODO: Parse from source
            outputs: vec![],   // TODO: Parse from source
            paradots: vec![],  // TODO: Parse from source
            ui_hints: None,    // TODO: Generate UI hints
            permissions: None, // TODO: Parse permissions
        })
    }
}

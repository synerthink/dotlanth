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

//! ParaDot manager - coordinates ParaDot spawning and execution during dot execution

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, instrument};

use super::registry::{ParaDotRegistry, ParaDotTypeInfo, WorkloadCharacteristics};
use crate::proto::vm_service::{ExecuteDotRequest, ParaDotDependency};

#[derive(Error, Debug)]
pub enum ParaDotManagerError {
    #[error("ParaDot spawning failed: {0}")]
    SpawningFailed(String),
    #[error("ParaDot execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Resource allocation failed: {0}")]
    ResourceAllocationFailed(String),
}

/// ParaDot manager coordinates automatic ParaDot spawning and execution
pub struct ParaDotManager {
    registry: Arc<ParaDotRegistry>,
    active_paradots: std::sync::RwLock<HashMap<String, ActiveParaDot>>,
}

#[derive(Clone, Debug)]
struct ActiveParaDot {
    paradot_id: String,
    paradot_type: String,
    spawn_time: u64,
    resource_usage: ResourceUsage,
}

#[derive(Clone, Debug)]
struct ResourceUsage {
    memory_mb: u64,
    cpu_percent: u32,
}

impl ParaDotManager {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ParaDotRegistry::new()),
            active_paradots: std::sync::RwLock::new(HashMap::new()),
        }
    }

    #[instrument(skip(self, abi_paradots, request))]
    pub async fn determine_and_spawn_paradots(&self, abi_paradots: &[ParaDotDependency], request: &ExecuteDotRequest) -> Result<Vec<String>, ParaDotManagerError> {
        info!("Determining ParaDots for dot execution");

        let mut spawned_paradots = Vec::new();

        // 1. Spawn required ParaDots from ABI
        for paradot_dep in abi_paradots {
            if paradot_dep.required {
                let paradot_id = self.spawn_abi_paradot(paradot_dep).await?;
                spawned_paradots.push(paradot_id);
            }
        }

        // 2. Analyze workload characteristics
        let workload = self.analyze_workload(request);

        // 3. Find suitable ParaDots for auto-spawning
        let suitable_paradots = self.registry.find_suitable_paradots(&workload);

        // 4. Spawn auto-selected ParaDots
        for paradot_type in suitable_paradots {
            if self.should_spawn_paradot(&paradot_type, &workload) {
                let paradot_id = self.spawn_auto_paradot(&paradot_type).await?;
                spawned_paradots.push(paradot_id);
            }
        }

        // 5. Spawn optional ParaDots from ABI if beneficial
        for paradot_dep in abi_paradots {
            if !paradot_dep.required && self.should_spawn_optional_paradot(paradot_dep, &workload) {
                let paradot_id = self.spawn_abi_paradot(paradot_dep).await?;
                spawned_paradots.push(paradot_id);
            }
        }

        info!("Spawned {} ParaDots for execution", spawned_paradots.len());
        Ok(spawned_paradots)
    }

    fn analyze_workload(&self, request: &ExecuteDotRequest) -> WorkloadCharacteristics {
        let total_data_size_kb = request.inputs.values().map(|data| data.len() as u64).sum::<u64>() / 1024;

        let input_count = request.inputs.len() as u32;

        // Estimate complexity based on input characteristics
        let estimated_complexity = self.estimate_complexity(request);

        // Detect patterns based on input names and types
        let patterns = self.detect_workload_patterns(request);

        WorkloadCharacteristics {
            total_data_size_kb,
            input_count,
            estimated_complexity,
            has_network_operations: self.has_network_patterns(&patterns),
            has_storage_operations: self.has_storage_patterns(&patterns),
            patterns,
        }
    }

    fn estimate_complexity(&self, request: &ExecuteDotRequest) -> f64 {
        let mut complexity = 0.0;

        // Base complexity from input count
        complexity += request.inputs.len() as f64 * 0.1;

        // Data size complexity
        let total_size = request.inputs.values().map(|data| data.len()).sum::<usize>();
        complexity += (total_size as f64 / 1024.0) * 0.05; // 0.05 per KB

        // Input name complexity (heuristic)
        for key in request.inputs.keys() {
            if key.contains("complex") || key.contains("calculation") || key.contains("compute") {
                complexity += 0.3;
            }
            if key.contains("data") || key.contains("list") || key.contains("array") {
                complexity += 0.2;
            }
        }

        complexity.min(1.0) // Cap at 1.0
    }

    fn detect_workload_patterns(&self, request: &ExecuteDotRequest) -> Vec<String> {
        let mut patterns = Vec::new();

        for key in request.inputs.keys() {
            let key_lower = key.to_lowercase();

            if key_lower.contains("data") || key_lower.contains("list") || key_lower.contains("array") {
                patterns.push("data_processing".to_string());
            }
            if key_lower.contains("calc") || key_lower.contains("compute") || key_lower.contains("math") {
                patterns.push("computation".to_string());
            }
            if key_lower.contains("api") || key_lower.contains("url") || key_lower.contains("endpoint") {
                patterns.push("network".to_string());
            }
            if key_lower.contains("file") || key_lower.contains("storage") || key_lower.contains("db") {
                patterns.push("storage".to_string());
            }
        }

        patterns.sort();
        patterns.dedup();
        patterns
    }

    fn has_network_patterns(&self, patterns: &[String]) -> bool {
        patterns.iter().any(|p| p.contains("network") || p.contains("api"))
    }

    fn has_storage_patterns(&self, patterns: &[String]) -> bool {
        patterns.iter().any(|p| p.contains("storage") || p.contains("db"))
    }

    fn should_spawn_paradot(&self, paradot_type: &ParaDotTypeInfo, workload: &WorkloadCharacteristics) -> bool {
        // Check resource availability
        if !self.has_available_resources(&paradot_type.resource_requirements) {
            return false;
        }

        // Check if we already have a similar ParaDot running
        if self.has_similar_active_paradot(&paradot_type.name) {
            return false;
        }

        // Additional heuristics for spawning decision
        true
    }

    fn should_spawn_optional_paradot(&self, paradot_dep: &ParaDotDependency, workload: &WorkloadCharacteristics) -> bool {
        // Spawn optional ParaDots only if they would significantly benefit the workload
        match paradot_dep.paradot_type.as_str() {
            "DataProcessing" => workload.total_data_size_kb > 5, // > 5KB
            "ComputationAccelerator" => workload.estimated_complexity > 0.5,
            "NetworkIO" => workload.has_network_operations,
            _ => false,
        }
    }

    async fn spawn_abi_paradot(&self, paradot_dep: &ParaDotDependency) -> Result<String, ParaDotManagerError> {
        let paradot_id = format!("abi_{}_{}", paradot_dep.name, uuid::Uuid::new_v4().to_string()[..8].to_string());

        info!("Spawning ABI ParaDot: {} ({})", paradot_dep.name, paradot_id);

        // TODO: Implement actual ParaDot spawning
        // This would:
        // 1. Load ParaDot implementation
        // 2. Initialize runtime environment
        // 3. Allocate resources
        // 4. Start ParaDot process/thread

        self.register_active_paradot(&paradot_id, &paradot_dep.paradot_type);

        Ok(paradot_id)
    }

    async fn spawn_auto_paradot(&self, paradot_type: &ParaDotTypeInfo) -> Result<String, ParaDotManagerError> {
        let paradot_id = format!("auto_{}_{}", paradot_type.name, uuid::Uuid::new_v4().to_string()[..8].to_string());

        info!("Auto-spawning ParaDot: {} ({})", paradot_type.name, paradot_id);

        // TODO: Implement actual ParaDot spawning

        self.register_active_paradot(&paradot_id, &paradot_type.name);

        Ok(paradot_id)
    }

    fn register_active_paradot(&self, paradot_id: &str, paradot_type: &str) {
        let active_paradot = ActiveParaDot {
            paradot_id: paradot_id.to_string(),
            paradot_type: paradot_type.to_string(),
            spawn_time: chrono::Utc::now().timestamp() as u64,
            resource_usage: ResourceUsage {
                memory_mb: 64, // Mock values
                cpu_percent: 10,
            },
        };

        let mut active_paradots = self.active_paradots.write().unwrap();
        active_paradots.insert(paradot_id.to_string(), active_paradot);
    }

    fn has_available_resources(&self, requirements: &super::registry::ResourceRequirements) -> bool {
        // TODO: Implement actual resource checking
        // For now, assume resources are available
        true
    }

    fn has_similar_active_paradot(&self, paradot_type: &str) -> bool {
        let active_paradots = self.active_paradots.read().unwrap();
        active_paradots.values().any(|p| p.paradot_type == paradot_type)
    }

    pub async fn cleanup_paradots(&self, paradot_ids: &[String]) {
        info!("Cleaning up {} ParaDots", paradot_ids.len());

        let mut active_paradots = self.active_paradots.write().unwrap();
        for paradot_id in paradot_ids {
            active_paradots.remove(paradot_id);
            // TODO: Implement actual ParaDot cleanup
        }
    }
}

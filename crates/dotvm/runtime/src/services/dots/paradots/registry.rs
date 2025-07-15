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

//! ParaDot registry - manages available ParaDot types and their capabilities

use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum ParaDotRegistryError {
    #[error("ParaDot type not found: {0}")]
    ParaDotTypeNotFound(String),
    #[error("ParaDot registration failed: {0}")]
    RegistrationFailed(String),
}

/// ParaDot registry manages available ParaDot types and their capabilities
pub struct ParaDotRegistry {
    paradot_types: RwLock<HashMap<String, ParaDotTypeInfo>>,
}

#[derive(Clone, Debug)]
pub struct ParaDotTypeInfo {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub auto_spawn_conditions: AutoSpawnConditions,
}

#[derive(Clone, Debug)]
pub struct ResourceRequirements {
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
    pub requires_network: bool,
    pub requires_storage: bool,
}

#[derive(Clone, Debug)]
pub struct AutoSpawnConditions {
    pub min_data_size_kb: Option<u64>,
    pub min_input_count: Option<u32>,
    pub complexity_threshold: Option<f64>,
    pub workload_patterns: Vec<String>,
}

impl ParaDotRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            paradot_types: RwLock::new(HashMap::new()),
        };

        // Register built-in ParaDot types
        registry.register_builtin_paradots();

        registry
    }

    pub fn get_paradot_type(&self, name: &str) -> Result<ParaDotTypeInfo, ParaDotRegistryError> {
        let paradot_types = self.paradot_types.read().unwrap();
        paradot_types.get(name).cloned().ok_or_else(|| ParaDotRegistryError::ParaDotTypeNotFound(name.to_string()))
    }

    pub fn list_available_types(&self) -> Vec<ParaDotTypeInfo> {
        let paradot_types = self.paradot_types.read().unwrap();
        paradot_types.values().cloned().collect()
    }

    pub fn find_suitable_paradots(&self, workload_characteristics: &WorkloadCharacteristics) -> Vec<ParaDotTypeInfo> {
        let paradot_types = self.paradot_types.read().unwrap();

        paradot_types
            .values()
            .filter(|paradot_type| self.is_suitable_for_workload(paradot_type, workload_characteristics))
            .cloned()
            .collect()
    }

    fn register_builtin_paradots(&mut self) {
        // Data Processing ParaDot
        self.register_paradot_type(ParaDotTypeInfo {
            name: "DataProcessor".to_string(),
            description: "Processes large datasets efficiently".to_string(),
            capabilities: vec!["data_transformation".to_string(), "filtering".to_string(), "aggregation".to_string()],
            resource_requirements: ResourceRequirements {
                max_memory_mb: 512,
                max_cpu_percent: 50,
                requires_network: false,
                requires_storage: true,
            },
            auto_spawn_conditions: AutoSpawnConditions {
                min_data_size_kb: Some(1), // 1KB+
                min_input_count: None,
                complexity_threshold: None,
                workload_patterns: vec!["data_processing".to_string(), "transformation".to_string()],
            },
        });

        // Computation Accelerator ParaDot
        self.register_paradot_type(ParaDotTypeInfo {
            name: "ComputationAccelerator".to_string(),
            description: "Accelerates mathematical computations".to_string(),
            capabilities: vec!["parallel_math".to_string(), "simd_operations".to_string(), "optimization".to_string()],
            resource_requirements: ResourceRequirements {
                max_memory_mb: 256,
                max_cpu_percent: 80,
                requires_network: false,
                requires_storage: false,
            },
            auto_spawn_conditions: AutoSpawnConditions {
                min_data_size_kb: None,
                min_input_count: Some(3), // 3+ inputs
                complexity_threshold: Some(0.7),
                workload_patterns: vec!["computation".to_string(), "math".to_string(), "calculation".to_string()],
            },
        });

        // Network I/O ParaDot
        self.register_paradot_type(ParaDotTypeInfo {
            name: "NetworkIO".to_string(),
            description: "Handles external API calls and network operations".to_string(),
            capabilities: vec!["http_requests".to_string(), "api_calls".to_string(), "data_fetching".to_string()],
            resource_requirements: ResourceRequirements {
                max_memory_mb: 128,
                max_cpu_percent: 20,
                requires_network: true,
                requires_storage: false,
            },
            auto_spawn_conditions: AutoSpawnConditions {
                min_data_size_kb: None,
                min_input_count: None,
                complexity_threshold: None,
                workload_patterns: vec!["api".to_string(), "network".to_string(), "external".to_string()],
            },
        });
    }

    fn register_paradot_type(&mut self, paradot_type: ParaDotTypeInfo) {
        let mut paradot_types = self.paradot_types.write().unwrap();
        paradot_types.insert(paradot_type.name.clone(), paradot_type);
    }

    fn is_suitable_for_workload(&self, paradot_type: &ParaDotTypeInfo, workload: &WorkloadCharacteristics) -> bool {
        let conditions = &paradot_type.auto_spawn_conditions;

        // Check data size requirement
        if let Some(min_size) = conditions.min_data_size_kb {
            if workload.total_data_size_kb < min_size {
                return false;
            }
        }

        // Check input count requirement
        if let Some(min_inputs) = conditions.min_input_count {
            if workload.input_count < min_inputs {
                return false;
            }
        }

        // Check complexity threshold
        if let Some(min_complexity) = conditions.complexity_threshold {
            if workload.estimated_complexity < min_complexity {
                return false;
            }
        }

        // Check workload pattern matching
        if !conditions.workload_patterns.is_empty() {
            let pattern_match = conditions.workload_patterns.iter().any(|pattern| workload.patterns.contains(pattern));
            if !pattern_match {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone)]
pub struct WorkloadCharacteristics {
    pub total_data_size_kb: u64,
    pub input_count: u32,
    pub estimated_complexity: f64,
    pub patterns: Vec<String>,
    pub has_network_operations: bool,
    pub has_storage_operations: bool,
}

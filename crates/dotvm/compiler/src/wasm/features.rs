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

//! WASM feature detection and management
//!
//! This module provides utilities for detecting and managing
//! WASM features and proposals.

use super::ast::WasmInstruction;

/// WASM feature detector
pub struct FeatureDetector;

impl FeatureDetector {
    /// Detect features used by an instruction
    pub fn detect_features(instruction: &WasmInstruction) -> Vec<String> {
        let mut features = Vec::new();

        match instruction {
            WasmInstruction::V128Load { .. } | WasmInstruction::V128Store { .. } | WasmInstruction::V128Const { .. } => {
                features.push("simd".to_string());
            }
            WasmInstruction::MemoryCopy | WasmInstruction::MemoryFill | WasmInstruction::MemoryInit { .. } | WasmInstruction::DataDrop { .. } => {
                features.push("bulk_memory".to_string());
            }
            WasmInstruction::RefNull { .. } | WasmInstruction::RefIsNull | WasmInstruction::RefFunc { .. } => {
                features.push("reference_types".to_string());
            }
            WasmInstruction::SelectWithType { .. } => {
                features.push("multi_value".to_string());
            }
            _ => {}
        }

        features
    }

    /// Check if an instruction requires a specific feature
    pub fn requires_feature(instruction: &WasmInstruction, feature: &str) -> bool {
        Self::detect_features(instruction).contains(&feature.to_string())
    }
}

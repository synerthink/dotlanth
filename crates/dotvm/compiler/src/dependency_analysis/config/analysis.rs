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

//! Analysis-specific configuration

use crate::dependency_analysis::core::traits::AnalysisType;

/// Configuration of which analyzers to run
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub enabled_analyzers: Vec<AnalysisType>,
}

impl AnalysisConfig {
    pub fn new(enabled: Vec<AnalysisType>) -> Self {
        Self { enabled_analyzers: enabled }
    }
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self::new(vec![AnalysisType::ControlFlow, AnalysisType::DataFlow, AnalysisType::StateAccess, AnalysisType::DependencyDetection])
    }
}

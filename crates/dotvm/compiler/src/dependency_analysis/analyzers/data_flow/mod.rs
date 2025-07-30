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

//! Data flow analysis for variable tracking and optimization

pub mod constant_prop;
pub mod def_use;
pub mod liveness;
pub mod reaching;

pub use constant_prop::ConstantPropagator;
pub use def_use::DefUseAnalyzer;
pub use liveness::LivenessAnalyzer;
pub use reaching::ReachingDefinitionsAnalyzer;

// Re-export types that were removed with legacy
#[derive(Debug, Clone)]
pub struct DataFlowAnalysis {
    pub variables: Vec<String>,
    pub definitions: Vec<String>,
    pub uses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DataFlowIssue {
    pub issue_type: DataFlowIssueType,
    pub location: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum DataFlowIssueType {
    UnusedVariable,
    UninitializedVariable,
    DeadCode,
}

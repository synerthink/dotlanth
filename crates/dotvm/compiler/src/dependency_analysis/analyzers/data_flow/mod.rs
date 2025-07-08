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
//!
//! This module provides comprehensive data flow analysis capabilities for tracking
//! how data moves through variables, registers, and memory locations in WebAssembly
//! modules and DotVM bytecode. These analyses are essential for optimization,
//! dead code elimination, and variable lifetime management.
//!
//! ## Core Analysis Types
//!
//! ### Definition-Use Analysis (`def_use`)
//! - **Purpose**: Tracks where variables are defined and where they are used
//! - **Output**: Definition-use chains linking variable definitions to their uses
//! - **Applications**: Dead code elimination, variable renaming, optimization
//! - **Algorithm**: Forward data flow analysis with reaching definitions
//!
//! ### Liveness Analysis (`liveness`)
//! - **Purpose**: Determines which variables are "live" at each program point
//! - **Definition**: A variable is live if it may be used before being redefined
//! - **Applications**: Register allocation, dead code elimination, memory optimization
//! - **Algorithm**: Backward data flow analysis from variable uses
//!
//! ### Reaching Definitions (`reaching`)
//! - **Purpose**: Identifies which definitions can reach each program point
//! - **Output**: Set of definitions that may reach each use of a variable
//! - **Applications**: Constant propagation, copy propagation, optimization
//! - **Algorithm**: Forward data flow analysis with definition propagation
//!
//! ### Constant Propagation (`constant_prop`)
//! - **Purpose**: Tracks constant values through the program
//! - **Optimization**: Replaces variable uses with known constant values
//! - **Applications**: Compile-time evaluation, dead code elimination
//! - **Integration**: Works with reaching definitions for comprehensive analysis
//!
//! ## Data Structures
//!
//! ### DataFlowAnalysis
//! - **Purpose**: Container for data flow analysis results
//! - **Contents**: Variable lists, definitions, uses, and relationships
//! - **Usage**: Provides structured access to analysis results
//!
//! ### DataFlowIssue
//! - **Purpose**: Represents potential issues found during analysis
//! - **Types**: Unused variables, uninitialized variables, dead code
//! - **Applications**: Code quality assessment, optimization opportunities
//!
//! ## Applications in Smart Contracts
//!
//! ### Gas Optimization
//! - Identifies unused variables that waste storage
//! - Finds redundant computations that can be eliminated
//! - Optimizes variable lifetimes to reduce memory usage
//!
//! ### Security Analysis
//! - Detects uninitialized variables that could cause vulnerabilities
//! - Identifies dead code that might hide malicious behavior
//! - Validates proper variable initialization patterns
//!
//! ### Code Quality
//! - Reports unused variables and dead code
//! - Suggests optimization opportunities
//! - Validates variable usage patterns
//!
//! ## Performance Characteristics
//!
//! - **Time Complexity**: O(n * d) where n is program size and d is data flow depth
//! - **Space Complexity**: O(n * v) where v is the number of variables
//! - **Convergence**: Uses iterative algorithms that converge to fixed points
//! - **Scalability**: Optimized for large programs with many variables

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

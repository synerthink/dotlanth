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

//! State access analysis for blockchain smart contracts
//!
//! This module provides specialized analysis for tracking and optimizing access to
//! blockchain state variables in smart contracts. It's designed specifically for
//! DotVM's blockchain environment where state access patterns directly impact
//! gas costs, security, and performance.
//!
//! ## Core Components
//!
//! ### Read-Write Analysis (`read_write`)
//! - **Purpose**: Tracks all read and write operations to state variables
//! - **Granularity**: Per-variable tracking with operation type classification
//! - **Output**: Detailed access patterns showing read/write sequences
//! - **Applications**: Gas optimization, caching strategies, conflict detection
//!
//! ### Conflict Detection (`conflicts`)
//! - **Purpose**: Identifies potential conflicts between concurrent state accesses
//! - **Scope**: Detects read-after-write, write-after-read, and write-after-write conflicts
//! - **Security**: Critical for preventing race conditions and reentrancy attacks
//! - **Output**: Conflict reports with severity levels and mitigation suggestions
//!
//! ### Access Optimization (`optimization`)
//! - **Purpose**: Provides hints for optimizing state access patterns
//! - **Strategies**: Batching, caching, reordering, and elimination opportunities
//! - **Gas Efficiency**: Reduces transaction costs through smarter access patterns
//! - **Integration**: Works with other analyses to provide comprehensive optimization
//!
//! ## Key Data Structures
//!
//! ### StateAccess
//! - **Purpose**: Represents a single access to a state variable
//! - **Information**: Variable name, access type, location, timestamp
//! - **Usage**: Building blocks for more complex analysis patterns
//!
//! ### StateAccessType
//! - **Read**: Variable is read but not modified
//! - **Write**: Variable is written/modified
//! - **ReadWrite**: Variable is both read and written in the same operation
//!
//! ### StateConflict
//! - **Purpose**: Represents a detected conflict between state accesses
//! - **Details**: Conflicting operations, affected variables, severity level
//! - **Resolution**: Suggested mitigation strategies and fixes
//!
//! ## Blockchain-Specific Considerations
//!
//! ### Gas Cost Optimization
//! - **SLOAD/SSTORE Tracking**: Monitors expensive storage operations
//! - **Access Patterns**: Identifies opportunities for batching operations
//! - **Caching Opportunities**: Suggests when to cache frequently accessed state
//! - **Redundancy Elimination**: Removes unnecessary duplicate accesses
//!
//! ### Security Analysis
//! - **Reentrancy Detection**: Identifies potential reentrancy vulnerabilities
//! - **State Consistency**: Ensures state modifications follow safe patterns
//! - **Access Control**: Validates that state access follows proper authorization
//! - **Atomicity**: Checks that related state changes are properly grouped
//!
//! ### Concurrency Safety
//! - **Race Condition Detection**: Identifies potential race conditions
//! - **Lock-Free Patterns**: Analyzes lock-free data structure usage
//! - **Ordering Constraints**: Ensures proper ordering of state operations
//! - **Isolation Levels**: Validates transaction isolation requirements
//!
//! ## Integration with DotVM Runtime
//!
//! ### Runtime Optimization
//! - **Prefetching**: Suggests state variables to prefetch
//! - **Caching Strategies**: Provides cache management hints
//! - **Batch Processing**: Identifies opportunities for batched operations
//! - **Memory Management**: Optimizes memory usage for state access
//!
//! ### Performance Monitoring
//! - **Access Frequency**: Tracks how often each state variable is accessed
//! - **Hot Paths**: Identifies frequently executed state access patterns
//! - **Bottleneck Detection**: Finds performance bottlenecks in state access
//! - **Resource Usage**: Monitors memory and computational resource usage
//!
//! ## Analysis Algorithms
//!
//! - **Static Analysis**: Compile-time analysis of state access patterns
//! - **Dynamic Profiling**: Runtime analysis for optimization feedback
//! - **Pattern Recognition**: Machine learning for access pattern classification
//! - **Predictive Analysis**: Forecasting future access patterns for optimization

pub mod conflicts;
pub mod optimization;
pub mod read_write;

pub use conflicts::{ConflictDetector, StateAccess, StateAccessType, StateConflict};
pub use optimization::AccessOptimizationHints;
pub use read_write::ReadWriteAnalyzer;

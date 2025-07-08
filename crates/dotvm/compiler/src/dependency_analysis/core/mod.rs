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

//! Core analysis framework and engine infrastructure
//!
//! This module contains the fundamental components that power the dependency analysis
//! system. It provides the core engine, analysis context, graph structures, and
//! scheduling infrastructure that coordinate all analysis activities.
//!
//! ## Core Components
//!
//! ### Analysis Engine (`engine`)
//! - **Purpose**: Central orchestrator for all dependency analysis operations
//! - **Responsibilities**: Coordinates analyzers, manages results, handles caching
//! - **Features**: Configurable analysis pipelines, result aggregation, error handling
//! - **Integration**: Interfaces with all other analysis modules
//!
//! ### Analysis Context (`context`)
//! - **Purpose**: Maintains shared state and configuration during analysis
//! - **Contents**: Analysis parameters, intermediate results, shared data structures
//! - **Scope**: Thread-safe context sharing across multiple analyzers
//! - **Lifecycle**: Created per analysis session, destroyed when analysis completes
//!
//! ### Dependency Graph (`graph`)
//! - **Purpose**: Represents dependencies as a directed graph structure
//! - **Nodes**: Modules, functions, variables, or other dependency entities
//! - **Edges**: Dependency relationships with type and strength information
//! - **Operations**: Graph traversal, cycle detection, topological sorting
//!
//! ### Analysis Scheduler (`scheduler`)
//! - **Purpose**: Manages execution order and parallelization of analyses
//! - **Strategies**: Dependency-aware scheduling, parallel execution, resource management
//! - **Optimization**: Load balancing, priority-based execution, resource allocation
//! - **Coordination**: Ensures analyses run in correct order with proper dependencies
//!
//! ### Core Traits (`traits`)
//! - **Purpose**: Defines common interfaces for all analysis components
//! - **Standardization**: Ensures consistent APIs across different analyzers
//! - **Extensibility**: Enables easy addition of new analysis types
//! - **Integration**: Facilitates composition of different analysis strategies
//!
//! ## Architecture Principles
//!
//! ### Modularity
//! - Each component has a well-defined responsibility
//! - Clean interfaces between components
//! - Easy to test and maintain individual components
//! - Supports incremental development and enhancement
//!
//! ### Extensibility
//! - Plugin architecture for adding new analyzers
//! - Configurable analysis pipelines
//! - Support for custom analysis strategies
//! - Integration points for external tools
//!
//! ### Performance
//! - Parallel execution where possible
//! - Efficient data structures for large programs
//! - Caching and memoization for repeated analyses
//! - Resource-aware scheduling and execution
//!
//! ### Reliability
//! - Comprehensive error handling and recovery
//! - Graceful degradation when analyses fail
//! - Validation of analysis results
//! - Robust handling of malformed input
//!
//! ## Integration with Compiler Pipeline
//!
//! The core framework integrates with the broader DotVM compiler pipeline:
//! - **Input**: Receives parsed AST or bytecode from earlier compiler stages
//! - **Processing**: Coordinates multiple analysis passes with proper dependencies
//! - **Output**: Provides structured analysis results to optimization and code generation
//! - **Feedback**: Supports iterative analysis and refinement
pub mod context;
pub mod engine;
pub mod graph;
pub mod scheduler;
pub mod traits;

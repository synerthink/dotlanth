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

//! Pass scheduling components

/// Stub for a dependency resolver
pub struct DependencyResolver;

/// Stub for parallelization configuration
pub struct ParallelizationHints;

/// Execution strategies for pass scheduling
pub enum ExecutionStrategy {
    /// Run passes one after another
    Sequential,
    /// Run passes in parallel where possible
    Parallel,
    /// Adaptive strategy based on profiling
    Adaptive,
}

/// Scheduler for ordering and executing passes
pub struct PassScheduler {
    /// Resolver for pass dependencies (stub)
    pub dependency_resolver: DependencyResolver,
    /// Strategy for execution (sequential, parallel, etc.)
    pub execution_strategy: ExecutionStrategy,
    /// Hints for parallel execution (stub)
    pub parallelization_hints: ParallelizationHints,
}

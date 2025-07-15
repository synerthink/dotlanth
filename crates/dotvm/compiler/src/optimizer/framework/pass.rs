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

//! Optimization pass interface

use crate::optimizer::framework::metrics::{OptimizationMetrics, OptimizationWarning};

/// Trait representing a single optimization pass
pub trait OptimizationPass {
    /// Type of input to the pass
    type Input;
    /// Type of output from the pass
    type Output;
    /// Configuration type for controlling the pass
    type Config;
    /// Metrics type produced by the pass
    type Metrics;

    /// Unique name of the pass
    fn name(&self) -> &str;
    /// Short description of the pass
    fn description(&self) -> &str;
    /// Names of passes that must run before this one
    fn dependencies(&self) -> &[&str];
    /// Names of passes that conflict with this one
    fn conflicts_with(&self) -> &[&str];

    /// Determine if the pass can run on the given input
    fn can_optimize(&self, input: &Self::Input, config: &Self::Config) -> bool;
    /// Run the pass, returning the transformed output, change flag, metrics, and warnings
    fn optimize(&mut self, input: Self::Input, config: &Self::Config) -> OptimizationResult<Self::Output>;
    /// Retrieve pass-local metrics
    fn metrics(&self) -> &Self::Metrics;
}

/// Result of running an optimization pass
pub struct OptimizationResult<T> {
    /// Transformed output
    pub output: T,
    /// Whether the pass changed the input
    pub changed: bool,
    /// Global metrics recorded for this pass invocation
    pub metrics: OptimizationMetrics,
    /// Warnings emitted during the pass
    pub warnings: Vec<OptimizationWarning>,
}

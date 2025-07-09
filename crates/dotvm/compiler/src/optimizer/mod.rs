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

//! Optimization passes for DotVM bytecode

pub mod analysis;
pub mod framework;
pub mod passes;
pub mod profiling;

use crate::optimizer::framework::metrics::OptimizationMetrics as PipelineMetrics;
use crate::optimizer::framework::pipeline::{OptimizationConfig, OptimizationPipeline};
use crate::optimizer::framework::scheduler::ExecutionStrategy;
use crate::optimizer::passes as opt_passes;
use crate::transpiler::types::TranspiledFunction;
use dotvm_core::bytecode::VmArchitecture;

/// Main optimizer that coordinates all optimization passes
pub struct Optimizer {
    pipeline: OptimizationPipeline,
}

impl Optimizer {
    /// Create a new optimizer with the given target architecture and level
    pub fn new(target_arch: VmArchitecture, optimization_level: u8) -> Self {
        let config = OptimizationConfig { target_arch, optimization_level };
        let mut pipeline = OptimizationPipeline::new(config, ExecutionStrategy::Sequential);
        // Register optimization passes in pipeline order
        // TODO: Re-enable passes once they implement the new trait
        // pipeline.add_pass(passes::constant_folding::ConstantFolder::new());
        // pipeline.add_pass(passes::dead_code::DeadCodeEliminator::new());
        // pipeline.add_pass(passes::peephole::PeepholeOptimizer::new(target_arch));
        Self { pipeline }
    }

    /// Optimize a list of transpiled functions through the pipeline
    pub fn optimize(&mut self, functions: Vec<TranspiledFunction>) -> Vec<TranspiledFunction> {
        self.pipeline.run(functions)
    }

    /// Get pipeline-level optimization metrics
    pub fn stats(&self) -> PipelineMetrics {
        self.pipeline.metrics().clone()
    }

    /// Reset all pipeline statistics and cache
    pub fn reset_stats(&mut self) {
        self.pipeline.reset();
    }
}

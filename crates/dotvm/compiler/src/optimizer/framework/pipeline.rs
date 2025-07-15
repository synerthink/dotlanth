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

//! Optimization framework: pass interface, pipeline, scheduler, and utilities

use crate::optimizer::framework::cache::{NoopCache, OptimizationCache};
use crate::optimizer::framework::metrics::{OptimizationMetrics, PassMetrics};
use crate::optimizer::framework::pass::{OptimizationPass, OptimizationResult};
use crate::optimizer::framework::scheduler::{DependencyResolver, ExecutionStrategy, ParallelizationHints, PassScheduler};
use crate::optimizer::passes::constant_folding::{ConstantFolder, FoldingStats};
use crate::transpiler::types::TranspiledFunction;
use dotvm_core::bytecode::VmArchitecture;

/// Configuration for the optimization pipeline
pub struct OptimizationConfig {
    pub target_arch: VmArchitecture,
    pub optimization_level: u8,
}

/// Trait for collecting pipeline-level metrics
pub trait MetricsCollector {
    /// Record metrics for a completed pass
    fn record_pass(&mut self, metrics: PassMetrics);
    /// Retrieve collected metrics
    fn collect(&self) -> &OptimizationMetrics;
}

/// No-op metrics collector that records metrics in memory
pub struct NoopMetricsCollector {
    metrics: OptimizationMetrics,
}

impl NoopMetricsCollector {
    /// Create a new collector
    pub fn new() -> Self {
        Self {
            metrics: OptimizationMetrics::default(),
        }
    }
}

impl MetricsCollector for NoopMetricsCollector {
    fn record_pass(&mut self, m: PassMetrics) {
        self.metrics.pass_metrics.push(m);
        self.metrics.total_passes += 1;
    }
    fn collect(&self) -> &OptimizationMetrics {
        &self.metrics
    }
}

/// Core optimization pipeline that runs a series of passes
pub struct OptimizationPipeline {
    passes: Vec<Box<dyn OptimizationPass<Input = TranspiledFunction, Output = TranspiledFunction, Config = OptimizationConfig, Metrics = FoldingStats>>>,
    scheduler: PassScheduler,
    metrics: NoopMetricsCollector,
    cache: NoopCache,
    config: OptimizationConfig,
}

impl OptimizationPipeline {
    /// Create a new pipeline with the given config and strategy
    pub fn new(config: OptimizationConfig, strategy: ExecutionStrategy) -> Self {
        Self {
            passes: Vec::new(),
            scheduler: PassScheduler {
                dependency_resolver: DependencyResolver,
                execution_strategy: strategy,
                parallelization_hints: ParallelizationHints,
            },
            metrics: NoopMetricsCollector::new(),
            cache: NoopCache,
            config,
        }
    }

    /// Add an optimization pass to the pipeline
    pub fn add_pass<P>(&mut self, pass: P)
    where
        P: OptimizationPass<Input = TranspiledFunction, Output = TranspiledFunction, Config = OptimizationConfig, Metrics = FoldingStats> + 'static,
    {
        self.passes.push(Box::new(pass));
    }

    /// Execute all passes on the given functions
    pub fn run(&mut self, functions: Vec<TranspiledFunction>) -> Vec<TranspiledFunction> {
        let mut current = functions;
        for pass in &mut self.passes {
            let mut next = Vec::with_capacity(current.len());
            for func in current {
                if pass.can_optimize(&func, &self.config) {
                    let result = pass.optimize(func, &self.config);
                    let metrics = PassMetrics {
                        pass_name: pass.name().to_string(),
                        duration_ms: 0,
                        changed: result.changed,
                    };
                    self.metrics.record_pass(metrics);
                    next.push(result.output);
                } else {
                    next.push(func);
                }
            }
            current = next;
        }
        current
    }

    /// Retrieve pipeline-level metrics
    pub fn metrics(&self) -> &OptimizationMetrics {
        self.metrics.collect()
    }

    /// Clear metrics and cache
    pub fn reset(&mut self) {
        self.metrics = NoopMetricsCollector::new();
        self.cache = NoopCache;
    }
}

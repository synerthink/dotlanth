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

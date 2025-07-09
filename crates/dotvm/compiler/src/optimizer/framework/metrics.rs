//! Optimization metrics and warnings

/// Metrics for a single pass invocation
#[derive(Debug, Clone)]
pub struct PassMetrics {
    /// Name of the pass
    pub pass_name: String,
    /// Duration of the pass in milliseconds
    pub duration_ms: u128,
    /// Whether the pass reported a change
    pub changed: bool,
}

/// Warning produced during optimization
#[derive(Debug, Clone)]
pub struct OptimizationWarning {
    /// Pass that emitted the warning
    pub pass_name: String,
    /// Warning message
    pub message: String,
}

/// Global optimization metrics recorded by the pipeline
#[derive(Default, Debug, Clone)]
pub struct OptimizationMetrics {
    /// Per-pass metrics summary
    pub pass_metrics: Vec<PassMetrics>,
    /// Total number of passes executed
    pub total_passes: usize,
}

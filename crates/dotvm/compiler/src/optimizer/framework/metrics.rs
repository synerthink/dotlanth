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

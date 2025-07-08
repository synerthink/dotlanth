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

//! Metrics collection for analysis

/// Analysis metrics (e.g., performance, counts)
#[derive(Debug, Clone, Default)]
pub struct AnalysisMetrics {
    pub nodes_analyzed: usize,
    pub dependencies_found: usize,
    pub duration_ms: u64,
}

impl AnalysisMetrics {
    pub fn new() -> Self {
        Self::default()
    }
}

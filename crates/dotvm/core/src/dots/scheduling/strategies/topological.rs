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

// Purpose: Implements the topological sorting strategy for scheduling.

use crate::dots::{DependencyGraph, ProcessingError, ProcessingOrder};

/// Schedules segments based on topological sort of the dependency graph.
///
/// # Arguments
/// * `dependency_graph`: The graph representing dependencies between dot segments.
///
/// # Returns
/// * `Result<ProcessingOrder, ProcessingError>`: The processing order if successful, or an error.
pub fn schedule_topological(dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
    let ordered_ids = dependency_graph.topological_sort()?;
    Ok(ProcessingOrder::new(ordered_ids))
}

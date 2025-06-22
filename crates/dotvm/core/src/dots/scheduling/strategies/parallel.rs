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

// Purpose: Implements the parallel scheduling strategy.

use crate::dots::{DependencyGraph, DotSegment, ProcessingError, ProcessingOrder};
use std::collections::HashSet;

/// Schedules segments for parallel processing where possible, based on dependencies.
///
/// # Arguments
/// * `_segments`: A slice of dot segments (currently unused but kept for API consistency).
/// * `dependency_graph`: The graph representing dependencies between dot segments.
///
/// # Returns
/// * `Result<ProcessingOrder, ProcessingError>`: The processing order with parallel batches if successful, or an error.
pub fn schedule_parallel(
    _segments: &[DotSegment], // Kept for API consistency, though not directly used in this version
    dependency_graph: &DependencyGraph,
) -> Result<ProcessingOrder, ProcessingError> {
    let ordered_ids = dependency_graph.topological_sort()?;
    if ordered_ids.is_empty() {
        return Ok(ProcessingOrder::with_parallelization(Vec::new(), Vec::new()));
    }

    let mut parallel_batches: Vec<Vec<String>> = Vec::new();
    let mut processed_ids: HashSet<String> = HashSet::new();

    while processed_ids.len() < ordered_ids.len() {
        let mut current_batch: Vec<String> = Vec::new();

        for segment_id in &ordered_ids {
            if processed_ids.contains(segment_id) {
                continue;
            }

            let dependencies = dependency_graph.get_dependencies(segment_id);
            let all_dependencies_processed = dependencies.iter().all(|dep_id| processed_ids.contains(dep_id));

            if all_dependencies_processed {
                current_batch.push(segment_id.clone());
            }
        }

        if current_batch.is_empty() {
            // This implies a cycle or an issue if not all segments are processed,
            // but topological sort should have caught cycles.
            // If ordered_ids is not empty and processed_ids < ordered_ids.len(),
            // this means some segments couldn't be scheduled.
            return Err(ProcessingError::SchedulingFailed(
                "Failed to create parallel batches, possible unresolved dependencies or cycle not caught by toposort. Processed all available segments.".to_string(),
            ));
        }

        for segment_id in &current_batch {
            processed_ids.insert(segment_id.clone());
        }
        parallel_batches.push(current_batch);
    }

    Ok(ProcessingOrder::with_parallelization(ordered_ids, parallel_batches))
}

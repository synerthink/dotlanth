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

// Purpose: Implements the by-type scheduling strategy.

use crate::dots::{DependencyGraph, DotSegment, ProcessingError, ProcessingOrder};
use std::collections::{HashMap, HashSet};

/// Schedules dot segments by type, respecting dependencies.
///
/// # Arguments
/// * `segments`: A slice of dot segments to schedule.
/// * `dependency_graph`: The graph representing dependencies between dot segments.
///
/// # Returns
/// * `Result<ProcessingOrder, ProcessingError>`: The processing order if successful, or an error.
pub fn schedule_by_type(segments: &[DotSegment], dependency_graph: &DependencyGraph) -> Result<ProcessingOrder, ProcessingError> {
    let topological_ids = dependency_graph.topological_sort()?;
    let type_order = ["SECTION", "ARTICLE", "CLAUSE"];

    let mut type_groups: HashMap<&str, Vec<String>> = HashMap::new();
    for segment in segments {
        type_groups.entry(segment.segment_type.as_str()).or_insert_with(Vec::new).push(segment.id.clone());
    }

    let mut ordered_ids: Vec<String> = Vec::new();
    let mut processed_segments: HashSet<String> = HashSet::new();

    process_segments_for_type_order(&type_order, &type_groups, dependency_graph, &mut ordered_ids, &mut processed_segments);

    append_remaining_segments_topologically(&topological_ids, dependency_graph, &mut ordered_ids, &mut processed_segments);

    // Final consolidation to ensure all topologically sortable segments are included
    // This primarily catches segments whose types are not in type_order or complex cases.
    let final_check_topological_ids = dependency_graph.topological_sort()?; // Re-sort or use initial if guaranteed no mutation
    if ordered_ids.len() < final_check_topological_ids.len() {
        for topo_id in final_check_topological_ids {
            if !processed_segments.contains(&topo_id) {
                // Check processed_segments, not ordered_ids for contains
                // Ensure dependencies are met before adding
                let dependencies = dependency_graph.get_dependencies(&topo_id);
                let all_dependencies_included = dependencies.iter().all(|dep_id| processed_segments.contains(dep_id));
                if all_dependencies_included {
                    ordered_ids.push(topo_id.clone());
                    processed_segments.insert(topo_id.clone()); // Mark as processed
                }
            }
        }
    }
    // Ensure the final list is unique and maintains a valid topological order if segments were added.
    // A simple way to ensure this is to rebuild based on topological_ids if discrepancies are found,
    // or to filter topological_ids by processed_segments and append.
    // For now, the above logic aims to append missing items if their deps are met.

    Ok(ProcessingOrder::new(ordered_ids))
}

/// Helper to process segments based on a predefined type order.
fn process_segments_for_type_order(
    type_order: &[&str],
    type_groups: &HashMap<&str, Vec<String>>,
    dependency_graph: &DependencyGraph,
    ordered_ids: &mut Vec<String>,
    processed_segments: &mut HashSet<String>,
) {
    for segment_type in type_order.iter() {
        if let Some(segment_ids_for_type) = type_groups.get(*segment_type) {
            let mut changed_in_iteration = true;
            while changed_in_iteration {
                changed_in_iteration = false;
                for segment_id in segment_ids_for_type {
                    if processed_segments.contains(segment_id) {
                        continue;
                    }
                    let dependencies = dependency_graph.get_dependencies(segment_id);
                    let all_dependencies_processed = dependencies.iter().all(|dep_id| processed_segments.contains(dep_id));

                    if all_dependencies_processed {
                        ordered_ids.push(segment_id.clone());
                        processed_segments.insert(segment_id.clone());
                        changed_in_iteration = true;
                    }
                }
            }
        }
    }
}

/// Helper to append remaining segments in topological order.
fn append_remaining_segments_topologically(topological_ids: &[String], dependency_graph: &DependencyGraph, ordered_ids: &mut Vec<String>, processed_segments: &mut HashSet<String>) {
    for segment_id in topological_ids {
        if !processed_segments.contains(segment_id) {
            let dependencies = dependency_graph.get_dependencies(segment_id);
            let all_dependencies_processed = dependencies.iter().all(|dep_id| processed_segments.contains(dep_id));
            if all_dependencies_processed {
                ordered_ids.push(segment_id.clone());
                processed_segments.insert(segment_id.clone());
            }
        }
    }
}

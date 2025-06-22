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

// Purpose: Implements the complexity-first scheduling strategy.

use crate::dots::{DependencyGraph, DotSegment, ProcessingError, ProcessingOrder};
use std::collections::{HashMap, VecDeque};

/// Schedules dot segments by complexity, with complex segments first if possible,
/// while respecting dependencies.
///
/// # Arguments
/// * `segments`: A slice of dot segments to schedule.
/// * `dependency_graph`: The graph representing dependencies between dot segments.
/// * `priority_fn_opt`: An optional function to calculate custom priority/complexity.
///
/// # Returns
/// * `Result<ProcessingOrder, ProcessingError>`: The processing order if successful, or an error.
pub fn schedule_by_complexity(segments: &[DotSegment], dependency_graph: &DependencyGraph, priority_fn_opt: &Option<Box<dyn Fn(&DotSegment) -> i32>>) -> Result<ProcessingOrder, ProcessingError> {
    let mut base_topological_ids = dependency_graph.topological_sort()?;
    if base_topological_ids.is_empty() && !segments.is_empty() {
        // If graph is empty (no dependencies) but segments exist, use all segment ids
        base_topological_ids = segments.iter().map(|s| s.id.clone()).collect();
    }

    let complexities = calculate_segment_complexities(segments, &base_topological_ids, priority_fn_opt);
    let (mut in_degree, graph_adj) = initialize_scheduling_graph_structures(&base_topological_ids, dependency_graph);

    let mut ordered_ids: Vec<String> = Vec::new();
    let mut queue: VecDeque<String> = base_topological_ids.iter().filter(|id| in_degree.get(*id).map_or(false, |d| *d == 0)).cloned().collect();

    while !queue.is_empty() {
        // Sort queue by complexity (descending) before picking
        queue
            .make_contiguous()
            .sort_unstable_by(|a, b| complexities.get(b).unwrap_or(&0).cmp(complexities.get(a).unwrap_or(&0)));

        let segment_id = queue.pop_front().unwrap();
        ordered_ids.push(segment_id.clone());

        if let Some(dependents) = graph_adj.get(&segment_id) {
            for dependent_id in dependents {
                if let Some(degree) = in_degree.get_mut(dependent_id) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(dependent_id.clone());
                    }
                }
            }
        }
    }

    if ordered_ids.len() < base_topological_ids.len() {
        return Err(ProcessingError::SchedulingFailed(
            "Complexity scheduling resulted in incomplete order, possibly due to cycle or missing segments in complexity map.".to_string(),
        ));
    }

    Ok(ProcessingOrder::new(ordered_ids))
}

/// Helper to calculate complexities for segments.
fn calculate_segment_complexities(segments: &[DotSegment], relevant_ids: &[String], priority_fn_opt: &Option<Box<dyn Fn(&DotSegment) -> i32>>) -> HashMap<String, i32> {
    let mut complexities: HashMap<String, i32> = HashMap::new();
    let segment_map: HashMap<String, &DotSegment> = segments.iter().map(|s| (s.id.clone(), s)).collect();

    for segment_id in relevant_ids {
        if let Some(segment) = segment_map.get(segment_id) {
            let complexity = if let Some(priority_fn) = priority_fn_opt {
                priority_fn(segment)
            } else {
                segment.content.len() as i32
            };
            complexities.insert(segment_id.clone(), complexity);
        } else {
            // Segment in graph but not in input segments list, assign default complexity
            complexities.insert(segment_id.clone(), 0);
        }
    }
    complexities
}

/// Helper to initialize graph structures for Kahn's algorithm.
fn initialize_scheduling_graph_structures(segment_ids: &[String], dependency_graph: &DependencyGraph) -> (HashMap<String, usize>, HashMap<String, Vec<String>>) {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph_adj: HashMap<String, Vec<String>> = HashMap::new();

    for id in segment_ids {
        in_degree.insert(id.clone(), dependency_graph.get_dependencies(id).len());
        graph_adj.insert(id.clone(), dependency_graph.get_dependents(id));
    }
    (in_degree, graph_adj)
}

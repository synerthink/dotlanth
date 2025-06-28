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

// Purpose: Implements hierarchical dependency detection based on segment types and order.

use crate::dots::dependencies::DependencyDetectionStrategy;
use crate::dots::{DependencyGraph, DependencyType, DotSegment, ProcessingError};
use std::collections::HashMap;

/// Detects hierarchical relationships based on segment types.
pub struct HierarchicalDetectionStrategy {}

impl DependencyDetectionStrategy for HierarchicalDetectionStrategy {
    /// Establishes dependencies based on:
    /// - Segment type hierarchy (SECTION > ARTICLE > CLAUSE)
    /// - Position within same dot
    fn detect_dependencies(&self, segments: &[DotSegment], graph: &mut DependencyGraph) -> Result<(), ProcessingError> {
        let segment_types = ["SECTION", "ARTICLE", "CLAUSE"];

        // Map the segment types in priority order.
        let type_priority: HashMap<&str, usize> = segment_types.iter().enumerate().map(|(i, &s_type)| (s_type, i)).collect();

        // Group segments by type
        let mut grouped_segments: HashMap<&str, Vec<&DotSegment>> = HashMap::new();
        for segment in segments {
            grouped_segments.entry(segment.segment_type.as_str()).or_default().push(segment);
        }

        // For each segment type, add dependency on higher priority types
        for (segment_type, segments_of_type) in &grouped_segments {
            if let Some(&type_prio) = type_priority.get(segment_type) {
                for segment in segments_of_type {
                    for higher_type in segment_types.iter().take(type_prio) {
                        if let Some(higher_segments) = grouped_segments.get(higher_type) {
                            for higher_segment in higher_segments {
                                if segment.dot_id == higher_segment.dot_id && segment.position > higher_segment.position {
                                    graph.add_dependency(&segment.id, &higher_segment.id, DependencyType::Prerequisite);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

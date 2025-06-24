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

// Purpose: Implements reference-based dependency detection.

use crate::dots::dependencies::DependencyDetectionStrategy;
use crate::dots::{DependencyGraph, DependencyType, DotSegment, ProcessingError};
use std::collections::HashMap;

/// Detects references through content analysis.
pub struct ReferenceDetectionStrategy {}

impl DependencyDetectionStrategy for ReferenceDetectionStrategy {
    /// Identifies dependencies by searching for:
    /// - "see {id}" patterns
    /// - "refer to {id}" patterns
    /// - "as per {id}" patterns
    fn detect_dependencies(&self, segments: &[DotSegment], graph: &mut DependencyGraph) -> Result<(), ProcessingError> {
        let segment_map: HashMap<&str, &DotSegment> = segments.iter().map(|s| (s.id.as_str(), s)).collect();

        for dependent in segments {
            for (_potential_dependency_id, dependency) in &segment_map {
                // Underscored unused variable
                // Self-dependencies are not allowed
                if dependent.id == dependency.id {
                    continue;
                }

                // Check for reference patterns more efficiently without format! allocations
                if self.contains_reference_pattern(&dependent.content, &dependency.id) {
                    graph.add_dependency(&dependent.id, &dependency.id, DependencyType::Reference);
                }
            }
        }
        Ok(())
    }
}

impl ReferenceDetectionStrategy {
    /// Efficiently checks for reference patterns without string allocations
    fn contains_reference_pattern(&self, content: &str, dependency_id: &str) -> bool {
        let content_lower = content.to_lowercase();
        let patterns = [("see ", 4), ("refer to ", 9), ("as per ", 7)];

        for (pattern, offset) in &patterns {
            let mut search_start = 0;
            while let Some(pos) = content_lower[search_start..].find(pattern) {
                let actual_pos = search_start + pos;
                let after_pattern = &content[actual_pos + offset..];
                if after_pattern.starts_with(dependency_id) {
                    // Ensure it's a complete word match (not part of a larger word)
                    let end_pos = dependency_id.len();
                    if after_pattern.len() == end_pos || after_pattern.chars().nth(end_pos).map_or(true, |c| !c.is_alphanumeric() && c != '_') {
                        return true;
                    }
                }
                search_start = actual_pos + 1;
            }
        }

        false
    }
}

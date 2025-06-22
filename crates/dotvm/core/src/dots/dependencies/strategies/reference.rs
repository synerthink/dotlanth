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

                if dependent.content.contains(&format!("see {}", dependency.id))
                    || dependent.content.contains(&format!("refer to {}", dependency.id))
                    || dependent.content.contains(&format!("as per {}", dependency.id))
                {
                    graph.add_dependency(&dependent.id, &dependency.id, DependencyType::Reference);
                }
            }
        }
        Ok(())
    }
}

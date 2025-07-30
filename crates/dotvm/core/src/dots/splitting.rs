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

//! Dot segmentation module
//!
//! Splits raw dot text into structured segments based on configurable patterns

use crate::dots::error::ProcessingError;
use crate::dots::lib::Dot;
use std::collections::HashMap;

/// Represents a segment of a dot after splitting
#[derive(Debug, Clone)]
pub struct DotSegment {
    /// Unique identifier for the segment
    pub id: String,

    /// Reference to the parent dot
    pub dot_id: String,

    /// Type of segment (e.g., "clause", "condition", "definition")
    pub segment_type: String,

    /// The actual content of the segment
    pub content: String,

    /// Position in the original dot
    pub position: usize,

    /// Metadata relevant to this segment
    pub metadata: HashMap<String, String>,
}

impl DotSegment {
    /// Create a new dot segment
    pub fn new(id: String, dot_id: String, segment_type: String, content: String, position: usize) -> Self {
        Self {
            id,
            dot_id,
            segment_type,
            content,
            position,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the segment
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Configuration for dot segmentation
#[derive(Debug, Clone)]
pub struct SegmentCriteria {
    /// Regular expression patterns to identify segment boundaries
    pub patterns: Vec<String>,

    /// Minimum segment size in characters
    pub min_size: usize,

    /// Maximum segment size in characters
    pub max_size: usize,

    /// Whether to preserve hierarchical relationships between segments
    pub preserve_hierarchy: bool,
}

impl Default for SegmentCriteria {
    fn default() -> Self {
        Self {
            patterns: vec![r"SECTION \d+".to_string(), r"ARTICLE \d+".to_string(), r"CLAUSE \d+".to_string()],
            min_size: 50,
            max_size: 5000,
            preserve_hierarchy: true,
        }
    }
}

/// Extracts segments from dots based on specified criteria
pub struct SegmentExtractor {
    #[allow(dead_code)] // TODO: Implement splitting logic based on these criteria
    criteria: SegmentCriteria,
}

impl Default for SegmentExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SegmentExtractor {
    /// Creates extractor with default criteria:
    /// - Patterns: SECTION/ARTICLE/CLAUSE headers
    /// - Min size: 50 chars
    /// - Max size: 5000 chars
    pub fn new() -> Self {
        Self { criteria: SegmentCriteria::default() }
    }

    /// Create a new segment extractor with custom criteria
    pub fn with_criteria(criteria: SegmentCriteria) -> Self {
        Self { criteria }
    }

    /// Processes dot content into segments
    ///
    /// # Steps
    /// 1. Validate non-empty content
    /// 2. Split by header patterns
    /// 3. Apply size constraints
    ///
    /// # Returns
    /// - Ok(Vec<DotSegment>): Valid segments
    /// - Err(ProcessingError): On empty content/split failure
    pub fn extract_segments(&self, dot: &Dot) -> Result<Vec<DotSegment>, ProcessingError> {
        if dot.content.trim().is_empty() {
            return Err(ProcessingError::SplittingFailed("Dot content is empty".to_string()));
        }

        let mut segments = Vec::new();
        let content_parts = self.split_by_patterns(&dot.content);

        for (i, (segment_type, content)) in content_parts.into_iter().enumerate() {
            let segment = DotSegment::new(format!("{}-seg-{}", dot.id, i), dot.id.clone(), segment_type, content, i);
            segments.push(segment);
        }

        if segments.is_empty() {
            return Err(ProcessingError::SplittingFailed("Dot could not be split into segments".to_string()));
        }

        Ok(segments)
    }

    /// Core splitting logic using line-by-line analysis
    fn split_by_patterns(&self, content: &str) -> Vec<(String, String)> {
        let mut result = Vec::new();

        // If no pattern matches, append all content as a single segment
        if !self.has_any_pattern_match(content) {
            result.push(("DEFAULT".to_string(), content.to_string()));
            return result;
        }

        // Division process
        let lines: Vec<&str> = content.split('\n').collect();
        let mut current_segment_type = "DEFAULT";
        let mut current_content = String::new();

        for line in lines {
            let segment_type = self.get_segment_type(line);

            if segment_type.is_some() {
                // If you found a new segment start, save the previous one
                if !current_content.is_empty() {
                    result.push((current_segment_type.to_string(), current_content.trim().to_string()));
                    current_content = String::new();
                }
                current_segment_type = segment_type.unwrap();
            }

            current_content.push_str(line);
            current_content.push('\n');
        }

        // Add last segment
        if !current_content.is_empty() {
            result.push((current_segment_type.to_string(), current_content.trim().to_string()));
        }

        result
    }

    /// Check if any pattern matches in the content
    fn has_any_pattern_match(&self, content: &str) -> bool {
        content.contains("SECTION") || content.contains("ARTICLE") || content.contains("CLAUSE")
    }

    /// Determine the segment type based on the line content
    fn get_segment_type(&self, line: &str) -> Option<&str> {
        if line.contains("SECTION") {
            Some("SECTION")
        } else if line.contains("ARTICLE") {
            Some("ARTICLE")
        } else if line.contains("CLAUSE") {
            Some("CLAUSE")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_segments() {
        let dot = Dot {
            id: "dot-001".to_string(),
            content: "SECTION 1\nThis is section 1 content.\n\nARTICLE 2\nThis is article 2 content.\n\nCLAUSE 3\nThis is clause 3 content.".to_string(),
        };

        let extractor = SegmentExtractor::new();
        let segments = extractor.extract_segments(&dot).unwrap();

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].segment_type, "SECTION");
        assert_eq!(segments[1].segment_type, "ARTICLE");
        assert_eq!(segments[2].segment_type, "CLAUSE");
    }

    #[test]
    fn test_empty_dot() {
        let dot = Dot {
            id: "empty-dot".to_string(),
            content: "".to_string(),
        };

        let extractor = SegmentExtractor::new();
        let result = extractor.extract_segments(&dot);

        assert!(result.is_err());
    }
}

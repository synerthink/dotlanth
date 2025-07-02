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

//! Parser context for tracking state and metadata

use super::super::ast::{SectionMetadata, SectionOrderValidator, WasmSectionType};
use std::collections::HashMap;

/// Parser context for tracking parsing state and collecting metadata
#[derive(Debug, Default)]
pub struct ParserContext {
    /// Section order validator
    pub section_validator: SectionOrderValidator,
    /// Section metadata
    pub section_metadata: Vec<SectionMetadata>,
    /// Current parsing position
    pub position: usize,
    /// Warnings collected during parsing
    pub warnings: Vec<String>,
    /// Performance metrics
    pub metrics: ParsingMetrics,
    /// Custom data for extensions
    pub custom_data: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl ParserContext {
    /// Create a new parser context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Record section metadata
    pub fn record_section(&mut self, metadata: SectionMetadata) {
        // Validate section order
        if let Err(error) = self.section_validator.validate_section(metadata.section_type) {
            self.add_warning(format!("Section order violation: {}", error));
        }

        self.section_metadata.push(metadata);
    }

    /// Get metadata for a specific section type
    pub fn get_section_metadata(&self, section_type: WasmSectionType) -> Vec<&SectionMetadata> {
        self.section_metadata.iter().filter(|meta| meta.section_type == section_type).collect()
    }

    /// Get the total size of all sections
    pub fn total_size(&self) -> usize {
        self.section_metadata.iter().map(|meta| meta.size).sum()
    }

    /// Get the number of sections
    pub fn section_count(&self) -> usize {
        self.section_metadata.len()
    }

    /// Check if a section type was encountered
    pub fn has_section(&self, section_type: WasmSectionType) -> bool {
        self.section_metadata.iter().any(|meta| meta.section_type == section_type)
    }

    /// Update parsing position
    pub fn update_position(&mut self, position: usize) {
        self.position = position;
    }

    /// Record parsing start time
    pub fn start_parsing(&mut self) {
        self.metrics.start_time = Some(std::time::Instant::now());
    }

    /// Record parsing completion
    pub fn finish_parsing(&mut self) {
        if let Some(start) = self.metrics.start_time {
            self.metrics.total_time = Some(start.elapsed());
        }
    }

    /// Record section parsing time
    pub fn record_section_time(&mut self, section_type: WasmSectionType, duration: std::time::Duration) {
        self.metrics.section_times.insert(section_type, duration);
    }

    /// Get parsing summary
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        summary.push_str("=== WASM Parsing Summary ===\n");
        summary.push_str(&format!("Total size: {} bytes\n", self.total_size()));
        summary.push_str(&format!("Sections: {}\n", self.section_count()));

        if let Some(total_time) = self.metrics.total_time {
            summary.push_str(&format!("Parse time: {:.2}ms\n", total_time.as_millis()));
        }

        if !self.warnings.is_empty() {
            summary.push_str(&format!("Warnings: {}\n", self.warnings.len()));
        }

        summary
    }

    /// Reset the context for reuse
    pub fn reset(&mut self) {
        self.section_validator.reset();
        self.section_metadata.clear();
        self.position = 0;
        self.warnings.clear();
        self.metrics = ParsingMetrics::default();
        self.custom_data.clear();
    }
}

/// Performance metrics for parsing
#[derive(Debug, Default)]
pub struct ParsingMetrics {
    /// Start time of parsing
    pub start_time: Option<std::time::Instant>,
    /// Total parsing time
    pub total_time: Option<std::time::Duration>,
    /// Time spent parsing each section type
    pub section_times: HashMap<WasmSectionType, std::time::Duration>,
    /// Number of bytes parsed
    pub bytes_parsed: usize,
    /// Peak memory usage during parsing
    pub peak_memory_usage: usize,
}

impl ParsingMetrics {
    /// Get parsing rate in bytes per second
    pub fn parsing_rate(&self) -> Option<f64> {
        if let Some(total_time) = self.total_time {
            let seconds = total_time.as_secs_f64();
            if seconds > 0.0 {
                return Some(self.bytes_parsed as f64 / seconds);
            }
        }
        None
    }

    /// Get the slowest section
    pub fn slowest_section(&self) -> Option<(WasmSectionType, std::time::Duration)> {
        self.section_times.iter().max_by_key(|(_, duration)| *duration).map(|(section, duration)| (*section, *duration))
    }

    /// Get performance report
    pub fn report(&self) -> String {
        let mut report = String::new();

        if let Some(total_time) = self.total_time {
            report.push_str(&format!("Total time: {:.2}ms\n", total_time.as_millis()));
        }

        if let Some(rate) = self.parsing_rate() {
            report.push_str(&format!("Parsing rate: {:.2} KB/s\n", rate / 1024.0));
        }

        if !self.section_times.is_empty() {
            report.push_str("\nSection times:\n");
            let mut sections: Vec<_> = self.section_times.iter().collect();
            sections.sort_by_key(|(_, duration)| *duration);
            sections.reverse();

            for (section, duration) in sections {
                report.push_str(&format!("  {}: {:.2}ms\n", section, duration.as_millis()));
            }
        }

        if let Some((section, duration)) = self.slowest_section() {
            report.push_str(&format!("\nSlowest section: {} ({:.2}ms)\n", section, duration.as_millis()));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_context() {
        let mut context = ParserContext::new();

        // Test warning addition
        context.add_warning("Test warning".to_string());
        assert_eq!(context.warnings.len(), 1);

        // Test section recording
        let metadata = SectionMetadata::new(WasmSectionType::Type, 100, 5, 0);
        context.record_section(metadata);
        assert_eq!(context.section_count(), 1);
        assert_eq!(context.total_size(), 100);
        assert!(context.has_section(WasmSectionType::Type));

        // Test position tracking
        context.update_position(50);
        assert_eq!(context.position, 50);
    }

    #[test]
    fn test_parsing_metrics() {
        let mut metrics = ParsingMetrics::default();
        metrics.bytes_parsed = 1000;

        // Simulate 1 second parsing time
        let start = std::time::Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        metrics.total_time = Some(start.elapsed());

        let rate = metrics.parsing_rate();
        assert!(rate.is_some());
        assert!(rate.unwrap() > 0.0);
    }

    #[test]
    fn test_section_metadata_filtering() {
        let mut context = ParserContext::new();

        context.record_section(SectionMetadata::new(WasmSectionType::Type, 100, 5, 0));
        context.record_section(SectionMetadata::new(WasmSectionType::Function, 200, 10, 100));
        context.record_section(SectionMetadata::new(WasmSectionType::Type, 50, 2, 300));

        let type_sections = context.get_section_metadata(WasmSectionType::Type);
        assert_eq!(type_sections.len(), 2);
        assert_eq!(type_sections[0].size, 100);
        assert_eq!(type_sections[1].size, 50);
    }

    #[test]
    fn test_context_reset() {
        let mut context = ParserContext::new();
        context.add_warning("Test".to_string());
        context.record_section(SectionMetadata::new(WasmSectionType::Type, 100, 5, 0));
        context.update_position(50);

        context.reset();
        assert_eq!(context.warnings.len(), 0);
        assert_eq!(context.section_count(), 0);
        assert_eq!(context.position, 0);
    }
}

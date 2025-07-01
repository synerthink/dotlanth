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

//! State access analysis for dependency tracking

use super::{AnalysisError, AnalysisResult, Analyzer};
use std::collections::{HashMap, HashSet};

/// Types of state access
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StateAccessType {
    /// Reading from state
    Read,
    /// Writing to state
    Write,
    /// Modifying existing state
    Modify,
    /// Creating new state
    Create,
    /// Deleting state
    Delete,
}

/// Information about a state access
#[derive(Debug, Clone)]
pub struct StateAccess {
    /// The state variable or location being accessed
    pub location: String,
    /// Type of access
    pub access_type: StateAccessType,
    /// Line number where the access occurs
    pub line_number: Option<usize>,
    /// Additional context or metadata
    pub context: HashMap<String, String>,
}

/// Result of state access analysis
#[derive(Debug, Clone)]
pub struct StateAccessAnalysis {
    /// All state accesses found
    pub accesses: Vec<StateAccess>,
    /// Unique state locations accessed
    pub locations: HashSet<String>,
    /// Access patterns by type
    pub access_patterns: HashMap<StateAccessType, Vec<String>>,
    /// Potential conflicts (read-write, write-write)
    pub conflicts: Vec<StateConflict>,
}

/// Represents a potential state access conflict
#[derive(Debug, Clone)]
pub struct StateConflict {
    /// The state location with conflict
    pub location: String,
    /// First conflicting access
    pub first_access: StateAccessType,
    /// Second conflicting access
    pub second_access: StateAccessType,
    /// Description of the conflict
    pub description: String,
}

/// Analyzer for state access patterns
#[derive(Debug)]
pub struct StateAccessAnalyzer {
    /// Whether to detect potential conflicts
    pub detect_conflicts: bool,
    /// Patterns to recognize state access
    pub access_patterns: HashMap<StateAccessType, Vec<String>>,
}

impl Default for StateAccessAnalyzer {
    fn default() -> Self {
        let mut access_patterns = HashMap::new();

        // Common read patterns
        access_patterns.insert(
            StateAccessType::Read,
            vec!["get_state".to_string(), "read_state".to_string(), "load_state".to_string(), "state.get".to_string()],
        );

        // Common write patterns
        access_patterns.insert(
            StateAccessType::Write,
            vec!["set_state".to_string(), "write_state".to_string(), "store_state".to_string(), "state.set".to_string()],
        );

        // Common modify patterns
        access_patterns.insert(StateAccessType::Modify, vec!["update_state".to_string(), "modify_state".to_string(), "state.update".to_string()]);

        // Common create patterns
        access_patterns.insert(StateAccessType::Create, vec!["create_state".to_string(), "new_state".to_string(), "state.create".to_string()]);

        // Common delete patterns
        access_patterns.insert(StateAccessType::Delete, vec!["delete_state".to_string(), "remove_state".to_string(), "state.delete".to_string()]);

        Self {
            detect_conflicts: true,
            access_patterns,
        }
    }
}

impl StateAccessAnalyzer {
    /// Create a new state access analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable conflict detection
    pub fn with_conflict_detection(mut self, detect: bool) -> Self {
        self.detect_conflicts = detect;
        self
    }

    /// Add a custom access pattern
    pub fn add_pattern(&mut self, access_type: StateAccessType, pattern: String) {
        self.access_patterns.entry(access_type).or_insert_with(Vec::new).push(pattern);
    }

    /// Detect state access type from a line of code
    fn detect_access_type(&self, line: &str) -> Option<StateAccessType> {
        let line_lower = line.to_lowercase();

        for (access_type, patterns) in &self.access_patterns {
            for pattern in patterns {
                if line_lower.contains(&pattern.to_lowercase()) {
                    return Some(access_type.clone());
                }
            }
        }

        None
    }

    /// Extract state location from a line of code
    fn extract_state_location(&self, line: &str, access_type: &StateAccessType) -> Option<String> {
        // Simple pattern matching - in a real implementation, this would be more sophisticated
        let line = line.trim();

        // Look for common patterns like state["key"], state.key, get_state("key")
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }

        if let Some(start) = line.find('\'') {
            if let Some(end) = line[start + 1..].find('\'') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }

        // Fallback: use the access type as a generic location
        Some(format!("unknown_{:?}", access_type).to_lowercase())
    }

    /// Detect conflicts in state accesses
    fn detect_conflicts(&self, accesses: &[StateAccess]) -> Vec<StateConflict> {
        let mut conflicts = Vec::new();
        let mut location_accesses: HashMap<String, Vec<&StateAccess>> = HashMap::new();

        // Group accesses by location
        for access in accesses {
            location_accesses.entry(access.location.clone()).or_insert_with(Vec::new).push(access);
        }

        // Check for conflicts within each location
        for (location, location_access_list) in location_accesses {
            let access_types: HashSet<_> = location_access_list.iter().map(|a| &a.access_type).collect();

            // Check for read-write conflicts
            if access_types.contains(&StateAccessType::Read)
                && (access_types.contains(&StateAccessType::Write) || access_types.contains(&StateAccessType::Modify) || access_types.contains(&StateAccessType::Delete))
            {
                conflicts.push(StateConflict {
                    location: location.clone(),
                    first_access: StateAccessType::Read,
                    second_access: if access_types.contains(&StateAccessType::Write) {
                        StateAccessType::Write
                    } else if access_types.contains(&StateAccessType::Modify) {
                        StateAccessType::Modify
                    } else {
                        StateAccessType::Delete
                    },
                    description: "Potential read-write conflict".to_string(),
                });
            }

            // Check for write-write conflicts
            let write_types = [StateAccessType::Write, StateAccessType::Modify, StateAccessType::Create, StateAccessType::Delete];

            let write_count = write_types.iter().filter(|t| access_types.contains(t)).count();

            if write_count > 1 {
                conflicts.push(StateConflict {
                    location,
                    first_access: StateAccessType::Write,
                    second_access: StateAccessType::Write,
                    description: "Potential write-write conflict".to_string(),
                });
            }
        }

        conflicts
    }
}

impl Analyzer for StateAccessAnalyzer {
    type Result = StateAccessAnalysis;

    fn analyze(&self, input: &str) -> AnalysisResult<Self::Result> {
        if input.trim().is_empty() {
            return Err(AnalysisError::EmptyInput);
        }

        let mut accesses = Vec::new();
        let mut locations = HashSet::new();
        let mut access_patterns: HashMap<StateAccessType, Vec<String>> = HashMap::new();

        for (line_number, line) in input.lines().enumerate() {
            if let Some(access_type) = self.detect_access_type(line) {
                if let Some(location) = self.extract_state_location(line, &access_type) {
                    let access = StateAccess {
                        location: location.clone(),
                        access_type: access_type.clone(),
                        line_number: Some(line_number + 1),
                        context: HashMap::new(),
                    };

                    accesses.push(access);
                    locations.insert(location.clone());

                    access_patterns.entry(access_type).or_insert_with(Vec::new).push(location);
                }
            }
        }

        let conflicts = if self.detect_conflicts { self.detect_conflicts(&accesses) } else { Vec::new() };

        Ok(StateAccessAnalysis {
            accesses,
            locations,
            access_patterns,
            conflicts,
        })
    }

    fn name(&self) -> &'static str {
        "StateAccessAnalyzer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_access_analyzer_creation() {
        let analyzer = StateAccessAnalyzer::new();
        assert!(analyzer.detect_conflicts);
        assert!(!analyzer.access_patterns.is_empty());
    }

    #[test]
    fn test_analyze_empty_input() {
        let analyzer = StateAccessAnalyzer::new();
        let result = analyzer.analyze("");
        assert!(matches!(result, Err(AnalysisError::EmptyInput)));
    }

    #[test]
    fn test_analyze_simple_state_access() {
        let analyzer = StateAccessAnalyzer::new();
        let input = r#"
            let value = get_state("counter");
            set_state("counter", value + 1);
        "#;

        let result = analyzer.analyze(input).unwrap();
        assert_eq!(result.accesses.len(), 2);
        assert!(result.locations.contains("counter"));
        assert!(!result.conflicts.is_empty()); // Should detect read-write conflict
    }

    #[test]
    fn test_detect_access_type() {
        let analyzer = StateAccessAnalyzer::new();

        assert_eq!(analyzer.detect_access_type("let x = get_state(key);"), Some(StateAccessType::Read));

        assert_eq!(analyzer.detect_access_type("set_state(key, value);"), Some(StateAccessType::Write));

        assert_eq!(analyzer.detect_access_type("update_state(key, new_value);"), Some(StateAccessType::Modify));

        assert_eq!(analyzer.detect_access_type("let x = some_function();"), None);
    }

    #[test]
    fn test_extract_state_location() {
        let analyzer = StateAccessAnalyzer::new();

        assert_eq!(analyzer.extract_state_location(r#"get_state("counter")"#, &StateAccessType::Read), Some("counter".to_string()));

        assert_eq!(analyzer.extract_state_location("get_state('balance')", &StateAccessType::Read), Some("balance".to_string()));
    }

    #[test]
    fn test_conflict_detection() {
        let analyzer = StateAccessAnalyzer::new();
        let accesses = vec![
            StateAccess {
                location: "counter".to_string(),
                access_type: StateAccessType::Read,
                line_number: Some(1),
                context: HashMap::new(),
            },
            StateAccess {
                location: "counter".to_string(),
                access_type: StateAccessType::Write,
                line_number: Some(2),
                context: HashMap::new(),
            },
        ];

        let conflicts = analyzer.detect_conflicts(&accesses);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].location, "counter");
    }

    #[test]
    fn test_custom_patterns() {
        let mut analyzer = StateAccessAnalyzer::new();
        analyzer.add_pattern(StateAccessType::Read, "custom_read".to_string());

        let input = "let value = custom_read(\"test\");";
        let result = analyzer.analyze(input).unwrap();

        assert_eq!(result.accesses.len(), 1);
        assert_eq!(result.accesses[0].access_type, StateAccessType::Read);
    }
}

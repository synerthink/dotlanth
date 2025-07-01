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

//! Core dependency detection algorithms

use super::Detector;
use std::collections::HashMap;

/// Types of dependencies that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependencyType {
    /// Module dependency (import/require)
    Module,
    /// Function dependency
    Function,
    /// Variable dependency
    Variable,
    /// Type dependency
    Type,
    /// Resource dependency (file, network, etc.)
    Resource,
    /// State dependency
    State,
    /// External library dependency
    Library,
    /// Custom dependency type
    Custom(String),
}

/// Information about a detected dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Name or identifier of the dependency
    pub name: String,
    /// Type of dependency
    pub dependency_type: DependencyType,
    /// Source location where dependency was found
    pub source_location: Option<SourceLocation>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Source location information
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Length of the dependency reference
    pub length: usize,
}

/// Main dependency detector interface
pub trait DependencyDetector: Detector<Dependency = DependencyInfo> {
    /// Add a custom pattern for dependency detection
    fn add_pattern(&mut self, pattern: String, dependency_type: DependencyType);

    /// Remove a pattern
    fn remove_pattern(&mut self, pattern: &str);

    /// Get all registered patterns
    fn get_patterns(&self) -> &HashMap<String, DependencyType>;
}

/// Basic dependency detector implementation
#[derive(Debug)]
pub struct BasicDependencyDetector {
    /// Patterns to match for different dependency types
    patterns: HashMap<String, DependencyType>,
    /// Whether to include line/column information
    include_location: bool,
}

impl Default for BasicDependencyDetector {
    fn default() -> Self {
        let mut patterns = HashMap::new();

        // Common dependency patterns
        patterns.insert("dep:".to_string(), DependencyType::Module);
        patterns.insert("import ".to_string(), DependencyType::Module);
        patterns.insert("require(".to_string(), DependencyType::Module);
        patterns.insert("use ".to_string(), DependencyType::Module);
        patterns.insert("include ".to_string(), DependencyType::Module);
        patterns.insert("from ".to_string(), DependencyType::Module);

        // Function patterns
        patterns.insert("call ".to_string(), DependencyType::Function);
        patterns.insert("invoke ".to_string(), DependencyType::Function);

        // State patterns
        patterns.insert("state.".to_string(), DependencyType::State);
        patterns.insert("get_state".to_string(), DependencyType::State);
        patterns.insert("set_state".to_string(), DependencyType::State);

        // Resource patterns
        patterns.insert("load(".to_string(), DependencyType::Resource);
        patterns.insert("read(".to_string(), DependencyType::Resource);
        patterns.insert("fetch(".to_string(), DependencyType::Resource);

        Self { patterns, include_location: true }
    }
}

impl BasicDependencyDetector {
    /// Create a new basic dependency detector
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable location tracking
    pub fn with_location_tracking(mut self, include: bool) -> Self {
        self.include_location = include;
        self
    }

    /// Extract dependency name from a line containing a pattern
    fn extract_dependency_name(&self, line: &str, pattern: &str) -> Option<String> {
        if let Some(start) = line.find(pattern) {
            let after_pattern = &line[start + pattern.len()..];

            // Try different extraction methods based on pattern type
            if pattern.ends_with(':') {
                // Pattern like "dep:module_name"
                let name = after_pattern.split_whitespace().next()?;
                return Some(name.to_string());
            }

            if pattern.ends_with('(') {
                // Pattern like "require(module_name)"
                if let Some(end) = after_pattern.find(')') {
                    let content = &after_pattern[..end];
                    // Remove quotes if present
                    let name = content.trim_matches('"').trim_matches('\'');
                    return Some(name.to_string());
                }
            }

            if pattern.ends_with(' ') {
                // Pattern like "import module_name" or "use module_name"
                let parts: Vec<&str> = after_pattern.split_whitespace().collect();
                if !parts.is_empty() {
                    // Handle "from module import item" pattern
                    if pattern.trim() == "from" && parts.len() >= 3 && parts[1] == "import" {
                        return Some(parts[0].to_string());
                    }
                    // Handle simple "import module" pattern
                    return Some(parts[0].to_string());
                }
            }

            if pattern.ends_with('.') {
                // Pattern like "state.property"
                let property = after_pattern.split_whitespace().next()?;
                return Some(format!("{}{}", pattern, property));
            }
        }

        None
    }

    /// Create source location information
    fn create_location(&self, line_num: usize, line: &str, pattern: &str) -> Option<SourceLocation> {
        if !self.include_location {
            return None;
        }

        if let Some(column) = line.find(pattern) {
            Some(SourceLocation {
                line: line_num,
                column: column + 1, // 1-based
                length: pattern.len(),
            })
        } else {
            None
        }
    }
}

impl Detector for BasicDependencyDetector {
    type Dependency = DependencyInfo;

    fn detect(&self, input: &str) -> Vec<Self::Dependency> {
        let mut dependencies = Vec::new();

        for (line_num, original_line) in input.lines().enumerate() {
            let line = original_line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Check each pattern
            for (pattern, dep_type) in &self.patterns {
                if line.contains(pattern) {
                    if let Some(name) = self.extract_dependency_name(line, pattern) {
                        let location = self.create_location(line_num + 1, original_line, pattern);

                        let mut metadata = HashMap::new();
                        metadata.insert("pattern".to_string(), pattern.clone());
                        metadata.insert("source_line".to_string(), original_line.to_string());

                        dependencies.push(DependencyInfo {
                            name,
                            dependency_type: dep_type.clone(),
                            source_location: location,
                            metadata,
                        });
                    }
                }
            }
        }

        dependencies
    }

    fn name(&self) -> &'static str {
        "BasicDependencyDetector"
    }
}

impl DependencyDetector for BasicDependencyDetector {
    fn add_pattern(&mut self, pattern: String, dependency_type: DependencyType) {
        self.patterns.insert(pattern, dependency_type);
    }

    fn remove_pattern(&mut self, pattern: &str) {
        self.patterns.remove(pattern);
    }

    fn get_patterns(&self) -> &HashMap<String, DependencyType> {
        &self.patterns
    }
}

/// Advanced dependency detector with regex support
#[derive(Debug)]
pub struct RegexDependencyDetector {
    /// Regex patterns mapped to dependency types
    regex_patterns: Vec<(regex::Regex, DependencyType)>,
    /// Whether to include location information
    include_location: bool,
}

impl RegexDependencyDetector {
    /// Create a new regex dependency detector
    pub fn new() -> Result<Self, regex::Error> {
        let mut regex_patterns = Vec::new();

        // Add common regex patterns
        regex_patterns.push((regex::Regex::new(r"import\s+([a-zA-Z_][a-zA-Z0-9_]*)")?, DependencyType::Module));

        regex_patterns.push((regex::Regex::new(r#"require\(["']([^"']+)["']\)"#)?, DependencyType::Module));

        regex_patterns.push((regex::Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")?, DependencyType::Function));

        Ok(Self {
            regex_patterns,
            include_location: true,
        })
    }

    /// Add a regex pattern
    pub fn add_regex_pattern(&mut self, pattern: regex::Regex, dependency_type: DependencyType) {
        self.regex_patterns.push((pattern, dependency_type));
    }
}

impl Detector for RegexDependencyDetector {
    type Dependency = DependencyInfo;

    fn detect(&self, input: &str) -> Vec<Self::Dependency> {
        let mut dependencies = Vec::new();

        for (line_num, line) in input.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            for (regex, dep_type) in &self.regex_patterns {
                for capture in regex.captures_iter(line) {
                    if let Some(matched) = capture.get(1) {
                        let name = matched.as_str().to_string();
                        let location = if self.include_location {
                            Some(SourceLocation {
                                line: line_num + 1,
                                column: matched.start() + 1,
                                length: matched.len(),
                            })
                        } else {
                            None
                        };

                        let mut metadata = HashMap::new();
                        metadata.insert("regex_pattern".to_string(), regex.as_str().to_string());
                        metadata.insert("source_line".to_string(), line.to_string());

                        dependencies.push(DependencyInfo {
                            name,
                            dependency_type: dep_type.clone(),
                            source_location: location,
                            metadata,
                        });
                    }
                }
            }
        }

        dependencies
    }

    fn name(&self) -> &'static str {
        "RegexDependencyDetector"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_detector_creation() {
        let detector = BasicDependencyDetector::new();
        assert!(!detector.patterns.is_empty());
        assert!(detector.include_location);
    }

    #[test]
    fn test_basic_detector_simple_dependency() {
        let detector = BasicDependencyDetector::new();
        let input = "dep:module1\ndep:module2";

        let deps = detector.detect(input);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "module1");
        assert_eq!(deps[1].name, "module2");
        assert!(matches!(deps[0].dependency_type, DependencyType::Module));
    }

    #[test]
    fn test_basic_detector_import_syntax() {
        let detector = BasicDependencyDetector::new();
        let input = r#"
            import math
            require("fs")
            use std::collections
        "#;

        let deps = detector.detect(input);
        assert_eq!(deps.len(), 3);

        let names: Vec<_> = deps.iter().map(|d| &d.name).collect();
        assert!(names.contains(&&"math".to_string()));
        assert!(names.contains(&&"fs".to_string()));
        assert!(names.contains(&&"std::collections".to_string()));
    }

    #[test]
    fn test_extract_dependency_name() {
        let detector = BasicDependencyDetector::new();

        assert_eq!(detector.extract_dependency_name("dep:module1", "dep:"), Some("module1".to_string()));

        assert_eq!(detector.extract_dependency_name("require(\"fs\")", "require("), Some("fs".to_string()));

        assert_eq!(detector.extract_dependency_name("import math", "import "), Some("math".to_string()));
    }

    #[test]
    fn test_source_location() {
        let detector = BasicDependencyDetector::new();
        let input = "  dep:module1  ";

        let deps = detector.detect(input);
        assert_eq!(deps.len(), 1);

        let location = deps[0].source_location.as_ref().unwrap();
        assert_eq!(location.line, 1);
        assert_eq!(location.column, 3); // "  dep:" starts at column 3
    }

    #[test]
    fn test_custom_patterns() {
        let mut detector = BasicDependencyDetector::new();
        detector.add_pattern("custom:".to_string(), DependencyType::Custom("test".to_string()));

        let input = "custom:my_dependency";
        let deps = detector.detect(input);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "my_dependency");
        assert!(matches!(deps[0].dependency_type, DependencyType::Custom(_)));
    }

    #[test]
    fn test_state_dependencies() {
        let detector = BasicDependencyDetector::new();
        let input = r#"
            let value = get_state("counter");
            set_state("balance", 100);
            state.property = value;
        "#;

        let deps = detector.detect(input);
        let state_deps: Vec<_> = deps.iter().filter(|d| matches!(d.dependency_type, DependencyType::State)).collect();

        assert!(!state_deps.is_empty());
    }

    #[test]
    fn test_metadata() {
        let detector = BasicDependencyDetector::new();
        let input = "dep:test_module";

        let deps = detector.detect(input);
        assert_eq!(deps.len(), 1);

        let metadata = &deps[0].metadata;
        assert!(metadata.contains_key("pattern"));
        assert!(metadata.contains_key("source_line"));
        assert_eq!(metadata["pattern"], "dep:");
    }

    #[test]
    fn test_regex_detector() {
        let detector = RegexDependencyDetector::new().unwrap();
        let input = r#"
            import math
            require("fs")
            console.log("test")
        "#;

        let deps = detector.detect(input);
        assert!(!deps.is_empty());

        // Should detect import, require, and function call
        let types: Vec<_> = deps.iter().map(|d| &d.dependency_type).collect();
        assert!(types.iter().any(|t| matches!(t, DependencyType::Module)));
        assert!(types.iter().any(|t| matches!(t, DependencyType::Function)));
    }
}

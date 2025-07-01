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

//! Dependency detection algorithms and pattern matching

pub mod dependency_detector;
pub mod pattern_matcher;

pub use dependency_detector::{DependencyDetector, DependencyInfo, DependencyType};
pub use pattern_matcher::{MatchResult, Pattern, PatternMatcher};

use std::collections::HashMap;

/// Common trait for all dependency detection algorithms
pub trait Detector {
    /// The type of dependencies this detector finds
    type Dependency;

    /// Detect dependencies in the given input
    fn detect(&self, input: &str) -> Vec<Self::Dependency>;

    /// Get the name of this detector
    fn name(&self) -> &'static str;

    /// Check if this detector can handle the given input
    fn can_detect(&self, input: &str) -> bool {
        !input.trim().is_empty()
    }
}

/// Registry for managing multiple detectors
pub struct DetectorRegistry {
    /// Registered detectors
    detectors: HashMap<String, Box<dyn Detector<Dependency = DependencyInfo>>>,
}

impl DetectorRegistry {
    /// Create a new detector registry
    pub fn new() -> Self {
        Self { detectors: HashMap::new() }
    }

    /// Register a new detector
    pub fn register<D>(&mut self, name: String, detector: D)
    where
        D: Detector<Dependency = DependencyInfo> + 'static,
    {
        self.detectors.insert(name, Box::new(detector));
    }

    /// Get a detector by name
    pub fn get(&self, name: &str) -> Option<&dyn Detector<Dependency = DependencyInfo>> {
        self.detectors.get(name).map(|d| d.as_ref())
    }

    /// Run all registered detectors on the input
    pub fn detect_all(&self, input: &str) -> HashMap<String, Vec<DependencyInfo>> {
        let mut results = HashMap::new();

        for (name, detector) in &self.detectors {
            if detector.can_detect(input) {
                let dependencies = detector.detect(input);
                results.insert(name.clone(), dependencies);
            }
        }

        results
    }

    /// List all registered detector names
    pub fn list_detectors(&self) -> Vec<&String> {
        self.detectors.keys().collect()
    }
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Register default detectors
        registry.register("basic".to_string(), dependency_detector::BasicDependencyDetector::new());

        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDetector;

    impl Detector for TestDetector {
        type Dependency = DependencyInfo;

        fn detect(&self, _input: &str) -> Vec<Self::Dependency> {
            vec![DependencyInfo {
                name: "test_dep".to_string(),
                dependency_type: DependencyType::Module,
                source_location: None,
                metadata: HashMap::new(),
            }]
        }

        fn name(&self) -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_detector_registry_creation() {
        let registry = DetectorRegistry::new();
        assert!(registry.detectors.is_empty());
    }

    #[test]
    fn test_detector_registry_register() {
        let mut registry = DetectorRegistry::new();
        registry.register("test".to_string(), TestDetector);

        assert_eq!(registry.detectors.len(), 1);
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_detector_registry_detect_all() {
        let mut registry = DetectorRegistry::new();
        registry.register("test".to_string(), TestDetector);

        let results = registry.detect_all("some input");
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("test"));
        assert_eq!(results["test"].len(), 1);
    }

    #[test]
    fn test_default_registry() {
        let registry = DetectorRegistry::default();
        assert!(!registry.detectors.is_empty());
        assert!(registry.get("basic").is_some());
    }
}

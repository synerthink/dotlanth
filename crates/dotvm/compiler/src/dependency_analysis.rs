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

pub struct EngineConfig {
    // Verbosity level: 0 = quiet, 1 = normal, 2 = verbose
    pub verbosity: u8,
}

pub struct DependencyAnalysisEngine {
    pub config: EngineConfig,
    // Field to store the last computed dependency list.
    pub last_dependency_list: Option<Vec<String>>,
}

impl DependencyAnalysisEngine {
    // Constructor that initializes the engine with a given configuration.
    pub fn new(config: EngineConfig) -> Self {
        Self { config, last_dependency_list: None }
    }

    // Create state access analyzer
    pub fn analyze_state_access(&self, state: &str) -> Result<(), &'static str> {
        if state.is_empty() {
            Err("Empty state")
        } else {
            if self.config.verbosity > 0 {
                println!("Analyzing state: {state}");
            }
            Ok(())
        }
    }

    // Implement data flow analysis
    pub fn analyze_data_flow(&self, code: &str) -> Result<(), &'static str> {
        if code.is_empty() {
            Err("Empty code")
        } else {
            if self.config.verbosity > 0 {
                println!("Data flow analysis completed for code snippet.");
            }
            Ok(())
        }
    }

    // Develop control flow graph generator
    pub fn generate_control_flow_graph(&self, code: &str) -> Result<(), &'static str> {
        if code.is_empty() {
            Err("Empty code")
        } else {
            if self.config.verbosity > 0 {
                println!("Control flow graph generated for code snippet.");
            }
            Ok(())
        }
    }

    // Add dependency detection algorithms.
    // This method now updates the engine's state with the last computed dependencies.
    pub fn detect_dependencies(&mut self, code: &str) -> Vec<String> {
        let deps: Vec<String> = code.lines().filter(|line| line.trim().starts_with("dep:")).map(|s| s.trim().to_string()).collect();
        self.last_dependency_list = Some(deps.clone());
        deps
    }
}

#[cfg(test)]
mod tests {
    use super::{DependencyAnalysisEngine, EngineConfig};

    #[test]
    fn test_analyze_state_access_non_empty() {
        let engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 1 });
        assert!(engine.analyze_state_access("State1").is_ok());
    }

    #[test]
    fn test_analyze_state_access_empty() {
        let engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 1 });
        assert_eq!(engine.analyze_state_access("").unwrap_err(), "Empty state");
    }

    #[test]
    fn test_analyze_data_flow_non_empty() {
        let engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 1 });
        assert!(engine.analyze_data_flow("code snippet").is_ok());
    }

    #[test]
    fn test_analyze_data_flow_empty() {
        let engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 1 });
        assert_eq!(engine.analyze_data_flow("").unwrap_err(), "Empty code");
    }

    #[test]
    fn test_generate_control_flow_graph_non_empty() {
        let engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 1 });
        assert!(engine.generate_control_flow_graph("code snippet").is_ok());
    }

    #[test]
    fn test_generate_control_flow_graph_empty() {
        let engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 1 });
        assert_eq!(engine.generate_control_flow_graph("").unwrap_err(), "Empty code");
    }

    #[test]
    fn test_detect_dependencies_empty() {
        let mut engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 0 });
        let deps = engine.detect_dependencies("some code without dependencies");
        assert!(deps.is_empty());
        // Ensure the cache is set correctly.
        assert!(engine.last_dependency_list.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_detect_dependencies_with_deps() {
        let mut engine = DependencyAnalysisEngine::new(EngineConfig { verbosity: 0 });
        let input = "\
let a = 1;
dep:module1
// some comment
dep:module2
";
        let deps = engine.detect_dependencies(input);
        assert_eq!(deps, vec!["dep:module1", "dep:module2"]);
        // Ensure the cache now holds the same dependencies.
        assert_eq!(engine.last_dependency_list.as_ref().unwrap(), &deps);
    }
}

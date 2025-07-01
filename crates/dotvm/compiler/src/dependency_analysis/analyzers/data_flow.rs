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

//! Data flow analysis for dependency tracking

use super::{AnalysisError, AnalysisResult, Analyzer};
use std::collections::{HashMap, HashSet};

/// Represents a variable in the data flow
#[derive(Debug, Clone)]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Variable scope (function, global, etc.)
    pub scope: String,
    /// Line where variable is defined
    pub definition_line: Option<usize>,
}

impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.scope == other.scope
    }
}

impl Eq for Variable {}

impl std::hash::Hash for Variable {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.scope.hash(state);
    }
}

/// Types of data flow operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowOperation {
    /// Variable definition/assignment
    Definition,
    /// Variable usage/read
    Usage,
    /// Variable modification
    Modification,
    /// Function call with variable as parameter
    FunctionCall,
    /// Return statement with variable
    Return,
}

/// Represents a data flow node
#[derive(Debug, Clone)]
pub struct DataFlowNode {
    /// The variable involved
    pub variable: Variable,
    /// Type of operation
    pub operation: DataFlowOperation,
    /// Line number where operation occurs
    pub line_number: usize,
    /// Dependencies (variables this operation depends on)
    pub dependencies: Vec<Variable>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Represents a data flow edge between two nodes
#[derive(Debug, Clone)]
pub struct DataFlowEdge {
    /// Source node (where data flows from)
    pub from: Variable,
    /// Target node (where data flows to)
    pub to: Variable,
    /// Type of data flow
    pub flow_type: DataFlowType,
    /// Line number where the flow occurs
    pub line_number: usize,
}

/// Types of data flow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowType {
    /// Direct assignment (a = b)
    DirectAssignment,
    /// Computation (a = b + c)
    Computation,
    /// Function parameter passing
    ParameterPassing,
    /// Function return value
    ReturnValue,
    /// Conditional flow (if/else)
    Conditional,
}

/// Result of data flow analysis
#[derive(Debug, Clone)]
pub struct DataFlowAnalysis {
    /// All variables found in the code
    pub variables: HashSet<Variable>,
    /// Data flow nodes
    pub nodes: Vec<DataFlowNode>,
    /// Data flow edges
    pub edges: Vec<DataFlowEdge>,
    /// Variable definitions
    pub definitions: HashMap<Variable, Vec<usize>>,
    /// Variable usages
    pub usages: HashMap<Variable, Vec<usize>>,
    /// Potential issues (undefined variables, unused variables)
    pub issues: Vec<DataFlowIssue>,
}

/// Represents a data flow issue
#[derive(Debug, Clone)]
pub struct DataFlowIssue {
    /// Type of issue
    pub issue_type: DataFlowIssueType,
    /// Variable involved
    pub variable: Variable,
    /// Line number where issue occurs
    pub line_number: usize,
    /// Description of the issue
    pub description: String,
}

/// Types of data flow issues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowIssueType {
    /// Variable used before definition
    UndefinedVariable,
    /// Variable defined but never used
    UnusedVariable,
    /// Variable may be uninitialized
    UninitializedVariable,
    /// Dead code (unreachable assignment)
    DeadCode,
}

/// Analyzer for data flow patterns
#[derive(Debug)]
pub struct DataFlowAnalyzer {
    /// Current scope being analyzed
    current_scope: String,
    /// Whether to detect unused variables
    pub detect_unused: bool,
    /// Whether to detect uninitialized variables
    pub detect_uninitialized: bool,
}

impl Default for DataFlowAnalyzer {
    fn default() -> Self {
        Self {
            current_scope: "global".to_string(),
            detect_unused: true,
            detect_uninitialized: true,
        }
    }
}

impl DataFlowAnalyzer {
    /// Create a new data flow analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable unused variable detection
    pub fn with_unused_detection(mut self, detect: bool) -> Self {
        self.detect_unused = detect;
        self
    }

    /// Enable or disable uninitialized variable detection
    pub fn with_uninitialized_detection(mut self, detect: bool) -> Self {
        self.detect_uninitialized = detect;
        self
    }

    /// Parse a line to extract variable operations
    fn parse_line(&self, line: &str, line_number: usize) -> Vec<DataFlowNode> {
        let mut nodes = Vec::new();
        let line = line.trim();

        if line.is_empty() || line.starts_with("//") {
            return nodes;
        }

        // Simple pattern matching for variable operations
        // In a real implementation, this would use a proper parser

        // Assignment patterns: let x = y, x = y
        if let Some(assignment) = self.parse_assignment(line, line_number) {
            nodes.push(assignment);
        }

        // Function call patterns: func(x, y)
        if let Some(call) = self.parse_function_call(line, line_number) {
            nodes.push(call);
        }

        // Return patterns: return x
        if let Some(return_node) = self.parse_return(line, line_number) {
            nodes.push(return_node);
        }

        nodes
    }

    /// Parse assignment operations
    fn parse_assignment(&self, line: &str, line_number: usize) -> Option<DataFlowNode> {
        // Look for patterns like "let x = y" or "x = y"
        if let Some(eq_pos) = line.find('=') {
            let left = line[..eq_pos].trim();
            let right = line[eq_pos + 1..].trim();

            // Extract variable name from left side
            let var_name = if left.starts_with("let ") { left[4..].trim() } else { left };

            if !var_name.is_empty() && is_valid_identifier(var_name) {
                let variable = Variable {
                    name: var_name.to_string(),
                    scope: self.current_scope.clone(),
                    definition_line: Some(line_number),
                };

                // Extract dependencies from right side
                let dependencies = self.extract_variables_from_expression(right);

                return Some(DataFlowNode {
                    variable,
                    operation: DataFlowOperation::Definition,
                    line_number,
                    dependencies,
                    metadata: HashMap::new(),
                });
            }
        }

        None
    }

    /// Parse function call operations
    fn parse_function_call(&self, line: &str, line_number: usize) -> Option<DataFlowNode> {
        // Look for patterns like "func(x, y)"
        if let Some(paren_pos) = line.find('(') {
            if let Some(close_paren) = line.rfind(')') {
                let func_name = line[..paren_pos].trim();
                let params = line[paren_pos + 1..close_paren].trim();

                if !func_name.is_empty() && is_valid_identifier(func_name) && !is_keyword(func_name) {
                    let variable = Variable {
                        name: func_name.to_string(),
                        scope: self.current_scope.clone(),
                        definition_line: None,
                    };

                    let dependencies = if !params.is_empty() { self.extract_variables_from_expression(params) } else { Vec::new() };

                    return Some(DataFlowNode {
                        variable,
                        operation: DataFlowOperation::FunctionCall,
                        line_number,
                        dependencies,
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        None
    }

    /// Parse return statements
    fn parse_return(&self, line: &str, line_number: usize) -> Option<DataFlowNode> {
        if line.starts_with("return ") {
            let return_expr = line[7..].trim();
            let dependencies = self.extract_variables_from_expression(return_expr);

            if !dependencies.is_empty() {
                // Use the first dependency as the main variable for the return
                let variable = dependencies[0].clone();

                // Don't include the main variable in dependencies to avoid duplication
                let filtered_dependencies: Vec<Variable> = dependencies.into_iter().filter(|dep| dep != &variable).collect();

                return Some(DataFlowNode {
                    variable,
                    operation: DataFlowOperation::Return,
                    line_number,
                    dependencies: filtered_dependencies,
                    metadata: HashMap::new(),
                });
            }
        }

        None
    }

    /// Extract variables from an expression
    fn extract_variables_from_expression(&self, expr: &str) -> Vec<Variable> {
        let mut variables = Vec::new();

        // Simple tokenization - split by common operators and delimiters
        let tokens: Vec<&str> = expr.split(&[' ', '+', '-', '*', '/', '(', ')', ',', ';'][..]).map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        for token in tokens {
            if is_valid_identifier(token) && !is_keyword(token) && !is_literal(token) {
                variables.push(Variable {
                    name: token.to_string(),
                    scope: self.current_scope.clone(),
                    definition_line: None,
                });
            }
        }

        variables
    }

    /// Detect data flow issues
    fn detect_issues(&self, analysis: &DataFlowAnalysis) -> Vec<DataFlowIssue> {
        let mut issues = Vec::new();

        if self.detect_unused {
            issues.extend(self.detect_unused_variables(analysis));
        }

        if self.detect_uninitialized {
            issues.extend(self.detect_uninitialized_variables(analysis));
        }

        issues
    }

    /// Detect unused variables
    fn detect_unused_variables(&self, analysis: &DataFlowAnalysis) -> Vec<DataFlowIssue> {
        let mut issues = Vec::new();

        for variable in &analysis.variables {
            let has_usages = analysis.usages.get(variable).map_or(false, |usages| !usages.is_empty());
            let has_definitions = analysis.definitions.get(variable).map_or(false, |defs| !defs.is_empty());

            if has_definitions && !has_usages {
                if let Some(def_lines) = analysis.definitions.get(variable) {
                    for &line in def_lines {
                        issues.push(DataFlowIssue {
                            issue_type: DataFlowIssueType::UnusedVariable,
                            variable: variable.clone(),
                            line_number: line,
                            description: format!("Variable '{}' is defined but never used", variable.name),
                        });
                    }
                }
            }
        }

        issues
    }

    /// Detect uninitialized variables
    fn detect_uninitialized_variables(&self, analysis: &DataFlowAnalysis) -> Vec<DataFlowIssue> {
        let mut issues = Vec::new();

        for variable in &analysis.variables {
            let has_definitions = analysis.definitions.get(variable).map_or(false, |defs| !defs.is_empty());
            let has_usages = analysis.usages.get(variable).map_or(false, |usages| !usages.is_empty());

            if has_usages && !has_definitions {
                if let Some(usage_lines) = analysis.usages.get(variable) {
                    for &line in usage_lines {
                        issues.push(DataFlowIssue {
                            issue_type: DataFlowIssueType::UndefinedVariable,
                            variable: variable.clone(),
                            line_number: line,
                            description: format!("Variable '{}' is used but never defined", variable.name),
                        });
                    }
                }
            }
        }

        issues
    }
}

impl Analyzer for DataFlowAnalyzer {
    type Result = DataFlowAnalysis;

    fn analyze(&self, input: &str) -> AnalysisResult<Self::Result> {
        if input.trim().is_empty() {
            return Err(AnalysisError::EmptyInput);
        }

        let mut variables = HashSet::new();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut definitions: HashMap<Variable, Vec<usize>> = HashMap::new();
        let mut usages: HashMap<Variable, Vec<usize>> = HashMap::new();

        // Parse each line
        for (line_number, line) in input.lines().enumerate() {
            let line_nodes = self.parse_line(line, line_number + 1);

            for node in line_nodes {
                variables.insert(node.variable.clone());

                // Track definitions and usages
                match node.operation {
                    DataFlowOperation::Definition => {
                        definitions.entry(node.variable.clone()).or_insert_with(Vec::new).push(node.line_number);
                    }
                    DataFlowOperation::Usage | DataFlowOperation::FunctionCall | DataFlowOperation::Return => {
                        usages.entry(node.variable.clone()).or_insert_with(Vec::new).push(node.line_number);
                    }
                    _ => {}
                }

                // Add dependencies to variables and usages
                for dep in &node.dependencies {
                    variables.insert(dep.clone());
                    usages.entry(dep.clone()).or_insert_with(Vec::new).push(node.line_number);

                    // Create data flow edge
                    edges.push(DataFlowEdge {
                        from: dep.clone(),
                        to: node.variable.clone(),
                        flow_type: DataFlowType::DirectAssignment, // Simplified
                        line_number: node.line_number,
                    });
                }

                nodes.push(node);
            }
        }

        let mut analysis = DataFlowAnalysis {
            variables,
            nodes,
            edges,
            definitions,
            usages,
            issues: Vec::new(),
        };

        analysis.issues = self.detect_issues(&analysis);

        Ok(analysis)
    }

    fn name(&self) -> &'static str {
        "DataFlowAnalyzer"
    }
}

/// Check if a string is a valid identifier
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().next().unwrap().is_alphabetic() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Check if a string is a keyword
fn is_keyword(s: &str) -> bool {
    matches!(s, "let" | "const" | "var" | "if" | "else" | "for" | "while" | "return" | "function")
}

/// Check if a string is a literal value
fn is_literal(s: &str) -> bool {
    s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() || s.starts_with('"') || s.starts_with('\'') || s == "true" || s == "false"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_flow_analyzer_creation() {
        let analyzer = DataFlowAnalyzer::new();
        assert!(analyzer.detect_unused);
        assert!(analyzer.detect_uninitialized);
    }

    #[test]
    fn test_analyze_empty_input() {
        let analyzer = DataFlowAnalyzer::new();
        let result = analyzer.analyze("");
        assert!(matches!(result, Err(AnalysisError::EmptyInput)));
    }

    #[test]
    fn test_analyze_simple_assignment() {
        let analyzer = DataFlowAnalyzer::new();
        let input = "let x = y + z;";

        let result = analyzer.analyze(input).unwrap();
        assert_eq!(result.variables.len(), 3); // x, y, z
        assert_eq!(result.nodes.len(), 1);
        assert!(!result.edges.is_empty());
    }

    #[test]
    fn test_parse_assignment() {
        let analyzer = DataFlowAnalyzer::new();

        let node = analyzer.parse_assignment("let x = y + 1", 1).unwrap();
        assert_eq!(node.variable.name, "x");
        assert_eq!(node.operation, DataFlowOperation::Definition);
        assert_eq!(node.line_number, 1);
        assert!(!node.dependencies.is_empty());
    }

    #[test]
    fn test_parse_function_call() {
        let analyzer = DataFlowAnalyzer::new();

        let node = analyzer.parse_function_call("func(x, y)", 1).unwrap();
        assert_eq!(node.variable.name, "func");
        assert_eq!(node.operation, DataFlowOperation::FunctionCall);
        assert_eq!(node.dependencies.len(), 2);
    }

    #[test]
    fn test_extract_variables() {
        let analyzer = DataFlowAnalyzer::new();
        let vars = analyzer.extract_variables_from_expression("x + y * z");

        assert_eq!(vars.len(), 3);
        let var_names: Vec<_> = vars.iter().map(|v| &v.name).collect();
        assert!(var_names.contains(&&"x".to_string()));
        assert!(var_names.contains(&&"y".to_string()));
        assert!(var_names.contains(&&"z".to_string()));
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("variable"));
        assert!(is_valid_identifier("var_name"));
        assert!(is_valid_identifier("var123"));
        assert!(!is_valid_identifier("123var"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("var-name"));
    }

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("let"));
        assert!(is_keyword("if"));
        assert!(is_keyword("return"));
        assert!(!is_keyword("variable"));
    }

    #[test]
    fn test_is_literal() {
        assert!(is_literal("123"));
        assert!(is_literal("3.14"));
        assert!(is_literal("\"string\""));
        assert!(is_literal("true"));
        assert!(!is_literal("variable"));
    }

    #[test]
    fn test_unused_variable_detection() {
        let analyzer = DataFlowAnalyzer::new();
        let input = r#"
            let x = 5;
            let y = 10;
            return y;
        "#;

        let result = analyzer.analyze(input).unwrap();
        let unused_issues: Vec<_> = result.issues.iter().filter(|i| i.issue_type == DataFlowIssueType::UnusedVariable).collect();

        assert_eq!(unused_issues.len(), 1);
        assert_eq!(unused_issues[0].variable.name, "x");
    }

    #[test]
    fn test_undefined_variable_detection() {
        let analyzer = DataFlowAnalyzer::new();
        let input = "let x = undefined_var + 1;";

        let result = analyzer.analyze(input).unwrap();
        let undefined_issues: Vec<_> = result.issues.iter().filter(|i| i.issue_type == DataFlowIssueType::UndefinedVariable).collect();

        assert_eq!(undefined_issues.len(), 1);
        assert_eq!(undefined_issues[0].variable.name, "undefined_var");
    }
}

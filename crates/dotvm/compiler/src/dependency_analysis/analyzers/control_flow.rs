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

//! Control flow analysis for dependency tracking

use super::{AnalysisError, AnalysisResult, Analyzer};
use std::collections::{HashMap, HashSet, VecDeque};

/// Types of control flow nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlFlowNodeType {
    /// Entry point of the program/function
    Entry,
    /// Exit point of the program/function
    Exit,
    /// Basic block (sequence of statements)
    BasicBlock,
    /// Conditional branch (if/else)
    Conditional,
    /// Loop header
    LoopHeader,
    /// Loop body
    LoopBody,
    /// Function call
    FunctionCall,
    /// Return statement
    Return,
    /// Jump/goto statement
    Jump,
}

/// Represents a node in the control flow graph
#[derive(Debug, Clone)]
pub struct ControlFlowNode {
    /// Unique identifier for this node
    pub id: usize,
    /// Type of control flow node
    pub node_type: ControlFlowNodeType,
    /// Line numbers covered by this node
    pub line_range: (usize, usize),
    /// Code content of this node
    pub content: String,
    /// Predecessors (nodes that can reach this node)
    pub predecessors: HashSet<usize>,
    /// Successors (nodes this node can reach)
    pub successors: HashSet<usize>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Represents an edge in the control flow graph
#[derive(Debug, Clone)]
pub struct ControlFlowEdge {
    /// Source node ID
    pub from: usize,
    /// Target node ID
    pub to: usize,
    /// Type of control flow edge
    pub edge_type: ControlFlowEdgeType,
    /// Condition for this edge (if applicable)
    pub condition: Option<String>,
}

/// Types of control flow edges
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlFlowEdgeType {
    /// Sequential execution
    Sequential,
    /// Conditional true branch
    ConditionalTrue,
    /// Conditional false branch
    ConditionalFalse,
    /// Loop back edge
    LoopBack,
    /// Function call edge
    FunctionCall,
    /// Return edge
    Return,
    /// Exception/error edge
    Exception,
}

/// Result of control flow analysis
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// All nodes in the graph
    pub nodes: HashMap<usize, ControlFlowNode>,
    /// All edges in the graph
    pub edges: Vec<ControlFlowEdge>,
    /// Entry node ID
    pub entry_node: usize,
    /// Exit node IDs
    pub exit_nodes: HashSet<usize>,
    /// Detected loops
    pub loops: Vec<ControlFlowLoop>,
    /// Unreachable code blocks
    pub unreachable_blocks: Vec<usize>,
    /// Complexity metrics
    pub complexity: ControlFlowComplexity,
}

/// Represents a loop in the control flow
#[derive(Debug, Clone)]
pub struct ControlFlowLoop {
    /// Header node of the loop
    pub header: usize,
    /// Body nodes of the loop
    pub body: HashSet<usize>,
    /// Back edges that form the loop
    pub back_edges: Vec<(usize, usize)>,
    /// Loop type
    pub loop_type: LoopType,
}

/// Types of loops
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopType {
    /// While loop
    While,
    /// For loop
    For,
    /// Do-while loop
    DoWhile,
    /// Infinite loop
    Infinite,
}

/// Control flow complexity metrics
#[derive(Debug, Clone, Default)]
pub struct ControlFlowComplexity {
    /// Cyclomatic complexity
    pub cyclomatic: usize,
    /// Number of nodes
    pub node_count: usize,
    /// Number of edges
    pub edge_count: usize,
    /// Number of decision points
    pub decision_points: usize,
    /// Maximum nesting depth
    pub max_nesting_depth: usize,
}

/// Analyzer for control flow patterns
#[derive(Debug)]
pub struct ControlFlowAnalyzer {
    /// Next node ID to assign
    next_node_id: usize,
    /// Whether to detect unreachable code
    pub detect_unreachable: bool,
    /// Whether to analyze loop structures
    pub analyze_loops: bool,
    /// Whether to calculate complexity metrics
    pub calculate_complexity: bool,
}

impl Default for ControlFlowAnalyzer {
    fn default() -> Self {
        Self {
            next_node_id: 0,
            detect_unreachable: true,
            analyze_loops: true,
            calculate_complexity: true,
        }
    }
}

impl ControlFlowAnalyzer {
    /// Create a new control flow analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable unreachable code detection
    pub fn with_unreachable_detection(mut self, detect: bool) -> Self {
        self.detect_unreachable = detect;
        self
    }

    /// Enable or disable loop analysis
    pub fn with_loop_analysis(mut self, analyze: bool) -> Self {
        self.analyze_loops = analyze;
        self
    }

    /// Enable or disable complexity calculation
    pub fn with_complexity_calculation(mut self, calculate: bool) -> Self {
        self.calculate_complexity = calculate;
        self
    }

    /// Get next node ID
    fn next_id(&mut self) -> usize {
        let id = self.next_node_id;
        self.next_node_id += 1;
        id
    }

    /// Parse the input and create basic blocks
    fn create_basic_blocks(&mut self, input: &str) -> Vec<ControlFlowNode> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = input.lines().collect();

        if lines.is_empty() {
            return blocks;
        }

        let mut current_block_start = 0;
        let mut current_block_content = String::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            current_block_content.push_str(line);
            current_block_content.push('\n');

            // Check if this line ends a basic block
            if self.is_block_terminator(trimmed) {
                // Create a block for the current content
                let node_type = self.determine_node_type(trimmed);
                let node = ControlFlowNode {
                    id: self.next_id(),
                    node_type,
                    line_range: (current_block_start + 1, i + 1),
                    content: current_block_content.trim().to_string(),
                    predecessors: HashSet::new(),
                    successors: HashSet::new(),
                    metadata: HashMap::new(),
                };
                blocks.push(node);

                // Start a new block
                current_block_start = i + 1;
                current_block_content.clear();
            }
        }

        // Handle remaining content
        if !current_block_content.trim().is_empty() {
            let node = ControlFlowNode {
                id: self.next_id(),
                node_type: ControlFlowNodeType::BasicBlock,
                line_range: (current_block_start + 1, lines.len()),
                content: current_block_content.trim().to_string(),
                predecessors: HashSet::new(),
                successors: HashSet::new(),
                metadata: HashMap::new(),
            };
            blocks.push(node);
        }

        blocks
    }

    /// Check if a line terminates a basic block
    fn is_block_terminator(&self, line: &str) -> bool {
        line.contains("if ")
            || line.contains("else")
            || line.contains("while ")
            || line.contains("for ")
            || line.contains("return")
            || line.contains("break")
            || line.contains("continue")
            || line.ends_with('{')
            || line.ends_with('}')
    }

    /// Determine the type of node based on the line content
    fn determine_node_type(&self, line: &str) -> ControlFlowNodeType {
        if line.contains("if ") {
            ControlFlowNodeType::Conditional
        } else if line.contains("while ") || line.contains("for ") {
            ControlFlowNodeType::LoopHeader
        } else if line.contains("return") {
            ControlFlowNodeType::Return
        } else if line.contains("(") && line.contains(")") && !line.contains("if") && !line.contains("while") {
            ControlFlowNodeType::FunctionCall
        } else {
            ControlFlowNodeType::BasicBlock
        }
    }

    /// Create edges between nodes
    fn create_edges(&self, nodes: &[ControlFlowNode]) -> Vec<ControlFlowEdge> {
        let mut edges = Vec::new();

        for (i, node) in nodes.iter().enumerate() {
            let node_edges = self.create_edges_for_node(node, nodes, i);
            edges.extend(node_edges);
        }

        edges
    }

    /// Create edges for a specific node based on its type
    fn create_edges_for_node(&self, node: &ControlFlowNode, nodes: &[ControlFlowNode], index: usize) -> Vec<ControlFlowEdge> {
        match node.node_type {
            ControlFlowNodeType::BasicBlock | ControlFlowNodeType::FunctionCall => {
                self.create_sequential_edge(node, nodes, index)
            }
            ControlFlowNodeType::Conditional => {
                self.create_conditional_edges(node, nodes, index)
            }
            ControlFlowNodeType::LoopHeader => {
                self.create_loop_edges(node, nodes, index)
            }
            ControlFlowNodeType::Return => {
                // No outgoing edges for return statements
                Vec::new()
            }
            _ => {
                self.create_sequential_edge(node, nodes, index)
            }
        }
    }

    /// Create a sequential edge to the next node
    fn create_sequential_edge(&self, node: &ControlFlowNode, nodes: &[ControlFlowNode], index: usize) -> Vec<ControlFlowEdge> {
        if index + 1 < nodes.len() {
            vec![ControlFlowEdge {
                from: node.id,
                to: nodes[index + 1].id,
                edge_type: ControlFlowEdgeType::Sequential,
                condition: None,
            }]
        } else {
            Vec::new()
        }
    }

    /// Create conditional edges (true and false branches)
    fn create_conditional_edges(&self, node: &ControlFlowNode, nodes: &[ControlFlowNode], index: usize) -> Vec<ControlFlowEdge> {
        let mut edges = Vec::new();

        // True branch
        if index + 1 < nodes.len() {
            edges.push(ControlFlowEdge {
                from: node.id,
                to: nodes[index + 1].id,
                edge_type: ControlFlowEdgeType::ConditionalTrue,
                condition: Some("true".to_string()),
            });
        }

        // False branch
        if let Some(else_node) = self.find_else_branch(nodes, index) {
            edges.push(ControlFlowEdge {
                from: node.id,
                to: else_node,
                edge_type: ControlFlowEdgeType::ConditionalFalse,
                condition: Some("false".to_string()),
            });
        }

        edges
    }

    /// Create loop edges (body and exit)
    fn create_loop_edges(&self, node: &ControlFlowNode, nodes: &[ControlFlowNode], index: usize) -> Vec<ControlFlowEdge> {
        let mut edges = Vec::new();

        // Loop body edge
        if index + 1 < nodes.len() {
            edges.push(ControlFlowEdge {
                from: node.id,
                to: nodes[index + 1].id,
                edge_type: ControlFlowEdgeType::ConditionalTrue,
                condition: Some("loop_condition".to_string()),
            });
        }

        // Exit edge (when loop condition is false)
        if let Some(exit_node) = self.find_loop_exit(nodes, index) {
            edges.push(ControlFlowEdge {
                from: node.id,
                to: exit_node,
                edge_type: ControlFlowEdgeType::ConditionalFalse,
                condition: Some("!loop_condition".to_string()),
            });
        }

        edges
    }

    /// Find the else branch for a conditional
    fn find_else_branch(&self, nodes: &[ControlFlowNode], if_index: usize) -> Option<usize> {
        // Simple heuristic: look for the next node that's not immediately following
        // In a real implementation, this would parse the actual structure
        if if_index + 2 < nodes.len() { Some(nodes[if_index + 2].id) } else { None }
    }

    /// Find the exit node for a loop
    fn find_loop_exit(&self, nodes: &[ControlFlowNode], loop_index: usize) -> Option<usize> {
        // Simple heuristic: find the next node after the loop body
        // In a real implementation, this would parse the actual structure
        for i in (loop_index + 1)..nodes.len() {
            if !matches!(nodes[i].node_type, ControlFlowNodeType::LoopBody) {
                return Some(nodes[i].id);
            }
        }
        None
    }

    /// Update predecessor and successor relationships
    fn update_relationships(&self, nodes: &mut HashMap<usize, ControlFlowNode>, edges: &[ControlFlowEdge]) {
        for edge in edges {
            if let Some(from_node) = nodes.get_mut(&edge.from) {
                from_node.successors.insert(edge.to);
            }
            if let Some(to_node) = nodes.get_mut(&edge.to) {
                to_node.predecessors.insert(edge.from);
            }
        }
    }

    /// Detect unreachable code blocks
    fn detect_unreachable(&self, graph: &ControlFlowGraph) -> Vec<usize> {
        if !self.detect_unreachable {
            return Vec::new();
        }

        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        // Start from entry node
        queue.push_back(graph.entry_node);
        reachable.insert(graph.entry_node);

        // BFS to find all reachable nodes
        while let Some(node_id) = queue.pop_front() {
            if let Some(node) = graph.nodes.get(&node_id) {
                for &successor in &node.successors {
                    if !reachable.contains(&successor) {
                        reachable.insert(successor);
                        queue.push_back(successor);
                    }
                }
            }
        }

        // Find unreachable nodes
        graph.nodes.keys().filter(|&&id| !reachable.contains(&id)).copied().collect()
    }

    /// Calculate complexity metrics
    fn calculate_complexity(&self, graph: &ControlFlowGraph) -> ControlFlowComplexity {
        if !self.calculate_complexity {
            return ControlFlowComplexity::default();
        }

        let node_count = graph.nodes.len();
        let edge_count = graph.edges.len();

        // Cyclomatic complexity: M = E - N + 2P (where P is number of connected components, usually 1)
        let cyclomatic = if edge_count >= node_count { edge_count - node_count + 2 } else { 1 };

        // Count decision points (conditionals and loops)
        let decision_points = graph
            .nodes
            .values()
            .filter(|node| matches!(node.node_type, ControlFlowNodeType::Conditional | ControlFlowNodeType::LoopHeader))
            .count();

        // Calculate maximum nesting depth (simplified)
        let max_nesting_depth = self.calculate_max_nesting_depth(graph);

        ControlFlowComplexity {
            cyclomatic,
            node_count,
            edge_count,
            decision_points,
            max_nesting_depth,
        }
    }

    /// Calculate maximum nesting depth
    fn calculate_max_nesting_depth(&self, graph: &ControlFlowGraph) -> usize {
        // Simplified calculation based on node types
        // In a real implementation, this would track actual nesting levels
        graph
            .nodes
            .values()
            .filter(|node| matches!(node.node_type, ControlFlowNodeType::Conditional | ControlFlowNodeType::LoopHeader))
            .count()
    }
}

impl Analyzer for ControlFlowAnalyzer {
    type Result = ControlFlowGraph;

    fn analyze(&self, input: &str) -> AnalysisResult<Self::Result> {
        if input.trim().is_empty() {
            return Err(AnalysisError::EmptyInput);
        }

        let mut analyzer = Self {
            next_node_id: 0,
            detect_unreachable: self.detect_unreachable,
            analyze_loops: self.analyze_loops,
            calculate_complexity: self.calculate_complexity,
        };

        // Create basic blocks
        let blocks = analyzer.create_basic_blocks(input);

        if blocks.is_empty() {
            return Err(AnalysisError::AnalysisFailed("No basic blocks found".to_string()));
        }

        // Create edges
        let edges = analyzer.create_edges(&blocks);

        // Convert blocks to hashmap
        let mut nodes: HashMap<usize, ControlFlowNode> = blocks.into_iter().map(|node| (node.id, node)).collect();

        // Update relationships
        analyzer.update_relationships(&mut nodes, &edges);

        // Determine entry and exit nodes
        let entry_node = nodes.values().find(|node| node.predecessors.is_empty()).map(|node| node.id).unwrap_or(0);

        let exit_nodes: HashSet<usize> = nodes
            .values()
            .filter(|node| node.successors.is_empty() || matches!(node.node_type, ControlFlowNodeType::Return))
            .map(|node| node.id)
            .collect();

        let graph = ControlFlowGraph {
            nodes,
            edges,
            entry_node,
            exit_nodes,
            loops: Vec::new(), // TODO: Implement loop detection
            unreachable_blocks: Vec::new(),
            complexity: ControlFlowComplexity::default(),
        };

        let unreachable_blocks = analyzer.detect_unreachable(&graph);
        let complexity = analyzer.calculate_complexity(&graph);

        Ok(ControlFlowGraph {
            unreachable_blocks,
            complexity,
            ..graph
        })
    }

    fn name(&self) -> &'static str {
        "ControlFlowAnalyzer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_flow_analyzer_creation() {
        let analyzer = ControlFlowAnalyzer::new();
        assert!(analyzer.detect_unreachable);
        assert!(analyzer.analyze_loops);
        assert!(analyzer.calculate_complexity);
    }

    #[test]
    fn test_analyze_empty_input() {
        let analyzer = ControlFlowAnalyzer::new();
        let result = analyzer.analyze("");
        assert!(matches!(result, Err(AnalysisError::EmptyInput)));
    }

    #[test]
    fn test_analyze_simple_sequence() {
        let analyzer = ControlFlowAnalyzer::new();
        let input = r#"
            let x = 1;
            let y = 2;
            return x + y;
        "#;

        let result = analyzer.analyze(input).unwrap();
        assert!(!result.nodes.is_empty());
        // For a simple sequence, we might have only one basic block, so edges might be empty
        assert_eq!(result.entry_node, 0);
    }

    #[test]
    fn test_analyze_conditional() {
        let analyzer = ControlFlowAnalyzer::new();
        let input = r#"
            if (x > 0) {
                return x;
            } else {
                return 0;
            }
        "#;

        let result = analyzer.analyze(input).unwrap();

        // Should have conditional nodes
        let conditional_nodes: Vec<_> = result.nodes.values().filter(|node| node.node_type == ControlFlowNodeType::Conditional).collect();
        assert!(!conditional_nodes.is_empty());

        // Should have conditional edges
        let conditional_edges: Vec<_> = result
            .edges
            .iter()
            .filter(|edge| matches!(edge.edge_type, ControlFlowEdgeType::ConditionalTrue | ControlFlowEdgeType::ConditionalFalse))
            .collect();
        assert!(!conditional_edges.is_empty());
    }

    #[test]
    fn test_is_block_terminator() {
        let analyzer = ControlFlowAnalyzer::new();

        assert!(analyzer.is_block_terminator("if (x > 0) {"));
        assert!(analyzer.is_block_terminator("return x;"));
        assert!(analyzer.is_block_terminator("while (true) {"));
        assert!(!analyzer.is_block_terminator("let x = 1;"));
    }

    #[test]
    fn test_determine_node_type() {
        let analyzer = ControlFlowAnalyzer::new();

        assert_eq!(analyzer.determine_node_type("if (x > 0)"), ControlFlowNodeType::Conditional);

        assert_eq!(analyzer.determine_node_type("while (true)"), ControlFlowNodeType::LoopHeader);

        assert_eq!(analyzer.determine_node_type("return x;"), ControlFlowNodeType::Return);

        assert_eq!(analyzer.determine_node_type("func(x, y);"), ControlFlowNodeType::FunctionCall);

        assert_eq!(analyzer.determine_node_type("let x = 1;"), ControlFlowNodeType::BasicBlock);
    }

    #[test]
    fn test_complexity_calculation() {
        let analyzer = ControlFlowAnalyzer::new();
        let input = r#"
            if (x > 0) {
                while (y < 10) {
                    y = y + 1;
                }
                return y;
            } else {
                return 0;
            }
        "#;

        let result = analyzer.analyze(input).unwrap();
        assert!(result.complexity.cyclomatic >= 1);
        assert!(result.complexity.decision_points > 0);
        assert!(result.complexity.node_count > 0);
        assert!(result.complexity.edge_count >= 0);
    }
}

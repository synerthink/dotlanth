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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use super::cost_model::{CostEstimate, CostModel, OperationCost};
use super::index_selector::{IndexSelector, QueryPredicate};

#[derive(Debug, Error)]
pub enum PlanGeneratorError {
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("No execution plan found: {0}")]
    NoExecutionPlan(String),
    #[error("Cost estimation failed: {0}")]
    CostEstimationFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub plan_id: String,
    pub root_node: PlanNode,
    pub estimated_cost: CostEstimate,
    pub estimated_rows: u64,
    pub parallelism_degree: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub node_id: String,
    pub operation: PlanOperation,
    pub children: Vec<PlanNode>,
    pub estimated_cost: CostEstimate,
    pub estimated_rows: u64,
    pub output_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanOperation {
    TableScan {
        table: String,
        predicates: Vec<QueryPredicate>,
        projection: Option<Vec<String>>,
    },
    IndexScan {
        table: String,
        index: String,
        predicates: Vec<QueryPredicate>,
        projection: Option<Vec<String>>,
    },
    Filter {
        predicates: Vec<QueryPredicate>,
    },
    Project {
        columns: Vec<String>,
    },
    Sort {
        columns: Vec<SortColumn>,
    },
    Join {
        join_type: JoinType,
        condition: JoinCondition,
        algorithm: JoinAlgorithm,
    },
    Aggregate {
        group_by: Vec<String>,
        aggregates: Vec<AggregateFunction>,
    },
    Limit {
        count: u64,
        offset: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    Cross,
    Semi,
    Anti,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinAlgorithm {
    NestedLoop,
    HashJoin,
    SortMerge,
    IndexNestedLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinCondition {
    pub left_columns: Vec<String>,
    pub right_columns: Vec<String>,
    pub operator: String, // "=", "<", ">", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortColumn {
    pub column: String,
    pub ascending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateFunction {
    pub function: String, // COUNT, SUM, AVG, etc.
    pub column: Option<String>,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan: QueryPlan,
    pub execution_strategy: ExecutionStrategy,
    pub resource_requirements: ResourceRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel { degree: usize },
    Vectorized,
    Streaming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub memory_mb: u64,
    pub cpu_cores: usize,
    pub io_operations: u64,
    pub temporary_storage_mb: u64,
}

pub struct QueryPlanner {
    cost_model: CostModel,
    index_selector: IndexSelector,
    table_metadata: HashMap<String, TableMetadata>,
}

#[derive(Debug, Clone)]
struct TableMetadata {
    row_count: u64,
    column_info: HashMap<String, ColumnInfo>,
    available_indexes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ColumnInfo {
    data_type: String,
    nullable: bool,
    cardinality: u64,
    selectivity: f64,
}

impl QueryPlanner {
    pub fn new() -> Self {
        Self {
            cost_model: CostModel::new(),
            index_selector: IndexSelector::new(),
            table_metadata: HashMap::new(),
        }
    }

    pub fn register_table(&mut self, table_name: String, metadata: TableMetadata) {
        self.table_metadata.insert(table_name, metadata);
    }

    pub fn generate_plan(&self, query: &ParsedQuery) -> Result<QueryPlan, PlanGeneratorError> {
        let mut plan_alternatives = Vec::new();

        // Generate different plan alternatives
        for scan_method in self.generate_scan_alternatives(&query.from_clause)? {
            let mut current_plan = scan_method;

            // Apply WHERE clauses
            if !query.where_predicates.is_empty() {
                current_plan = self.add_filter_node(current_plan, &query.where_predicates)?;
            }

            // Apply JOINs
            for join in &query.joins {
                current_plan = self.add_join_node(current_plan, join)?;
            }

            // Apply GROUP BY
            if !query.group_by.is_empty() || !query.aggregates.is_empty() {
                current_plan = self.add_aggregate_node(current_plan, &query.group_by, &query.aggregates)?;
            }

            // Apply ORDER BY
            if !query.order_by.is_empty() {
                current_plan = self.add_sort_node(current_plan, &query.order_by)?;
            }

            // Apply LIMIT
            if let Some(limit) = query.limit {
                current_plan = self.add_limit_node(current_plan, limit, query.offset)?;
            }

            // Apply SELECT (projection)
            current_plan = self.add_projection_node(current_plan, &query.select_columns)?;

            plan_alternatives.push(current_plan);
        }

        // Select the best plan based on cost
        let best_plan = plan_alternatives
            .into_iter()
            .min_by(|a, b| a.estimated_cost.total_cost.partial_cmp(&b.estimated_cost.total_cost).unwrap())
            .ok_or_else(|| PlanGeneratorError::NoExecutionPlan("No valid plans generated".to_string()))?;

        Ok(QueryPlan {
            plan_id: format!("plan_{}", uuid::Uuid::new_v4()),
            root_node: best_plan,
            estimated_cost: CostEstimate::zero(), // Will be calculated from root node
            estimated_rows: 0,
            parallelism_degree: 1,
        })
    }

    fn generate_scan_alternatives(&self, table: &str) -> Result<Vec<PlanNode>, PlanGeneratorError> {
        let mut alternatives = Vec::new();

        let metadata = self.table_metadata.get(table).ok_or_else(|| PlanGeneratorError::InvalidQuery(format!("Table {table} not found")))?;

        // Full table scan
        let table_scan = PlanNode {
            node_id: format!("scan_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::TableScan {
                table: table.to_string(),
                predicates: vec![],
                projection: None,
            },
            children: vec![],
            estimated_cost: self.cost_model.estimate_operation_cost(&OperationCost::TableScan {
                rows: metadata.row_count,
                selectivity: 1.0,
            }),
            estimated_rows: metadata.row_count,
            output_columns: metadata.column_info.keys().cloned().collect(),
        };
        alternatives.push(table_scan);

        // Index scans for each available index
        for index_name in &metadata.available_indexes {
            let index_scan = PlanNode {
                node_id: format!("index_scan_{}", uuid::Uuid::new_v4()),
                operation: PlanOperation::IndexScan {
                    table: table.to_string(),
                    index: index_name.clone(),
                    predicates: vec![],
                    projection: None,
                },
                children: vec![],
                estimated_cost: self.cost_model.estimate_operation_cost(&OperationCost::IndexScan {
                    rows: metadata.row_count / 10,         // Assume index is selective
                    index_pages: metadata.row_count / 100, // Rough estimate
                }),
                estimated_rows: metadata.row_count / 10,
                output_columns: metadata.column_info.keys().cloned().collect(),
            };
            alternatives.push(index_scan);
        }

        Ok(alternatives)
    }

    fn add_filter_node(&self, child: PlanNode, predicates: &[QueryPredicate]) -> Result<PlanNode, PlanGeneratorError> {
        let selectivity = predicates.iter().map(|p| p.selectivity.unwrap_or(0.1)).fold(1.0, |acc, sel| acc * sel);

        let estimated_rows = (child.estimated_rows as f64 * selectivity) as u64;
        let cpu_cost = child.estimated_rows as f64 * 0.001; // Small CPU cost per row

        Ok(PlanNode {
            node_id: format!("filter_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::Filter { predicates: predicates.to_vec() },
            children: vec![child],
            estimated_cost: CostEstimate::new(cpu_cost, 0.0, 0.0, 0.0),
            estimated_rows,
            output_columns: vec![], // Inherit from child
        })
    }

    fn add_join_node(&self, left: PlanNode, join: &JoinSpec) -> Result<PlanNode, PlanGeneratorError> {
        // For simplicity, assume we're joining with a table scan of the right table
        let right_metadata = self
            .table_metadata
            .get(&join.table)
            .ok_or_else(|| PlanGeneratorError::InvalidQuery(format!("Table {} not found", join.table)))?;

        let right_child = PlanNode {
            node_id: format!("scan_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::TableScan {
                table: join.table.clone(),
                predicates: vec![],
                projection: None,
            },
            children: vec![],
            estimated_cost: self.cost_model.estimate_operation_cost(&OperationCost::TableScan {
                rows: right_metadata.row_count,
                selectivity: 1.0,
            }),
            estimated_rows: right_metadata.row_count,
            output_columns: right_metadata.column_info.keys().cloned().collect(),
        };

        let join_cost = self.cost_model.estimate_operation_cost(&OperationCost::Join {
            left_rows: left.estimated_rows,
            right_rows: right_child.estimated_rows,
            join_type: format!("{:?}", join.join_type),
        });

        // Estimate output rows (simplified)
        let estimated_rows = match join.join_type {
            JoinType::Inner => (left.estimated_rows * right_child.estimated_rows) / 10, // Assume 10% selectivity
            JoinType::LeftOuter => left.estimated_rows,
            JoinType::Cross => left.estimated_rows * right_child.estimated_rows,
            _ => left.estimated_rows,
        };

        Ok(PlanNode {
            node_id: format!("join_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::Join {
                join_type: join.join_type.clone(),
                condition: join.condition.clone(),
                algorithm: JoinAlgorithm::HashJoin, // Default algorithm
            },
            children: vec![left, right_child],
            estimated_cost: join_cost,
            estimated_rows,
            output_columns: vec![], // Combination of left and right columns
        })
    }

    fn add_aggregate_node(&self, child: PlanNode, group_by: &[String], aggregates: &[AggregateFunction]) -> Result<PlanNode, PlanGeneratorError> {
        let estimated_groups = if group_by.is_empty() {
            1 // No GROUP BY means single aggregate result
        } else {
            child.estimated_rows / 10 // Assume 10% unique groups
        };

        let aggregate_cost = self.cost_model.estimate_operation_cost(&OperationCost::Aggregate {
            rows: child.estimated_rows,
            groups: estimated_groups,
        });

        Ok(PlanNode {
            node_id: format!("agg_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::Aggregate {
                group_by: group_by.to_vec(),
                aggregates: aggregates.to_vec(),
            },
            children: vec![child],
            estimated_cost: aggregate_cost,
            estimated_rows: estimated_groups,
            output_columns: group_by.iter().chain(aggregates.iter().map(|a| &a.alias)).cloned().collect(),
        })
    }

    fn add_sort_node(&self, child: PlanNode, sort_columns: &[SortColumn]) -> Result<PlanNode, PlanGeneratorError> {
        let sort_cost = self.cost_model.estimate_operation_cost(&OperationCost::Sort {
            rows: child.estimated_rows,
            columns: sort_columns.len(),
        });

        Ok(PlanNode {
            node_id: format!("sort_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::Sort { columns: sort_columns.to_vec() },
            estimated_cost: sort_cost,
            estimated_rows: child.estimated_rows,
            output_columns: child.output_columns.clone(),
            children: vec![child],
        })
    }

    fn add_limit_node(&self, child: PlanNode, count: u64, offset: Option<u64>) -> Result<PlanNode, PlanGeneratorError> {
        let effective_rows = std::cmp::min(child.estimated_rows, count + offset.unwrap_or(0));

        let output_columns = child.output_columns.clone();
        Ok(PlanNode {
            node_id: format!("limit_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::Limit { count, offset },
            estimated_cost: CostEstimate::new(0.001, 0.0, 0.0, 0.0), // Minimal cost
            estimated_rows: effective_rows,
            output_columns,
            children: vec![child],
        })
    }

    fn add_projection_node(&self, child: PlanNode, columns: &[String]) -> Result<PlanNode, PlanGeneratorError> {
        let cpu_cost = child.estimated_rows as f64 * 0.0001; // Very small cost per row
        let estimated_rows = child.estimated_rows;

        Ok(PlanNode {
            node_id: format!("proj_{}", uuid::Uuid::new_v4()),
            operation: PlanOperation::Project { columns: columns.to_vec() },
            children: vec![child],
            estimated_cost: CostEstimate::new(cpu_cost, 0.0, 0.0, 0.0),
            estimated_rows,
            output_columns: columns.to_vec(),
        })
    }

    pub fn create_execution_plan(&self, query_plan: QueryPlan) -> ExecutionPlan {
        let resource_requirements = self.estimate_resource_requirements(&query_plan.root_node);

        let execution_strategy = if query_plan.estimated_rows > 100000 {
            ExecutionStrategy::Parallel { degree: 4 }
        } else {
            ExecutionStrategy::Sequential
        };

        ExecutionPlan {
            plan: query_plan,
            execution_strategy,
            resource_requirements,
        }
    }

    fn estimate_resource_requirements(&self, node: &PlanNode) -> ResourceRequirements {
        let mut memory_mb = 0u64;
        let mut io_operations = 0u64;

        // Estimate based on operation type
        match &node.operation {
            PlanOperation::TableScan { .. } => {
                io_operations += node.estimated_rows / 100; // Assume 100 rows per page
            }
            PlanOperation::Sort { .. } => {
                memory_mb += (node.estimated_rows * 64) / (1024 * 1024); // 64 bytes per row
            }
            PlanOperation::Join { algorithm, .. } => {
                if let JoinAlgorithm::HashJoin = algorithm {
                    memory_mb += (node.estimated_rows * 32) / (1024 * 1024); // Hash table overhead
                }
            }
            _ => {}
        }

        // Add requirements from children
        for child in &node.children {
            let child_reqs = self.estimate_resource_requirements(child);
            memory_mb += child_reqs.memory_mb;
            io_operations += child_reqs.io_operations;
        }

        ResourceRequirements {
            memory_mb,
            cpu_cores: 1,
            io_operations,
            temporary_storage_mb: memory_mb / 2, // Estimate temp storage
        }
    }
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

// Simplified query representation for demonstration
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub select_columns: Vec<String>,
    pub from_clause: String,
    pub where_predicates: Vec<QueryPredicate>,
    pub joins: Vec<JoinSpec>,
    pub group_by: Vec<String>,
    pub aggregates: Vec<AggregateFunction>,
    pub order_by: Vec<SortColumn>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct JoinSpec {
    pub table: String,
    pub join_type: JoinType,
    pub condition: JoinCondition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_planner_creation() {
        let planner = QueryPlanner::new();
        assert_eq!(planner.table_metadata.len(), 0);
    }

    #[test]
    fn test_resource_estimation() {
        let planner = QueryPlanner::new();

        let node = PlanNode {
            node_id: "test".to_string(),
            operation: PlanOperation::TableScan {
                table: "users".to_string(),
                predicates: vec![],
                projection: None,
            },
            children: vec![],
            estimated_cost: CostEstimate::zero(),
            estimated_rows: 1000,
            output_columns: vec!["id".to_string(), "name".to_string()],
        };

        let requirements = planner.estimate_resource_requirements(&node);
        assert!(requirements.io_operations > 0);
    }
}

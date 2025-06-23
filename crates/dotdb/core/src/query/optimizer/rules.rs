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

use super::rule_engine::{OptimizationRule, RuleApplication};

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("Rule application failed: {0}")]
    ApplicationFailed(String),
    #[error("Invalid plan structure: {0}")]
    InvalidPlan(String),
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExpressionType {
    Constant(String),
    Column(String),
    Binary {
        operator: String,
        left: Box<ExpressionType>,
        right: Box<ExpressionType>,
    },
    Function {
        name: String,
        args: Vec<ExpressionType>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub plan_id: String,
    pub operations: Vec<PlanOperation>,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanOperation {
    TableScan { table: String, predicates: Vec<ExpressionType> },
    IndexScan { table: String, index: String, predicates: Vec<ExpressionType> },
    Filter { predicates: Vec<ExpressionType> },
    Project { columns: Vec<String> },
    Join { join_type: JoinType, condition: ExpressionType },
    Sort { columns: Vec<String> },
    Limit { count: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Rule for pushing predicates down the query tree
pub struct PredicatePushdownRule;

impl OptimizationRule for PredicatePushdownRule {
    fn name(&self) -> &str {
        "PredicatePushdown"
    }

    fn apply(&self, plan: &QueryPlan) -> Result<RuleApplication, RuleError> {
        let mut modified = false;
        let mut new_operations = Vec::new();

        for operation in &plan.operations {
            match operation {
                PlanOperation::Filter { predicates } => {
                    // Try to push predicates down to table scan
                    if let Some(PlanOperation::TableScan { table, predicates: existing_preds }) = new_operations.last_mut() {
                        // Merge predicates
                        let mut combined_predicates = existing_preds.clone();
                        combined_predicates.extend(predicates.clone());

                        *new_operations.last_mut().unwrap() = PlanOperation::TableScan {
                            table: table.clone(),
                            predicates: combined_predicates,
                        };

                        modified = true;
                        continue;
                    }
                }
                _ => {}
            }
            new_operations.push(operation.clone());
        }

        if modified {
            let optimized_plan = QueryPlan {
                plan_id: format!("{}_pushdown", plan.plan_id),
                operations: new_operations,
                estimated_cost: plan.estimated_cost * 0.8, // Assume 20% improvement
                estimated_rows: plan.estimated_rows,
            };

            Ok(RuleApplication {
                rule_name: self.name().to_string(),
                applied: true,
                original_cost: plan.estimated_cost,
                new_cost: optimized_plan.estimated_cost,
                optimized_plan: Some(optimized_plan),
                description: "Pushed predicates down to table scan".to_string(),
            })
        } else {
            Ok(RuleApplication {
                rule_name: self.name().to_string(),
                applied: false,
                original_cost: plan.estimated_cost,
                new_cost: plan.estimated_cost,
                optimized_plan: None,
                description: "No predicates to push down".to_string(),
            })
        }
    }

    fn applicable(&self, plan: &QueryPlan) -> bool {
        // Check if there are filter operations that can be pushed down
        plan.operations.iter().any(|op| matches!(op, PlanOperation::Filter { .. }))
    }
}

/// Rule for folding constant expressions
pub struct ConstantFoldingRule;

impl OptimizationRule for ConstantFoldingRule {
    fn name(&self) -> &str {
        "ConstantFolding"
    }

    fn apply(&self, plan: &QueryPlan) -> Result<RuleApplication, RuleError> {
        let mut modified = false;
        let mut new_operations = Vec::new();

        for operation in &plan.operations {
            let optimized_op = match operation {
                PlanOperation::Filter { predicates } => {
                    let folded_predicates = predicates.iter().map(|pred| self.fold_constants(pred)).collect::<Result<Vec<_>, _>>()?;

                    if folded_predicates != *predicates {
                        modified = true;
                    }

                    PlanOperation::Filter { predicates: folded_predicates }
                }
                _ => operation.clone(),
            };

            new_operations.push(optimized_op);
        }

        if modified {
            let optimized_plan = QueryPlan {
                plan_id: format!("{}_folded", plan.plan_id),
                operations: new_operations,
                estimated_cost: plan.estimated_cost * 0.95, // Small improvement
                estimated_rows: plan.estimated_rows,
            };

            Ok(RuleApplication {
                rule_name: self.name().to_string(),
                applied: true,
                original_cost: plan.estimated_cost,
                new_cost: optimized_plan.estimated_cost,
                optimized_plan: Some(optimized_plan),
                description: "Folded constant expressions".to_string(),
            })
        } else {
            Ok(RuleApplication {
                rule_name: self.name().to_string(),
                applied: false,
                original_cost: plan.estimated_cost,
                new_cost: plan.estimated_cost,
                optimized_plan: None,
                description: "No constants to fold".to_string(),
            })
        }
    }

    fn applicable(&self, plan: &QueryPlan) -> bool {
        // Check if there are constant expressions that can be folded
        plan.operations.iter().any(|op| match op {
            PlanOperation::Filter { predicates } => predicates.iter().any(|pred| self.has_foldable_constants(pred)),
            _ => false,
        })
    }
}

impl ConstantFoldingRule {
    fn fold_constants(&self, expr: &ExpressionType) -> Result<ExpressionType, RuleError> {
        match expr {
            ExpressionType::Binary { operator, left, right } => {
                let folded_left = self.fold_constants(left)?;
                let folded_right = self.fold_constants(right)?;

                // Try to evaluate if both operands are constants
                if let (ExpressionType::Constant(left_val), ExpressionType::Constant(right_val)) = (&folded_left, &folded_right) {
                    match operator.as_str() {
                        "+" => {
                            if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                                return Ok(ExpressionType::Constant((l + r).to_string()));
                            }
                        }
                        "-" => {
                            if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                                return Ok(ExpressionType::Constant((l - r).to_string()));
                            }
                        }
                        "*" => {
                            if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                                return Ok(ExpressionType::Constant((l * r).to_string()));
                            }
                        }
                        "/" => {
                            if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
                                if r != 0.0 {
                                    return Ok(ExpressionType::Constant((l / r).to_string()));
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Ok(ExpressionType::Binary {
                    operator: operator.clone(),
                    left: Box::new(folded_left),
                    right: Box::new(folded_right),
                })
            }
            ExpressionType::Function { name, args } => {
                let folded_args = args.iter().map(|arg| self.fold_constants(arg)).collect::<Result<Vec<_>, _>>()?;

                Ok(ExpressionType::Function {
                    name: name.clone(),
                    args: folded_args,
                })
            }
            _ => Ok(expr.clone()),
        }
    }

    fn has_foldable_constants(&self, expr: &ExpressionType) -> bool {
        match expr {
            ExpressionType::Binary { left, right, .. } => {
                matches!((left.as_ref(), right.as_ref()), (ExpressionType::Constant(_), ExpressionType::Constant(_)))
            }
            ExpressionType::Function { args, .. } => args.iter().any(|arg| self.has_foldable_constants(arg)),
            _ => false,
        }
    }
}

/// Rule for reordering joins to optimize execution
pub struct JoinReorderingRule {
    cardinality_estimates: HashMap<String, u64>,
}

impl JoinReorderingRule {
    pub fn new(cardinality_estimates: HashMap<String, u64>) -> Self {
        Self { cardinality_estimates }
    }
}

impl OptimizationRule for JoinReorderingRule {
    fn name(&self) -> &str {
        "JoinReordering"
    }

    fn apply(&self, plan: &QueryPlan) -> Result<RuleApplication, RuleError> {
        // For this example, we'll implement a simple heuristic:
        // Smaller tables should be on the left side of joins

        let mut modified = false;
        let mut new_operations = Vec::new();

        for operation in &plan.operations {
            match operation {
                PlanOperation::Join { join_type: _, condition } => {
                    // This is a simplified example - in practice, you'd need more
                    // sophisticated join reordering algorithms
                    if self.should_reorder_join(&condition) {
                        modified = true;
                        // Apply reordering logic here
                    }
                    new_operations.push(operation.clone());
                }
                _ => new_operations.push(operation.clone()),
            }
        }

        if modified {
            let optimized_plan = QueryPlan {
                plan_id: format!("{}_reordered", plan.plan_id),
                operations: new_operations,
                estimated_cost: plan.estimated_cost * 0.7, // Assume significant improvement
                estimated_rows: plan.estimated_rows,
            };

            Ok(RuleApplication {
                rule_name: self.name().to_string(),
                applied: true,
                original_cost: plan.estimated_cost,
                new_cost: optimized_plan.estimated_cost,
                optimized_plan: Some(optimized_plan),
                description: "Reordered joins for better performance".to_string(),
            })
        } else {
            Ok(RuleApplication {
                rule_name: self.name().to_string(),
                applied: false,
                original_cost: plan.estimated_cost,
                new_cost: plan.estimated_cost,
                optimized_plan: None,
                description: "No beneficial join reordering found".to_string(),
            })
        }
    }

    fn applicable(&self, plan: &QueryPlan) -> bool {
        plan.operations.iter().any(|op| matches!(op, PlanOperation::Join { .. }))
    }
}

impl JoinReorderingRule {
    fn should_reorder_join(&self, _condition: &ExpressionType) -> bool {
        // Simplified heuristic - in practice, this would analyze
        // the join condition and table cardinalities
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predicate_pushdown_rule() {
        let rule = PredicatePushdownRule;

        let plan = QueryPlan {
            plan_id: "test_plan".to_string(),
            operations: vec![
                PlanOperation::TableScan {
                    table: "users".to_string(),
                    predicates: vec![],
                },
                PlanOperation::Filter {
                    predicates: vec![ExpressionType::Column("age".to_string())],
                },
            ],
            estimated_cost: 100.0,
            estimated_rows: 1000,
        };

        assert!(rule.applicable(&plan));
        let result = rule.apply(&plan).unwrap();
        assert!(result.applied);
        assert!(result.new_cost < result.original_cost);
    }

    #[test]
    fn test_constant_folding_rule() {
        let rule = ConstantFoldingRule;

        let plan = QueryPlan {
            plan_id: "test_plan".to_string(),
            operations: vec![PlanOperation::Filter {
                predicates: vec![ExpressionType::Binary {
                    operator: "+".to_string(),
                    left: Box::new(ExpressionType::Constant("5".to_string())),
                    right: Box::new(ExpressionType::Constant("3".to_string())),
                }],
            }],
            estimated_cost: 100.0,
            estimated_rows: 1000,
        };

        assert!(rule.applicable(&plan));
        let result = rule.apply(&plan).unwrap();
        assert!(result.applied);
    }

    #[test]
    fn test_expression_folding() {
        let rule = ConstantFoldingRule;

        let expr = ExpressionType::Binary {
            operator: "+".to_string(),
            left: Box::new(ExpressionType::Constant("10".to_string())),
            right: Box::new(ExpressionType::Constant("20".to_string())),
        };

        let folded = rule.fold_constants(&expr).unwrap();
        assert!(matches!(folded, ExpressionType::Constant(val) if val == "30"));
    }

    #[test]
    fn test_join_reordering_rule() {
        let cardinalities = HashMap::new();
        let rule = JoinReorderingRule::new(cardinalities);

        let plan = QueryPlan {
            plan_id: "test_plan".to_string(),
            operations: vec![PlanOperation::Join {
                join_type: JoinType::Inner,
                condition: ExpressionType::Column("id".to_string()),
            }],
            estimated_cost: 100.0,
            estimated_rows: 1000,
        };

        assert!(rule.applicable(&plan));
    }
}

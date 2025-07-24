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
use thiserror::Error;

use super::rules::{QueryPlan, RuleError};

#[derive(Debug, Error)]
pub enum RuleEngineError {
    #[error("Rule application failed: {0}")]
    RuleApplicationFailed(String),
    #[error("Maximum iterations exceeded")]
    MaxIterationsExceeded,
    #[error("Invalid rule configuration: {0}")]
    InvalidConfiguration(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleApplication {
    pub rule_name: String,
    pub applied: bool,
    pub original_cost: f64,
    pub new_cost: f64,
    pub optimized_plan: Option<QueryPlan>,
    pub description: String,
}

pub trait OptimizationRule: Send + Sync {
    fn name(&self) -> &str;
    fn apply(&self, plan: &QueryPlan) -> Result<RuleApplication, RuleError>;
    fn applicable(&self, plan: &QueryPlan) -> bool;
}

pub struct RuleEngine {
    rules: Vec<Box<dyn OptimizationRule>>,
    max_iterations: usize,
    cost_improvement_threshold: f64,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            max_iterations: 10,
            cost_improvement_threshold: 0.01, // 1% improvement
        }
    }

    pub fn add_rule(&mut self, rule: Box<dyn OptimizationRule>) {
        self.rules.push(rule);
    }

    pub fn optimize(&self, initial_plan: QueryPlan) -> Result<(QueryPlan, Vec<RuleApplication>), RuleEngineError> {
        let mut current_plan = initial_plan;
        let mut applications = Vec::new();
        let mut iterations = 0;

        while iterations < self.max_iterations {
            let mut improved = false;
            iterations += 1;

            for rule in &self.rules {
                if rule.applicable(&current_plan) {
                    match rule.apply(&current_plan) {
                        Ok(application) => {
                            if application.applied {
                                let improvement = (application.original_cost - application.new_cost) / application.original_cost;

                                if improvement >= self.cost_improvement_threshold
                                    && let Some(optimized_plan) = application.optimized_plan.clone()
                                {
                                    current_plan = optimized_plan;
                                    improved = true;
                                }
                            }
                            applications.push(application);
                        }
                        Err(e) => {
                            return Err(RuleEngineError::RuleApplicationFailed(e.to_string()));
                        }
                    }
                }
            }

            if !improved {
                break;
            }
        }

        if iterations >= self.max_iterations {
            return Err(RuleEngineError::MaxIterationsExceeded);
        }

        Ok((current_plan, applications))
    }

    pub fn set_max_iterations(&mut self, max_iterations: usize) {
        self.max_iterations = max_iterations;
    }

    pub fn set_cost_improvement_threshold(&mut self, threshold: f64) {
        self.cost_improvement_threshold = threshold;
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::optimizer::rules::{ConstantFoldingRule, ExpressionType, PlanOperation, PredicatePushdownRule};

    #[test]
    fn test_rule_engine_creation() {
        let engine = RuleEngine::new();
        assert_eq!(engine.rules.len(), 0);
        assert_eq!(engine.max_iterations, 10);
    }

    #[test]
    fn test_rule_engine_with_rules() {
        let mut engine = RuleEngine::new();
        engine.add_rule(Box::new(PredicatePushdownRule));
        engine.add_rule(Box::new(ConstantFoldingRule));

        assert_eq!(engine.rules.len(), 2);
    }

    #[test]
    fn test_optimization_process() {
        let mut engine = RuleEngine::new();
        engine.add_rule(Box::new(PredicatePushdownRule));

        let plan = QueryPlan {
            plan_id: "test".to_string(),
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

        let result = engine.optimize(plan);
        assert!(result.is_ok());

        let (optimized_plan, applications) = result.unwrap();
        assert!(!applications.is_empty());
    }
}

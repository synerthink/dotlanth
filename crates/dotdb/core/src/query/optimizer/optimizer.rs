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

use super::rule_engine::{OptimizationRule, RuleApplication, RuleEngine};
use super::rules::{ConstantFoldingRule, JoinReorderingRule, PredicatePushdownRule, QueryPlan};
use crate::statistics::StatisticsCollector;

#[derive(Debug, Error)]
pub enum OptimizerError {
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
    #[error("Invalid query plan: {0}")]
    InvalidQueryPlan(String),
    #[error("Statistics unavailable: {0}")]
    StatisticsUnavailable(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationContext {
    pub table_statistics: HashMap<String, TableStats>,
    pub index_information: HashMap<String, IndexInfo>,
    pub system_settings: SystemSettings,
    pub optimization_goals: OptimizationGoals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub row_count: u64,
    pub page_count: u64,
    pub average_row_size: f64,
    pub cardinalities: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub index_name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub size_bytes: u64,
    pub selectivity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub memory_limit_mb: u64,
    pub cpu_cores: usize,
    pub io_cost_factor: f64,
    pub cpu_cost_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationGoals {
    pub minimize_cost: bool,
    pub minimize_memory: bool,
    pub maximize_parallelism: bool,
    pub prefer_indexes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub original_plan: QueryPlan,
    pub optimized_plan: QueryPlan,
    pub optimization_steps: Vec<RuleApplication>,
    pub cost_improvement: f64,
    pub optimization_time_ms: u64,
    pub confidence_score: f64,
}

pub struct QueryOptimizer {
    rule_engine: RuleEngine,
    statistics_collector: Option<StatisticsCollector>,
    optimization_context: OptimizationContext,
}

impl QueryOptimizer {
    pub fn new(context: OptimizationContext) -> Self {
        let mut rule_engine = RuleEngine::new();

        // Add default optimization rules
        rule_engine.add_rule(Box::new(PredicatePushdownRule));
        rule_engine.add_rule(Box::new(ConstantFoldingRule));

        // Add join reordering rule with cardinality estimates
        let cardinality_estimates: HashMap<String, u64> = context.table_statistics.iter().map(|(table, stats)| (table.clone(), stats.row_count)).collect();
        rule_engine.add_rule(Box::new(JoinReorderingRule::new(cardinality_estimates)));

        Self {
            rule_engine,
            statistics_collector: None,
            optimization_context: context,
        }
    }

    pub fn with_statistics(mut self, collector: StatisticsCollector) -> Self {
        self.statistics_collector = Some(collector);
        self
    }

    pub fn optimize(&self, plan: QueryPlan) -> Result<OptimizationResult, OptimizerError> {
        let start_time = std::time::Instant::now();

        // Validate the input plan
        self.validate_plan(&plan)?;

        // Update statistics if available
        if let Some(ref collector) = self.statistics_collector {
            self.update_plan_statistics(&plan, collector)?;
        }

        // Apply optimization rules
        let (optimized_plan, applications) = self.rule_engine.optimize(plan.clone()).map_err(|e| OptimizerError::OptimizationFailed(e.to_string()))?;

        let optimization_time = start_time.elapsed().as_millis() as u64;

        // Calculate improvement metrics
        let cost_improvement = if plan.estimated_cost > 0.0 {
            (plan.estimated_cost - optimized_plan.estimated_cost) / plan.estimated_cost
        } else {
            0.0
        };

        let confidence_score = self.calculate_confidence_score(&applications);

        Ok(OptimizationResult {
            original_plan: plan,
            optimized_plan,
            optimization_steps: applications,
            cost_improvement,
            optimization_time_ms: optimization_time,
            confidence_score,
        })
    }

    pub fn add_optimization_rule(&mut self, rule: Box<dyn OptimizationRule>) {
        self.rule_engine.add_rule(rule);
    }

    pub fn set_optimization_parameters(&mut self, max_iterations: usize, improvement_threshold: f64) {
        self.rule_engine.set_max_iterations(max_iterations);
        self.rule_engine.set_cost_improvement_threshold(improvement_threshold);
    }

    pub fn update_context(&mut self, context: OptimizationContext) {
        self.optimization_context = context;
    }

    fn validate_plan(&self, plan: &QueryPlan) -> Result<(), OptimizerError> {
        if plan.operations.is_empty() {
            return Err(OptimizerError::InvalidQueryPlan("Plan contains no operations".to_string()));
        }

        if plan.estimated_cost < 0.0 {
            return Err(OptimizerError::InvalidQueryPlan("Plan has negative estimated cost".to_string()));
        }

        // Additional validation logic can be added here
        Ok(())
    }

    fn update_plan_statistics(&self, _plan: &QueryPlan, _collector: &StatisticsCollector) -> Result<(), OptimizerError> {
        // In a real implementation, this would update plan statistics
        // based on current table statistics from the collector
        Ok(())
    }

    fn calculate_confidence_score(&self, applications: &[RuleApplication]) -> f64 {
        if applications.is_empty() {
            return 1.0; // No changes means high confidence in original plan
        }

        let successful_applications = applications.iter().filter(|app| app.applied).count();
        let total_applications = applications.len();

        if total_applications == 0 {
            1.0
        } else {
            successful_applications as f64 / total_applications as f64
        }
    }

    pub fn explain_optimization(&self, result: &OptimizationResult) -> String {
        let mut explanation = String::new();

        explanation.push_str(&format!(
            "Query Optimization Summary:\n\
             Original Cost: {:.2}\n\
             Optimized Cost: {:.2}\n\
             Cost Improvement: {:.1}%\n\
             Optimization Time: {}ms\n\
             Confidence Score: {:.1}%\n\n",
            result.original_plan.estimated_cost,
            result.optimized_plan.estimated_cost,
            result.cost_improvement * 100.0,
            result.optimization_time_ms,
            result.confidence_score * 100.0
        ));

        explanation.push_str("Applied Optimizations:\n");
        for (i, step) in result.optimization_steps.iter().enumerate() {
            if step.applied {
                explanation.push_str(&format!(
                    "{}. {} - {}\n   Cost: {:.2} -> {:.2} ({:.1}% improvement)\n",
                    i + 1,
                    step.rule_name,
                    step.description,
                    step.original_cost,
                    step.new_cost,
                    ((step.original_cost - step.new_cost) / step.original_cost) * 100.0
                ));
            }
        }

        explanation
    }

    pub fn benchmark_optimization(&self, plan: QueryPlan, iterations: usize) -> Result<BenchmarkResult, OptimizerError> {
        let mut total_time = 0u64;
        let mut total_improvement = 0.0;
        let mut successful_optimizations = 0;

        for _ in 0..iterations {
            match self.optimize(plan.clone()) {
                Ok(result) => {
                    total_time += result.optimization_time_ms;
                    total_improvement += result.cost_improvement;
                    successful_optimizations += 1;
                }
                Err(_) => {
                    // Skip failed optimizations
                }
            }
        }

        Ok(BenchmarkResult {
            iterations,
            successful_optimizations,
            average_time_ms: if successful_optimizations > 0 { total_time / successful_optimizations as u64 } else { 0 },
            average_improvement: if successful_optimizations > 0 { total_improvement / successful_optimizations as f64 } else { 0.0 },
            success_rate: successful_optimizations as f64 / iterations as f64,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub iterations: usize,
    pub successful_optimizations: usize,
    pub average_time_ms: u64,
    pub average_improvement: f64,
    pub success_rate: f64,
}

impl Default for OptimizationContext {
    fn default() -> Self {
        Self {
            table_statistics: HashMap::new(),
            index_information: HashMap::new(),
            system_settings: SystemSettings {
                memory_limit_mb: 1024,
                cpu_cores: 4,
                io_cost_factor: 1.0,
                cpu_cost_factor: 1.0,
            },
            optimization_goals: OptimizationGoals {
                minimize_cost: true,
                minimize_memory: false,
                maximize_parallelism: false,
                prefer_indexes: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::optimizer::rules::{ExpressionType, PlanOperation};

    #[test]
    fn test_optimizer_creation() {
        let context = OptimizationContext::default();
        let optimizer = QueryOptimizer::new(context);
        assert!(optimizer.statistics_collector.is_none());
    }

    #[test]
    fn test_plan_validation() {
        let context = OptimizationContext::default();
        let optimizer = QueryOptimizer::new(context);

        let empty_plan = QueryPlan {
            plan_id: "empty".to_string(),
            operations: vec![],
            estimated_cost: 100.0,
            estimated_rows: 1000,
        };

        let result = optimizer.validate_plan(&empty_plan);
        assert!(result.is_err());
    }

    #[test]
    fn test_optimization_process() {
        let context = OptimizationContext::default();
        let optimizer = QueryOptimizer::new(context);

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

        let result = optimizer.optimize(plan);
        assert!(result.is_ok());

        let optimization_result = result.unwrap();
        assert!(optimization_result.confidence_score >= 0.0);
        assert!(optimization_result.confidence_score <= 1.0);
    }

    #[test]
    fn test_confidence_calculation() {
        let context = OptimizationContext::default();
        let optimizer = QueryOptimizer::new(context);

        let applications = vec![
            RuleApplication {
                rule_name: "Test".to_string(),
                applied: true,
                original_cost: 100.0,
                new_cost: 80.0,
                optimized_plan: None,
                description: "Test rule".to_string(),
            },
            RuleApplication {
                rule_name: "Test2".to_string(),
                applied: false,
                original_cost: 80.0,
                new_cost: 80.0,
                optimized_plan: None,
                description: "Test rule 2".to_string(),
            },
        ];

        let confidence = optimizer.calculate_confidence_score(&applications);
        assert_eq!(confidence, 0.5); // 1 out of 2 applications succeeded
    }

    #[test]
    fn test_optimization_parameters() {
        let context = OptimizationContext::default();
        let mut optimizer = QueryOptimizer::new(context);

        optimizer.set_optimization_parameters(5, 0.05);
        // Parameters should be set successfully (no assertion needed as it's void)
    }
}

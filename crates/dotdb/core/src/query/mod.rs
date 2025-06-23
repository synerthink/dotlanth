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

//! # Query Processing Module
//!
//! This module provides comprehensive query processing capabilities including
//! query planning, optimization, and execution planning for DotDB.
//!
//! ## Modules
//!
//! - `planner`: Query planning and execution plan generation
//! - `optimizer`: Query optimization with rule-based optimizations
//!
//! ## Architecture
//!
//! The query processing pipeline follows this flow:
//! 1. **Planning**: Convert SQL queries into execution plans
//! 2. **Optimization**: Apply optimization rules to improve performance
//! 3. **Execution**: Execute the optimized plan

pub mod optimizer;
pub mod planner;

// Re-export commonly used types from optimizer
pub use optimizer::{
    optimizer::{OptimizationResult, QueryOptimizer},
    rule_engine::{OptimizationRule, RuleApplication, RuleEngine},
    rules::RuleError,
};

// Re-export commonly used types from planner
pub use planner::{
    cost_model::{CostEstimate, CostModel},
    index_selector::{IndexRecommendation, IndexSelector, QueryPredicate},
    plan_generator::{PlanNode, PlanOperation, QueryPlan, QueryPlanner},
};

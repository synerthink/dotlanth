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

//! Query Planning System
//!
//! This module provides cost-based query planning capabilities for DotDB.
//! It generates and evaluates alternative execution plans for queries,
//! using collected statistics to make optimal decisions.

pub mod cost_model;
pub mod index_selector;
pub mod plan_generator;

// Re-export commonly used types
pub use cost_model::{CostEstimate, CostModel, OperationCost};
pub use index_selector::{IndexRecommendation, IndexSelector, IndexUsageHint};
pub use plan_generator::{ExecutionPlan, PlanNode, QueryPlan, QueryPlanner};

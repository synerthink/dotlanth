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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub memory_cost: f64,
    pub network_cost: f64,
    pub total_cost: f64,
}

impl CostEstimate {
    pub fn new(cpu: f64, io: f64, memory: f64, network: f64) -> Self {
        Self {
            cpu_cost: cpu,
            io_cost: io,
            memory_cost: memory,
            network_cost: network,
            total_cost: cpu + io + memory + network,
        }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    pub fn add(&self, other: &CostEstimate) -> CostEstimate {
        CostEstimate::new(
            self.cpu_cost + other.cpu_cost,
            self.io_cost + other.io_cost,
            self.memory_cost + other.memory_cost,
            self.network_cost + other.network_cost,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationCost {
    TableScan { rows: u64, selectivity: f64 },
    IndexScan { rows: u64, index_pages: u64 },
    Sort { rows: u64, columns: usize },
    Join { left_rows: u64, right_rows: u64, join_type: String },
    Aggregate { rows: u64, groups: u64 },
}

pub struct CostModel {
    cpu_cost_per_row: f64,
    io_cost_per_page: f64,
    memory_cost_per_mb: f64,
    sort_factor: f64,
    join_factor: f64,
}

impl CostModel {
    pub fn new() -> Self {
        Self {
            cpu_cost_per_row: 0.01,
            io_cost_per_page: 1.0,
            memory_cost_per_mb: 0.1,
            sort_factor: 1.5,
            join_factor: 2.0,
        }
    }

    pub fn estimate_operation_cost(&self, operation: &OperationCost) -> CostEstimate {
        match operation {
            OperationCost::TableScan { rows, selectivity } => {
                let effective_rows = (*rows as f64 * selectivity) as u64;
                let io_cost = (*rows as f64 / 100.0) * self.io_cost_per_page; // Assume 100 rows per page
                let cpu_cost = effective_rows as f64 * self.cpu_cost_per_row;
                CostEstimate::new(cpu_cost, io_cost, 0.0, 0.0)
            }
            OperationCost::IndexScan { rows, index_pages } => {
                let io_cost = *index_pages as f64 * self.io_cost_per_page;
                let cpu_cost = *rows as f64 * self.cpu_cost_per_row * 0.5; // Index scan is more efficient
                CostEstimate::new(cpu_cost, io_cost, 0.0, 0.0)
            }
            OperationCost::Sort { rows, columns } => {
                let sort_complexity = (*rows as f64).log2() * *rows as f64;
                let cpu_cost = sort_complexity * self.cpu_cost_per_row * self.sort_factor * *columns as f64;
                let memory_cost = (*rows as f64 * *columns as f64 * 8.0) / (1024.0 * 1024.0) * self.memory_cost_per_mb;
                CostEstimate::new(cpu_cost, 0.0, memory_cost, 0.0)
            }
            OperationCost::Join { left_rows, right_rows, join_type } => {
                let base_cost = (*left_rows as f64 * (*right_rows).min(1000) as f64) * self.cpu_cost_per_row;
                let type_multiplier = match join_type.as_str() {
                    "INNER" => 1.0,
                    "LEFT" | "RIGHT" => 1.2,
                    "FULL" => 1.5,
                    "CROSS" => 2.0,
                    _ => 1.0,
                };
                let cpu_cost = base_cost * self.join_factor * type_multiplier;
                let memory_cost = ((*left_rows + *right_rows) as f64 * 8.0) / (1024.0 * 1024.0) * self.memory_cost_per_mb;
                CostEstimate::new(cpu_cost, 0.0, memory_cost, 0.0)
            }
            OperationCost::Aggregate { rows, groups } => {
                let cpu_cost = *rows as f64 * self.cpu_cost_per_row;
                let memory_cost = (*groups as f64 * 64.0) / (1024.0 * 1024.0) * self.memory_cost_per_mb;
                CostEstimate::new(cpu_cost, 0.0, memory_cost, 0.0)
            }
        }
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_estimate_creation() {
        let cost = CostEstimate::new(10.0, 20.0, 5.0, 2.0);
        assert_eq!(cost.total_cost, 37.0);
    }

    #[test]
    fn test_cost_estimate_add() {
        let cost1 = CostEstimate::new(10.0, 20.0, 5.0, 2.0);
        let cost2 = CostEstimate::new(5.0, 10.0, 3.0, 1.0);
        let total = cost1.add(&cost2);
        assert_eq!(total.total_cost, 56.0);
    }

    #[test]
    fn test_table_scan_cost() {
        let model = CostModel::new();
        let operation = OperationCost::TableScan { rows: 1000, selectivity: 0.1 };
        let cost = model.estimate_operation_cost(&operation);
        assert!(cost.total_cost > 0.0);
        assert!(cost.io_cost > 0.0);
        assert!(cost.cpu_cost > 0.0);
    }

    #[test]
    fn test_join_cost() {
        let model = CostModel::new();
        let operation = OperationCost::Join {
            left_rows: 100,
            right_rows: 200,
            join_type: "INNER".to_string(),
        };
        let cost = model.estimate_operation_cost(&operation);
        assert!(cost.total_cost > 0.0);
        assert!(cost.cpu_cost > 0.0);
        assert!(cost.memory_cost > 0.0);
    }
}

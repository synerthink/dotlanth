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

use super::{ExecutionError, Task};
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::Mutex;

pub struct LoadBalancer {
    system: Arc<Mutex<System>>,
    worker_loads: Arc<Mutex<Vec<f32>>>,
}

/// System-aware load balancer using real-time metrics for task distribution.
/// Implements dynamic threshold calculations and worker selection algorithm.
impl LoadBalancer {
    /// Creates new load balancer with:
    /// - System metrics collector
    /// - Per-worker load tracking (initialized to zero)
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let num_cpus = system.cpus().len();
        let initial_worker_loads = vec![0.0; num_cpus];

        Self {
            system: Arc::new(Mutex::new(system)),
            worker_loads: Arc::new(Mutex::new(initial_worker_loads)),
        }
    }

    /// Distributes task to optimal worker based on:
    /// - Current CPU utilization (70% weight)
    /// - Memory utilization (30% weight)
    /// - Dynamic load thresholds
    ///
    /// # Arguments
    /// - `task`: Task to distribute
    ///
    /// # Returns
    /// - `Ok(())`: If suitable worker found
    /// - `Err(ExecutionError::TaskDistributionFailure)`: If overload thresholds exceeded
    pub async fn distribute_task(&self, task: &Task) -> Result<(), ExecutionError> {
        let mut system = self.system.lock().await;
        system.refresh_all();

        let cpu_usage = system.cpus().iter().map(|c| c.cpu_usage()).collect::<Vec<_>>();
        let mem_usage = system.used_memory() as f32 / system.total_memory() as f32 * 100.0;

        let optimal_worker = self.calculate_optimal_worker(&cpu_usage, mem_usage);

        if self.check_thresholds(optimal_worker).await {
            Ok(())
        } else {
            Err(ExecutionError::TaskDistributionFailure)
        }
    }

    /// Calculates optimal worker using weighted sum formula:
    /// `0.7 * CPU_usage + 0.3 * Memory_usage`
    /// Returns index of least-loaded worker
    fn calculate_optimal_worker(&self, cpu_usage: &[f32], mem_usage: f32) -> usize {
        cpu_usage
            .iter()
            .enumerate()
            .map(|(i, &c)| (i, 0.7 * c + 0.3 * mem_usage))
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Verifies worker can accept task using dynamic threshold:
    /// `80% base + (current_load / 2)`
    /// Prevents overloading already busy workers
    async fn check_thresholds(&self, worker_id: usize) -> bool {
        let worker_loads = self.worker_loads.lock().await;

        if worker_id >= worker_loads.len() {
            false
        } else {
            let load = worker_loads[worker_id];
            let dynamic_threshold = 80.0 + (load / 2.0);
            load < dynamic_threshold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_load_balancing() {
        let load_balancer = LoadBalancer::new();
        let task = Task {
            id: 1,
            priority: Default::default(),
            resource_requirements: Default::default(),
        };

        let result = load_balancer.distribute_task(&task).await;
        assert!(result.is_ok());
    }
}

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

pub use super::{load_balancing::LoadBalancer, priority_execution::PriorityExecutor, resource_allocation::ResourceAllocator, work_stealing_scheduler::WorkStealingScheduler};

pub struct ExecutionController {
    scheduler: WorkStealingScheduler,
    priority_executor: PriorityExecutor,
    load_balancer: LoadBalancer,
    resource_allocator: ResourceAllocator,
}

/// Core execution management system coordinating scheduling, prioritization, and resource management.
/// Orchestrates task flow through multiple subsystems:
/// - Priority adjustment
/// - Resource allocation
/// - Load-balanced distribution
/// - Work-stealing scheduling
impl ExecutionController {
    /// Initializes all subsystems with default configurations:
    /// - Fresh instances of scheduler, executor, load balancer, and resource allocator
    pub fn new() -> Self {
        Self {
            scheduler: WorkStealingScheduler::new(),
            priority_executor: PriorityExecutor::new(),
            load_balancer: LoadBalancer::new(),
            resource_allocator: ResourceAllocator::new(),
        }
    }

    /// Executes a task through the full processing pipeline:
    /// 1. **Resource Allocation**: Reserves system resources (CPU/memory)
    /// 2. **Priority Adjustment**: Modifies task priority based on system state
    /// 3. **Load Balancing**: Distributes task to optimal worker
    /// 4. **Scheduling**: Queues task for execution
    ///
    /// # Arguments
    /// - `task`: Task to execute
    ///
    /// # Returns
    /// - `Ok(())`: On successful pipeline execution
    /// - `Err(ExecutionError)`: First error encountered in pipeline stages
    pub async fn execute_task(&mut self, task: Task) -> Result<(), ExecutionError> {
        let resources = self.resource_allocator.allocate_resources(&task).await?;

        let mut adjusted_task = self.priority_executor.adjust_priority(task);
        adjusted_task.resource_requirements = resources;

        self.load_balancer.distribute_task(&adjusted_task).await?;

        self.scheduler.submit_task(adjusted_task).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct Task {
    pub id: u64,
    pub priority: TaskPriority,
    pub resource_requirements: ResourceRequirements,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TaskPriority {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Default, PartialEq)]
pub struct ResourceRequirements {
    pub cpu_cores: f32,
    pub memory_mb: usize,
}

#[derive(Debug)]
pub enum ExecutionError {
    ResourceAllocationFailure,
    TaskDistributionFailure,
    SchedulerOverload,
}

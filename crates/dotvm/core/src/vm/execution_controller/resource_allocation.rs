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

use super::{ExecutionError, ResourceRequirements, Task, TaskPriority};
use sysinfo::System;
use tokio::sync::Mutex;

pub struct ResourceAllocator {
    system: Mutex<System>,
    allocated_resources: Mutex<Vec<ResourceRequirements>>,
}

/// Resource management system with priority-based allocation rules.
/// Implements:
/// - Priority-driven resource boosting
/// - System capacity validation
impl ResourceAllocator {
    /// Initializes with:
    /// - System metrics collector
    /// - Resource tracking registry
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system: Mutex::new(system),
            allocated_resources: Mutex::new(Vec::new()),
        }
    }

    /// Allocates resources with priority handling:
    /// 1. **Priority Boosting**: +20% resources for High/Critical tasks
    /// 2. **Validation Checks**:
    ///    - Available memory >= requested * 1.2 (with boost)
    ///    - CPU cores available >= ceiling(boosted request)
    ///
    /// # Arguments
    /// - `task`: Task requiring resources
    ///
    /// # Returns
    /// - `Ok(ResourceRequirements)`: Allocated resources (possibly boosted)
    /// - `Err(ExecutionError::ResourceAllocationFailure)`: If system cannot satisfy request
    pub async fn allocate_resources(&self, task: &Task) -> Result<ResourceRequirements, ExecutionError> {
        let mut system = self.system.lock().await;
        system.refresh_all();

        let mut adjusted_req = task.resource_requirements.clone();
        if task.priority >= TaskPriority::High {
            adjusted_req.memory_mb = (adjusted_req.memory_mb as f32 * 1.2) as usize;
            adjusted_req.cpu_cores *= 1.2;
        }

        if system.available_memory() > adjusted_req.memory_mb as u64 * 1_024_000 && system.cpus().len() >= adjusted_req.cpu_cores.ceil() as usize {
            self.allocated_resources.lock().await.push(adjusted_req.clone());
            Ok(adjusted_req)
        } else {
            Err(ExecutionError::ResourceAllocationFailure)
        }
    }

    /// Releases resources from allocation registry
    pub async fn release_resources(&self, task: &Task) {
        let mut resources = self.allocated_resources.lock().await;
        resources.retain(|r| r != &task.resource_requirements);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_allocation() {
        let allocator = ResourceAllocator::new();
        let task = Task {
            id: 1,
            priority: Default::default(),
            resource_requirements: ResourceRequirements { cpu_cores: 1.0, memory_mb: 100 },
        };

        let result = allocator.allocate_resources(&task).await;
        assert!(result.is_ok());
    }
}

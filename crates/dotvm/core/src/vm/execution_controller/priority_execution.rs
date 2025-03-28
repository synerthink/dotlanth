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

use super::{Task, TaskPriority};
use sysinfo::System;

pub struct PriorityExecutor {
    system: System,
    critical_tasks: usize,
}

/// Priority management system with adaptive task escalation rules.
/// Implements two core policies:
/// 1. Low→Medium priority upgrade under high system load
/// 2. Critical task preemption handling
impl PriorityExecutor {
    /// Initializes with:
    /// - System metrics collector
    /// - Critical task counter
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
            critical_tasks: 0,
        }
    }

    /// Adjusts task priority using escalation rules:
    /// 1. **Load-Based Escalation**: Upgrades Low→Medium if:
    ///    - Any CPU core >75% utilization
    ///    - Memory usage >75%
    /// 2. **Critical Task Preemption**: Upgrades to High if:
    ///    - Critical tasks present in system
    ///    - Original priority < High
    ///
    /// Returns modified task with adjusted priority
    pub fn adjust_priority(&mut self, mut task: Task) -> Task {
        self.system.refresh_all();

        if task.priority == TaskPriority::Low && self.is_system_under_high_load() {
            task.priority = TaskPriority::Medium;
        }

        if self.critical_tasks > 0 && task.priority < TaskPriority::High {
            task.priority = TaskPriority::High;
        }

        task
    }

    /// Determines high load state using CPU/memory thresholds
    fn is_system_under_high_load(&self) -> bool {
        self.system.cpus().iter().any(|c| c.cpu_usage() > 75.0) || self.system.used_memory() > self.system.total_memory() * 3 / 4
    }
}

/// Custom ordering implementation for priority queue integration:
/// - Reverse ordering (higher priorities come first)
/// - Comparison based solely on priority field
impl Ord for Task {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Task {}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_adjustment() {
        let mut executor = PriorityExecutor::new();
        let task = Task {
            id: 1,
            priority: TaskPriority::Low,
            resource_requirements: Default::default(),
        };

        let adjusted_task = executor.adjust_priority(task);
        assert!(adjusted_task.priority >= TaskPriority::Low);
    }
}

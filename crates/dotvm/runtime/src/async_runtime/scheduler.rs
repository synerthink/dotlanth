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

use crate::async_runtime::lib::{RuntimeError, RuntimeResult, TaskId, TaskMetrics, TaskState};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Task execution priority levels
///
/// # Variants
/// - Low: Background/non-critical tasks
/// - Normal: Default execution priority
/// - High: Time-sensitive operations
/// - Critical: System-critical tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Priority {
    /// Convert to numeric representation for array indexing
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Normal => write!(f, "Normal"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// Scheduled task container with execution context
///
/// # Lifecycle
/// 1. Created with future and priority
/// 2. Scheduled in executor
/// 3. Polled until completion
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Execution priority level
    pub priority: Priority,
    /// Pinned future for safe polling
    future: Pin<Box<dyn Future<Output = RuntimeResult<()>> + Send>>,
    /// Execution telemetry
    metrics: TaskMetrics,
    /// Timestamp of scheduling
    scheduled_time: Instant,
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("priority", &self.priority)
            .field("metrics", &self.metrics)
            .field("scheduled_time", &self.scheduled_time)
            .finish()
    }
}

impl Task {
    /// Create new task with execution context
    ///
    /// # Arguments
    /// - future: Async computation to execute
    /// - priority: Scheduling priority level
    pub fn new<F>(future: F, priority: Priority) -> Self
    where
        F: Future<Output = RuntimeResult<()>> + Send + 'static,
    {
        Self {
            id: TaskId::new(),
            priority,
            future: Box::pin(future),
            metrics: TaskMetrics::new(),
            scheduled_time: Instant::now(),
        }
    }

    /// Access task metrics (immutable)
    pub fn metrics(&self) -> &TaskMetrics {
        &self.metrics
    }

    /// Access task metrics (mutable)
    pub fn metrics_mut(&mut self) -> &mut TaskMetrics {
        &mut self.metrics
    }

    /// Get mutable future reference for polling
    pub fn future_mut(&mut self) -> Pin<&mut (dyn Future<Output = RuntimeResult<()>> + Send)> {
        self.future.as_mut()
    }
}

/// Custom ordering for tasks to create a proper priority queue
/// Tasks with higher priority and earlier scheduled time come first
impl Ord for Task {
    /// Priority-first ordering with FIFO fallback
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let priority_cmp = self.priority.cmp(&other.priority);
        if priority_cmp == std::cmp::Ordering::Equal {
            // For equal priorities, compare by scheduled time (earlier is greater)
            other.scheduled_time.cmp(&self.scheduled_time)
        } else {
            priority_cmp
        }
    }
}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}

/// Thread-safe priority queue wrapper
///
/// # Concurrency
/// Uses Mutex-protected BinaryHeap for thread-safe operations
#[derive(Debug)]
pub struct TaskQueue {
    queue: BinaryHeap<WrappedTask>,
}

/// Atomic reference wrapper for task ordering
#[derive(Debug)]
struct WrappedTask(Arc<Mutex<Task>>);

// Custom ordering implementations for wrapped tasks
impl Ord for WrappedTask {
    /// Delegates to underlying Task ordering
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let task_a = self.0.lock().unwrap();
        let task_b = other.0.lock().unwrap();
        task_a.cmp(&task_b)
    }
}

impl PartialOrd for WrappedTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for WrappedTask {
    fn eq(&self, other: &Self) -> bool {
        let task_a = self.0.lock().unwrap();
        let task_b = other.0.lock().unwrap();
        task_a.id == task_b.id
    }
}

impl Eq for WrappedTask {}

impl TaskQueue {
    /// Initialize empty queue
    pub fn new() -> Self {
        Self { queue: BinaryHeap::new() }
    }

    /// Add task to queue
    ///
    /// # Thread Safety
    /// Lock-free push to heap with atomic references
    pub fn push(&mut self, task: Arc<Mutex<Task>>) {
        self.queue.push(WrappedTask(task));
    }

    /// Remove highest priority task
    ///
    /// # Returns
    /// None if queue is empty
    pub fn pop(&mut self) -> Option<Arc<Mutex<Task>>> {
        self.queue.pop().map(|w| w.0)
    }

    /// Inspect next task without removal
    pub fn peek(&self) -> Option<&Arc<Mutex<Task>>> {
        self.queue.peek().map(|w| &w.0)
    }

    /// Check empty state
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get current queue size
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Main task scheduler with priority management
///
/// # Architecture
/// - Maintains 4 priority queues (Low, Normal, High, Critical)
/// - Provides O(1) task lookup via TaskId
/// - Implements work-stealing for priority enforcement
#[derive(Debug)]
pub struct AsyncTaskScheduler {
    /// Primary task storage
    task_queue: Mutex<TaskQueue>,
    /// Fast task ID lookup
    task_map: Mutex<HashMap<TaskId, Arc<Mutex<Task>>>>,
    /// Priority-specific queues
    priority_queues: [Mutex<VecDeque<Arc<Mutex<Task>>>>; 4],
}

impl AsyncTaskScheduler {
    /// Initialize new scheduler instance
    pub fn new() -> Self {
        Self {
            task_queue: Mutex::new(TaskQueue::new()),
            task_map: Mutex::new(HashMap::new()),
            priority_queues: [
                Mutex::new(VecDeque::new()), // Low
                Mutex::new(VecDeque::new()), // Normal
                Mutex::new(VecDeque::new()), // High
                Mutex::new(VecDeque::new()), // Critical
            ],
        }
    }

    /// Schedule new task with priority
    ///
    /// # Workflow
    /// 1. Create task with unique ID
    /// 2. Add to priority queue
    /// 3. Add to main queue
    /// 4. Register in task map
    pub fn schedule<F>(&self, future: F, priority: Priority) -> RuntimeResult<TaskId>
    where
        F: Future<Output = RuntimeResult<()>> + Send + 'static,
    {
        let task = Task::new(future, priority);
        let task_id = task.id;
        let task = Arc::new(Mutex::new(task));

        // Add to the priority queue
        {
            let mut queue = self.priority_queues[priority.as_u8() as usize].lock().unwrap();
            queue.push_back(Arc::clone(&task));
        }

        // Add to the main queue
        {
            let mut queue = self.task_queue.lock().unwrap();
            queue.push(Arc::clone(&task));
        }

        // Add to the task map
        {
            let mut map = self.task_map.lock().unwrap();
            map.insert(task_id, task);
        }

        Ok(task_id)
    }

    /// Get next executable task
    ///
    /// # Priority Order
    /// 1. Critical -> High -> Normal -> Low
    /// 2. FIFO within same priority level
    pub fn next_task(&self) -> Option<Arc<Mutex<Task>>> {
        // First try to get a task from the main queue
        {
            let mut queue = self.task_queue.lock().unwrap();
            if let Some(task) = queue.pop() {
                return Some(task);
            }
        }

        // If main queue is empty, try priority queues in order
        for priority_level in (0..=3).rev() {
            let mut queue = self.priority_queues[priority_level].lock().unwrap();
            if let Some(task) = queue.pop_front() {
                return Some(task);
            }
        }

        None
    }

    /// Remove task from all queues
    ///
    /// # Complexity
    /// O(n) operation due to queue scanning
    pub fn remove_task(&self, task_id: TaskId) -> RuntimeResult<()> {
        let mut map = self.task_map.lock().unwrap();
        let task = map.remove(&task_id).ok_or_else(|| RuntimeError::Internal(format!("Task {} not found", task_id)))?;

        // Remove from priority queues
        for queue in &self.priority_queues {
            let mut q = queue.lock().unwrap();
            q.retain(|t| t.lock().unwrap().id != task_id);
        }

        // Remove from main task_queue
        let mut task_queue = self.task_queue.lock().unwrap();
        let mut new_heap = BinaryHeap::new();
        while let Some(wrapped_task) = task_queue.queue.pop() {
            if wrapped_task.0.lock().unwrap().id != task_id {
                new_heap.push(wrapped_task);
            }
        }
        task_queue.queue = new_heap;

        Ok(())
    }

    /// Check task existence
    pub fn has_task(&self, task_id: TaskId) -> bool {
        let map = self.task_map.lock().unwrap();
        map.contains_key(&task_id)
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: TaskId) -> Option<Arc<Mutex<Task>>> {
        let map = self.task_map.lock().unwrap();
        map.get(&task_id).cloned()
    }

    /// Get total pending tasks
    pub fn pending_tasks_count(&self) -> usize {
        let map = self.task_map.lock().unwrap();
        map.len()
    }

    /// Get priority distribution statistics
    pub fn task_stats(&self) -> HashMap<Priority, usize> {
        let mut stats = HashMap::new();
        stats.insert(Priority::Low, self.priority_queues[0].lock().unwrap().len());
        stats.insert(Priority::Normal, self.priority_queues[1].lock().unwrap().len());
        stats.insert(Priority::High, self.priority_queues[2].lock().unwrap().len());
        stats.insert(Priority::Critical, self.priority_queues[3].lock().unwrap().len());
        stats
    }
}

impl Default for AsyncTaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Helper function to create test futures
    fn test_future() -> impl Future<Output = RuntimeResult<()>> {
        async { Ok(()) }
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new(test_future(), Priority::Normal);
        assert_eq!(task.priority, Priority::Normal);
        assert_eq!(task.metrics().state, TaskState::Created);
    }

    #[test]
    fn test_task_ordering() {
        let task1 = Task::new(test_future(), Priority::Normal);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let task2 = Task::new(test_future(), Priority::Normal);

        // For equal priorities, earlier tasks should come first
        assert!(task1 > task2);

        let task3 = Task::new(test_future(), Priority::High);
        // Higher priority should come first regardless of time
        assert!(task3 > task1);
    }

    #[test]
    fn test_task_queue() {
        let mut queue = TaskQueue::new();
        assert!(queue.is_empty());

        queue.push(Arc::new(Mutex::new(Task::new(test_future(), Priority::Normal))));
        queue.push(Arc::new(Mutex::new(Task::new(test_future(), Priority::High))));
        queue.push(Arc::new(Mutex::new(Task::new(test_future(), Priority::Low))));

        assert_eq!(queue.len(), 3);

        // First should be High priority
        let task = queue.pop().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::High);

        // Next should be Normal priority
        let task = queue.pop().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::Normal);

        // Last should be Low priority
        let task = queue.pop().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::Low);

        assert!(queue.is_empty());
    }

    #[test]
    fn test_scheduler_scheduling() {
        let scheduler = AsyncTaskScheduler::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = counter.clone();
        let task_id = scheduler
            .schedule(
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
                Priority::Normal,
            )
            .unwrap();

        assert!(scheduler.has_task(task_id));
        assert_eq!(scheduler.pending_tasks_count(), 1);

        let task = scheduler.next_task().unwrap();
        assert_eq!(task.lock().unwrap().id, task_id);
    }

    #[test]
    fn test_scheduler_priority_ordering() {
        let scheduler = AsyncTaskScheduler::new();

        let task_id1 = scheduler.schedule(test_future(), Priority::Low).unwrap();
        let task_id2 = scheduler.schedule(test_future(), Priority::Normal).unwrap();
        let task_id3 = scheduler.schedule(test_future(), Priority::High).unwrap();
        let task_id4 = scheduler.schedule(test_future(), Priority::Critical).unwrap();

        // Tasks should be returned in priority order

        // First should be Critical
        let task = scheduler.next_task().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::Critical);

        // Next should be High
        let task = scheduler.next_task().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::High);

        // Next should be Normal
        let task = scheduler.next_task().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::Normal);

        // Last should be Low
        let task = scheduler.next_task().unwrap();
        assert_eq!(task.lock().unwrap().priority, Priority::Low);
    }

    #[test]
    fn test_task_removal() {
        let scheduler = AsyncTaskScheduler::new();
        let task_id = scheduler.schedule(test_future(), Priority::Normal).unwrap();

        assert!(scheduler.has_task(task_id));
        scheduler.remove_task(task_id).unwrap();
        assert!(!scheduler.has_task(task_id));

        // Removing a non-existent task should fail
        assert!(scheduler.remove_task(task_id).is_err());
    }

    #[test]
    fn test_task_stats() {
        let scheduler = AsyncTaskScheduler::new();

        // Add some tasks with different priorities
        scheduler.schedule(test_future(), Priority::Low).unwrap();
        scheduler.schedule(test_future(), Priority::Low).unwrap();
        scheduler.schedule(test_future(), Priority::Normal).unwrap();
        scheduler.schedule(test_future(), Priority::High).unwrap();
        scheduler.schedule(test_future(), Priority::Critical).unwrap();

        let stats = scheduler.task_stats();
        assert_eq!(*stats.get(&Priority::Low).unwrap(), 2);
        assert_eq!(*stats.get(&Priority::Normal).unwrap(), 1);
        assert_eq!(*stats.get(&Priority::High).unwrap(), 1);
        assert_eq!(*stats.get(&Priority::Critical).unwrap(), 1);
    }
}

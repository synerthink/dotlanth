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

use crate::async_runtime::executor::FutureExecutor;
use crate::async_runtime::lib::{RuntimeError, RuntimeResult, TaskId, TaskState};
use crate::async_runtime::scheduler::{AsyncTaskScheduler, Priority};
use futures_timer::Delay;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Comprehensive result of a task status check operation
///
/// # Variants
/// - Terminal states: Completed/Failed/TimedOut/Cancelled
/// - Transient states: Ready/InProgress
/// - Error state: NotFound
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollResult {
    /// Task is queued and ready for execution
    Ready(TaskId),
    /// Task finished successfully
    Completed(TaskId),
    /// Task failed with error message
    Failed(TaskId, String),
    /// Task exceeded allowed execution time
    TimedOut(TaskId),
    /// Task was cancelled by user request
    Cancelled(TaskId),
    /// Task is currently executing
    InProgress(TaskId),
    /// Task ID not found in system
    NotFound(TaskId),
}

impl PollResult {
    /// Extract task ID from any poll result
    ///
    /// # Guarantees
    /// Returns ID even for NotFound results for consistent error handling
    pub fn task_id(&self) -> TaskId {
        match self {
            Self::Ready(id) => *id,
            Self::Completed(id) => *id,
            Self::Failed(id, _) => *id,
            Self::TimedOut(id) => *id,
            Self::Cancelled(id) => *id,
            Self::InProgress(id) => *id,
            Self::NotFound(id) => *id,
        }
    }

    /// Check if task reached terminal state
    ///
    /// # Returns
    /// True for Completed/Failed/TimedOut/Cancelled
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Failed(_, _) | Self::TimedOut(_) | Self::Cancelled(_))
    }

    /// Check for successful completion
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed(_))
    }

    /// Check for any failure state
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_, _) | Self::TimedOut(_) | Self::Cancelled(_))
    }
}

/// Aggregate status of multiple polling operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollStatus {
    /// All monitored tasks completed
    AllCompleted,
    /// Partial completion count
    SomeCompleted(usize),
    /// Tasks still in execution
    InProgress(usize),
    /// No tasks provided for monitoring
    NoTasks,
}

/// Configurable parameters for polling behavior
///
/// # Defaults
/// - interval: 50ms
/// - timeout: 30s
/// - max_polls: None (unlimited)
#[derive(Debug, Clone)]
pub struct PollConfig {
    /// Delay between polling attempts
    pub interval: Duration,
    /// Maximum polling iterations
    pub max_polls: Option<usize>,
    /// Total allowed monitoring duration
    pub timeout: Option<Duration>,
    /// Enable diagnostic logging
    pub log_results: bool,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(50),
            max_polls: None,
            timeout: Some(Duration::from_secs(30)),
            log_results: false,
        }
    }
}

/// Central monitoring service for task status tracking
///
/// # Architecture
/// - Integrates with scheduler and executor
/// - Maintains historical results
/// - Implements configurable polling strategies
pub struct Poller {
    /// Reference to task scheduler
    scheduler: Arc<AsyncTaskScheduler>,
    /// Optional executor reference for completion checks
    executor: Option<Arc<FutureExecutor>>,
    /// Current polling configuration
    config: PollConfig,
    /// Last recorded results per task
    last_results: Mutex<HashMap<TaskId, PollResult>>,
    /// Completion timestamps for result retention
    completion_times: Mutex<HashMap<TaskId, Instant>>,
}

impl Poller {
    /// Initialize with scheduler only
    ///
    /// # Limitations
    /// Cannot detect executor-side completions
    pub fn new(scheduler: Arc<AsyncTaskScheduler>) -> Self {
        Self {
            scheduler,
            executor: None,
            config: PollConfig::default(),
            last_results: Mutex::new(HashMap::new()),
            completion_times: Mutex::new(HashMap::new()),
        }
    }

    /// Initialize with full execution context
    ///
    /// # Advantages
    /// Can detect both scheduler and executor states
    pub fn with_executor(scheduler: Arc<AsyncTaskScheduler>, executor: Arc<FutureExecutor>) -> Self {
        Self {
            scheduler,
            executor: Some(executor),
            config: PollConfig::default(),
            last_results: Mutex::new(HashMap::new()),
            completion_times: Mutex::new(HashMap::new()),
        }
    }

    /// Update polling parameters
    ///
    /// # Thread Safety
    /// Requires exclusive access (mut self)
    pub fn set_config(&mut self, config: PollConfig) {
        self.config = config;
    }

    /// Check single task status
    ///
    /// # Workflow
    /// 1. Check executor completion records
    /// 2. Verify scheduler existence
    /// 3. Inspect task state
    /// 4. Update internal tracking
    pub fn poll_task(&self, task_id: TaskId) -> PollResult {
        // Check if the task is completed in the executor's records
        if let Some(executor) = &self.executor {
            if let Some(completed_task) = executor.get_completed_task(task_id) {
                // Update last results and completion times
                let mut last_results = self.last_results.lock().unwrap();
                let result = completed_task.clone();
                last_results.insert(task_id, result.clone());
                let mut completion_times = self.completion_times.lock().unwrap();
                completion_times.insert(task_id, Instant::now());
                return result;
            }
        }
        // Check if task exists in scheduler
        if !self.scheduler.has_task(task_id) {
            // Check if it's in our completed tasks
            let completion_times = self.completion_times.lock().unwrap();
            if completion_times.contains_key(&task_id) {
                // Task has completed, return last result
                let last_results = self.last_results.lock().unwrap();
                if let Some(result) = last_results.get(&task_id) {
                    return result.clone();
                }
            }

            return PollResult::NotFound(task_id);
        }

        // Get the task
        let task_opt = self.scheduler.get_task(task_id);
        if let Some(task) = task_opt {
            // Task exists, check its status
            let result = match task.lock() {
                Ok(guard) => match guard.metrics().state {
                    TaskState::Completed => PollResult::Completed(task_id),
                    TaskState::Failed => PollResult::Failed(task_id, "Task execution failed".to_string()),
                    TaskState::Cancelled => PollResult::Cancelled(task_id),
                    TaskState::Running | TaskState::Waiting => PollResult::InProgress(task_id),
                    TaskState::Created | TaskState::Scheduled => PollResult::Ready(task_id),
                },
                Err(_) => PollResult::InProgress(task_id), // Can't lock, assume in progress
            };

            // Update last result
            let mut last_results = self.last_results.lock().unwrap();
            last_results.insert(task_id, result.clone());

            // If task completed, record completion time
            if result.is_complete() {
                let mut completion_times = self.completion_times.lock().unwrap();
                completion_times.entry(task_id).or_insert_with(Instant::now);
            }

            return result;
        }

        PollResult::NotFound(task_id)
    }

    /// Batch status check for multiple tasks
    ///
    /// # Performance
    /// Uses parallel polling where possible
    pub fn poll_tasks(&self, task_ids: &[TaskId]) -> HashMap<TaskId, PollResult> {
        let mut results = HashMap::new();

        for &task_id in task_ids {
            let result = self.poll_task(task_id);
            results.insert(task_id, result);
        }

        results
    }

    /// Get aggregate status of task group
    ///
    /// # Use Cases
    /// - Progress tracking
    /// - Completion detection
    pub fn check_status(&self, task_ids: &[TaskId]) -> PollStatus {
        if task_ids.is_empty() {
            return PollStatus::NoTasks;
        }

        let results = self.poll_tasks(task_ids);

        let total = task_ids.len();
        let completed = results.values().filter(|r| r.is_complete()).count();

        if completed == total {
            PollStatus::AllCompleted
        } else if completed > 0 {
            PollStatus::SomeCompleted(completed)
        } else {
            PollStatus::InProgress(total - completed)
        }
    }

    /// Block until task completion
    ///
    /// # Cancellation
    /// Respects global timeout/max polls configuration
    ///
    /// # Errors
    /// - Timeout
    /// - TaskNotFound
    /// - Execution failures
    pub fn wait_for_task(&self, task_id: TaskId) -> RuntimeResult<PollResult> {
        let start_time = Instant::now();
        let mut poll_count = 0;

        loop {
            let result = self.poll_task(task_id);

            if result.is_complete() {
                return match result {
                    PollResult::Completed(_) => Ok(result),
                    PollResult::Failed(_, error) => Err(RuntimeError::ExecutionFailed(error)),
                    PollResult::TimedOut(_) => Err(RuntimeError::Timeout),
                    PollResult::Cancelled(_) => Err(RuntimeError::Cancelled),
                    _ => Err(RuntimeError::Internal("Unexpected poll result".into())),
                };
            }

            if let PollResult::NotFound(_) = result {
                return Err(RuntimeError::Internal("Task not found".into()));
            }

            // Check if we've exceeded max polls
            poll_count += 1;
            if let Some(max) = self.config.max_polls {
                if poll_count >= max {
                    return Err(RuntimeError::Timeout);
                }
            }

            // Check if we've exceeded timeout
            if let Some(timeout) = self.config.timeout {
                if start_time.elapsed() >= timeout {
                    return Err(RuntimeError::Timeout);
                }
            }

            // Wait before polling again
            std::thread::sleep(self.config.interval);

            // If we have an executor, run it once to make progress
            if let Some(executor) = &self.executor {
                executor.tick();
            }
        }
    }

    /// Block until all tasks complete
    ///
    /// # Performance
    /// Efficiently removes completed tasks from tracking
    pub fn wait_for_all(&self, task_ids: &[TaskId]) -> RuntimeResult<HashMap<TaskId, PollResult>> {
        let start_time = Instant::now();
        let mut poll_count = 0;
        let mut remaining: Vec<TaskId> = task_ids.to_vec();
        let mut results = HashMap::new();

        while !remaining.is_empty() {
            let poll_results = self.poll_tasks(&remaining);

            // Remove completed tasks
            remaining.retain(|id| {
                if let Some(result) = poll_results.get(id) {
                    if result.is_complete() {
                        results.insert(*id, result.clone());
                        false
                    } else {
                        true
                    }
                } else {
                    false
                }
            });

            if remaining.is_empty() {
                break;
            }

            // Check if we've exceeded max polls
            poll_count += 1;
            if let Some(max) = self.config.max_polls {
                if poll_count >= max {
                    return Err(RuntimeError::Timeout);
                }
            }

            // Check if we've exceeded timeout
            if let Some(timeout) = self.config.timeout {
                if start_time.elapsed() >= timeout {
                    return Err(RuntimeError::Timeout);
                }
            }

            // Wait before polling again
            std::thread::sleep(self.config.interval);

            // If we have an executor, run it once to make progress
            if let Some(executor) = &self.executor {
                executor.tick();
            }
        }

        Ok(results)
    }

    /// Block until first task completes
    ///
    /// # Use Cases
    /// - Race condition handling
    /// - Quick failure detection
    pub fn wait_for_any(&self, task_ids: &[TaskId]) -> RuntimeResult<PollResult> {
        let start_time = Instant::now();
        let mut poll_count = 0;

        loop {
            let poll_results = self.poll_tasks(task_ids);

            // Check if any task has completed
            for result in poll_results.values() {
                if result.is_complete() {
                    return match result {
                        PollResult::Completed(_) => Ok(result.clone()),
                        PollResult::Failed(_, error) => Err(RuntimeError::ExecutionFailed(error.clone())),
                        PollResult::TimedOut(_) => Err(RuntimeError::Timeout),
                        PollResult::Cancelled(_) => Err(RuntimeError::Cancelled),
                        _ => Err(RuntimeError::Internal("Unexpected poll result".into())),
                    };
                }
            }

            // Check if all tasks are not found
            if poll_results.values().all(|r| matches!(r, PollResult::NotFound(_))) {
                return Err(RuntimeError::Internal("All tasks not found".into()));
            }

            // Check if we've exceeded max polls
            poll_count += 1;
            if let Some(max) = self.config.max_polls {
                if poll_count >= max {
                    return Err(RuntimeError::Timeout);
                }
            }

            // Check if we've exceeded timeout
            if let Some(timeout) = self.config.timeout {
                if start_time.elapsed() >= timeout {
                    return Err(RuntimeError::Timeout);
                }
            }

            // Wait before polling again
            std::thread::sleep(self.config.interval);

            // If we have an executor, run it once to make progress
            if let Some(executor) = &self.executor {
                executor.tick();
            }
        }
    }

    /// Prune old completion records
    ///
    /// # Retention Policy
    /// Removes records older than specified duration
    /// Propagates cleanup to executor
    pub fn cleanup_completed(&self, older_than: Duration) {
        {
            let now = Instant::now();
            let mut completion_times = self.completion_times.lock().unwrap();
            let mut last_results = self.last_results.lock().unwrap();
            let to_remove: Vec<TaskId> = completion_times.iter().filter(|(_, ct)| now.duration_since(**ct) >= older_than).map(|(&tid, _)| tid).collect();
            for tid in to_remove {
                completion_times.remove(&tid);
                last_results.remove(&tid);
            }
        }
        if let Some(executor) = &self.executor {
            executor.cleanup_completed_tasks(older_than);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_runtime::executor::FutureExecutor;
    use crate::async_runtime::scheduler::{AsyncTaskScheduler, Priority};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    // Helper function for simple futures
    async fn simple_task() -> RuntimeResult<()> {
        Ok(())
    }

    // Helper function for failing tasks
    async fn failing_task() -> RuntimeResult<()> {
        Err(RuntimeError::ExecutionFailed("Test failure".into()))
    }

    // Helper function for long-running tasks
    async fn long_task() -> RuntimeResult<()> {
        Delay::new(Duration::from_millis(500)).await;
        Ok(())
    }

    // Helper function for cancel-able tasks
    fn cancellable_task(flag: Arc<AtomicBool>) -> impl std::future::Future<Output = RuntimeResult<()>> {
        async move {
            for _ in 0..10 {
                if flag.load(Ordering::SeqCst) {
                    return Err(RuntimeError::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(())
        }
    }

    #[test]
    fn test_poll_task() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let poller = Poller::with_executor(scheduler.clone(), executor.clone());

        // Schedule a task
        let task_id = executor.submit(simple_task(), Priority::Normal).unwrap();

        // Initial poll should show Ready or InProgress
        let result = poller.poll_task(task_id);
        assert!(matches!(result, PollResult::Ready(_) | PollResult::InProgress(_)));

        // Run the task
        executor.tick();

        // Poll should now show Completed
        let result = poller.poll_task(task_id);
        assert!(matches!(result, PollResult::Completed(_)));
    }

    #[test]
    fn test_poll_failing_task() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let poller = Poller::with_executor(scheduler.clone(), executor.clone());

        // Schedule a failing task
        let task_id = executor.submit(failing_task(), Priority::Normal).unwrap();

        // Run the task
        executor.tick();

        // Poll should show Failed
        let result = poller.poll_task(task_id);
        assert!(matches!(result, PollResult::Failed(_, _)));
    }

    #[test]
    fn test_poll_multiple_tasks() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let poller = Poller::with_executor(scheduler.clone(), executor.clone());

        // Schedule multiple tasks
        let task_id1 = executor.submit(simple_task(), Priority::Normal).unwrap();
        let task_id2 = executor.submit(failing_task(), Priority::Normal).unwrap();
        let task_id3 = executor.submit(long_task(), Priority::High).unwrap();

        // Poll all tasks
        let results = poller.poll_tasks(&[task_id1, task_id2, task_id3]);
        assert_eq!(results.len(), 3);

        // Run tasks
        executor.tick();

        // Poll again
        let results = poller.poll_tasks(&[task_id1, task_id2, task_id3]);
        assert!(matches!(results.get(&task_id1), Some(PollResult::Completed(_))));
        assert!(matches!(results.get(&task_id2), Some(PollResult::Failed(_, _))));

        // Long task might still be running
        let long_result = results.get(&task_id3).unwrap();
        match long_result {
            PollResult::Completed(_) | PollResult::InProgress(_) => (),
            _ => panic!("Expected Completed or InProgress, got {:?}", long_result),
        }
    }

    #[test]
    fn test_wait_for_task() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let mut poller = Poller::with_executor(scheduler.clone(), executor.clone());

        // Set a short polling interval
        poller.set_config(PollConfig {
            interval: Duration::from_millis(10),
            max_polls: None,
            timeout: Some(Duration::from_secs(1)),
            log_results: false,
        });

        // Schedule a simple task
        let task_id = executor.submit(simple_task(), Priority::Normal).unwrap();

        // Wait for it to complete
        let result = poller.wait_for_task(task_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wait_for_failing_task() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let mut poller = Poller::with_executor(scheduler.clone(), executor.clone());

        // Set a short polling interval
        poller.set_config(PollConfig {
            interval: Duration::from_millis(10),
            max_polls: None,
            timeout: Some(Duration::from_secs(1)),
            log_results: false,
        });

        // Schedule a failing task
        let task_id = executor.submit(failing_task(), Priority::Normal).unwrap();

        // Wait for it to complete - should return an error
        let result = poller.wait_for_task(task_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RuntimeError::ExecutionFailed(_)));
    }

    #[test]
    fn test_wait_for_all() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let mut poller = Poller::with_executor(scheduler.clone(), executor.clone());

        // Set a short polling interval
        poller.set_config(PollConfig {
            interval: Duration::from_millis(10),
            max_polls: None,
            timeout: Some(Duration::from_secs(1)),
            log_results: false,
        });

        // Schedule multiple tasks
        let task_id1 = executor.submit(simple_task(), Priority::Normal).unwrap();
        let task_id2 = executor.submit(simple_task(), Priority::Normal).unwrap();

        // Wait for all to complete
        let results = poller.wait_for_all(&[task_id1, task_id2]).unwrap();
        assert_eq!(results.len(), 2);
        assert!(matches!(results.get(&task_id1), Some(PollResult::Completed(_))));
        assert!(matches!(results.get(&task_id2), Some(PollResult::Completed(_))));
    }

    #[test]
    fn test_wait_for_any() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let mut poller = Poller::with_executor(scheduler.clone(), executor.clone());

        poller.set_config(PollConfig {
            interval: Duration::from_millis(10),
            timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        });

        let task_id1 = executor.submit(long_task(), Priority::Low).unwrap();
        let task_id2 = executor.submit(simple_task(), Priority::High).unwrap();

        let result = poller.wait_for_any(&[task_id1, task_id2]).unwrap();
        assert_eq!(result.task_id(), task_id2);
    }

    #[test]
    fn test_cleanup_completed() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let poller = Poller::with_executor(scheduler.clone(), executor.clone());

        let task_id = executor.submit(simple_task(), Priority::Normal).unwrap();
        executor.run_until_complete();

        // Allow the completed task to age first
        std::thread::sleep(Duration::from_millis(2));

        // Then clean up
        poller.cleanup_completed(Duration::from_millis(1));

        let result = poller.poll_task(task_id);
        assert!(matches!(result, PollResult::NotFound(_)));
    }
}

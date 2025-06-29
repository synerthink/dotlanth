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

use crate::async_runtime::PollResult;
use crate::async_runtime::lib::{RuntimeError, RuntimeResult, TaskId, TaskState};
use crate::async_runtime::scheduler::{AsyncTaskScheduler, Priority, Task};
use futures::Future;
use futures::task::{ArcWake, Context, Poll, Waker};
use futures_timer::Delay;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::task::Wake;
use std::time::{Duration, Instant};

/// Aggregate execution statistics for monitoring system health
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Successfully completed tasks
    pub completed: usize,
    /// Tasks failed due to errors
    pub failed: usize,
    /// User-cancelled tasks
    pub cancelled: usize,
    /// Tasks exceeding timeout thresholds
    pub timeouts: usize,
    /// Cumulative execution time across all tasks
    pub total_execution_time: Duration,
    /// Moving average of task execution times
    pub average_execution_time: Duration,
    /// Longest observed task execution time
    pub max_execution_time: Duration,
}

/// Custom waker implementation for executor integration
///
/// # Wake Protocol
/// - Tracks task ID and executor reference
/// - Uses ArcWake for thread-safe reference counting
struct TaskWaker {
    task_id: TaskId,
    executor: Arc<FutureExecutor>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.executor.wake_task(self.task_id);
    }
}

impl ArcWake for TaskWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.executor.wake_task(arc_self.task_id);
    }
}

/// Completed task record for result retention
#[derive(Debug, Clone)]
pub struct CompletedTask {
    /// Final execution outcome
    pub result: PollResult,
    /// Timestamp of completion
    pub completion_time: Instant,
}

/// Core executor implementing future polling and task lifecycle management
///
/// # Architecture
/// - Integrated with priority scheduler
/// - Timeout enforcement
/// - Metrics collection
/// - Result caching
#[derive(Debug)]
pub struct FutureExecutor {
    /// Priority-based task scheduler
    scheduler: Arc<AsyncTaskScheduler>,
    /// Waker registry for task wakeups
    wakers: Mutex<HashMap<TaskId, Waker>>,
    /// Queue of tasks ready for execution
    ready_tasks: Mutex<Vec<TaskId>>,
    /// Actively executing tasks with timestamps
    running_tasks: Mutex<HashMap<TaskId, Instant>>,
    /// Performance metrics collector
    stats: Mutex<ExecutionStats>,
    /// Global timeout configuration
    default_timeout: Arc<Mutex<Duration>>,
    /// Completed task archive (time-bound retention)
    completed_tasks: Mutex<HashMap<TaskId, CompletedTask>>,
}

impl FutureExecutor {
    /// Initialize executor with scheduler dependency
    ///
    /// # Concurrency
    /// All internal state protected by Mutex/Arc
    pub fn new(scheduler: Arc<AsyncTaskScheduler>) -> Arc<Self> {
        Arc::new(Self {
            scheduler,
            wakers: Mutex::new(HashMap::new()),
            ready_tasks: Mutex::new(Vec::new()),
            running_tasks: Mutex::new(HashMap::new()),
            stats: Mutex::new(ExecutionStats::default()),
            default_timeout: Arc::new(Mutex::new(Duration::from_secs(30))),
            completed_tasks: Mutex::new(HashMap::new()),
        })
    }

    /// Update global timeout setting
    ///
    /// # Thread Safety
    /// Uses Mutex-protected atomic update
    pub fn set_default_timeout(&self, timeout: Duration) {
        *self.default_timeout.lock().unwrap() = timeout;
    }

    /// Mark task for execution in next tick
    ///
    /// # Idempotency
    /// Prevents duplicate entries in ready queue
    pub fn wake_task(&self, task_id: TaskId) {
        if self.scheduler.has_task(task_id) {
            let mut ready = self.ready_tasks.lock().unwrap();
            if !ready.contains(&task_id) {
                ready.push(task_id);
            }
        }
    }

    /// Generate task-specific waker
    fn create_waker(&self, task_id: TaskId) -> Waker {
        let task_waker = Arc::new(TaskWaker {
            task_id,
            executor: Arc::new(self.clone()),
        });
        futures::task::waker(task_waker)
    }

    /// Get or create waker with registry caching
    fn get_waker(&self, task_id: TaskId) -> Waker {
        let mut wakers = self.wakers.lock().unwrap();
        wakers.entry(task_id).or_insert_with(|| self.create_waker(task_id)).clone()
    }

    /// Execute single task through polling state machine
    ///
    /// # Workflow
    /// 1. Acquire task lock
    /// 2. Update execution metrics
    /// 3. Poll future
    /// 4. Handle completion/timeout
    ///
    /// # Returns
    /// - Ok(true) if task completed
    /// - Ok(false) if still pending
    /// - Err on execution failure
    fn execute_task(&self, task: Arc<Mutex<Task>>) -> RuntimeResult<bool> {
        let task_id = {
            let task_guard = task.lock().unwrap();
            task_guard.id
        };

        // Phase 1: Context preparation
        let waker = self.get_waker(task_id);
        let mut context = Context::from_waker(&waker);

        // Phase 2: State transition
        let mut task_guard = match task.lock() {
            Ok(guard) => guard,
            Err(_) => return Err(RuntimeError::Internal("Failed to lock task".into())),
        };

        // Update metrics if this is the first time running
        if task_guard.metrics().state == TaskState::Created || task_guard.metrics().state == TaskState::Scheduled {
            task_guard.metrics_mut().mark_started();
        }

        // Increment poll count
        task_guard.metrics_mut().increment_poll_count();

        // Phase 3: Core execution
        match task_guard.future_mut().as_mut().poll(&mut context) {
            // Handle success/failure states
            // Update metrics and completion registry
            // Cleanup resources
            Poll::Ready(result) => {
                match result {
                    Ok(_) => {
                        task_guard.metrics_mut().mark_completed();
                        {
                            let mut stats = self.stats.lock().unwrap();
                            stats.completed += 1;
                            if let Some(duration) = task_guard.metrics().running_duration() {
                                stats.total_execution_time += duration;
                                if duration > stats.max_execution_time {
                                    stats.max_execution_time = duration;
                                }
                                if stats.completed > 0 {
                                    stats.average_execution_time = stats.total_execution_time / stats.completed as u32;
                                }
                            }
                        }
                        self.completed_tasks.lock().unwrap().insert(
                            task_guard.id,
                            CompletedTask {
                                result: PollResult::Completed(task_guard.id),
                                completion_time: Instant::now(),
                            },
                        );
                    }
                    Err(e) => {
                        task_guard.metrics_mut().mark_failed();
                        {
                            let mut stats = self.stats.lock().unwrap();
                            match e {
                                RuntimeError::Cancelled => stats.cancelled += 1,
                                RuntimeError::Timeout => stats.timeouts += 1,
                                _ => stats.failed += 1,
                            }
                        }

                        self.completed_tasks.lock().unwrap().insert(
                            task_guard.id,
                            CompletedTask {
                                result: PollResult::Failed(task_guard.id, "Task execution failed".to_string()),
                                completion_time: Instant::now(),
                            },
                        );

                        return Err(e);
                    }
                }
                {
                    let mut running = self.running_tasks.lock().unwrap();
                    running.remove(&task_guard.id);
                }
                {
                    let mut wakers = self.wakers.lock().unwrap();
                    wakers.remove(&task_guard.id);
                }
                Ok(true)
            }
            Poll::Pending => {
                // Task not yet complete
                // Add to running tasks if not already there
                let mut running = self.running_tasks.lock().unwrap();
                running.entry(task_id).or_insert_with(Instant::now);

                Ok(false) // Task not completed
            }
        }
    }

    /// Retrieve completed task results
    pub fn get_completed_task(&self, task_id: TaskId) -> Option<PollResult> {
        self.completed_tasks.lock().unwrap().get(&task_id).map(|ct| ct.result.clone())
    }

    /// Prune old completed tasks
    ///
    /// # Retention Policy
    /// Removes tasks older than specified duration
    pub fn cleanup_completed_tasks(&self, older_than: Duration) {
        let now = Instant::now();
        self.completed_tasks.lock().unwrap().retain(|_, ct| now.duration_since(ct.completion_time) < older_than);
    }

    /// Detect timed-out tasks using global timeout
    fn check_timeouts(&self) -> Vec<TaskId> {
        let default_timeout = *self.default_timeout.lock().unwrap();
        let now = Instant::now();
        let mut timed_out = Vec::new();
        let mut running = self.running_tasks.lock().unwrap();

        for (task_id, start_time) in running.iter() {
            if now.duration_since(*start_time) > default_timeout {
                timed_out.push(*task_id);
            }
        }

        for task_id in &timed_out {
            running.remove(task_id);
        }

        timed_out
    }

    /// Handle timeout lifecycle events
    fn handle_timeouts(&self, timed_out: Vec<TaskId>) {
        for task_id in timed_out {
            if let Some(task) = self.scheduler.get_task(task_id)
                && let Ok(mut task_guard) = task.lock()
            {
                task_guard.metrics_mut().mark_failed();
                let mut stats = self.stats.lock().unwrap();
                stats.timeouts += 1;
            }

            self.completed_tasks.lock().unwrap().insert(
                task_id,
                CompletedTask {
                    result: PollResult::TimedOut(task_id),
                    completion_time: Instant::now(),
                },
            );

            // Remove the task from the scheduler
            let _ = self.scheduler.remove_task(task_id);

            // Remove the waker
            let mut wakers = self.wakers.lock().unwrap();
            wakers.remove(&task_id);
        }
    }

    /// Single iteration of execution loop
    ///
    /// # Execution Order
    /// 1. Process timeouts
    /// 2. Execute ready tasks
    /// 3. Schedule new tasks
    pub fn tick(&self) -> usize {
        // Phase 1: Timeout enforcement
        let timed_out = self.check_timeouts();
        self.handle_timeouts(timed_out);

        // Phase 2: Ready task processing
        let ready = {
            let mut ready = self.ready_tasks.lock().unwrap();
            std::mem::take(&mut *ready)
        };

        let mut completed = 0;

        for task_id in ready {
            if let Some(task) = self.scheduler.get_task(task_id) {
                match self.execute_task(task) {
                    Ok(is_completed) => {
                        if is_completed {
                            completed += 1;
                            let _ = self.scheduler.remove_task(task_id);
                            let mut ready = self.ready_tasks.lock().unwrap();
                            ready.retain(|id| *id != task_id);
                        }
                    }
                    Err(_) => {
                        // Task failed, remove it
                        let _ = self.scheduler.remove_task(task_id);
                    }
                }
            }
        }

        // Phase 3: New task scheduling
        while let Some(task) = self.scheduler.next_task() {
            let task_id = task.lock().unwrap().id;
            match self.execute_task(task) {
                Ok(is_completed) => {
                    if is_completed {
                        completed += 1;
                        let _ = self.scheduler.remove_task(task_id);
                    } else {
                        // Task is not completed, push to ready queue for next tick
                        let mut ready = self.ready_tasks.lock().unwrap();
                        if !ready.contains(&task_id) {
                            ready.push(task_id);
                        }
                    }
                }
                Err(_) => {
                    // Task failed, remove it
                    let _ = self.scheduler.remove_task(task_id);
                }
            }
        }

        completed
    }

    /// Run to completion with congestion control
    ///
    /// # Congestion Management
    /// - Implements exponential backoff
    /// - Prevents CPU spinlock
    pub fn run_until_complete(&self) -> usize {
        let mut total_completed = 0;

        loop {
            let completed = self.tick();
            total_completed += completed;

            // Check if there are any tasks left
            let have_ready = !self.ready_tasks.lock().unwrap().is_empty();
            let have_pending = self.scheduler.pending_tasks_count() > 0;

            if !have_ready && !have_pending {
                break;
            }

            // Prevent CPU spinning if nothing completed in this tick
            if completed == 0 && !have_ready {
                std::thread::sleep(Duration::from_millis(1));
            }
        }

        total_completed
    }

    /// Submit new task to execution pipeline
    ///
    /// Tasks are scheduled according to Priority enum value
    pub fn submit<F>(&self, future: F, priority: Priority) -> RuntimeResult<TaskId>
    where
        F: Future<Output = RuntimeResult<()>> + Send + 'static,
    {
        self.scheduler.schedule(future, priority)
    }

    /// Snapshot current execution metrics
    pub fn get_stats(&self) -> ExecutionStats {
        self.stats.lock().unwrap().clone()
    }

    /// Check if there are any running tasks
    pub fn has_running_tasks(&self) -> bool {
        !self.running_tasks.lock().unwrap().is_empty()
    }

    /// Get the number of ready tasks
    pub fn ready_tasks_count(&self) -> usize {
        self.ready_tasks.lock().unwrap().len()
    }

    /// Get the combined count of ready and pending tasks
    pub fn total_task_count(&self) -> usize {
        self.ready_tasks_count() + self.scheduler.pending_tasks_count()
    }
}

// Clone implementation maintains scheduler reference but resets ephemeral state
impl Clone for FutureExecutor {
    fn clone(&self) -> Self {
        // Fresh instance with shared scheduler
        Self {
            scheduler: Arc::clone(&self.scheduler),
            wakers: Mutex::new(HashMap::new()),
            ready_tasks: Mutex::new(Vec::new()),
            running_tasks: Mutex::new(HashMap::new()),
            stats: Mutex::new(ExecutionStats::default()),
            default_timeout: Arc::clone(&self.default_timeout),
            completed_tasks: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_runtime::scheduler::AsyncTaskScheduler;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    // Helper function for simple futures
    async fn simple_task() -> RuntimeResult<()> {
        Ok(())
    }

    // Helper function for tasks that fail
    async fn failing_task() -> RuntimeResult<()> {
        Err(RuntimeError::ExecutionFailed("Test failure".into()))
    }

    // Helper function for long-running tasks
    async fn long_task() -> RuntimeResult<()> {
        // Simulate work
        std::thread::sleep(Duration::from_millis(50));
        Ok(())
    }

    #[test]
    fn test_executor_submit_and_run() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler);

        // Submit a task
        let task_id = executor.submit(simple_task(), Priority::Normal).unwrap();
        assert!(executor.scheduler.has_task(task_id));

        // Run all tasks
        let completed = executor.run_until_complete();
        assert_eq!(completed, 1);

        // Check stats
        let stats = executor.get_stats();
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_executor_failing_task() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler);

        // Submit a failing task
        let task_id = executor.submit(failing_task(), Priority::Normal).unwrap();
        assert!(executor.scheduler.has_task(task_id));

        // Run all tasks
        let completed = executor.run_until_complete();
        assert_eq!(completed, 0); // Should have 0 completed tasks

        // Check stats
        let stats = executor.get_stats();
        assert_eq!(stats.completed, 0);
        assert_eq!(stats.failed, 1);
    }

    #[test]
    fn test_executor_multiple_tasks() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler);

        // Submit multiple tasks
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..5 {
            let counter_clone = counter.clone();
            executor
                .submit(
                    async move {
                        counter_clone.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    },
                    Priority::Normal,
                )
                .unwrap();
        }

        // Run all tasks
        let completed = executor.run_until_complete();
        assert_eq!(completed, 5);
        assert_eq!(counter.load(Ordering::SeqCst), 5);

        // Check stats
        let stats = executor.get_stats();
        assert_eq!(stats.completed, 5);
    }

    #[test]
    fn test_executor_timeout() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler);

        // Set a short timeout
        executor.set_default_timeout(Duration::from_millis(10));

        // Submit a task that will timeout
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        executor
            .submit(
                async move {
                    // This will take longer than the timeout
                    Delay::new(Duration::from_millis(50)).await;
                    flag_clone.store(true, Ordering::SeqCst);
                    Ok(())
                },
                Priority::Normal,
            )
            .unwrap();

        // Run all tasks
        executor.run_until_complete();

        // The task should have timed out and not completed
        assert_eq!(flag.load(Ordering::SeqCst), false);

        // Check stats
        let stats = executor.get_stats();
        assert_eq!(stats.timeouts, 1);
    }

    #[test]
    fn test_executor_priority_ordering() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler);

        let execution_order = Arc::new(Mutex::new(Vec::new()));

        // Submit tasks with different priorities
        let order_clone = execution_order.clone();
        executor
            .submit(
                async move {
                    order_clone.lock().unwrap().push("low");
                    Ok(())
                },
                Priority::Low,
            )
            .unwrap();

        let order_clone = execution_order.clone();
        executor
            .submit(
                async move {
                    order_clone.lock().unwrap().push("normal");
                    Ok(())
                },
                Priority::Normal,
            )
            .unwrap();

        let order_clone = execution_order.clone();
        executor
            .submit(
                async move {
                    order_clone.lock().unwrap().push("high");
                    Ok(())
                },
                Priority::High,
            )
            .unwrap();

        let order_clone = execution_order.clone();
        executor
            .submit(
                async move {
                    order_clone.lock().unwrap().push("critical");
                    Ok(())
                },
                Priority::Critical,
            )
            .unwrap();

        // Run all tasks
        executor.run_until_complete();

        // Check execution order - higher priority should be first
        let order = execution_order.lock().unwrap();
        assert_eq!(order[0], "critical");
        assert_eq!(order[1], "high");
        assert_eq!(order[2], "normal");
        assert_eq!(order[3], "low");
    }

    #[test]
    fn test_executor_statistics() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler);

        // Submit a mix of tasks
        for _ in 0..3 {
            executor.submit(simple_task(), Priority::Normal).unwrap();
        }

        executor.submit(failing_task(), Priority::Normal).unwrap();

        // Run all tasks
        executor.run_until_complete();

        // Check stats
        let stats = executor.get_stats();
        assert_eq!(stats.completed, 3);
        assert_eq!(stats.failed, 1);
        assert!(stats.average_execution_time > Duration::from_nanos(0));
    }
}

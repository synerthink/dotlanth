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
use crate::async_runtime::lib::{RuntimeError, RuntimeResult, TaskId};
use crate::async_runtime::scheduler::AsyncTaskScheduler;
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

/// Atomic cancellation signal with timeout enforcement
///
/// # Thread Safety
/// - Uses `AtomicBool` for thread-safe cancellation status
/// - `Mutex<String>` protects cancellation reason
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    task_id: TaskId,
    created_at: Instant,
    timeout: Duration,
    cancellation_reason: Arc<Mutex<Option<String>>>,
}

impl CancellationToken {
    /// Creates new token with automatic timeout enforcement
    ///
    /// # Arguments
    /// - `task_id`: Associated task identifier
    /// - `timeout`: Maximum allowed execution duration
    pub fn new(task_id: TaskId, timeout: Duration) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            task_id,
            created_at: Instant::now(),
            timeout,
            cancellation_reason: Arc::new(Mutex::new(None)),
        }
    }

    /// Checks cancellation status with automatic timeout validation
    pub fn is_cancelled(&self) -> bool {
        self.check_timeout();
        self.cancelled.load(Ordering::Acquire)
    }

    /// Cancel the associated task with optional reason
    pub fn cancel(&self, reason: Option<&str>) -> RuntimeResult<()> {
        // Rule 1: Store cancellation reason if provided
        if let Some(reason) = reason {
            *self.cancellation_reason.lock().unwrap() = Some(reason.to_string());
        }

        // Rule 2: Update atomic cancellation flag
        self.cancelled.store(true, Ordering::Release);
        Ok(())
    }

    /// Calculates time since token creation
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Check and enforce timeout
    pub fn check_timeout(&self) -> bool {
        if self.age() > self.timeout {
            self.cancel(Some("Timeout")).ok();
            true
        } else {
            false
        }
    }

    /// Retrieves cancellation reason with minimal locking
    pub fn cancellation_reason(&self) -> Option<String> {
        self.cancellation_reason.lock().unwrap().clone()
    }
}

/// Task cancellation controller with metrics integration
///
/// # Concurrency
/// Contains only thread-safe components (Arc-wrapped)
#[derive(Debug, Clone)]
pub struct TaskHandle {
    token: CancellationToken,
    scheduler: Arc<AsyncTaskScheduler>,
    executor: Arc<FutureExecutor>,
    metrics: Arc<CancellationMetrics>,
}

/// Atomic metrics collection structure
#[derive(Debug, Default)]
struct CancellationMetrics {
    total_cancelled: AtomicU64,
    avg_latency_micros: AtomicU64,
    max_latency_micros: AtomicU64,
}

/// Handle for managing task cancellation
impl TaskHandle {
    /// Constructs new handle with shared metrics
    pub fn new(token: CancellationToken, scheduler: Arc<AsyncTaskScheduler>, executor: Arc<FutureExecutor>, metrics: Arc<CancellationMetrics>) -> Self {
        Self { token, scheduler, executor, metrics }
    }

    /// Executes cancellation with custom cleanup logic
    ///
    /// # Workflow
    /// 1. Execute provided cleanup function
    /// 2. Perform standard cancellation
    /// 3. Update performance metrics
    ///
    /// # Errors
    /// Propagates errors from cleanup function
    pub fn cancel_gracefully<F>(&self, cleanup: F) -> RuntimeResult<()>
    where
        F: FnOnce() -> RuntimeResult<()>,
    {
        let start_time = Instant::now();

        // Phase 1: Custom cleanup
        cleanup()?;

        // Phase 2: Standard cancellation
        self.cancel()?;

        // Phase 3: Metrics update
        let latency = start_time.elapsed().as_micros() as u64;
        self.metrics
            .avg_latency_micros
            .store((self.metrics.avg_latency_micros.load(Ordering::Relaxed) + latency) / 2, Ordering::Relaxed);
        self.metrics.max_latency_micros.fetch_max(latency, Ordering::Relaxed);

        Ok(())
    }

    /// Standard cancellation procedure
    ///
    /// # Steps
    /// 1. Mark token as cancelled
    /// 2. Update task metrics
    /// 3. Remove task from scheduler
    /// 4. Notify executor
    /// 5. Update global metrics
    pub fn cancel(&self) -> RuntimeResult<()> {
        // Step 1: Trigger cancellation
        self.token.cancel(None)?;

        let task_id = self.token.task_id;

        // Step 2: Update task metrics
        if let Some(task) = self.scheduler.get_task(task_id)
            && let Ok(mut task) = task.lock()
        {
            task.metrics_mut().mark_cancelled();
            if let Some(reason) = self.token.cancellation_reason() {
                task.metrics_mut().cancel_reason = Some(reason);
            }
        }

        // Step 3: Remove from scheduler (tolerate 'not found' errors)
        if let Err(e) = self.scheduler.remove_task(task_id)
            && let RuntimeError::Internal(ref msg) = e
            && !msg.contains("not found")
        {
            return Err(e);
        }

        // Step 4: Notify executor
        self.executor.wake_task(task_id);

        // Step 5: Update system metrics
        self.metrics.total_cancelled.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }
}

/// System-wide cancellation manager with background maintenance
///
/// # Safety
/// - All operations protected by Mutex/Atomic
/// - Background thread runs every 10ms
#[derive(Debug)]
pub struct CancellationSystem {
    handles: Arc<Mutex<HashMap<TaskId, Arc<TaskHandle>>>>,
    metrics: Arc<CancellationMetrics>,
    background_cleaner: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Default for CancellationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationSystem {
    /// Initializes system with background cleanup thread
    pub fn new() -> Self {
        let system = Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(CancellationMetrics::default()),
            background_cleaner: Mutex::new(None),
        };
        system.start_background_cleaner();
        system
    }

    /// Starts background thread for periodic task cleanup and metrics updates
    ///
    /// # Workflow
    /// 1. Runs every 10ms in a dedicated thread
    /// 2. Three-phase maintenance process:
    ///    - Phase 1: Detect timed-out tasks
    ///    - Phase 2: Cancel expired tasks and update metrics
    ///    - Phase 3: Prune completed cancellations
    ///
    /// # Concurrency
    /// - Uses `Mutex<HashMap>` for thread-safe task tracking
    /// - Atomic operations for lock-free metrics updates
    fn start_background_cleaner(&self) {
        let handles = self.handles.clone();
        let metrics = self.metrics.clone(); // Used for atomic metrics updates

        let handle = std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(10));

                // Phase 1: Detect timeout candidates
                let mut to_remove = vec![];
                {
                    let handles = handles.lock().unwrap();
                    // Rule: Check all tasks for timeout expiration
                    for (id, handle) in handles.iter() {
                        if handle.token.check_timeout() {
                            to_remove.push(*id);
                        }
                    }
                }

                // Metrics Update: Atomic bulk increment for timeout-triggered cancellations
                let timeout_count = to_remove.len() as u64;
                if timeout_count > 0 {
                    // Optimization: Single atomic operation for multiple timeouts
                    metrics.total_cancelled.fetch_add(timeout_count, Ordering::Relaxed);
                }

                // Phase 2: Process expired tasks
                for id in to_remove {
                    // Graceful cleanup: Attempt proper cancellation
                    if let Some(h) = handles.lock().unwrap().get(&id) {
                        let _ = h.cancel(); // Fire-and-forget pattern
                    }
                }

                // Phase 3: Memory cleanup
                // Retention policy: Remove handles for completed cancellations
                handles.lock().unwrap().retain(|_, h| !h.token.is_cancelled());
            }
        });

        // Store thread handle for potential future join
        *self.background_cleaner.lock().unwrap() = Some(handle);
    }

    /// Registers new task with cancellation tracking
    pub fn register_task(&self, task_id: TaskId, scheduler: Arc<AsyncTaskScheduler>, executor: Arc<FutureExecutor>, timeout: Duration) -> (CancellationToken, Arc<TaskHandle>) {
        let token = CancellationToken::new(task_id, timeout);
        let handle = Arc::new(TaskHandle::new(token.clone(), scheduler, executor, self.metrics.clone()));

        self.handles.lock().unwrap().insert(task_id, handle.clone());
        (token, handle)
    }

    /// Cancels task by ID
    pub fn cancel_task(&self, task_id: TaskId) -> RuntimeResult<()> {
        if let Some(handle) = self.handles.lock().unwrap().remove(&task_id) {
            handle.cancel()?;
        }
        Ok(())
    }

    /// Returns current system statistics
    pub fn stats(&self) -> CancellationStats {
        CancellationStats {
            total: self.metrics.total_cancelled.load(Ordering::Relaxed),
            avg_latency_micros: self.metrics.avg_latency_micros.load(Ordering::Relaxed),
            max_latency_micros: self.metrics.max_latency_micros.load(Ordering::Relaxed),
        }
    }
}

/// System cancellation statistics snapshot
#[derive(Debug, Clone)]
pub struct CancellationStats {
    pub total: u64,
    pub avg_latency_micros: u64,
    pub max_latency_micros: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_runtime::scheduler::AsyncTaskScheduler;
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    async fn cancellable_task(token: CancellationToken) -> RuntimeResult<()> {
        let mut iterations = 0;
        loop {
            if token.is_cancelled() {
                return Err(RuntimeError::Cancelled);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
            iterations += 1;
            if iterations > 100 {
                return Ok(());
            }
        }
    }

    #[test]
    fn test_basic_cancellation() {
        let scheduler = Arc::new(AsyncTaskScheduler::new());
        let executor = FutureExecutor::new(scheduler.clone());
        let cs = CancellationSystem::new();

        let task_id = TaskId::new();
        let (token, handle) = cs.register_task(task_id, scheduler.clone(), executor.clone(), Duration::from_secs(10));

        executor.submit(cancellable_task(token.clone()), Priority::Normal).unwrap();
        handle.cancel().unwrap();

        assert!(token.is_cancelled());
        assert_eq!(cs.stats().total, 1);
    }

    #[test]
    fn test_timeout_cancellation() {
        let cs = CancellationSystem::new();
        let (token, _) = cs.register_task(
            TaskId::new(),
            Arc::new(AsyncTaskScheduler::new()),
            FutureExecutor::new(Arc::new(AsyncTaskScheduler::new())),
            Duration::from_millis(50),
        );

        std::thread::sleep(Duration::from_millis(100));
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_graceful_cancellation() {
        let cs = CancellationSystem::new();
        let (token, handle) = cs.register_task(
            TaskId::new(),
            Arc::new(AsyncTaskScheduler::new()),
            FutureExecutor::new(Arc::new(AsyncTaskScheduler::new())),
            Duration::from_secs(10),
        );

        let cleanup_called = Arc::new(AtomicBool::new(false));
        let cc = cleanup_called.clone();

        handle
            .cancel_gracefully(|| {
                cc.store(true, Ordering::Relaxed);
                Ok(())
            })
            .unwrap();

        assert!(cleanup_called.load(Ordering::Relaxed));
        assert!(token.is_cancelled());
    }
}

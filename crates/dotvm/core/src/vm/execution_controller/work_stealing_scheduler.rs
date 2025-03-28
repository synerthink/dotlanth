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

use super::{ExecutionError, Task, TaskPriority};
use crossbeam_deque::{Steal, Stealer, Worker};
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

#[derive(Clone)]
pub struct WorkStealingScheduler {
    workers: Arc<Mutex<Vec<Worker<Task>>>>,
    stealers: Arc<Vec<Stealer<Task>>>,
    task_sender: mpsc::Sender<Task>,
    priority_queues: Arc<Mutex<Vec<BinaryHeap<Task>>>>,
    task_receiver: Arc<Mutex<mpsc::Receiver<Task>>>,
}

/// Work-stealing task scheduler with priority queue integration.
/// Features:
/// - Per-worker FIFO queues
/// - Cross-worker task stealing
/// - Priority-based preemption
impl WorkStealingScheduler {
    /// Initializes scheduler with:
    /// - CPU-count worker threads
    /// - Priority queues per worker
    /// - MPSC channel for task submission
    pub fn new() -> Self {
        let num_workers = num_cpus::get();
        let (task_sender, task_receiver) = mpsc::channel(1000);

        let (workers, stealers): (Vec<_>, Vec<_>) = (0..num_workers)
            .map(|_| {
                let worker = Worker::new_fifo();
                let stealer = worker.stealer();
                (worker, stealer)
            })
            .unzip();

        let priority_queues = vec![BinaryHeap::new(); num_workers];

        Self {
            workers: Arc::new(Mutex::new(workers)),
            stealers: Arc::new(stealers),
            priority_queues: Arc::new(Mutex::new(priority_queues)),
            task_sender,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
        }
    }

    /// Starts worker threads with execution loop:
    /// 1. Check local priority queue
    /// 2. Check local worker queue
    /// 3. Attempt work stealing
    /// 4. Wait for new tasks
    pub async fn start(&self) -> Result<(), ExecutionError> {
        let mut handles = vec![];
        for i in 0..num_cpus::get() {
            let scheduler = self.clone();
            handles.push(tokio::spawn(async move {
                scheduler.run_worker(i).await;
            }));
        }

        for handle in handles {
            handle.await.map_err(|_| ExecutionError::SchedulerOverload)?;
        }

        Ok(())
    }

    /// Core work execution loop for individual scheduler workers.
    /// Implements prioritized task processing with work-stealing fallback.
    ///
    /// Execution flow per iteration:
    /// 1. **Priority Queue Check**: Processes highest-priority tasks first
    /// 2. **Local Queue Check**: Handles regular FIFO tasks
    /// 3. **Work Stealing Attempt**: Seeks tasks from other workers
    /// 4. **New Task Reception**: Blocks until new tasks arrive
    ///
    /// # Locking Strategy
    /// - Uses scoped locks to minimize contention
    /// - Explicit drops for early lock release
    /// - Separation of priority/local queue locks
    ///
    /// # Error Resilience
    /// - Automatic lock release on drop
    /// - Continuous loop survives individual task failures
    async fn run_worker(&self, worker_id: usize) {
        loop {
            // STAGE 1: Acquire locks with minimal scope
            let mut priority_queues = self.priority_queues.lock().await;
            let mut workers = self.workers.lock().await;

            // STAGE 2: Priority task processing
            if let Some(task) = priority_queues[worker_id].pop() {
                // Early lock release before execution
                drop(priority_queues);
                drop(workers);
                Self::execute_task(task).await;
                continue; // Restart loop for fresh state check
            }

            // STAGE 3: Local queue processing
            if let Some(task) = workers[worker_id].pop() {
                drop(priority_queues);
                drop(workers);
                Self::execute_task(task).await;
                continue;
            }

            // STAGE 4: Work stealing attempt
            if let Some(task) = self.steal_task(worker_id).await {
                drop(priority_queues);
                drop(workers);
                Self::execute_task(task).await;
                continue;
            }

            // STAGE 5: Prepare for blocking wait
            drop(priority_queues);
            drop(workers);

            // STAGE 6: Receive new tasks
            if let Some(task) = self.task_receiver.lock().await.recv().await {
                self.priority_queues.lock().await[worker_id].push(task);
            }
        }
    }

    /// Submits task to scheduler via channel
    pub async fn submit_task(&self, task: Task) -> Result<(), ExecutionError> {
        self.task_sender.send(task).await.map_err(|_| ExecutionError::SchedulerOverload)
    }

    /// Work-stealing algorithm:
    /// - Iterates through other workers' queues
    /// - Attempts to steal oldest task
    async fn steal_task(&self, worker_id: usize) -> Option<Task> {
        let workers = self.workers.lock().await;
        for (i, stealer) in self.stealers.iter().enumerate() {
            if i == worker_id {
                continue;
            }
            match stealer.steal() {
                Steal::Success(task) => return Some(task),
                _ => continue,
            }
        }
        None
    }

    /// Task execution wrapper
    async fn execute_task(task: Task) {
        tokio::task::spawn_blocking(move || {
            println!("Executing task {}", task.id);
        })
        .await
        .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[tokio::test]
    async fn test_work_stealing_basic() {
        let scheduler = WorkStealingScheduler::new();
        let task = Task {
            id: 1,
            priority: TaskPriority::Medium,
            resource_requirements: Default::default(),
        };

        let result = scheduler.submit_task(task).await;
        assert!(result.is_ok());
    }
}

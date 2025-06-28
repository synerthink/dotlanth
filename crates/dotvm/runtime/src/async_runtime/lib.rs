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

use std::fmt;
use std::time::{Duration, Instant};

/// Result type for async runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Error types for the async runtime
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// Task has been cancelled
    Cancelled,
    /// Task timed out
    Timeout,
    /// Task execution failed
    ExecutionFailed(String),
    /// Resource allocation error
    ResourceError(String),
    /// Internal runtime error
    Internal(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Task cancelled"),
            Self::Timeout => write!(f, "Task timed out"),
            Self::ExecutionFailed(msg) => write!(f, "Execution failed: {msg}"),
            Self::ResourceError(msg) => write!(f, "Resource error: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Represents the state of a task in the async runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is created but not yet scheduled
    Created,
    /// Task is scheduled but not yet started
    Scheduled,
    /// Task is currently running
    Running,
    /// Task is waiting for some condition or resource
    Waiting,
    /// Task has completed successfully
    Completed,
    /// Task has failed
    Failed,
    /// Task has been cancelled
    Cancelled,
}

/// Statistics collector for task execution
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    /// When the task was created
    pub created_at: Instant,
    /// When the task started execution
    pub started_at: Option<Instant>,
    /// When the task completed (success or failure)
    pub completed_at: Option<Instant>,
    /// Current state of the task
    pub state: TaskState,
    /// Number of times the task has been polled
    pub poll_count: usize,
    pub cancel_reason: Option<String>,
}

impl TaskMetrics {
    /// Create new task metrics
    pub fn new() -> Self {
        Self {
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            state: TaskState::Created,
            poll_count: 0,
            cancel_reason: None,
        }
    }

    /// Get the duration the task has been running
    pub fn running_duration(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            (Some(start), None) => Some(Instant::now().duration_since(start)),
            _ => None,
        }
    }

    /// Get the total duration from creation to completion
    pub fn total_duration(&self) -> Duration {
        match self.completed_at {
            Some(end) => end.duration_since(self.created_at),
            None => Instant::now().duration_since(self.created_at),
        }
    }

    /// Mark task as started
    pub fn mark_started(&mut self) {
        self.started_at = Some(Instant::now());
        self.state = TaskState::Running;
    }

    /// Mark task as completed
    pub fn mark_completed(&mut self) {
        self.completed_at = Some(Instant::now());
        self.state = TaskState::Completed;
    }

    /// Mark task as failed
    pub fn mark_failed(&mut self) {
        self.completed_at = Some(Instant::now());
        self.state = TaskState::Failed;
    }

    /// Mark task as cancelled
    pub fn mark_cancelled(&mut self) {
        self.completed_at = Some(Instant::now());
        self.state = TaskState::Cancelled;
    }

    /// Increment poll count
    pub fn increment_poll_count(&mut self) {
        self.poll_count += 1;
    }
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// A unique identifier for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskId {
    /// Generate a new unique task ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Task-{}", self.0)
    }
}

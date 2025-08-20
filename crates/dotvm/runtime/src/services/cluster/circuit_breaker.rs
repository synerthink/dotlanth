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

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Circuit is open, requests are rejected
    HalfOpen, // Testing if the service has recovered
}

#[derive(Debug)]
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    last_failure_time: AtomicU64,
    last_success_time: AtomicU64,
    next_attempt: AtomicU64,
    failure_threshold: usize,
    timeout: Duration,
    retry_timeout: Duration,
    success_threshold: usize,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, timeout: Duration, retry_timeout: Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            last_failure_time: AtomicU64::new(0),
            last_success_time: AtomicU64::new(0),
            next_attempt: AtomicU64::new(0),
            failure_threshold,
            timeout,
            retry_timeout,
            success_threshold: 3, // Default: require 3 successful calls to close circuit
        }
    }

    pub async fn can_execute(&self) -> bool {
        let state = *self.state.read().await;
        let now = Self::current_time_millis();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let next_attempt = self.next_attempt.load(Ordering::Relaxed);
                if now >= next_attempt {
                    // Try to move to half-open state
                    let mut state_guard = self.state.write().await;
                    if *state_guard == CircuitState::Open && now >= self.next_attempt.load(Ordering::Relaxed) {
                        *state_guard = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::Relaxed);
                        tracing::info!("Circuit breaker moved to HalfOpen state");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub async fn record_success(&self) {
        let now = Self::current_time_millis();
        self.last_success_time.store(now, Ordering::Relaxed);

        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

                if success_count >= self.success_threshold {
                    // Close the circuit
                    let mut state_guard = self.state.write().await;
                    *state_guard = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    tracing::info!("Circuit breaker closed after successful recovery");
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but just in case
                tracing::warn!("Received success signal while circuit breaker is open");
            }
        }
    }

    pub async fn record_failure(&self) {
        let now = Self::current_time_millis();
        self.last_failure_time.store(now, Ordering::Relaxed);

        let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                if failure_count >= self.failure_threshold {
                    // Open the circuit
                    let mut state_guard = self.state.write().await;
                    if *state_guard == CircuitState::Closed {
                        *state_guard = CircuitState::Open;
                        self.next_attempt.store(now + self.retry_timeout.as_millis() as u64, Ordering::Relaxed);
                        tracing::warn!(failure_count = failure_count, threshold = self.failure_threshold, "Circuit breaker opened due to failures");
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Move back to open state
                let mut state_guard = self.state.write().await;
                *state_guard = CircuitState::Open;
                self.next_attempt.store(now + self.retry_timeout.as_millis() as u64, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
                tracing::warn!("Circuit breaker reopened due to failure in HalfOpen state");
            }
            CircuitState::Open => {
                // Update next attempt time
                self.next_attempt.store(now + self.retry_timeout.as_millis() as u64, Ordering::Relaxed);
            }
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }

    pub fn is_open(&self) -> bool {
        // Non-async version for quick checks
        if let Ok(state) = self.state.try_read() {
            *state == CircuitState::Open
        } else {
            false // Conservative default
        }
    }

    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(Ordering::Relaxed)
    }

    pub fn get_success_count(&self) -> usize {
        self.success_count.load(Ordering::Relaxed)
    }

    pub fn get_failure_rate(&self) -> f64 {
        let failures = self.failure_count.load(Ordering::Relaxed);
        let successes = self.success_count.load(Ordering::Relaxed);
        let total = failures + successes;

        if total == 0 { 0.0 } else { (failures as f64 / total as f64) * 100.0 }
    }

    pub async fn reset(&self) {
        let mut state_guard = self.state.write().await;
        *state_guard = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.next_attempt.store(0, Ordering::Relaxed);
        tracing::info!("Circuit breaker manually reset to Closed state");
    }

    pub async fn force_open(&self) {
        let mut state_guard = self.state.write().await;
        *state_guard = CircuitState::Open;
        let now = Self::current_time_millis();
        self.next_attempt.store(now + self.retry_timeout.as_millis() as u64, Ordering::Relaxed);
        tracing::warn!("Circuit breaker manually forced to Open state");
    }

    pub fn get_stats(&self) -> CircuitBreakerStats {
        let now = Self::current_time_millis();
        let last_failure = self.last_failure_time.load(Ordering::Relaxed);
        let last_success = self.last_success_time.load(Ordering::Relaxed);

        CircuitBreakerStats {
            state: if let Ok(state) = self.state.try_read() { state.clone() } else { CircuitState::Closed },
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            failure_threshold: self.failure_threshold,
            success_threshold: self.success_threshold,
            failure_rate: self.get_failure_rate(),
            last_failure_time: if last_failure > 0 { Some(last_failure) } else { None },
            last_success_time: if last_success > 0 { Some(last_success) } else { None },
            next_attempt_time: if self.is_open() { Some(self.next_attempt.load(Ordering::Relaxed)) } else { None },
            timeout_duration_ms: self.timeout.as_millis() as u64,
            retry_timeout_duration_ms: self.retry_timeout.as_millis() as u64,
        }
    }

    fn current_time_millis() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: usize,
    pub success_count: usize,
    pub failure_threshold: usize,
    pub success_threshold: usize,
    pub failure_rate: f64,
    pub last_failure_time: Option<u64>,
    pub last_success_time: Option<u64>,
    pub next_attempt_time: Option<u64>,
    pub timeout_duration_ms: u64,
    pub retry_timeout_duration_ms: u64,
}

#[derive(Debug)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,
    pub timeout: Duration,
    pub retry_timeout: Duration,
    pub success_threshold: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            retry_timeout: Duration::from_secs(10),
            success_threshold: 3,
        }
    }
}

impl CircuitBreakerConfig {
    pub fn new(failure_threshold: usize, timeout: Duration, retry_timeout: Duration, success_threshold: usize) -> Self {
        Self {
            failure_threshold,
            timeout,
            retry_timeout,
            success_threshold,
        }
    }

    pub fn create_circuit_breaker(&self) -> CircuitBreaker {
        let mut cb = CircuitBreaker::new(self.failure_threshold, self.timeout, self.retry_timeout);
        cb.success_threshold = self.success_threshold;
        cb
    }
}

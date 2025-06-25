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

use std::sync::Mutex; // Removed Arc
use std::time::SystemTime;

/// Enum representing possible system states.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Running,
    Paused,
    Error,
}

/// Enum representing events that trigger state transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Start,
    Pause,
    Resume,
    Stop,
    Fail,
}

/// Error type for state transition failures.
#[derive(Debug)]
pub enum TransitionError {
    InvalidTransition,
    RollbackFailed(String),
    NotImplemented,
}

/// Struct holding details about a state transition.
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub current: State,
    pub target: State,
    pub timestamp: SystemTime,
    pub event: Event,
}

/// Manages state transitions with concurrency safety.
pub struct StateManager {
    pub current_state: Mutex<State>,
}

/// Core state transition handler with thread-safe mutex protection.
/// Manages state changes through events, direct updates, and rollbacks.
impl StateManager {
    /// Creates a new instance with the specified initial state.
    ///
    /// # Arguments
    /// - `initial`: Starting state of the system (e.g., `State::Idle`)
    pub fn new(initial: State) -> Self {
        StateManager { current_state: Mutex::new(initial) }
    }

    /// Processes an event to trigger state transitions.
    /// Follows predefined transition rules:
    /// - Idle + Start ➔ Running
    /// - Running + Pause ➔ Paused
    /// - Paused + Resume ➔ Running
    /// - Running + Stop ➔ Idle
    /// - Running + Fail ➔ Error
    ///
    /// # Arguments
    /// - `event`: Triggering event (e.g., `Event::Start`)
    ///
    /// # Returns
    /// - `Ok(())`: On valid transition
    /// - `Err(TransitionError::InvalidTransition)`: For unsupported event/state pairs
    pub fn process_event(&self, event: &Event) -> Result<(), TransitionError> {
        let mut current = self.current_state.lock().unwrap(); // Thread-safe lock
        let new_state = match (&*current, event) {
            // Valid transitions
            (State::Idle, Event::Start) => State::Running,
            (State::Running, Event::Pause) => State::Paused,
            (State::Paused, Event::Resume) => State::Running,
            (State::Running, Event::Stop) => State::Idle,
            (State::Running, Event::Fail) => State::Error,
            // Invalid combinations
            _ => return Err(TransitionError::InvalidTransition),
        };
        *current = new_state; // Apply state change
        Ok(())
    }

    /// Directly changes the state with validation:
    /// - Blocks transitions to/from `Error` state
    ///
    /// # Arguments
    /// - `new_state`: Target state
    ///
    /// # Returns
    /// - `Ok(())`: On valid change
    /// - `Err(TransitionError::InvalidTransition)`: For `Error`-related attempts
    pub fn change_state(&self, new_state: State) -> Result<(), TransitionError> {
        let mut current = self.current_state.lock().unwrap();
        // Block Error state transitions
        if *current == State::Error || new_state == State::Error {
            return Err(TransitionError::InvalidTransition);
        }
        *current = new_state;
        Ok(())
    }

    /// Reverts the current state to a specified `previous_state` with validations:
    /// - **Mutex Lock Safety**: Handles mutex poisoning errors
    /// - **No-Op Rule**: Skips update if current state matches target
    /// - **Error State Policy**: Only allows rollback to `Idle` from `Error`
    /// - **State Update**: Applies the rollback if validations pass
    ///
    /// # Arguments
    /// - `previous_state`: Target state to revert to
    ///
    /// # Returns
    /// - `Ok(())`: On successful rollback or no-op
    /// - `Err(TransitionError)`: For failures (lock errors/invalid transitions)
    pub fn rollback(&self, previous_state: State) -> Result<(), TransitionError> {
        // Rule 1: Acquire mutex lock with error propagation
        let mut current = self.current_state.lock().map_err(|_| TransitionError::RollbackFailed("Mutex lock failed (poisoning)".into()))?;

        // Rule 2: Skip update if current == target (no-op)
        if *current == previous_state {
            return Ok(()); // No change needed
        }

        // Rule 3: Error state transition constraints
        if *current == State::Error && previous_state != State::Idle {
            return Err(TransitionError::InvalidTransition); // Error → Non-Idle blocked
        }

        // Rule 4: Apply the rollback
        *current = previous_state;
        Ok(()) // Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_initial_state() {
        let manager = StateManager::new(State::Idle);
        let current = manager.current_state.lock().unwrap();
        assert_eq!(*current, State::Idle, "Initial state should be Idle");
    }

    #[test]
    fn test_process_event_start() {
        let manager = StateManager::new(State::Idle);
        let result = manager.process_event(&Event::Start);
        assert!(result.is_ok(), "Processing a Start event should succeed");
    }

    #[test]
    fn test_change_state() {
        let manager = StateManager::new(State::Idle);
        let result = manager.change_state(State::Running);
        assert!(result.is_ok(), "Changing state to Running should succeed");
        let current = manager.current_state.lock().unwrap();
        assert_eq!(*current, State::Running, "Current state should be Running");
    }

    #[test]
    fn test_invalid_transition() {
        // For this test, assume that transitioning from Paused to Error is invalid.
        let manager = StateManager::new(State::Paused);
        let result = manager.change_state(State::Error);
        assert!(result.is_err(), "Invalid transition should return an error");
        let current = manager.current_state.lock().unwrap();
        assert_eq!(*current, State::Paused, "State should remain unchanged on invalid transition");
    }

    #[test]
    fn test_rollback_mechanism() {
        let manager = StateManager::new(State::Running);
        // Simulate a failed transition and call rollback.
        let _ = manager.change_state(State::Paused); // assume failure occurs here
        let rollback_result = manager.rollback(State::Running);
        assert!(rollback_result.is_ok(), "Rollback should succeed");
        let current = manager.current_state.lock().unwrap();
        assert_eq!(*current, State::Running, "State should rollback to Running");
    }

    #[test]
    fn test_concurrent_transitions() {
        let manager = Arc::new(StateManager::new(State::Idle));

        let mut handles = vec![];
        for _ in 0..5 {
            let mgr_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                // All threads try to process a Start event concurrently.
                mgr_clone.process_event(&Event::Start).unwrap_or_else(|_| ());
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let current = manager.current_state.lock().unwrap();
        // Expected final state after concurrent Start events is Running.
        assert_eq!(*current, State::Running, "After concurrent Start events, state should be Running");
    }
}

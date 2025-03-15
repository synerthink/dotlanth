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

use super::state_transitions::{State, TransitionError};
use std::sync::Mutex;

/// Trait for managing rollback of state transitions.
pub trait RollbackManager {
    /// Rolls back the current state to the provided previous state.
    ///
    /// # Arguments
    ///
    /// * `previous_state` - The state to revert to.
    ///
    /// # Returns
    ///
    /// * Ok(()) if the rollback succeeds.
    /// * Err(TransitionError) if the rollback fails.
    fn rollback_to(&self, previous_state: State) -> Result<(), TransitionError>;
}

/// Default implementation for rollback management.
/// It holds the current state in a Mutex for concurrency safety.
pub struct DefaultRollbackManager {
    pub current_state: Mutex<State>,
}

impl DefaultRollbackManager {
    /// Creates a new DefaultRollbackManager with an initial state.
    pub fn new(initial: State) -> Self {
        DefaultRollbackManager { current_state: Mutex::new(initial) }
    }
}

impl RollbackManager for DefaultRollbackManager {
    fn rollback_to(&self, previous_state: State) -> Result<(), TransitionError> {
        unimplemented!("DefaultRollbackManager::rollback_to is not implemented yet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollback_to_previous_state() {
        // Initialize the rollback manager with a known state.
        let initial_state = State::Running;
        let manager = DefaultRollbackManager::new(initial_state.clone());

        // Simulate a failed transition by attempting to change state,
        // then rollback to the previous state.
        // For this test, we assume a failure occurred and we wish to revert to Running.
        let rollback_result = manager.rollback_to(State::Running);
        assert!(rollback_result.is_ok(), "Rollback should succeed");

        let current_state = manager.current_state.lock().unwrap();
        assert_eq!(*current_state, State::Running, "State should rollback to Running");
    }

    #[test]
    fn test_rollback_no_change() {
        // Test that rolling back to the current state leaves the state unchanged.
        let initial_state = State::Idle;
        let manager = DefaultRollbackManager::new(initial_state.clone());

        let rollback_result = manager.rollback_to(State::Idle);
        assert!(rollback_result.is_ok(), "Rollback to same state should succeed");

        let current_state = manager.current_state.lock().unwrap();
        assert_eq!(*current_state, State::Idle, "State should remain Idle after rollback");
    }
}

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

use super::state_transitions::{Event, State, TransitionError};
use std::fmt;

/// Error type for state merge failures.
#[derive(Debug, PartialEq)]
pub enum MergeError {
    Conflict(String),
    OperationFailed(String),
    NotImplemented,
}

impl fmt::Display for MergeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeError::Conflict(s) => write!(f, "Merge conflict: {}", s),
            MergeError::OperationFailed(s) => write!(f, "Merge operation failed: {}", s),
            MergeError::NotImplemented => write!(f, "Merge operation not implemented"),
        }
    }
}

impl std::error::Error for MergeError {}

/// The StateMerger trait defines how to merge two instances of a state type.
pub trait StateMerger<T>
where
    T: Clone,
{
    /// Merges two state instances into a single, unified state.
    ///
    /// # Arguments
    ///
    /// * `state_a` - The first state.
    /// * `state_b` - The second state.
    ///
    /// # Returns
    ///
    /// * On success, returns the merged state.
    /// * On failure, returns a MergeError.
    fn merge(&self, state_a: &T, state_b: &T) -> Result<T, MergeError>;
}

/// DefaultStateMerger is the production implementation of StateMerger.
pub struct DefaultStateMerger;

impl DefaultStateMerger {
    /// Creates a new instance of DefaultStateMerger.
    pub fn new() -> Self {
        DefaultStateMerger
    }
}

/// Production implementation of `StateMerger` for the `State` enum.
/// Defines merging rules for system states, prioritizing stability and conflict avoidance.
impl StateMerger<State> for DefaultStateMerger {
    /// Merges two `State` instances according to system rules:
    /// - Same states ➔ Return the state (no-op)
    /// - Error state conflicts ➔ Reject with error
    /// - Idle/Running conflicts ➔ Reject with error
    /// - All other cases ➔ Prefer `state_a` (default)
    ///
    /// # Arguments
    /// - `state_a`: Primary state (priority in non-conflict cases)
    /// - `state_b`: Secondary state
    ///
    /// # Returns
    /// - `Ok(State)`: Merged state or `state_a` clone
    /// - `Err(MergeError::Conflict)`: For unsupported combinations
    fn merge(&self, state_a: &State, state_b: &State) -> Result<State, MergeError> {
        use State::*;
        match (state_a, state_b) {
            // Rule 1: Identical states
            (a, b) if a == b => Ok(a.clone()), // No merging needed

            // Rule 2: Error state dominance
            (Error, _) | (_, Error) => Err(MergeError::Conflict("Cannot merge with Error".into())), // Error states abort merging

            // Rule 3: Idle-Running conflict
            (Idle, Running) | (Running, Idle) => Err(MergeError::Conflict("Idle+Running conflict".into())), // Mutually exclusive states

            // Rule 4: Default behavior
            _ => Ok(state_a.clone()), // Prefer the primary state (`state_a`)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::state_transitions::state_transitions::State;

    // For testing, define a simple state structure.
    #[derive(Clone, Debug, PartialEq)]
    struct TestState {
        counter: i32,
        log: String,
    }

    /// Custom implementation of the `StateMerger` trait for `TestState`.
    /// This handles merging two `TestState` instances with specific rules for testing purposes.
    impl StateMerger<TestState> for DefaultStateMerger {
        /// Merges two `TestState` instances according to test-specific logic:
        /// - **Identical states** return the same state (no-op).
        /// - **Logs** are concatenated with `|`, ignoring empty logs.
        /// - **Counters** are summed.
        ///
        /// # Arguments
        /// - `a`: First `TestState` instance.
        /// - `b`: Second `TestState` instance.
        ///
        /// # Returns
        /// - `Ok(TestState)`: Merged state following the rules above.
        /// - `Err(MergeError)`: Not used in tests, but required by the trait.
        fn merge(&self, a: &TestState, b: &TestState) -> Result<TestState, MergeError> {
            // Rule 1: If both states are identical, return one of them unchanged.
            // This avoids unnecessary operations and ensures test stability.
            if a == b {
                return Ok(a.clone());
            }

            // Rule 2: Merge logs conditionally:
            // - If both logs are empty: Use an empty string.
            // - If one log is empty: Use the non-empty log.
            // - If both are non-empty: Concatenate with `|`.
            let merged_log = match (a.log.is_empty(), b.log.is_empty()) {
                (true, true) => String::new(),                    // Both empty → no log
                (true, false) => b.log.clone(),                   // Only `b` has a log
                (false, true) => a.log.clone(),                   // Only `a` has a log
                (false, false) => format!("{}|{}", a.log, b.log), // Merge logs with delimiter
            };

            // Rule 3: Sum counters to simulate cumulative state changes.
            // This aligns with test expectations (e.g., `test_merge_different_states`).
            Ok(TestState {
                counter: a.counter + b.counter,
                log: merged_log,
            })
        }
    }

    // For demonstration, we will later require the merge function to:
    // - Sum the counters.
    // - Concatenate the logs, separated by a delimiter.
    #[test]
    fn test_merge_identical_states() {
        let state = TestState { counter: 10, log: "log1".into() };
        let merger = DefaultStateMerger::new();
        let merged = merger.merge(&state, &state);
        match merged {
            Ok(result) => {
                // Expect the merged state to be identical to the original since both are same.
                assert_eq!(result, state, "Merging identical states should return the same state");
            }
            Err(_) => panic!("Merging identical states should succeed"),
        }
    }

    #[test]
    fn test_merge_different_states() {
        let state_a = TestState { counter: 5, log: "A".into() };
        let state_b = TestState { counter: 7, log: "B".into() };
        let merger = DefaultStateMerger::new();
        let merged = merger.merge(&state_a, &state_b);
        match merged {
            Ok(result) => {
                // Expected behavior (for demonstration):
                // counter is the sum (12) and log is "A|B"
                // Note: Production logic should define the exact rules.
                let expected = TestState { counter: 12, log: "A|B".into() };
                assert_eq!(result, expected, "Merged state should sum counters and merge logs with a delimiter");
            }
            Err(err) => panic!("Expected successful merge, got error: {:?}", err),
        }
    }

    #[test]
    fn test_merge_is_deterministic() {
        let state_a = TestState { counter: 3, log: "start".into() };
        let state_b = TestState { counter: 4, log: "end".into() };
        let merger = DefaultStateMerger::new();
        let merged_first = merger.merge(&state_a, &state_b).expect("First merge should succeed");
        let merged_second = merger.merge(&state_a, &state_b).expect("Second merge should be identical");
        assert_eq!(merged_first, merged_second, "Merge operations should be deterministic");
    }

    #[test]
    fn test_merge_with_empty_state() {
        let state_a = TestState { counter: 10, log: "non-empty".into() };
        let state_b = TestState { counter: 0, log: "".into() };
        let merger = DefaultStateMerger::new();
        let merged = merger.merge(&state_a, &state_b);
        match merged {
            Ok(result) => {
                // Define expected behavior (for example, if one state is "empty", result should be state_a).
                let expected = state_a.clone();
                assert_eq!(result, expected, "Merging with an empty state should yield the non-empty state");
            }
            Err(_) => panic!("Merging with an empty state should succeed"),
        }
    }
}

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
/// Its merge functionality is left unimplemented to drive TDD.
pub struct DefaultStateMerger;

impl DefaultStateMerger {
    /// Creates a new instance of DefaultStateMerger.
    pub fn new() -> Self {
        DefaultStateMerger
    }
}

impl<T> StateMerger<T> for DefaultStateMerger
where
    T: Clone,
{
    fn merge(&self, _state_a: &T, _state_b: &T) -> Result<T, MergeError> {
        unimplemented!("DefaultStateMerger::merge is not implemented yet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // For testing, define a simple state structure.
    #[derive(Clone, Debug, PartialEq)]
    struct TestState {
        counter: i32,
        log: String,
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

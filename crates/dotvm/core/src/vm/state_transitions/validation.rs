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

/// Validates if a transition is allowed based on the current state and event.
///
/// # Returns
/// * Ok(()) if the transition is allowed,
/// * Err(TransitionError::InvalidTransition) otherwise.
///
/// Allowed transitions (example):
/// - Idle + Start   -> Running
/// - Running + Pause -> Paused
/// - Paused + Resume -> Running
/// - Running + Stop   -> Idle
pub fn validate_transition(current: &State, event: &Event) -> Result<(), TransitionError> {
    // TDD: Add pattern matching to verify allowed transitions.
    unimplemented!("transition_validation::validate_transition is not implemented yet")
}

/// Checks that state invariants hold within the current state.
///
/// # Returns
/// * Ok(()) if invariants hold,
/// * Err(TransitionError) if any invariant is violated.
pub fn check_state_invariants(current: &State) -> Result<(), TransitionError> {
    // TDD: Validate overall state properties.
    unimplemented!("transition_validation::check_state_invariants is not implemented yet")
}

/// (Optional) Validates that the provided event is recognized by the system.
///
/// # Returns
/// * Ok(()) if the event is valid,
/// * Err(TransitionError) if the event is unsupported.
pub fn validate_event(event: &Event) -> Result<(), TransitionError> {
    // TDD: Check if the event belongs to our enum or known set.
    unimplemented!("transition_validation::validate_event is not implemented yet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transition_idle_to_running() {
        let current = State::Idle;
        let event = Event::Start;
        let result = validate_transition(&current, &event);
        assert!(result.is_ok(), "Transition from Idle with Start event should be valid");
    }

    #[test]
    fn test_valid_transition_running_to_paused() {
        let current = State::Running;
        let event = Event::Pause;
        let result = validate_transition(&current, &event);
        assert!(result.is_ok(), "Transition from Running with Pause event should be valid");
    }

    #[test]
    fn test_invalid_transition_paused_to_start() {
        let current = State::Paused;
        let event = Event::Start; // Assuming Start is not allowed from Paused.
        let result = validate_transition(&current, &event);
        assert!(result.is_err(), "Transition from Paused with a Start event should be invalid");
        if let Err(TransitionError::InvalidTransition) = result {
            // Expected error.
        } else {
            panic!("Expected an InvalidTransition error");
        }
    }

    #[test]
    fn test_invalid_transition_from_error_state() {
        let current = State::Error;
        let event = Event::Resume;
        let result = validate_transition(&current, &event);
        assert!(result.is_err(), "Any transition attempted from Error state should be invalid");
    }

    #[test]
    fn test_unknown_or_invalid_event() {
        // If there is an event not supported by the validation logic, it should fail.
        let current = State::Idle;
        let event = Event::Stop; // Assuming Stop is not valid in Idle.
        let result = validate_transition(&current, &event);
        assert!(result.is_err(), "Transition with an invalid event should be rejected");
    }

    #[test]
    fn test_state_invariants_check() {
        // Test that the invariants check passes for a valid state.
        let current = State::Running;
        let result = check_state_invariants(&current);
        assert!(result.is_ok(), "Invariants for Running state should hold");
    }

    #[test]
    fn test_event_validation() {
        // Test that the event validation function recognizes valid events.
        let event = Event::Start;
        let result = validate_event(&event);
        assert!(result.is_ok(), "Event Start should be recognized as valid");
    }
}

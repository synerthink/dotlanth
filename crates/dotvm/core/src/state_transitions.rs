// state_transitions.rs
//
// This module implements state transition logic for DOTVM.
// It defines the possible states, provides functionality to process events,
// perform state transitions with concurrency control, and logs each transition event.

use std::sync::{Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use log::{info, error};

/// Represents all valid states in the system.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Initialized,
    Running,
    Paused,
    Terminated,
    Error,
}

/// Custom error type for transition failures.
#[derive(Debug)]
pub enum TransitionError {
    InvalidTransition(String),
    StateLockError(String),
    Other(String),
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionError::InvalidTransition(msg) => write!(f, "Invalid transition: {}", msg),
            TransitionError::StateLockError(msg) => write!(f, "State lock error: {}", msg),
            TransitionError::Other(msg) => write!(f, "Transition error: {}", msg),
        }
    }
}

impl std::error::Error for TransitionError {}

/// Events that can trigger state transitions.
#[derive(Debug)]
pub enum Event {
    Start,
    Pause,
    Resume,
    Stop,
    Reset,
    Custom(String),
}

/// Struct to record a state transition along with metadata.
#[derive(Debug)]
pub struct StateTransition {
    pub current_state: State,
    pub target_state: State,
    pub timestamp: u64,  // UNIX timestamp in seconds
    pub event: Event,
}

/// Manages state transitions with thread safety.
pub struct StateManager {
    state: Mutex<State>,
}

impl StateManager {
    /// Creates a new StateManager with the given initial state.
    pub fn new(initial_state: State) -> Self {
        StateManager {
            state: Mutex::new(initial_state),
        }
    }

    /// Processes an event to trigger the appropriate state transition.
    ///
    /// # Arguments
    ///
    /// * `event` - A reference to the event that triggers a transition.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transition is successful.
    /// * `Err(TransitionError)` if the transition is invalid or a locking error occurs.
    pub fn process_event(&self, event: &Event) -> Result<(), TransitionError> {
        // Determine the new state based on the event.
        let new_state = match event {
            Event::Start => State::Running,
            Event::Pause => State::Paused,
            Event::Resume => State::Running,
            Event::Stop => State::Terminated,
            Event::Reset => State::Initialized,
            Event::Custom(s) => {
                // Custom events are not handled in this basic implementation.
                return Err(TransitionError::InvalidTransition(format!("Custom event '{}' is not handled", s)));
            }
        };

        info!("Processing event: {:?}, attempting to transition to state: {:?}", event, new_state);
        self.change_state(new_state)
    }

    /// Changes the current state to a new state after validating the transition.
    ///
    /// # Arguments
    ///
    /// * `new_state` - The target state to transition into.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the state change is valid and applied.
    /// * `Err(TransitionError)` if the transition is invalid or if a state lock error occurs.
    pub fn change_state(&self, new_state: State) -> Result<(), TransitionError> {
        let mut current_state = self.state.lock().map_err(|_| TransitionError::StateLockError("Failed to acquire state lock".to_string()))?;
        if !Self::is_valid_transition(&current_state, &new_state) {
            error!("Invalid state transition attempted from {:?} to {:?}", *current_state, new_state);
            // Rollback logic can be added here if needed.
            return Err(TransitionError::InvalidTransition(format!("Cannot transition from {:?} to {:?}", *current_state, new_state)));
        }
        info!("Transitioning from {:?} to {:?}", *current_state, new_state);
        *current_state = new_state;
        Ok(())
    }
    
    /// Validates whether a transition from one state to another is allowed.
    fn is_valid_transition(from: &State, to: &State) -> bool {
        match (from, to) {
            // Define allowed transitions.
            (State::Initialized, State::Running) => true,
            (State::Running, State::Paused) => true,
            (State::Paused, State::Running) => true,
            (State::Running, State::Terminated) => true,
            (State::Paused, State::Terminated) => true,
            (State::Terminated, State::Initialized) => true,
            (State::Error, State::Initialized) => true,
            // Allow staying in the same state.
            (s1, s2) if s1 == s2 => true,
            _ => false,
        }
    }

    /// Retrieves the current state.
    pub fn get_state(&self) -> Result<State, TransitionError> {
        let state = self.state.lock().map_err(|_| TransitionError::StateLockError("Failed to acquire state lock".to_string()))?;
        Ok(state.clone())
    }
}

/// Returns the current UNIX timestamp in seconds.
pub fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// Creates a new state transition record.
///
/// # Arguments
///
/// * `current` - The current state.
/// * `target` - The target state.
/// * `event` - The event that triggered the transition.
pub fn create_transition(current: State, target: State, event: Event) -> StateTransition {
    StateTransition {
        current_state: current,
        target_state: target,
        timestamp: current_timestamp(),
        event,
    }
}

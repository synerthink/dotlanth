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

use crate::finalizer::finality_confirmation::FinalityConfirmation;
use crate::finalizer::finality_validation::FinalityValidator;
use crate::finalizer::lib::FinalityError;
use crate::finalizer::lib::FinalityResult;
use crate::finalizer::lib::{FinalityStatus, State, StateTransition};
use crate::finalizer::logging_audit::AuditLogger;
use dashmap::DashMap;
use futures::StreamExt;
use ring::signature::Ed25519KeyPair;
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

/// Core component for instant finality protocol implementing immediate state transition processing
pub struct InstantFinalityModule {
    validator: Arc<FinalityValidator>,               // Validation rules executor
    logger: Arc<AuditLogger>,                        // Audit trail recorder
    state_store: Arc<Mutex<DashMap<String, State>>>, // Thread-safe state storage
    signing_key: RwLock<Option<Ed25519KeyPair>>,     // Cryptographic signing key
}

impl InstantFinalityModule {
    /// Initialize with validator, logger, and genesis state
    pub async fn new(validator: Arc<FinalityValidator>, logger: Arc<AuditLogger>, initial_state: State) -> Self {
        // Insert initial state using async mutex
        let state_store = Arc::new(Mutex::new(DashMap::new()));

        {
            let state_store_locked = state_store.lock().await;
            state_store_locked.insert("current".to_string(), initial_state);
        }

        Self {
            validator,
            logger,
            state_store,
            signing_key: RwLock::new(None),
        }
    }

    /// Load cryptographic signing key from raw bytes
    pub async fn initialize_signing_key(&self, pkcs8_bytes: Vec<u8>) {
        let sk = Ed25519KeyPair::from_pkcs8(&pkcs8_bytes).unwrap();
        *self.signing_key.write().await = Some(sk);
    }

    /// Main transition processing pipeline
    pub async fn process_transition(&self, transition: StateTransition) -> FinalityResult<FinalityConfirmation> {
        // Log the received state transition proposal
        self.logger.log_transition_proposal(&transition);

        // Step 1: Validate the state transition
        let validation_result = self.validator.validate_transition(&transition);

        // Log validation result
        self.logger.log_validation_result(&transition, &validation_result);

        // Step 2: Process based on validation result
        if validation_result.is_valid {
            // Step 3: If validation passes, finalize the state transition
            self.finalize_transition(transition).await
        } else {
            // If validation fails, return error
            let error_msg = validation_result.error_message.unwrap_or_else(|| "Unknown validation error".to_string());

            self.logger.log_finalization_failure(&transition, &error_msg);
            Err(FinalityError::Validation(error_msg))
        }
    }

    /// Atomic state transition finalization
    pub async fn finalize_transition(&self, transition: StateTransition) -> FinalityResult<FinalityConfirmation> {
        let state_store = self.state_store.lock().await; // Mutex ile state_store'a erişim

        // Verify state version consistency
        let current = state_store
            .get("current")
            .ok_or_else(|| FinalityError::Internal(Error::other("State not initialized")))?
            .value()
            .clone();

        // Critical version check prevents conflicting transitions
        if transition.state_before.version != current.version || transition.state_before.data != current.data {
            return Err(FinalityError::Validation("State version or data mismatch - state has been updated".to_string()));
        }

        // Update global state
        state_store.insert("current".to_string(), transition.state_after.clone());

        // Generate confirmation with optional cryptographic signature
        let mut conf = FinalityConfirmation::new(transition.clone(), FinalityStatus::Finalized, "Transition successfully finalized");
        if let Some(sk) = &*self.signing_key.read().await {
            conf.add_signature(sk); // Attach Ed25519 signature
        }

        self.logger.log_finalization_success(&transition, &conf);
        Ok(conf)
    }

    /// Get current state
    pub async fn get_current_state(&self) -> Option<State> {
        let state_store = self.state_store.lock().await;
        state_store.get("current").map(|entry| entry.value().clone())
    }

    /// Set current state
    pub async fn set_current_state(&self, state: State) {
        let state_store = self.state_store.lock().await;
        state_store.insert("current".to_string(), state);
    }

    /// Check if a transition has been finalized (look in state history)
    pub async fn is_transition_finalized(&self, transition_id: &str) -> bool {
        // Implementation can be expanded to check a historical record
        // For now, we'll just check if the current state resulted from this transition
        if let Some(current) = self.get_current_state().await {
            // In a real implementation, we'd check a transaction history
            // This is a placeholder check and should be replaced with actual implementation
            transition_id.contains(&current.data)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finalizer::lib::{State, TransitionMetadata, generate_unique_id};
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::runtime::Runtime;

    fn create_initial_state() -> State {
        State {
            data: "genesis".to_string(),
            version: 0,
        }
    }

    // Utility function to get current timestamp in milliseconds
    fn get_current_timestamp() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
    }

    // Create a transition based on the current state
    async fn create_transition_from_current(module: &InstantFinalityModule, new_data: String) -> StateTransition {
        let current_state = module.get_current_state().await.unwrap();

        let mut transition = StateTransition::new(
            generate_unique_id("trans"),
            current_state.clone(), // Use the ACTUAL current state
            State {
                data: new_data,
                version: current_state.version + 1,
            },
            TransitionMetadata {
                initiator: "test_user".to_string(),
                reason: "test_reason".to_string(),
                additional_info: None,
            },
        );

        // Set a valid timestamp
        transition.timestamp = get_current_timestamp();

        transition
    }

    /// End-to-end test of transition lifecycle
    #[test]
    fn test_state_transition_flow() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Setup module with initial state
            let initial_state = create_initial_state();
            let validator = Arc::new(FinalityValidator::new()); // Using default validator
            let module = InstantFinalityModule::new(validator, Arc::new(AuditLogger::new()), initial_state.clone()).await;

            // Generate PKCS#8 and initialize module with raw bytes
            let rng = SystemRandom::new();
            let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
            let pkcs8_bytes = pkcs8_doc.as_ref().to_vec();
            module.initialize_signing_key(pkcs8_bytes.clone()).await;

            // Create transition using current state
            let transition = create_transition_from_current(&module, "state_1".to_string()).await;

            let result = module.process_transition(transition.clone()).await;

            // Verify results
            assert!(result.is_ok());
            let confirmation = result.unwrap();
            assert!(confirmation.is_valid());

            // Check state update
            let new_state = module.get_current_state().await.unwrap();
            assert_eq!(new_state.version, 1);
            assert_eq!(new_state.data, "state_1");

            // Verify signature
            let sk = Ed25519KeyPair::from_pkcs8(&pkcs8_bytes).unwrap();
            let vk = sk.public_key();
            assert!(confirmation.verify_signature(&vk));
        });
    }

    /// Test using custom validator with strict parameters
    #[test]
    fn test_with_custom_validator() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Create test dependencies with custom validator
            let validator = Arc::new(FinalityValidator::with_config(
                10,                                                 // min_timestamp_delta - using a low value for tests
                true,                                               // strict_version_increment
                vec!["test_user".to_string(), "admin".to_string()], // authorized_initiators
            ));
            let logger = Arc::new(AuditLogger::new());
            let initial_state = create_initial_state();

            let module = InstantFinalityModule::new(validator, logger, initial_state).await;

            // Create transition using actual current state
            let transition = create_transition_from_current(&module, "state_1".to_string()).await;

            // Process transition
            let result = module.process_transition(transition.clone()).await;

            // Verify successful processing
            assert!(result.is_ok(), "First transition should succeed. Error: {:?}", result.err());
            let confirmation = result.unwrap();
            assert_eq!(confirmation.status, FinalityStatus::Finalized);

            // Create another transition with unauthorized user
            let current_state = module.get_current_state().await.unwrap();
            let mut invalid_transition = StateTransition::new(
                generate_unique_id("trans"),
                current_state.clone(),
                State {
                    data: "state_2".to_string(),
                    version: current_state.version + 1,
                },
                TransitionMetadata {
                    initiator: "unauthorized_user".to_string(), // Unauthorized user
                    reason: "test_reason".to_string(),
                    additional_info: None,
                },
            );
            invalid_transition.timestamp = get_current_timestamp();

            // Process invalid transition
            let result = module.process_transition(invalid_transition).await;

            // Verify processing failed due to validation
            assert!(result.is_err());
            if let Err(FinalityError::Validation(err)) = result {
                assert!(err.contains("not authorized"), "Expected 'not authorized' error, got: {}", err);
            } else {
                panic!("Expected validation error");
            }
        });
    }

    /// Stress test for concurrent transition attempts
    #[test]
    fn test_concurrent_transitions() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let initial_state = create_initial_state();
            let validator = Arc::new(FinalityValidator::with_config(
                10,                            // min_timestamp_delta - using a low value for tests
                true,                          // strict_version_increment
                vec!["test_user".to_string()], // authorized_initiators
            ));
            let module: Arc<InstantFinalityModule> = Arc::new(InstantFinalityModule::new(validator, Arc::new(AuditLogger::new()), initial_state).await);

            let mut futures = FuturesUnordered::new();

            for i in 0..10 {
                let module = module.clone();
                futures.push(async move {
                    // Each task gets the current state each time
                    let current_state = module.get_current_state().await.unwrap();

                    // Add a small delay to simulate concurrent operations
                    tokio::time::sleep(tokio::time::Duration::from_millis(i * 5)).await;

                    // Create transition based on the current state we just fetched
                    let mut transition = StateTransition::new(
                        generate_unique_id("trans"),
                        current_state.clone(),
                        State {
                            data: format!("concurrent_state_{}", i),
                            version: current_state.version + 1,
                        },
                        TransitionMetadata {
                            initiator: "test_user".to_string(),
                            reason: "test_reason".to_string(),
                            additional_info: None,
                        },
                    );
                    transition.timestamp = get_current_timestamp();

                    // Attempt to process the transition
                    let result = module.process_transition(transition).await;
                    result.is_ok() // Return true if successful
                });
            }

            // Collect the results of all tasks
            let mut success_count = 0;
            while let Some(result) = futures.next().await {
                if result {
                    success_count += 1;
                }
            }

            // Only one transition should succeed
            assert_eq!(success_count, 1, "Only one transition should succeed");
        });
    }

    #[test]
    fn test_duplicate_transition_processing() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Setup module with initial state
            let initial_state = create_initial_state();
            let validator = Arc::new(FinalityValidator::with_config(
                10,                            // min_timestamp_delta
                true,                          // strict_version_increment
                vec!["test_user".to_string()], // authorized_initiators
            ));
            let module = InstantFinalityModule::new(validator, Arc::new(AuditLogger::new()), initial_state.clone()).await;

            // Create transition using the actual current state
            let transition = create_transition_from_current(&module, "state_1".to_string()).await;

            // Process transition first time
            let result1 = module.process_transition(transition.clone()).await;
            assert!(result1.is_ok(), "First transition should succeed. Error: {:?}", result1.err());

            // Process same transition second time - should fail because state has been updated
            let result2 = module.process_transition(transition.clone()).await;

            // Verify processing failed due to state mismatch
            assert!(result2.is_err());
            if let Err(FinalityError::Validation(err)) = result2 {
                assert!(err.contains("state has been updated"), "Expected 'state has been updated' error, got: {}", err);
            } else {
                panic!("Expected validation error, got: {:?}", result2);
            }
        });
    }
}

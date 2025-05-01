use crate::rollback::lib::{LogLevel, RollbackError, RollbackResult, SystemState, log_event};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Result of a verification operation
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// Verification passed
    Valid,
    /// Verification failed with specific error message
    Invalid(String),
}

/// Represents a verification check for system consistency
#[derive(Clone)]
pub struct VerificationCheck {
    /// Name or identifier for the check
    pub name: String,
    /// Function that performs the verification check
    pub check_fn: Arc<dyn Fn(&SystemState) -> VerificationResult + Send + Sync>,
    /// Indicates if this check is critical (system cannot continue if it fails)
    pub is_critical: bool,
}

/// Trait that defines verification capabilities
pub trait ConsistencyVerifier {
    /// Adds a verification check
    fn add_verification_check(&mut self, name: &str, check_fn: impl Fn(&SystemState) -> VerificationResult + Send + Sync + 'static, is_critical: bool) -> RollbackResult<()>;

    /// Verifies system consistency
    fn verify_consistency(&self, state: &SystemState) -> VerificationResult;

    /// Gets all verification checks
    fn get_verification_checks(&self) -> Vec<String>;

    /// Runs a specific verification check
    fn run_verification_check(&self, name: &str, state: &SystemState) -> RollbackResult<VerificationResult>;
}

/// Default implementation of ConsistencyVerifier
pub struct DefaultConsistencyVerifier {
    verification_checks: Mutex<HashMap<String, VerificationCheck>>,
}

impl DefaultConsistencyVerifier {
    /// Creates a new DefaultConsistencyVerifier
    pub fn new() -> Self {
        Self {
            verification_checks: Mutex::new(HashMap::new()),
        }
    }

    /// Gets all verification check details
    pub fn get_check_details(&self) -> HashMap<String, bool> {
        let checks = self.verification_checks.lock().unwrap();
        checks.iter().map(|(name, check)| (name.clone(), check.is_critical)).collect()
    }
}

impl Default for DefaultConsistencyVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsistencyVerifier for DefaultConsistencyVerifier {
    fn add_verification_check(&mut self, name: &str, check_fn: impl Fn(&SystemState) -> VerificationResult + Send + Sync + 'static, is_critical: bool) -> RollbackResult<()> {
        let mut checks = self
            .verification_checks
            .lock()
            .map_err(|_| RollbackError::VerificationFailed("Failed to acquire verification checks lock".to_string()))?;

        let check = VerificationCheck {
            name: name.to_string(),
            check_fn: Arc::new(check_fn),
            is_critical,
        };

        checks.insert(name.to_string(), check);
        log_event(
            LogLevel::Info,
            "ConsistencyVerifier",
            &format!("Added {} verification check: {}", if is_critical { "critical" } else { "non-critical" }, name),
        );

        Ok(())
    }

    fn verify_consistency(&self, state: &SystemState) -> VerificationResult {
        log_event(LogLevel::Info, "ConsistencyVerifier", "Starting system consistency verification");

        let checks = match self.verification_checks.lock() {
            Ok(checks) => checks,
            Err(_) => {
                let error_msg = "Failed to acquire verification checks lock".to_string();
                log_event(LogLevel::Error, "ConsistencyVerifier", &error_msg);
                return VerificationResult::Invalid(error_msg);
            }
        };

        if checks.is_empty() {
            log_event(LogLevel::Warning, "ConsistencyVerifier", "No verification checks registered");
            return VerificationResult::Valid;
        }

        let mut failures = Vec::new();

        // Run all checks and collect failures
        for (name, check) in checks.iter() {
            match (check.check_fn)(state) {
                VerificationResult::Valid => {
                    log_event(LogLevel::Info, "ConsistencyVerifier", &format!("Check '{}' passed", name));
                }
                VerificationResult::Invalid(reason) => {
                    let failure_msg = format!("Check '{}' failed: {}", name, reason);
                    log_event(LogLevel::Warning, "ConsistencyVerifier", &failure_msg);

                    if check.is_critical {
                        log_event(LogLevel::Error, "ConsistencyVerifier", &format!("Critical check '{}' failed, verification aborted", name));
                        return VerificationResult::Invalid(failure_msg);
                    }

                    failures.push(failure_msg);
                }
            }
        }

        if failures.is_empty() {
            log_event(LogLevel::Info, "ConsistencyVerifier", "All verification checks passed");
            VerificationResult::Valid
        } else {
            let failure_summary = format!("{} verification checks failed: {}", failures.len(), failures.join("; "));
            log_event(LogLevel::Warning, "ConsistencyVerifier", &failure_summary);
            VerificationResult::Invalid(failure_summary)
        }
    }

    fn get_verification_checks(&self) -> Vec<String> {
        match self.verification_checks.lock() {
            Ok(checks) => checks.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    fn run_verification_check(&self, name: &str, state: &SystemState) -> RollbackResult<VerificationResult> {
        let checks = self
            .verification_checks
            .lock()
            .map_err(|_| RollbackError::VerificationFailed("Failed to acquire verification checks lock".to_string()))?;

        let check = checks.get(name).ok_or_else(|| RollbackError::VerificationFailed(format!("Verification check '{}' not found", name)))?;

        log_event(LogLevel::Info, "ConsistencyVerifier", &format!("Running verification check: {}", name));

        let result = (check.check_fn)(state);

        match &result {
            VerificationResult::Valid => {
                log_event(LogLevel::Info, "ConsistencyVerifier", &format!("Check '{}' passed", name));
            }
            VerificationResult::Invalid(reason) => {
                log_event(LogLevel::Warning, "ConsistencyVerifier", &format!("Check '{}' failed: {}", name, reason));
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_verification_check() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add test check
        let result = verifier.add_verification_check("test_check", |_state| VerificationResult::Valid, true);

        assert!(result.is_ok());

        // Verify check was added
        let checks = verifier.get_verification_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0], "test_check");
    }

    #[test]
    fn test_verify_consistency_all_valid() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add test checks
        verifier.add_verification_check("check1", |_state| VerificationResult::Valid, false).unwrap();

        verifier.add_verification_check("check2", |_state| VerificationResult::Valid, true).unwrap();

        // Test verification
        let state = SystemState::new();
        let result = verifier.verify_consistency(&state);

        assert_eq!(result, VerificationResult::Valid);
    }

    #[test]
    fn test_verify_consistency_non_critical_failure() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add test checks
        verifier.add_verification_check("check1", |_state| VerificationResult::Valid, false).unwrap();

        verifier
            .add_verification_check(
                "check2",
                |_state| VerificationResult::Invalid("Test failure".to_string()),
                false, // Non-critical
            )
            .unwrap();

        // Test verification
        let state = SystemState::new();
        let result = verifier.verify_consistency(&state);

        match result {
            VerificationResult::Invalid(msg) => {
                assert!(msg.contains("check2"));
                assert!(msg.contains("Test failure"));
            }
            _ => panic!("Expected Invalid result"),
        }
    }

    #[test]
    fn test_verify_consistency_critical_failure() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add test checks - first valid, second critical and invalid
        verifier.add_verification_check("check1", |_state| VerificationResult::Valid, false).unwrap();

        verifier
            .add_verification_check(
                "check2",
                |_state| VerificationResult::Invalid("Critical test failure".to_string()),
                true, // Critical check
            )
            .unwrap();

        verifier
            .add_verification_check("check3", |_state| VerificationResult::Invalid("This check shouldn't run".to_string()), false)
            .unwrap();

        // Test verification
        let state = SystemState::new();
        let result = verifier.verify_consistency(&state);

        // Should immediately fail on critical check
        match result {
            VerificationResult::Invalid(msg) => {
                assert!(msg.contains("check2"));
                assert!(msg.contains("Critical test failure"));
                // Shouldn't contain check3 failure since verification stops at first critical failure
                assert!(!msg.contains("check3"));
            }
            _ => panic!("Expected Invalid result due to critical check failure"),
        }
    }

    #[test]
    fn test_run_verification_check() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add test checks
        verifier.add_verification_check("valid_check", |_state| VerificationResult::Valid, false).unwrap();

        verifier
            .add_verification_check("invalid_check", |_state| VerificationResult::Invalid("Test failure reason".to_string()), true)
            .unwrap();

        // Test running specific checks
        let state = SystemState::new();

        // Test valid check
        let valid_result = verifier.run_verification_check("valid_check", &state).unwrap();
        assert_eq!(valid_result, VerificationResult::Valid);

        // Test invalid check
        let invalid_result = verifier.run_verification_check("invalid_check", &state).unwrap();
        match invalid_result {
            VerificationResult::Invalid(msg) => {
                assert_eq!(msg, "Test failure reason");
            }
            _ => panic!("Expected Invalid result"),
        }

        // Test non-existent check
        let non_existent_result = verifier.run_verification_check("non_existent", &state);
        assert!(non_existent_result.is_err());
    }

    #[test]
    fn test_get_check_details() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add test checks with different criticality
        verifier.add_verification_check("critical_check", |_state| VerificationResult::Valid, true).unwrap();

        verifier.add_verification_check("non_critical_check", |_state| VerificationResult::Valid, false).unwrap();

        // Get and verify check details
        let check_details = verifier.get_check_details();

        assert_eq!(check_details.len(), 2);
        assert_eq!(check_details.get("critical_check"), Some(&true));
        assert_eq!(check_details.get("non_critical_check"), Some(&false));
    }

    #[test]
    fn test_empty_verifier() {
        // Setup
        let verifier = DefaultConsistencyVerifier::new();
        let state = SystemState::new();

        // Test verification with no checks
        let result = verifier.verify_consistency(&state);
        assert_eq!(result, VerificationResult::Valid);
    }

    #[test]
    fn test_verification_with_state_dependent_checks() {
        // Setup
        let mut verifier = DefaultConsistencyVerifier::new();

        // Add check that examines state
        verifier
            .add_verification_check(
                "state_check",
                |state| {
                    if state.is_empty() {
                        VerificationResult::Valid
                    } else {
                        VerificationResult::Invalid("State should be empty".to_string())
                    }
                },
                false,
            )
            .unwrap();

        // Test with different states
        let empty_state = SystemState::new();
        let mut non_empty_state = SystemState::new();
        non_empty_state.insert("test_key".to_string(), "test_value".as_bytes().to_vec());

        let empty_result = verifier.verify_consistency(&empty_state);
        assert_eq!(empty_result, VerificationResult::Valid);

        let non_empty_result = verifier.verify_consistency(&non_empty_state);
        match non_empty_result {
            VerificationResult::Invalid(msg) => {
                assert!(msg.contains("State should be empty"));
            }
            _ => panic!("Expected Invalid result"),
        }
    }
}

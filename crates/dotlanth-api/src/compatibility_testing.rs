// Dotlanth
// Copyright (C) 2025 Synerthink

//! Compatibility testing automation for API versioning

use crate::versioning::{ApiVersion, CompatibilityChecker, DeprecationManager, ProtocolType, SchemaEvolutionManager, ServiceType, VersionRegistry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;
use tokio::time::{Duration, timeout};

/// Compatibility testing errors
#[derive(Error, Debug)]
pub enum CompatibilityTestError {
    #[error("Test execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Test timeout: {0}")]
    Timeout(String),
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
    #[error("Test setup failed: {0}")]
    SetupFailed(String),
}

/// Test case for compatibility validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityTestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub protocol: ProtocolType,
    pub service: ServiceType,
    pub from_version: ApiVersion,
    pub to_version: ApiVersion,
    pub test_data: TestData,
    pub expected_result: ExpectedResult,
    pub timeout_seconds: u64,
}

/// Test data for compatibility tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestData {
    pub request_data: Value,
    pub response_data: Option<Value>,
    pub features_used: Vec<String>,
    pub headers: HashMap<String, String>,
}

/// Expected result for compatibility tests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedResult {
    pub should_succeed: bool,
    pub expected_warnings: Vec<String>,
    pub expected_errors: Vec<String>,
    pub expected_transformations: Option<Value>,
}

/// Test execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub details: HashMap<String, Value>,
}

/// Test suite for automated compatibility testing
#[derive(Debug, Clone)]
pub struct CompatibilityTestSuite {
    test_cases: Vec<CompatibilityTestCase>,
    version_registry: VersionRegistry,
    compatibility_checker: CompatibilityChecker,
    schema_manager: SchemaEvolutionManager,
    deprecation_manager: DeprecationManager,
}

impl CompatibilityTestSuite {
    /// Create a new test suite
    pub fn new(version_registry: VersionRegistry, compatibility_checker: CompatibilityChecker, schema_manager: SchemaEvolutionManager, deprecation_manager: DeprecationManager) -> Self {
        let mut suite = Self {
            test_cases: Vec::new(),
            version_registry,
            compatibility_checker,
            schema_manager,
            deprecation_manager,
        };

        suite.initialize_default_tests();
        suite
    }

    /// Add a test case to the suite
    pub fn add_test_case(&mut self, test_case: CompatibilityTestCase) {
        self.test_cases.push(test_case);
    }

    /// Run all compatibility tests
    pub async fn run_all_tests(&mut self) -> Vec<TestResult> {
        let mut results = Vec::new();

        let test_cases = self.test_cases.clone();
        for test_case in &test_cases {
            let result = self.run_test_case(test_case).await;
            results.push(result);
        }

        results
    }

    /// Run a specific test case
    pub async fn run_test_case(&mut self, test_case: &CompatibilityTestCase) -> TestResult {
        let start_time = std::time::Instant::now();

        // Run test with timeout
        let test_future = self.execute_test_case(test_case);
        let result = match timeout(Duration::from_secs(test_case.timeout_seconds), test_future).await {
            Ok(result) => result,
            Err(_) => {
                return TestResult {
                    test_id: test_case.id.clone(),
                    success: false,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    warnings: Vec::new(),
                    errors: vec![format!("Test timed out after {} seconds", test_case.timeout_seconds)],
                    details: HashMap::new(),
                };
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(mut test_result) => {
                test_result.execution_time_ms = execution_time;
                test_result
            }
            Err(e) => TestResult {
                test_id: test_case.id.clone(),
                success: false,
                execution_time_ms: execution_time,
                warnings: Vec::new(),
                errors: vec![e.to_string()],
                details: HashMap::new(),
            },
        }
    }

    /// Execute individual test case
    async fn execute_test_case(&mut self, test_case: &CompatibilityTestCase) -> Result<TestResult, CompatibilityTestError> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let mut details = HashMap::new();
        let mut success = true;

        // Test 1: Version compatibility check
        let compatibility_result = self
            .compatibility_checker
            .check_compatibility(&test_case.protocol, &test_case.service, &test_case.from_version, &test_case.to_version);

        details.insert("compatibility_check".to_string(), serde_json::to_value(&compatibility_result).unwrap());

        if !compatibility_result.is_compatible && test_case.expected_result.should_succeed {
            errors.push("Expected compatibility but versions are incompatible".to_string());
            success = false;
        }

        if compatibility_result.is_compatible && !test_case.expected_result.should_succeed {
            errors.push("Expected incompatibility but versions are compatible".to_string());
            success = false;
        }

        // Test 2: Schema validation
        if let Err(e) = self
            .schema_manager
            .validate_data(&test_case.protocol, &test_case.service, &test_case.from_version, &test_case.test_data.request_data)
        {
            if test_case.expected_result.should_succeed {
                errors.push(format!("Schema validation failed: {}", e));
                success = false;
            } else {
                details.insert("expected_schema_failure".to_string(), Value::String(e.to_string()));
            }
        }

        // Test 3: Data transformation
        if test_case.from_version != test_case.to_version {
            match self.schema_manager.transform_data(
                &test_case.protocol,
                &test_case.service,
                &test_case.from_version,
                &test_case.to_version,
                &test_case.test_data.request_data,
            ) {
                Ok(transformed) => {
                    details.insert("transformed_data".to_string(), transformed);
                }
                Err(e) => {
                    if test_case.expected_result.should_succeed {
                        errors.push(format!("Data transformation failed: {}", e));
                        success = false;
                    }
                }
            }
        }

        // Test 4: Feature compatibility
        if let Err(e) = self
            .compatibility_checker
            .validate_request_compatibility(&test_case.protocol, &test_case.service, &test_case.to_version, &test_case.test_data.features_used)
        {
            if test_case.expected_result.should_succeed {
                errors.push(format!("Feature compatibility check failed: {}", e));
                success = false;
            }
        }

        // Test 5: Deprecation warnings
        let deprecation_warnings = self
            .deprecation_manager
            .generate_warnings(&test_case.protocol, &test_case.service, &test_case.to_version, &test_case.test_data.features_used);

        if !deprecation_warnings.is_empty() {
            warnings.extend(deprecation_warnings);
        }

        // Validate expected warnings and errors
        for expected_warning in &test_case.expected_result.expected_warnings {
            if !warnings.iter().any(|w| w.contains(expected_warning)) {
                errors.push(format!("Expected warning not found: {}", expected_warning));
                success = false;
            }
        }

        for expected_error in &test_case.expected_result.expected_errors {
            if !errors.iter().any(|e| e.contains(expected_error)) {
                errors.push(format!("Expected error not found: {}", expected_error));
                success = false;
            }
        }

        Ok(TestResult {
            test_id: test_case.id.clone(),
            success,
            execution_time_ms: 0, // Will be set by caller
            warnings,
            errors,
            details,
        })
    }

    /// Run tests for a specific protocol/service combination
    pub async fn run_tests_for_service(&mut self, protocol: &ProtocolType, service: &ServiceType) -> Vec<TestResult> {
        let mut results = Vec::new();

        let test_cases = self.test_cases.clone();
        for test_case in &test_cases {
            if test_case.protocol == *protocol && test_case.service == *service {
                let result = self.run_test_case(test_case).await;
                results.push(result);
            }
        }

        results
    }

    /// Generate test report
    pub fn generate_report(&self, results: &[TestResult]) -> CompatibilityTestReport {
        let total_tests = results.len();
        let passed_tests = results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;

        let avg_execution_time = if total_tests > 0 {
            results.iter().map(|r| r.execution_time_ms).sum::<u64>() / total_tests as u64
        } else {
            0
        };

        let all_warnings: Vec<_> = results.iter().flat_map(|r| r.warnings.iter()).cloned().collect();

        let all_errors: Vec<_> = results.iter().flat_map(|r| r.errors.iter()).cloned().collect();

        CompatibilityTestReport {
            total_tests,
            passed_tests,
            failed_tests,
            avg_execution_time_ms: avg_execution_time,
            warnings: all_warnings,
            errors: all_errors,
            test_results: results.to_vec(),
        }
    }

    /// Initialize default test cases
    fn initialize_default_tests(&mut self) {
        let v1_0_0 = ApiVersion::new(1, 0, 0);
        let v1_1_0 = ApiVersion::new(1, 1, 0);
        let v2_0_0 = ApiVersion::new(2, 0, 0);

        // Test 1: Forward compatibility (1.0.0 -> 1.1.0)
        self.add_test_case(CompatibilityTestCase {
            id: "vm_forward_compatibility".to_string(),
            name: "VM Service Forward Compatibility".to_string(),
            description: "Test forward compatibility from v1.0.0 to v1.1.0".to_string(),
            protocol: ProtocolType::Rest,
            service: ServiceType::Vm,
            from_version: v1_0_0.clone(),
            to_version: v1_1_0.clone(),
            test_data: TestData {
                request_data: serde_json::json!({
                    "dot_id": "test_dot",
                    "inputs": {}
                }),
                response_data: None,
                features_used: vec!["execute_dot".to_string()],
                headers: HashMap::new(),
            },
            expected_result: ExpectedResult {
                should_succeed: true,
                expected_warnings: Vec::new(),
                expected_errors: Vec::new(),
                expected_transformations: None,
            },
            timeout_seconds: 30,
        });

        // Test 2: Breaking change detection (1.0.0 -> 2.0.0)
        self.add_test_case(CompatibilityTestCase {
            id: "vm_breaking_change".to_string(),
            name: "VM Service Breaking Change Detection".to_string(),
            description: "Test detection of breaking changes from v1.0.0 to v2.0.0".to_string(),
            protocol: ProtocolType::Rest,
            service: ServiceType::Vm,
            from_version: v1_0_0.clone(),
            to_version: v2_0_0.clone(),
            test_data: TestData {
                request_data: serde_json::json!({
                    "dot_id": "test_dot",
                    "inputs": {}
                }),
                response_data: None,
                features_used: vec!["execute_dot".to_string()],
                headers: HashMap::new(),
            },
            expected_result: ExpectedResult {
                should_succeed: false,
                expected_warnings: Vec::new(),
                expected_errors: vec!["breaking changes".to_string()],
                expected_transformations: None,
            },
            timeout_seconds: 30,
        });

        // Test 3: Database service compatibility
        self.add_test_case(CompatibilityTestCase {
            id: "db_forward_compatibility".to_string(),
            name: "Database Service Forward Compatibility".to_string(),
            description: "Test database service forward compatibility".to_string(),
            protocol: ProtocolType::Rest,
            service: ServiceType::Database,
            from_version: v1_0_0,
            to_version: v1_1_0,
            test_data: TestData {
                request_data: serde_json::json!({
                    "collection": "test_collection",
                    "key": "test_key",
                    "value": "test_value"
                }),
                response_data: None,
                features_used: vec!["put".to_string()],
                headers: HashMap::new(),
            },
            expected_result: ExpectedResult {
                should_succeed: true,
                expected_warnings: Vec::new(),
                expected_errors: Vec::new(),
                expected_transformations: None,
            },
            timeout_seconds: 30,
        });
    }

    /// Get all test cases
    pub fn test_cases(&self) -> &[CompatibilityTestCase] {
        &self.test_cases
    }
}

/// Test report for compatibility testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityTestReport {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub avg_execution_time_ms: u64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub test_results: Vec<TestResult>,
}

impl CompatibilityTestReport {
    /// Check if all tests passed
    pub fn all_tests_passed(&self) -> bool {
        self.failed_tests == 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tests == 0 {
            100.0
        } else {
            (self.passed_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    /// Generate summary string
    pub fn summary(&self) -> String {
        format!(
            "Tests: {} passed, {} failed, {} total ({:.1}% success rate)",
            self.passed_tests,
            self.failed_tests,
            self.total_tests,
            self.success_rate()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compatibility_test_suite() {
        let version_registry = VersionRegistry::new();
        let compatibility_checker = CompatibilityChecker::new();
        let schema_manager = SchemaEvolutionManager::new();
        let deprecation_manager = DeprecationManager::default();

        let mut test_suite = CompatibilityTestSuite::new(version_registry, compatibility_checker, schema_manager, deprecation_manager);

        let results = test_suite.run_all_tests().await;
        let report = test_suite.generate_report(&results);

        assert!(report.total_tests > 0);
        println!("Test report: {}", report.summary());
    }

    #[test]
    fn test_report_generation() {
        let results = vec![
            TestResult {
                test_id: "test1".to_string(),
                success: true,
                execution_time_ms: 100,
                warnings: Vec::new(),
                errors: Vec::new(),
                details: HashMap::new(),
            },
            TestResult {
                test_id: "test2".to_string(),
                success: false,
                execution_time_ms: 200,
                warnings: Vec::new(),
                errors: vec!["Test failed".to_string()],
                details: HashMap::new(),
            },
        ];

        let version_registry = VersionRegistry::new();
        let compatibility_checker = CompatibilityChecker::new();
        let schema_manager = SchemaEvolutionManager::new();
        let deprecation_manager = DeprecationManager::default();

        let test_suite = CompatibilityTestSuite::new(version_registry, compatibility_checker, schema_manager, deprecation_manager);

        let report = test_suite.generate_report(&results);

        assert_eq!(report.total_tests, 2);
        assert_eq!(report.passed_tests, 1);
        assert_eq!(report.failed_tests, 1);
        assert_eq!(report.success_rate(), 50.0);
    }
}

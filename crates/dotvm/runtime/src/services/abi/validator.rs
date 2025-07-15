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

//! ABI validator - validates ABI definitions and data against ABIs

use std::collections::HashMap;
use thiserror::Error;
use tracing::{error, info, instrument};

use crate::proto::vm_service::{AbiField, DotAbi, ValidateAbiRequest, ValidateAbiResponse, ValidationError, ValidationWarning};

#[derive(Error, Debug)]
pub enum ValidatorError {
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    #[error("Invalid type: {0}")]
    InvalidType(String),
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

/// ABI validator validates ABI definitions and data
pub struct AbiValidator {
    // TODO: Add type system, constraint validators, etc.
}

impl AbiValidator {
    pub fn new() -> Self {
        Self {}
    }

    #[instrument(skip(self, request))]
    pub async fn validate_abi(&self, request: ValidateAbiRequest) -> Result<ValidateAbiResponse, ValidatorError> {
        info!("Validating ABI");

        let abi = request.abi.ok_or_else(|| ValidatorError::ValidationFailed("No ABI provided".to_string()))?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate ABI structure
        self.validate_abi_structure(&abi, &mut errors, &mut warnings)?;

        // Validate input fields
        self.validate_fields(&abi.inputs, "input", &mut errors, &mut warnings)?;

        // Validate output fields
        self.validate_fields(&abi.outputs, "output", &mut errors, &mut warnings)?;

        // Validate paradot dependencies
        self.validate_paradots(&abi.paradots, &mut errors, &mut warnings)?;

        // Validate permissions if present
        if let Some(permissions) = &abi.permissions {
            self.validate_permissions(permissions, &mut errors, &mut warnings)?;
        }

        let valid = errors.is_empty();

        Ok(ValidateAbiResponse { valid, errors, warnings })
    }

    /// Validate data against an ABI
    pub async fn validate_data_against_abi(
        &self,
        data: &HashMap<String, Vec<u8>>,
        abi: &DotAbi,
        data_type: &str, // "input" or "output"
    ) -> Result<(), ValidatorError> {
        info!("Validating {} data against ABI", data_type);

        let fields = match data_type {
            "input" => &abi.inputs,
            "output" => &abi.outputs,
            _ => return Err(ValidatorError::ValidationFailed("Invalid data type".to_string())),
        };

        // Check required fields
        for field in fields {
            if field.required && !data.contains_key(&field.name) {
                return Err(ValidatorError::ValidationFailed(format!("Missing required {} field: {}", data_type, field.name)));
            }
        }

        // Validate each provided field
        for (field_name, field_data) in data {
            if let Some(field_def) = fields.iter().find(|f| f.name == *field_name) {
                self.validate_field_data(field_data, field_def)?;
            } else {
                return Err(ValidatorError::ValidationFailed(format!("Unknown {} field: {}", data_type, field_name)));
            }
        }

        Ok(())
    }

    // Private validation methods
    fn validate_abi_structure(&self, abi: &DotAbi, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> Result<(), ValidatorError> {
        // Validate dot name
        if abi.dot_name.is_empty() {
            errors.push(ValidationError {
                field: "dot_name".to_string(),
                message: "Dot name cannot be empty".to_string(),
                error_code: "EMPTY_DOT_NAME".to_string(),
            });
        }

        // Validate version format
        if !self.is_valid_version(&abi.version) {
            errors.push(ValidationError {
                field: "version".to_string(),
                message: "Invalid version format".to_string(),
                error_code: "INVALID_VERSION".to_string(),
            });
        }

        // Check for description
        if abi.description.is_empty() {
            warnings.push(ValidationWarning {
                field: "description".to_string(),
                message: "Description is empty".to_string(),
                warning_code: "EMPTY_DESCRIPTION".to_string(),
            });
        }

        Ok(())
    }

    fn validate_fields(&self, fields: &[AbiField], field_type: &str, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> Result<(), ValidatorError> {
        let mut field_names = std::collections::HashSet::new();

        for field in fields {
            // Check for duplicate field names
            if !field_names.insert(&field.name) {
                errors.push(ValidationError {
                    field: format!("{}s.{}", field_type, field.name),
                    message: format!("Duplicate {} field name", field_type),
                    error_code: "DUPLICATE_FIELD_NAME".to_string(),
                });
            }

            // Validate field name
            if field.name.is_empty() {
                errors.push(ValidationError {
                    field: format!("{}s", field_type),
                    message: format!("{} field name cannot be empty", field_type),
                    error_code: "EMPTY_FIELD_NAME".to_string(),
                });
            }

            // Validate field type
            if let Some(field_type_def) = &field.field_type {
                self.validate_field_type(field_type_def, &field.name, errors)?;
            } else {
                errors.push(ValidationError {
                    field: format!("{}s.{}.type", field_type, field.name),
                    message: "Field type is required".to_string(),
                    error_code: "MISSING_FIELD_TYPE".to_string(),
                });
            }

            // Validate constraints if present
            if let Some(constraints) = &field.constraints {
                self.validate_field_constraints(constraints, &field.name, errors, warnings)?;
            }
        }

        Ok(())
    }

    fn validate_field_type(&self, field_type: &crate::proto::vm_service::AbiType, field_name: &str, errors: &mut Vec<ValidationError>) -> Result<(), ValidatorError> {
        // Validate type name
        if field_type.type_name.is_empty() {
            errors.push(ValidationError {
                field: format!("{}.type", field_name),
                message: "Type name cannot be empty".to_string(),
                error_code: "EMPTY_TYPE_NAME".to_string(),
            });
        }

        // TODO: Validate that type name is a known type
        let known_types = vec!["String", "Integer", "Float", "Boolean", "Binary", "Array", "Object", "DateTime", "UUID", "Currency"];

        if !known_types.contains(&field_type.type_name.as_str()) {
            errors.push(ValidationError {
                field: format!("{}.type", field_name),
                message: format!("Unknown type: {}", field_type.type_name),
                error_code: "UNKNOWN_TYPE".to_string(),
            });
        }

        Ok(())
    }

    fn validate_field_constraints(
        &self,
        constraints: &crate::proto::vm_service::FieldConstraints,
        field_name: &str,
        errors: &mut Vec<ValidationError>,
        warnings: &mut Vec<ValidationWarning>,
    ) -> Result<(), ValidatorError> {
        // TODO: Validate constraints based on field type
        // For now, just check basic constraint validity

        if !constraints.pattern.is_empty() {
            // Validate regex pattern
            if let Err(_) = regex::Regex::new(&constraints.pattern) {
                errors.push(ValidationError {
                    field: format!("{}.constraints.pattern", field_name),
                    message: "Invalid regex pattern".to_string(),
                    error_code: "INVALID_REGEX".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_paradots(&self, paradots: &[crate::proto::vm_service::ParaDotDependency], errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> Result<(), ValidatorError> {
        let mut paradot_names = std::collections::HashSet::new();

        for paradot in paradots {
            // Check for duplicate paradot names
            if !paradot_names.insert(&paradot.name) {
                errors.push(ValidationError {
                    field: format!("paradots.{}", paradot.name),
                    message: "Duplicate paradot name".to_string(),
                    error_code: "DUPLICATE_PARADOT_NAME".to_string(),
                });
            }

            // Validate paradot name
            if paradot.name.is_empty() {
                errors.push(ValidationError {
                    field: "paradots".to_string(),
                    message: "ParaDot name cannot be empty".to_string(),
                    error_code: "EMPTY_PARADOT_NAME".to_string(),
                });
            }

            // Validate paradot type
            if paradot.paradot_type.is_empty() {
                errors.push(ValidationError {
                    field: format!("paradots.{}.type", paradot.name),
                    message: "ParaDot type cannot be empty".to_string(),
                    error_code: "EMPTY_PARADOT_TYPE".to_string(),
                });
            }
        }

        Ok(())
    }

    fn validate_permissions(&self, permissions: &crate::proto::vm_service::PermissionConfig, errors: &mut Vec<ValidationError>, warnings: &mut Vec<ValidationWarning>) -> Result<(), ValidatorError> {
        // TODO: Validate permission configuration
        // For now, just basic validation

        if permissions.public_operations.is_empty() && permissions.protected_operations.is_empty() {
            warnings.push(ValidationWarning {
                field: "permissions".to_string(),
                message: "No operations defined in permissions".to_string(),
                warning_code: "NO_OPERATIONS_DEFINED".to_string(),
            });
        }

        Ok(())
    }

    fn validate_field_data(&self, data: &[u8], field_def: &AbiField) -> Result<(), ValidatorError> {
        // TODO: Implement actual data validation against field type and constraints
        // For now, just check that data is not empty for required fields

        if field_def.required && data.is_empty() {
            return Err(ValidatorError::ValidationFailed(format!("Required field '{}' cannot be empty", field_def.name)));
        }

        Ok(())
    }

    fn is_valid_version(&self, version: &str) -> bool {
        // Simple semantic version validation
        let parts: Vec<&str> = version.split('.').collect();
        parts.len() == 3 && parts.iter().all(|part| part.parse::<u32>().is_ok())
    }
}

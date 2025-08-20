// Dotlanth
// Copyright (C) 2025 Synerthink

use crate::versioning::{ApiVersion, ProtocolType, ServiceType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Schema evolution errors
#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Schema validation failed: {0}")]
    ValidationFailed(String),
    #[error("Incompatible schema change: {0}")]
    IncompatibleChange(String),
    #[error("Schema not found: {protocol}/{service} v{version}")]
    SchemaNotFound { protocol: String, service: String, version: String },
    #[error("Transformation failed: {0}")]
    TransformationFailed(String),
}

/// Schema definition for API endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSchema {
    /// Schema identifier
    pub id: String,
    /// Protocol this schema applies to
    pub protocol: ProtocolType,
    /// Service this schema applies to
    pub service: ServiceType,
    /// API version
    pub version: ApiVersion,
    /// Schema content (JSON Schema format)
    pub schema: Value,
    /// Fields that are required
    pub required_fields: Vec<String>,
    /// Fields that are optional
    pub optional_fields: Vec<String>,
    /// Deprecated fields
    pub deprecated_fields: Vec<String>,
    /// Schema metadata
    pub metadata: HashMap<String, String>,
}

/// Schema transformation rule for version conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTransformation {
    /// Source version
    pub from_version: ApiVersion,
    /// Target version
    pub to_version: ApiVersion,
    /// Protocol and service this applies to
    pub protocol: ProtocolType,
    pub service: ServiceType,
    /// Transformation rules
    pub rules: Vec<TransformationRule>,
    /// Whether this transformation is bidirectional
    pub bidirectional: bool,
}

/// Individual transformation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationRule {
    /// Rule type
    pub rule_type: TransformationType,
    /// Source field path
    pub source_path: String,
    /// Target field path
    pub target_path: String,
    /// Default value for new fields
    pub default_value: Option<Value>,
    /// Transformation function (for complex mappings)
    pub transform_function: Option<String>,
    /// Validation rule for transformed value
    pub validation: Option<String>,
}

/// Types of schema transformations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformationType {
    /// Direct field mapping (rename)
    FieldMapping,
    /// Add new field with default value
    FieldAddition,
    /// Remove field
    FieldRemoval,
    /// Split one field into multiple
    FieldSplit,
    /// Merge multiple fields into one
    FieldMerge,
    /// Transform field type/format
    FieldTransform,
    /// Conditional transformation
    ConditionalTransform,
}

/// Schema compatibility level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityLevel {
    /// Fully compatible (no breaking changes)
    Full,
    /// Forward compatible (newer can read older)
    Forward,
    /// Backward compatible (older can read newer)
    Backward,
    /// Incompatible (breaking changes)
    None,
}

/// Schema evolution manager
#[derive(Debug, Clone)]
pub struct SchemaEvolutionManager {
    /// Registered schemas by version
    schemas: HashMap<(ProtocolType, ServiceType, ApiVersion), ApiSchema>,
    /// Transformation rules between versions
    transformations: HashMap<(ProtocolType, ServiceType, ApiVersion, ApiVersion), SchemaTransformation>,
    /// Schema validation cache
    validation_cache: HashMap<String, bool>,
}

impl Default for SchemaEvolutionManager {
    fn default() -> Self {
        let mut manager = Self {
            schemas: HashMap::new(),
            transformations: HashMap::new(),
            validation_cache: HashMap::new(),
        };

        manager.initialize_default_schemas();
        manager
    }
}

impl SchemaEvolutionManager {
    /// Create a new schema evolution manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema for a specific version
    pub fn register_schema(&mut self, schema: ApiSchema) {
        let key = (schema.protocol.clone(), schema.service.clone(), schema.version.clone());
        self.schemas.insert(key, schema);
    }

    /// Register a transformation between versions
    pub fn register_transformation(&mut self, transformation: SchemaTransformation) {
        let key = (
            transformation.protocol.clone(),
            transformation.service.clone(),
            transformation.from_version.clone(),
            transformation.to_version.clone(),
        );
        self.transformations.insert(key, transformation);
    }

    /// Get schema for a specific version
    pub fn get_schema(&self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion) -> Option<&ApiSchema> {
        let key = (protocol.clone(), service.clone(), version.clone());
        self.schemas.get(&key)
    }

    /// Validate data against a schema
    pub fn validate_data(&mut self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion, data: &Value) -> Result<(), SchemaError> {
        let schema = self.get_schema(protocol, service, version).ok_or_else(|| SchemaError::SchemaNotFound {
            protocol: protocol.to_string(),
            service: service.to_string(),
            version: version.to_string(),
        })?;

        // Create cache key
        let cache_key = format!("{}:{}:{}:{}", protocol, service, version, data.to_string());

        // Check cache first
        if let Some(&is_valid) = self.validation_cache.get(&cache_key) {
            if !is_valid {
                return Err(SchemaError::ValidationFailed("Cached validation failure".to_string()));
            }
            return Ok(());
        }

        // Validate against JSON schema
        let validation_result = self.validate_against_json_schema(&schema.schema, data);

        // Cache result
        self.validation_cache.insert(cache_key, validation_result.is_ok());

        validation_result
    }

    /// Transform data between schema versions
    pub fn transform_data(&self, protocol: &ProtocolType, service: &ServiceType, from_version: &ApiVersion, to_version: &ApiVersion, data: &Value) -> Result<Value, SchemaError> {
        // If versions are the same, no transformation needed
        if from_version == to_version {
            return Ok(data.clone());
        }

        // Look for direct transformation
        let key = (protocol.clone(), service.clone(), from_version.clone(), to_version.clone());
        if let Some(transformation) = self.transformations.get(&key) {
            return self.apply_transformation(transformation, data);
        }

        // Look for reverse transformation
        let reverse_key = (protocol.clone(), service.clone(), to_version.clone(), from_version.clone());
        if let Some(transformation) = self.transformations.get(&reverse_key) {
            if transformation.bidirectional {
                return self.apply_reverse_transformation(transformation, data);
            }
        }

        // Try to find a transformation path
        self.find_transformation_path(protocol, service, from_version, to_version, data)
    }

    /// Check compatibility between two schema versions
    pub fn check_compatibility(&self, protocol: &ProtocolType, service: &ServiceType, from_version: &ApiVersion, to_version: &ApiVersion) -> CompatibilityLevel {
        let from_schema = match self.get_schema(protocol, service, from_version) {
            Some(schema) => schema,
            None => return CompatibilityLevel::None,
        };

        let to_schema = match self.get_schema(protocol, service, to_version) {
            Some(schema) => schema,
            None => return CompatibilityLevel::None,
        };

        // Check field compatibility
        let forward_compatible = self.is_forward_compatible(from_schema, to_schema);
        let backward_compatible = self.is_backward_compatible(from_schema, to_schema);

        match (forward_compatible, backward_compatible) {
            (true, true) => CompatibilityLevel::Full,
            (true, false) => CompatibilityLevel::Forward,
            (false, true) => CompatibilityLevel::Backward,
            (false, false) => CompatibilityLevel::None,
        }
    }

    /// Get all available versions for a protocol/service
    pub fn get_available_versions(&self, protocol: &ProtocolType, service: &ServiceType) -> Vec<ApiVersion> {
        let mut versions: Vec<_> = self.schemas.keys().filter(|(p, s, _)| p == protocol && s == service).map(|(_, _, v)| v.clone()).collect();

        versions.sort_by(|a, b| b.cmp(a)); // Newest first
        versions
    }

    /// Apply transformation rules to data
    fn apply_transformation(&self, transformation: &SchemaTransformation, data: &Value) -> Result<Value, SchemaError> {
        let mut result = data.clone();

        for rule in &transformation.rules {
            result = self.apply_transformation_rule(rule, &result)?;
        }

        Ok(result)
    }

    /// Apply reverse transformation
    fn apply_reverse_transformation(&self, transformation: &SchemaTransformation, data: &Value) -> Result<Value, SchemaError> {
        let mut result = data.clone();

        // Apply rules in reverse order with swapped source/target
        for rule in transformation.rules.iter().rev() {
            let reverse_rule = TransformationRule {
                rule_type: rule.rule_type.clone(),
                source_path: rule.target_path.clone(),
                target_path: rule.source_path.clone(),
                default_value: rule.default_value.clone(),
                transform_function: rule.transform_function.clone(),
                validation: rule.validation.clone(),
            };
            result = self.apply_transformation_rule(&reverse_rule, &result)?;
        }

        Ok(result)
    }

    /// Apply a single transformation rule
    fn apply_transformation_rule(&self, rule: &TransformationRule, data: &Value) -> Result<Value, SchemaError> {
        let mut result = data.clone();

        match rule.rule_type {
            TransformationType::FieldMapping => {
                // Move field from source to target path
                if let Some(value) = self.get_field_value(&result, &rule.source_path) {
                    result = self.set_field_value(result, &rule.target_path, value)?;
                    result = self.remove_field_value(result, &rule.source_path)?;
                }
            }
            TransformationType::FieldAddition => {
                // Add new field with default value
                if let Some(default) = &rule.default_value {
                    result = self.set_field_value(result, &rule.target_path, default.clone())?;
                }
            }
            TransformationType::FieldRemoval => {
                // Remove field
                result = self.remove_field_value(result, &rule.source_path)?;
            }
            TransformationType::FieldTransform => {
                // Transform field value
                if let Some(value) = self.get_field_value(&result, &rule.source_path) {
                    let transformed = self.transform_field_value(value, rule)?;
                    result = self.set_field_value(result, &rule.target_path, transformed)?;
                    if rule.source_path != rule.target_path {
                        result = self.remove_field_value(result, &rule.source_path)?;
                    }
                }
            }
            // Additional transformation types can be implemented here
            _ => {
                return Err(SchemaError::TransformationFailed(format!("Unsupported transformation type: {:?}", rule.rule_type)));
            }
        }

        Ok(result)
    }

    /// Get field value from JSON data using dot notation path
    fn get_field_value(&self, data: &Value, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;

        for part in parts {
            match current {
                Value::Object(map) => {
                    current = map.get(part)?;
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(current.clone())
    }

    /// Set field value in JSON data using dot notation path
    fn set_field_value(&self, mut data: Value, path: &str, value: Value) -> Result<Value, SchemaError> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Ok(value);
        }

        // Ensure data is an object
        if !data.is_object() {
            data = Value::Object(serde_json::Map::new());
        }

        let mut current = &mut data;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                if let Value::Object(map) = current {
                    map.insert(part.to_string(), value.clone());
                }
            } else {
                // Intermediate part - ensure object exists
                if let Value::Object(map) = current {
                    let entry = map.entry(part.to_string()).or_insert_with(|| Value::Object(serde_json::Map::new()));
                    current = entry;
                }
            }
        }

        Ok(data)
    }

    /// Remove field value from JSON data
    fn remove_field_value(&self, mut data: Value, path: &str) -> Result<Value, SchemaError> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Ok(data);
        }

        let mut current = &mut data;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - remove the field
                if let Value::Object(map) = current {
                    map.remove(*part);
                }
            } else {
                // Navigate to parent
                if let Value::Object(map) = current {
                    if let Some(entry) = map.get_mut(*part) {
                        current = entry;
                    } else {
                        break; // Path doesn't exist
                    }
                }
            }
        }

        Ok(data)
    }

    /// Transform field value based on transformation function
    fn transform_field_value(&self, value: Value, rule: &TransformationRule) -> Result<Value, SchemaError> {
        // In a real implementation, this would support pluggable transformation functions
        // For now, implement some basic transformations
        match rule.transform_function.as_deref() {
            Some("string_to_number") => {
                if let Value::String(s) = value {
                    s.parse::<f64>()
                        .map(|n| Value::Number(serde_json::Number::from_f64(n).unwrap()))
                        .map_err(|e| SchemaError::TransformationFailed(e.to_string()))
                } else {
                    Ok(value)
                }
            }
            Some("number_to_string") => {
                if let Value::Number(n) = value {
                    Ok(Value::String(n.to_string()))
                } else {
                    Ok(value)
                }
            }
            _ => Ok(value), // No transformation
        }
    }

    /// Validate data against JSON schema
    fn validate_against_json_schema(&self, schema: &Value, data: &Value) -> Result<(), SchemaError> {
        // Basic validation - in a real implementation, use a proper JSON Schema validator
        // like jsonschema crate

        // For now, just check basic structure
        if schema.is_object() && data.is_object() {
            Ok(())
        } else if schema.is_array() && data.is_array() {
            Ok(())
        } else if schema.is_string() && data.is_string() {
            Ok(())
        } else {
            Err(SchemaError::ValidationFailed("Schema mismatch".to_string()))
        }
    }

    /// Check if newer schema is forward compatible with older
    fn is_forward_compatible(&self, from_schema: &ApiSchema, to_schema: &ApiSchema) -> bool {
        // Forward compatible if all required fields in from_schema are still present in to_schema
        from_schema
            .required_fields
            .iter()
            .all(|field| to_schema.required_fields.contains(field) || to_schema.optional_fields.contains(field))
    }

    /// Check if older schema is backward compatible with newer
    fn is_backward_compatible(&self, from_schema: &ApiSchema, to_schema: &ApiSchema) -> bool {
        // Backward compatible if no new required fields were added
        to_schema
            .required_fields
            .iter()
            .all(|field| from_schema.required_fields.contains(field) || from_schema.optional_fields.contains(field))
    }

    /// Find transformation path between versions
    fn find_transformation_path(&self, protocol: &ProtocolType, service: &ServiceType, from_version: &ApiVersion, to_version: &ApiVersion, data: &Value) -> Result<Value, SchemaError> {
        // Simple implementation - in practice, this would use a graph algorithm
        // to find the shortest transformation path through intermediate versions

        Err(SchemaError::TransformationFailed(format!("No transformation path found from {} to {}", from_version, to_version)))
    }

    /// Initialize default schemas for current API versions
    fn initialize_default_schemas(&mut self) {
        let v1_0_0 = ApiVersion::new(1, 0, 0);

        // VM service schema
        let vm_schema = ApiSchema {
            id: "vm_v1_0_0".to_string(),
            protocol: ProtocolType::Rest,
            service: ServiceType::Vm,
            version: v1_0_0.clone(),
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dot_id": {"type": "string"},
                    "inputs": {"type": "object"},
                    "paradots_enabled": {"type": "boolean"}
                }
            }),
            required_fields: vec!["dot_id".to_string()],
            optional_fields: vec!["inputs".to_string(), "paradots_enabled".to_string()],
            deprecated_fields: vec![],
            metadata: HashMap::new(),
        };

        self.register_schema(vm_schema);

        // Database service schema
        let db_schema = ApiSchema {
            id: "db_v1_0_0".to_string(),
            protocol: ProtocolType::Rest,
            service: ServiceType::Database,
            version: v1_0_0,
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "collection": {"type": "string"},
                    "key": {"type": "string"},
                    "value": {"type": "string"}
                }
            }),
            required_fields: vec!["collection".to_string(), "key".to_string()],
            optional_fields: vec!["value".to_string()],
            deprecated_fields: vec![],
            metadata: HashMap::new(),
        };

        self.register_schema(db_schema);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_registration() {
        let mut manager = SchemaEvolutionManager::new();
        let version = ApiVersion::new(1, 0, 0);

        let schema = manager.get_schema(&ProtocolType::Rest, &ServiceType::Vm, &version);
        assert!(schema.is_some());
    }

    #[test]
    fn test_data_validation() {
        let mut manager = SchemaEvolutionManager::new();
        let version = ApiVersion::new(1, 0, 0);

        let data = serde_json::json!({
            "dot_id": "test_dot",
            "inputs": {}
        });

        let result = manager.validate_data(&ProtocolType::Rest, &ServiceType::Vm, &version, &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_field_operations() {
        let manager = SchemaEvolutionManager::new();
        let data = serde_json::json!({
            "user": {
                "name": "John",
                "age": 30
            }
        });

        assert_eq!(manager.get_field_value(&data, "user.name"), Some(Value::String("John".to_string())));

        let updated = manager.set_field_value(data, "user.email", Value::String("john@example.com".to_string())).unwrap();
        assert!(updated.get("user").unwrap().get("email").is_some());
    }
}

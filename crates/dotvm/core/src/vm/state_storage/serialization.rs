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

use bincode;
use serde::{Serialize, de::DeserializeOwned};
use serde_json;
use std::fmt;

/// Defines errors that may occur during serialization or deserialization.
#[derive(Debug, PartialEq)]
pub enum SerializationError {
    /// An error occurred during serialization.
    SerializeError(String),
    /// An error occurred during deserialization.
    DeserializeError(String),
    /// Operation not implemented.
    NotImplemented,
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SerializationError::SerializeError(s) => write!(f, "SerializeError: {}", s),
            SerializationError::DeserializeError(s) => write!(f, "DeserializeError: {}", s),
            SerializationError::NotImplemented => write!(f, "NotImplemented"),
        }
    }
}

impl std::error::Error for SerializationError {}

/// The StateSerializer trait defines the interface to serialize and deserialize state data.
/// It is generic over type T, which must implement Serialize and DeserializeOwned.
pub trait StateSerializer<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Serializes a value into a vector of bytes.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to the value to be serialized.
    ///
    /// # Returns
    ///
    /// * A Result containing a Vec<u8> representing the serialized data on success.
    /// * A SerializationError on failure.
    fn serialize(&self, value: &T) -> Result<Vec<u8>, SerializationError>;

    /// Deserializes a value from a slice of bytes.
    ///
    /// # Arguments
    ///
    /// * `data` - A byte slice containing the serialized data.
    ///
    /// # Returns
    ///
    /// * A Result containing the deserialized value on success.
    /// * A SerializationError on failure.
    fn deserialize(&self, data: &[u8]) -> Result<T, SerializationError>;
}

/// JsonStateSerializer is our production implementation for JSON serialization.
pub struct JsonStateSerializer;

impl<T> StateSerializer<T> for JsonStateSerializer
where
    T: Serialize + DeserializeOwned,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, SerializationError> {
        serde_json::to_vec(value).map_err(|e| SerializationError::SerializeError(e.to_string()))
    }

    fn deserialize(&self, data: &[u8]) -> Result<T, SerializationError> {
        serde_json::from_slice(data).map_err(|e| SerializationError::DeserializeError(e.to_string()))
    }
}

/// BinaryStateSerializer is our production implementation for binary serialization.
/// Replace the unimplemented macros with a binary serializer (e.g. using bincode or MessagePack).
pub struct BinaryStateSerializer;

impl<T> StateSerializer<T> for BinaryStateSerializer
where
    T: Serialize + DeserializeOwned,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, SerializationError> {
        bincode::serde::encode_to_vec(value, bincode::config::standard()).map_err(|e| SerializationError::SerializeError(e.to_string()))
    }

    fn deserialize(&self, data: &[u8]) -> Result<T, SerializationError> {
        bincode::serde::decode_from_slice(data, bincode::config::standard())
            .map(|(result, _)| result)
            .map_err(|e| SerializationError::DeserializeError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// A simple test structure to verify round-trip serialization/deserialization.
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestState {
        key: String,
        value: String,
    }

    /// A more complex test structure with nested fields.
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct ComplexState {
        id: u32,
        name: String,
        metadata: Option<Vec<String>>,
    }

    // --- JSON Serialization Tests ---
    #[test]
    fn test_json_serializer_roundtrip() {
        let test_state = TestState {
            key: "test".into(),
            value: "data".into(),
        };
        let serializer = JsonStateSerializer;
        // Expect the serializer to eventually produce valid bytes.
        let bytes = serializer.serialize(&test_state).expect("JSON serialization should succeed when implemented");
        // Check that the byte vector is not empty.
        assert!(!bytes.is_empty(), "Serialized bytes should not be empty");
        let result: TestState = serializer.deserialize(&bytes).expect("JSON deserialization should succeed when implemented");
        assert_eq!(result, test_state, "JSON roundtrip must yield the original value");
    }

    #[test]
    fn test_json_serializer_empty_state() {
        let test_state = TestState { key: "".into(), value: "".into() };
        let serializer = JsonStateSerializer;
        let bytes = serializer.serialize(&test_state).expect("Serialization of empty state should succeed");
        assert!(!bytes.is_empty(), "Serialized bytes for empty state must not be empty");
        let result: TestState = serializer.deserialize(&bytes).expect("Deserialization of empty state should succeed");
        assert_eq!(result, test_state, "Empty state should be preserved after JSON roundtrip");
    }

    #[test]
    fn test_json_serializer_complex_state() {
        let complex = ComplexState {
            id: 42,
            name: "Complex".into(),
            metadata: Some(vec!["a".into(), "b".into()]),
        };
        let serializer = JsonStateSerializer;
        let bytes = serializer.serialize(&complex).expect("Serialization of complex state should succeed");
        assert!(!bytes.is_empty(), "Serialized bytes for complex state must not be empty");
        let result: ComplexState = serializer.deserialize(&bytes).expect("Deserialization of complex state should succeed");
        assert_eq!(result, complex, "Complex state must be preserved after JSON roundtrip");
    }

    #[test]
    fn test_json_serializer_invalid_deserialization() {
        let serializer = JsonStateSerializer;
        // Create an invalid byte slice (non-JSON data).
        let invalid_bytes = b"invalid data";
        let result: Result<TestState, SerializationError> = serializer.deserialize(invalid_bytes);
        assert!(result.is_err(), "JSON deserialization of invalid bytes should return an error");
    }

    // --- Binary Serialization Tests ---
    #[test]
    fn test_binary_serializer_roundtrip() {
        let test_state = TestState {
            key: "binary".into(),
            value: "data".into(),
        };
        let serializer = BinaryStateSerializer;
        let bytes = serializer.serialize(&test_state).expect("Binary serialization should succeed when implemented");
        assert!(!bytes.is_empty(), "Serialized binary bytes should not be empty");
        let result: TestState = serializer.deserialize(&bytes).expect("Binary deserialization should succeed when implemented");
        assert_eq!(result, test_state, "Binary roundtrip must yield the original value");
    }

    #[test]
    fn test_binary_serializer_empty_state() {
        let test_state = TestState { key: "".into(), value: "".into() };
        let serializer = BinaryStateSerializer;
        let bytes = serializer.serialize(&test_state).expect("Binary serialization of empty state should succeed");
        assert!(!bytes.is_empty(), "Serialized binary bytes for empty state must not be empty");
        let result: TestState = serializer.deserialize(&bytes).expect("Binary deserialization of empty state should succeed");
        assert_eq!(result, test_state, "Empty state should be preserved after binary roundtrip");
    }

    #[test]
    fn test_binary_serializer_complex_state() {
        let complex = ComplexState {
            id: 101,
            name: "BinaryComplex".into(),
            metadata: None,
        };
        let serializer = BinaryStateSerializer;
        let bytes = serializer.serialize(&complex).expect("Binary serialization of complex state should succeed");
        assert!(!bytes.is_empty(), "Serialized binary bytes for complex state must not be empty");
        let result: ComplexState = serializer.deserialize(&bytes).expect("Binary deserialization of complex state should succeed");
        assert_eq!(result, complex, "Complex state must be preserved after binary roundtrip");
    }

    #[test]
    fn test_binary_serializer_invalid_deserialization() {
        let serializer = BinaryStateSerializer;
        // Create an invalid byte slice (not produced by a proper binary serializer).
        let invalid_bytes = b"not a valid binary format";
        let result: Result<TestState, SerializationError> = serializer.deserialize(invalid_bytes);
        assert!(result.is_err(), "Binary deserialization of invalid bytes should return an error");
    }
}

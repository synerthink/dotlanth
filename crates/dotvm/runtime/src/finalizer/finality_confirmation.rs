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

use crate::finalizer::lib::{FinalityStatus, StateTransition, generate_timestamp, generate_unique_id};
use ring::signature::{ED25519, Ed25519KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};

/// Cryptographic proof of finalized state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityConfirmation {
    pub id: String,                  // Unique confirmation ID
    pub transition: StateTransition, // Associated state transition
    pub status: FinalityStatus,      // Finality status (e.g., Finalized)
    pub timestamp: u64,              // Confirmation creation time
    pub message: String,             // Human-readable status message
    #[serde(with = "signature_encoding")]
    pub signature: Option<Vec<u8>>, // Ed25519 signature (base64 encoded)
}

/// Custom serialization/deserialization for cryptographic signatures
mod signature_encoding {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(data: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(sig) => serializer.serialize_str(&base64::encode(sig)), // Base64 for web-safe encoding
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            Some(sig) => base64::decode(&sig).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

impl FinalityConfirmation {
    /// Create new unsigned confirmation
    pub fn new(transition: StateTransition, status: FinalityStatus, message: &str) -> Self {
        Self {
            id: generate_unique_id("conf"),
            transition,
            status,
            timestamp: generate_timestamp(),
            message: message.to_string(),
            signature: None, // Requires explicit signing
        }
    }

    /// Check if confirmation is in finalized state
    pub fn is_valid(&self) -> bool {
        self.status == FinalityStatus::Finalized // Business logic validity
    }

    /// Construct deterministic signing payload
    fn signing_message(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "id": self.id,
            "status": self.status.to_string(),
            "timestamp": self.timestamp,
            "transition_id": self.transition.id,
            "state_before_version": self.transition.state_before.version,
            "state_after_version": self.transition.state_after.version,
            "message": self.message,
        }))
        .expect("Serialization failed")
    }

    /// Attach cryptographic signature using private key
    pub fn add_signature(&mut self, sk: &Ed25519KeyPair) {
        let sig = sk.sign(&self.signing_message());
        self.signature = Some(sig.as_ref().to_vec());
    }

    /// Verify signature against public key
    pub fn verify_signature<K: AsRef<[u8]>>(&self, verifying_key: &K) -> bool {
        let sig_bytes = match &self.signature {
            Some(s) => s.as_slice(),
            None => return false,
        };
        UnparsedPublicKey::new(&ED25519, verifying_key.as_ref()).verify(&self.signing_message(), sig_bytes).is_ok()
    }

    /// Human-readable format for debugging/display
    pub fn to_string_representation(&self) -> String {
        let signature_str = match &self.signature {
            Some(sig) => format!("Signature: {}", base64::encode(sig)),
            None => "Unsigned".to_string(),
        };
        format!(
            "Finality Confirmation [{}]\n\
             Status: {}\n\
             Timestamp: {}\n\
             Transition ID: {}\n\
             State Change: v{} -> v{}\n\
             Message: {}\n\
             {}",
            self.id, self.status, self.timestamp, self.transition.id, self.transition.state_before.version, self.transition.state_after.version, self.message, signature_str
        )
    }

    /// JSON representation for API responses
    pub fn to_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "status": self.status.to_string(),
            "timestamp": self.timestamp,
            "transition_id": self.transition.id,
            "state_before_version": self.transition.state_before.version,
            "state_after_version": self.transition.state_after.version,
            "message": self.message,
            "signature": self.signature.as_ref().map(base64::encode),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finalizer::lib::{State, TransitionMetadata, generate_unique_id};
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn create_test_transition() -> StateTransition {
        StateTransition::new(
            generate_unique_id("trans"),
            State {
                data: "old_state".to_string(),
                version: 1,
            },
            State {
                data: "new_state".to_string(),
                version: 2,
            },
            TransitionMetadata {
                initiator: "test_user".to_string(),
                reason: "test_reason".to_string(),
                additional_info: None,
            },
        )
    }

    #[test]
    fn test_confirmation_creation() {
        let transition = create_test_transition();
        let confirmation = FinalityConfirmation::new(transition.clone(), FinalityStatus::Finalized, "Test confirmation");

        assert_eq!(confirmation.status, FinalityStatus::Finalized);
        assert_eq!(confirmation.transition.id, transition.id);
        assert!(!confirmation.id.is_empty());
        assert!(confirmation.timestamp > 0);
        assert_eq!(confirmation.message, "Test confirmation");
        assert!(confirmation.signature.is_none());
    }

    #[test]
    fn test_signature_addition() {
        let transition = create_test_transition();
        let mut confirmation = FinalityConfirmation::new(transition, FinalityStatus::Finalized, "Test confirmation");

        assert!(confirmation.signature.is_none());

        // SystemRandom ile PKCS#8 dokument oluşturup anahtarları çıkarıyoruz
        let rng = SystemRandom::new();
        let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8_bytes = pkcs8_doc.as_ref();
        let sk = Ed25519KeyPair::from_pkcs8(pkcs8_bytes).unwrap();

        confirmation.add_signature(&sk);
        let sig = confirmation.signature.as_ref().unwrap();
        assert_eq!(sig.len(), 64); // ED25519 imza uzunluğu
    }

    #[test]
    fn test_confirmation_validity() {
        let transition = create_test_transition();

        // Finalized – valid
        let finalized = FinalityConfirmation::new(transition.clone(), FinalityStatus::Finalized, "Finalized confirmation");
        assert!(finalized.is_valid());

        // Failed – invalid
        let failed = FinalityConfirmation::new(transition, FinalityStatus::Failed, "Failed confirmation");
        assert!(!failed.is_valid());

        // Henüz imza yokken verify false dönmeli
        let rng = SystemRandom::new();
        let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8_bytes = pkcs8_doc.as_ref();
        let sk = Ed25519KeyPair::from_pkcs8(pkcs8_bytes).unwrap();
        let vk = sk.public_key();
        assert!(!failed.verify_signature(&vk));
    }

    #[test]
    fn test_string_representation() {
        let transition = create_test_transition();
        let mut confirmation = FinalityConfirmation::new(transition, FinalityStatus::Finalized, "Test confirmation");

        // İmzalanmamış hali
        let str_rep = confirmation.to_string_representation();
        assert!(str_rep.contains("Unsigned"));
        assert!(str_rep.contains(&confirmation.id));

        // İmzalı hali
        let rng = SystemRandom::new();
        let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8_bytes = pkcs8_doc.as_ref();
        let sk = Ed25519KeyPair::from_pkcs8(pkcs8_bytes).unwrap();
        confirmation.add_signature(&sk);

        let str_rep_signed = confirmation.to_string_representation();
        assert!(str_rep_signed.contains("Signature:"));
    }

    #[test]
    fn test_serializable_representation() {
        let transition = create_test_transition();
        let confirmation = FinalityConfirmation::new(transition, FinalityStatus::Finalized, "Test confirmation");

        let serialized = confirmation.to_serializable();
        assert_eq!(serialized["id"], confirmation.id);
        assert_eq!(serialized["status"], "FINALIZED");
        assert_eq!(serialized["message"], "Test confirmation");
    }

    #[test]
    fn test_signature_verification() {
        let transition = create_test_transition();
        let mut confirmation = FinalityConfirmation::new(transition, FinalityStatus::Finalized, "Test confirmation");

        let rng = SystemRandom::new();
        let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8_bytes = pkcs8_doc.as_ref();
        let sk = Ed25519KeyPair::from_pkcs8(pkcs8_bytes).unwrap();
        confirmation.add_signature(&sk);

        let vk = sk.public_key();
        assert!(confirmation.verify_signature(&vk));

        let mut tampered = confirmation.clone();
        tampered.message = "Tampered".to_string();
        assert!(!tampered.verify_signature(&vk));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let transition = create_test_transition();
        let mut confirmation = FinalityConfirmation::new(transition, FinalityStatus::Finalized, "Test confirmation");

        let rng = SystemRandom::new();
        let pkcs8_doc = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
        let pkcs8_bytes = pkcs8_doc.as_ref();
        let sk = Ed25519KeyPair::from_pkcs8(pkcs8_bytes).unwrap();
        let vk = sk.public_key();

        confirmation.add_signature(&sk);
        let serialized = serde_json::to_string(&confirmation).unwrap();
        let deserialized: FinalityConfirmation = serde_json::from_str(&serialized).unwrap();

        assert_eq!(confirmation.signature, deserialized.signature);
        assert!(deserialized.verify_signature(&vk));
    }
}

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

//! Concrete implementations of cryptographic providers

use super::crypto_provider::*;
use aes_gcm::{AeadInPlace, Aes256Gcm, KeyInit, Nonce};
use blake3::Hasher as Blake3Hasher;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit as ChaChaKeyInit};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use k256::ecdsa::{Signature as EcdsaSignature, SigningKey as EcdsaSigningKey, VerifyingKey as EcdsaVerifyingKey};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::types::{Field, PrimeField64};
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2::plonk::proof::ProofWithPublicInputs;
use rand_core::{OsRng, RngCore};
use ring::digest::{Context, SHA256};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Default hash provider implementation using audited libraries
pub struct DefaultHashProvider;

impl HashProvider for DefaultHashProvider {
    fn hash(&self, algorithm: HashAlgorithm, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                Ok(hasher.finalize().to_vec())
            }
            HashAlgorithm::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(data);
                Ok(hasher.finalize().as_bytes().to_vec())
            }
            HashAlgorithm::Keccak256 => {
                // Using ring's SHA256 as placeholder - in production, use proper Keccak
                let mut context = Context::new(&SHA256);
                context.update(data);
                Ok(context.finish().as_ref().to_vec())
            }
        }
    }
}

/// Default signature provider implementation
pub struct DefaultSignatureProvider;

impl SignatureProvider for DefaultSignatureProvider {
    fn sign(&self, private_key: &CryptoKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match private_key.algorithm {
            SignatureAlgorithm::Ed25519 => {
                if private_key.key_data.len() != 32 {
                    return Err(CryptoError::InvalidKey("Ed25519 private key must be 32 bytes".to_string()));
                }

                let signing_key = SigningKey::from_bytes(
                    private_key
                        .key_data
                        .as_slice()
                        .try_into()
                        .map_err(|_| CryptoError::InvalidKey("Invalid Ed25519 key format".to_string()))?,
                );

                let signature = signing_key.sign(data);
                Ok(signature.to_bytes().to_vec())
            }
            SignatureAlgorithm::EcdsaSecp256k1 => {
                let signing_key = EcdsaSigningKey::from_slice(&private_key.key_data).map_err(|e| CryptoError::InvalidKey(format!("Invalid ECDSA key: {}", e)))?;

                let signature: EcdsaSignature = signing_key.sign(data);
                Ok(signature.to_bytes().to_vec())
            }
        }
    }

    fn verify(&self, public_key: &CryptoKey, signature: &[u8], data: &[u8]) -> Result<bool, CryptoError> {
        match public_key.algorithm {
            SignatureAlgorithm::Ed25519 => {
                if public_key.key_data.len() != 32 {
                    return Err(CryptoError::InvalidKey("Ed25519 public key must be 32 bytes".to_string()));
                }

                let verifying_key = VerifyingKey::from_bytes(
                    public_key
                        .key_data
                        .as_slice()
                        .try_into()
                        .map_err(|_| CryptoError::InvalidKey("Invalid Ed25519 public key format".to_string()))?,
                )
                .map_err(|e| CryptoError::InvalidKey(format!("Invalid Ed25519 public key: {}", e)))?;

                let signature = Signature::from_bytes(signature.try_into().map_err(|_| CryptoError::InvalidSignature("Invalid Ed25519 signature format".to_string()))?);

                Ok(verifying_key.verify(data, &signature).is_ok())
            }
            SignatureAlgorithm::EcdsaSecp256k1 => {
                let verifying_key = EcdsaVerifyingKey::from_sec1_bytes(&public_key.key_data).map_err(|e| CryptoError::InvalidKey(format!("Invalid ECDSA public key: {}", e)))?;

                let signature = EcdsaSignature::from_bytes(signature.into()).map_err(|e| CryptoError::InvalidSignature(format!("Invalid ECDSA signature: {}", e)))?;

                Ok(verifying_key.verify(data, &signature).is_ok())
            }
        }
    }

    fn generate_keypair(&self, algorithm: SignatureAlgorithm) -> Result<(CryptoKey, CryptoKey), CryptoError> {
        match algorithm {
            SignatureAlgorithm::Ed25519 => {
                let signing_key = SigningKey::generate(&mut OsRng);
                let verifying_key = signing_key.verifying_key();

                let private_key = CryptoKey::new(algorithm, signing_key.to_bytes().to_vec());
                let public_key = CryptoKey::new(algorithm, verifying_key.to_bytes().to_vec());

                Ok((private_key, public_key))
            }
            SignatureAlgorithm::EcdsaSecp256k1 => {
                let signing_key = EcdsaSigningKey::random(&mut OsRng);
                let verifying_key = signing_key.verifying_key();

                let private_key = CryptoKey::new(algorithm, signing_key.to_bytes().to_vec());
                let public_key = CryptoKey::new(algorithm, verifying_key.to_sec1_bytes().to_vec());

                Ok((private_key, public_key))
            }
        }
    }
}

/// Default encryption provider implementation
pub struct DefaultEncryptionProvider;

impl EncryptionProvider for DefaultEncryptionProvider {
    fn encrypt(&self, key: &EncryptionKey, data: &[u8], nonce: Option<&[u8]>) -> Result<Vec<u8>, CryptoError> {
        match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                if key.key_data.len() != 32 {
                    return Err(CryptoError::InvalidKey("AES-256-GCM key must be 32 bytes".to_string()));
                }

                let cipher = Aes256Gcm::new_from_slice(&key.key_data).map_err(|e| CryptoError::EncryptionFailed(format!("Failed to create AES cipher: {}", e)))?;

                let nonce_bytes = if let Some(n) = nonce {
                    if n.len() != 12 {
                        return Err(CryptoError::EncryptionFailed("AES-GCM nonce must be 12 bytes".to_string()));
                    }
                    n.to_vec()
                } else {
                    // Generate random nonce
                    let mut bytes = [0u8; 12];
                    OsRng.fill_bytes(&mut bytes);
                    bytes.to_vec()
                };
                let nonce = Nonce::from_slice(&nonce_bytes);

                let mut buffer = data.to_vec();
                cipher
                    .encrypt_in_place(nonce, b"", &mut buffer)
                    .map_err(|e| CryptoError::EncryptionFailed(format!("AES encryption failed: {}", e)))?;

                // Prepend nonce to ciphertext
                let mut result = nonce.to_vec();
                result.extend_from_slice(&buffer);
                Ok(result)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                if key.key_data.len() != 32 {
                    return Err(CryptoError::InvalidKey("ChaCha20Poly1305 key must be 32 bytes".to_string()));
                }

                let cipher = ChaCha20Poly1305::new_from_slice(&key.key_data).map_err(|e| CryptoError::EncryptionFailed(format!("Failed to create ChaCha20 cipher: {}", e)))?;

                let nonce_bytes = if let Some(n) = nonce {
                    if n.len() != 12 {
                        return Err(CryptoError::EncryptionFailed("ChaCha20Poly1305 nonce must be 12 bytes".to_string()));
                    }
                    n.try_into().map_err(|_| CryptoError::EncryptionFailed("Invalid nonce length".to_string()))?
                } else {
                    // Generate random nonce
                    let mut bytes = [0u8; 12];
                    OsRng.fill_bytes(&mut bytes);
                    bytes
                };
                let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);

                let mut buffer = data.to_vec();
                cipher
                    .encrypt_in_place(nonce, b"", &mut buffer)
                    .map_err(|e| CryptoError::EncryptionFailed(format!("ChaCha20 encryption failed: {}", e)))?;

                // Prepend nonce to ciphertext
                let mut result = nonce.to_vec();
                result.extend_from_slice(&buffer);
                Ok(result)
            }
        }
    }

    fn decrypt(&self, key: &EncryptionKey, encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                if encrypted_data.len() < 12 {
                    return Err(CryptoError::DecryptionFailed("Encrypted data too short".to_string()));
                }

                let cipher = Aes256Gcm::new_from_slice(&key.key_data).map_err(|e| CryptoError::DecryptionFailed(format!("Failed to create AES cipher: {}", e)))?;

                let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
                let nonce = Nonce::from_slice(nonce_bytes);

                let mut buffer = ciphertext.to_vec();
                cipher
                    .decrypt_in_place(nonce, b"", &mut buffer)
                    .map_err(|e| CryptoError::DecryptionFailed(format!("AES decryption failed: {}", e)))?;

                Ok(buffer)
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                if encrypted_data.len() < 12 {
                    return Err(CryptoError::DecryptionFailed("Encrypted data too short".to_string()));
                }

                let cipher = ChaCha20Poly1305::new_from_slice(&key.key_data).map_err(|e| CryptoError::DecryptionFailed(format!("Failed to create ChaCha20 cipher: {}", e)))?;

                let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
                let nonce = chacha20poly1305::Nonce::from_slice(nonce_bytes);

                let mut buffer = ciphertext.to_vec();
                cipher
                    .decrypt_in_place(nonce, b"", &mut buffer)
                    .map_err(|e| CryptoError::DecryptionFailed(format!("ChaCha20 decryption failed: {}", e)))?;

                Ok(buffer)
            }
        }
    }

    fn generate_key(&self, algorithm: EncryptionAlgorithm) -> Result<EncryptionKey, CryptoError> {
        let mut key_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut key_bytes);
        Ok(EncryptionKey::new(algorithm, key_bytes.to_vec()))
    }
}

/// Default secure random provider implementation
pub struct DefaultSecureRandomProvider;

impl SecureRandomProvider for DefaultSecureRandomProvider {
    fn generate_bytes(&self, count: usize) -> Result<Vec<u8>, CryptoError> {
        let mut bytes = vec![0u8; count];
        OsRng.fill_bytes(&mut bytes);
        Ok(bytes)
    }

    fn generate_u64(&self) -> Result<u64, CryptoError> {
        Ok(OsRng.next_u64())
    }
}

/// PLONKY2-based ZK provider implementation
pub struct Plonky2ZkProvider;

impl ZkProvider for Plonky2ZkProvider {
    fn generate_proof(&self, circuit_data: &[u8], witness: &[u8]) -> Result<Vec<u8>, CryptoError> {
        type C = PoseidonGoldilocksConfig;
        type F = GoldilocksField;

        // Parse circuit configuration from circuit_data
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, 2>::new(config);

        // For demonstration, create a simple circuit that proves knowledge of a secret
        // In practice, circuit_data would contain the actual circuit description
        let secret_target = builder.add_virtual_target();
        let public_hash_target = builder.add_virtual_target();

        // Simple constraint: hash(secret) == public_hash (simplified for demo)
        let computed_hash = builder.square(secret_target);
        builder.connect(computed_hash, public_hash_target);

        // Register public inputs
        builder.register_public_input(public_hash_target);

        // Build the circuit
        let data = builder.build::<C>();

        // Parse witness data
        if witness.len() < 16 {
            return Err(CryptoError::ZkProofFailed("Insufficient witness data".to_string()));
        }

        // Extract secret and public values from witness
        let secret_bytes = &witness[0..8];
        let public_bytes = &witness[8..16];

        let secret_value = u64::from_le_bytes(secret_bytes.try_into().map_err(|_| CryptoError::ZkProofFailed("Invalid secret format".to_string()))?);
        let public_value = u64::from_le_bytes(public_bytes.try_into().map_err(|_| CryptoError::ZkProofFailed("Invalid public input format".to_string()))?);

        // Create witness
        let mut pw = PartialWitness::new();
        pw.set_target(secret_target, F::from_canonical_u64(secret_value));
        pw.set_target(public_hash_target, F::from_canonical_u64(public_value));

        // Generate proof
        let proof = data.prove(pw).map_err(|e| CryptoError::ZkProofFailed(format!("Proof generation failed: {:?}", e)))?;

        // Serialize proof to bytes (using serde_json as fallback since PLONKY2 proofs don't implement bincode traits)
        let proof_bytes = serde_json::to_vec(&proof).map_err(|e| CryptoError::ZkProofFailed(format!("Proof serialization failed: {}", e)))?;

        Ok(proof_bytes)
    }

    fn verify_proof(&self, proof: &[u8], public_inputs: &[u8]) -> Result<bool, CryptoError> {
        type C = PoseidonGoldilocksConfig;
        type F = GoldilocksField;

        // Deserialize proof
        let proof: ProofWithPublicInputs<F, C, 2> = serde_json::from_slice(proof).map_err(|e| CryptoError::ZkVerificationFailed(format!("Proof deserialization failed: {}", e)))?;

        // Parse expected public inputs
        if public_inputs.len() < 8 {
            return Err(CryptoError::ZkVerificationFailed("Insufficient public inputs".to_string()));
        }

        let expected_public = u64::from_le_bytes(
            public_inputs[0..8]
                .try_into()
                .map_err(|_| CryptoError::ZkVerificationFailed("Invalid public input format".to_string()))?,
        );

        // Check that proof's public inputs match expected
        if proof.public_inputs.len() != 1 {
            return Err(CryptoError::ZkVerificationFailed("Invalid number of public inputs".to_string()));
        }

        let proof_public = proof.public_inputs[0].to_canonical_u64();
        if proof_public != expected_public {
            return Ok(false);
        }

        // Rebuild circuit for verification (in practice, this would be cached)
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, 2>::new(config);

        let secret_target = builder.add_virtual_target();
        let public_hash_target = builder.add_virtual_target();
        let computed_hash = builder.square(secret_target);
        builder.connect(computed_hash, public_hash_target);
        builder.register_public_input(public_hash_target);

        let circuit_data = builder.build::<C>();
        let verifier_data = &circuit_data.verifier_only;

        // Verify the proof
        circuit_data
            .verify(proof)
            .map_err(|e| CryptoError::ZkVerificationFailed(format!("Proof verification failed: {:?}", e)))?;

        Ok(true)
    }
}

/// Create default cryptographic opcode executor with all providers
pub fn create_default_crypto_executor() -> CryptographicOpcodeExecutor {
    CryptographicOpcodeExecutor::new(
        Arc::new(DefaultHashProvider),
        Arc::new(DefaultSignatureProvider),
        Arc::new(DefaultEncryptionProvider),
        Arc::new(DefaultSecureRandomProvider),
        Arc::new(Plonky2ZkProvider),
    )
}

/// Create cryptographic executor with placeholder ZK provider (for testing)
pub fn create_crypto_executor_with_placeholder_zk() -> CryptographicOpcodeExecutor {
    CryptographicOpcodeExecutor::new(
        Arc::new(DefaultHashProvider),
        Arc::new(DefaultSignatureProvider),
        Arc::new(DefaultEncryptionProvider),
        Arc::new(DefaultSecureRandomProvider),
        Arc::new(PlaceholderZkProvider),
    )
}

/// Placeholder ZK provider (for testing or when PLONKY2 is not desired)
pub struct PlaceholderZkProvider;

impl ZkProvider for PlaceholderZkProvider {
    fn generate_proof(&self, _circuit_data: &[u8], _witness: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::ZkProofFailed("ZK proofs not implemented in placeholder provider".to_string()))
    }

    fn verify_proof(&self, _proof: &[u8], _public_inputs: &[u8]) -> Result<bool, CryptoError> {
        Err(CryptoError::ZkVerificationFailed("ZK verification not implemented in placeholder provider".to_string()))
    }
}

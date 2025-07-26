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

//! Cryptographic providers for secure operations within the VM

use crate::vm::errors::VMError;
use std::sync::Arc;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Hash algorithms supported by the VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha256,
    Blake3,
    Keccak256,
}

/// Signature algorithms supported by the VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    Ed25519,
    EcdsaSecp256k1,
}

/// Encryption algorithms supported by the VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Cryptographic key material (zeroized on drop for security)
#[derive(Clone)]
pub struct CryptoKey {
    pub algorithm: SignatureAlgorithm,
    pub key_data: Vec<u8>,
}

impl CryptoKey {
    pub fn new(algorithm: SignatureAlgorithm, key_data: Vec<u8>) -> Self {
        Self { algorithm, key_data }
    }
}

impl Drop for CryptoKey {
    fn drop(&mut self) {
        self.key_data.zeroize();
    }
}

/// Encryption key material (zeroized on drop for security)
#[derive(Clone)]
pub struct EncryptionKey {
    pub algorithm: EncryptionAlgorithm,
    pub key_data: Vec<u8>,
}

impl Drop for EncryptionKey {
    fn drop(&mut self) {
        self.key_data.zeroize();
    }
}

impl EncryptionKey {
    pub fn new(algorithm: EncryptionAlgorithm, key_data: Vec<u8>) -> Self {
        Self { algorithm, key_data }
    }
}

/// Hash provider interface
pub trait HashProvider: Send + Sync {
    fn hash(&self, algorithm: HashAlgorithm, data: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

/// Signature provider interface
pub trait SignatureProvider: Send + Sync {
    fn sign(&self, private_key: &CryptoKey, data: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn verify(&self, public_key: &CryptoKey, signature: &[u8], data: &[u8]) -> Result<bool, CryptoError>;
    fn generate_keypair(&self, algorithm: SignatureAlgorithm) -> Result<(CryptoKey, CryptoKey), CryptoError>;
}

/// Encryption provider interface
pub trait EncryptionProvider: Send + Sync {
    fn encrypt(&self, key: &EncryptionKey, data: &[u8], nonce: Option<&[u8]>) -> Result<Vec<u8>, CryptoError>;
    fn decrypt(&self, key: &EncryptionKey, encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn generate_key(&self, algorithm: EncryptionAlgorithm) -> Result<EncryptionKey, CryptoError>;
}

/// Secure random provider interface
pub trait SecureRandomProvider: Send + Sync {
    fn generate_bytes(&self, count: usize) -> Result<Vec<u8>, CryptoError>;
    fn generate_u64(&self) -> Result<u64, CryptoError>;
}

/// Zero-knowledge proof provider interface (placeholder for future PLONKY2 integration)
pub trait ZkProvider: Send + Sync {
    fn generate_proof(&self, circuit_data: &[u8], witness: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn verify_proof(&self, proof: &[u8], public_inputs: &[u8]) -> Result<bool, CryptoError>;
}

/// Cryptographic errors
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Hash computation failed: {0}")]
    HashFailed(String),

    #[error("Random generation failed: {0}")]
    RandomFailed(String),

    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    #[error("ZK proof generation failed: {0}")]
    ZkProofFailed(String),

    #[error("ZK proof verification failed: {0}")]
    ZkVerificationFailed(String),
}

impl From<CryptoError> for VMError {
    fn from(err: CryptoError) -> Self {
        VMError::CryptographicError(err.to_string())
    }
}

/// Main cryptographic opcode executor
pub struct CryptographicOpcodeExecutor {
    pub hash_provider: Arc<dyn HashProvider>,
    pub signature_provider: Arc<dyn SignatureProvider>,
    pub encryption_provider: Arc<dyn EncryptionProvider>,
    pub random_provider: Arc<dyn SecureRandomProvider>,
    pub zk_provider: Arc<dyn ZkProvider>,
}

impl CryptographicOpcodeExecutor {
    pub fn new(
        hash_provider: Arc<dyn HashProvider>,
        signature_provider: Arc<dyn SignatureProvider>,
        encryption_provider: Arc<dyn EncryptionProvider>,
        random_provider: Arc<dyn SecureRandomProvider>,
        zk_provider: Arc<dyn ZkProvider>,
    ) -> Self {
        Self {
            hash_provider,
            signature_provider,
            encryption_provider,
            random_provider,
            zk_provider,
        }
    }

    pub fn execute_hash(&self, algorithm: HashAlgorithm, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.hash_provider.hash(algorithm, data)
    }

    pub fn execute_sign(&self, private_key: &CryptoKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.signature_provider.sign(private_key, data)
    }

    pub fn execute_verify(&self, public_key: &CryptoKey, signature: &[u8], data: &[u8]) -> Result<bool, CryptoError> {
        self.signature_provider.verify(public_key, signature, data)
    }

    pub fn execute_encrypt(&self, key: &EncryptionKey, data: &[u8], nonce: Option<&[u8]>) -> Result<Vec<u8>, CryptoError> {
        self.encryption_provider.encrypt(key, data, nonce)
    }

    pub fn execute_decrypt(&self, key: &EncryptionKey, encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.encryption_provider.decrypt(key, encrypted_data)
    }

    pub fn execute_secure_random(&self, byte_count: usize) -> Result<Vec<u8>, CryptoError> {
        self.random_provider.generate_bytes(byte_count)
    }

    pub fn execute_zk_proof(&self, circuit_data: &[u8], witness: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.zk_provider.generate_proof(circuit_data, witness)
    }

    pub fn execute_zk_verify(&self, proof: &[u8], public_inputs: &[u8]) -> Result<bool, CryptoError> {
        self.zk_provider.verify_proof(proof, public_inputs)
    }
}

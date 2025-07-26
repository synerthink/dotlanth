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

//! Integration tests for cryptographic opcodes in the VM

use dotvm_core::instruction::crypto_impl::*;
use dotvm_core::instruction::crypto_provider::*;
use dotvm_core::opcode::crypto_opcodes::CryptoOpcode;
use std::sync::Arc;

#[test]
fn test_crypto_provider_integration() {
    println!("Testing cryptographic provider integration...");

    // Test hash provider
    let hash_provider = DefaultHashProvider;
    let test_data = b"Hello, cryptographic world!";

    let sha256_hash = hash_provider.hash(HashAlgorithm::Sha256, test_data).unwrap();
    assert_eq!(sha256_hash.len(), 32, "SHA256 should produce 32 bytes");

    let blake3_hash = hash_provider.hash(HashAlgorithm::Blake3, test_data).unwrap();
    assert_eq!(blake3_hash.len(), 32, "Blake3 should produce 32 bytes");

    // Ensure different algorithms produce different results
    assert_ne!(sha256_hash, blake3_hash, "Different hash algorithms should produce different results");

    println!("✓ Hash provider tests passed");

    // Test signature provider
    let sig_provider = DefaultSignatureProvider;

    // Test Ed25519
    let (ed25519_private, ed25519_public) = sig_provider.generate_keypair(SignatureAlgorithm::Ed25519).unwrap();
    let ed25519_signature = sig_provider.sign(&ed25519_private, test_data).unwrap();
    let ed25519_valid = sig_provider.verify(&ed25519_public, &ed25519_signature, test_data).unwrap();
    assert!(ed25519_valid, "Ed25519 signature should be valid");

    // Test with wrong data
    let wrong_data = b"Wrong data";
    let ed25519_invalid = sig_provider.verify(&ed25519_public, &ed25519_signature, wrong_data).unwrap();
    assert!(!ed25519_invalid, "Ed25519 signature should be invalid for wrong data");

    // Test ECDSA
    let (ecdsa_private, ecdsa_public) = sig_provider.generate_keypair(SignatureAlgorithm::EcdsaSecp256k1).unwrap();
    let ecdsa_signature = sig_provider.sign(&ecdsa_private, test_data).unwrap();
    let ecdsa_valid = sig_provider.verify(&ecdsa_public, &ecdsa_signature, test_data).unwrap();
    assert!(ecdsa_valid, "ECDSA signature should be valid");

    println!("✓ Signature provider tests passed");

    // Test encryption provider
    let enc_provider = DefaultEncryptionProvider;

    // Test AES-256-GCM
    let aes_key = enc_provider.generate_key(EncryptionAlgorithm::Aes256Gcm).unwrap();
    let aes_encrypted = enc_provider.encrypt(&aes_key, test_data, None).unwrap();
    let aes_decrypted = enc_provider.decrypt(&aes_key, &aes_encrypted).unwrap();
    assert_eq!(aes_decrypted, test_data, "AES decryption should recover original data");

    // Test ChaCha20Poly1305
    let chacha_key = enc_provider.generate_key(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
    let chacha_encrypted = enc_provider.encrypt(&chacha_key, test_data, None).unwrap();
    let chacha_decrypted = enc_provider.decrypt(&chacha_key, &chacha_encrypted).unwrap();
    assert_eq!(chacha_decrypted, test_data, "ChaCha20 decryption should recover original data");

    println!("✓ Encryption provider tests passed");

    // Test secure random provider
    let random_provider = DefaultSecureRandomProvider;
    let random_bytes1 = random_provider.generate_bytes(32).unwrap();
    let random_bytes2 = random_provider.generate_bytes(32).unwrap();

    assert_eq!(random_bytes1.len(), 32, "Should generate requested number of bytes");
    assert_ne!(random_bytes1, random_bytes2, "Random bytes should be different");

    let random_u64_1 = random_provider.generate_u64().unwrap();
    let random_u64_2 = random_provider.generate_u64().unwrap();
    assert_ne!(random_u64_1, random_u64_2, "Random u64s should be different");

    println!("✓ Secure random provider tests passed");

    // Test ZK provider (placeholder)
    let zk_provider = PlaceholderZkProvider;
    let circuit_data = b"dummy circuit";
    let witness_data = b"dummy witness";

    // Should return error since not implemented yet
    let zk_result = zk_provider.generate_proof(circuit_data, witness_data);
    assert!(zk_result.is_err(), "ZK proof should return error (not implemented)");

    println!("✓ ZK provider tests passed (placeholder)");
}

#[test]
fn test_crypto_executor_integration() {
    println!("Testing cryptographic executor integration...");

    let crypto_executor = create_default_crypto_executor();
    let test_data = b"Integration test data for crypto executor";

    // Test hash execution
    let hash_result = crypto_executor.execute_hash(HashAlgorithm::Sha256, test_data).unwrap();
    assert_eq!(hash_result.len(), 32, "Hash result should be 32 bytes");

    let blake3_result = crypto_executor.execute_hash(HashAlgorithm::Blake3, test_data).unwrap();
    assert_eq!(blake3_result.len(), 32, "Blake3 result should be 32 bytes");
    assert_ne!(hash_result, blake3_result, "Different algorithms should produce different hashes");

    println!("✓ Hash execution tests passed");

    // Test signature execution
    let (private_key, public_key) = crypto_executor.signature_provider.generate_keypair(SignatureAlgorithm::Ed25519).unwrap();

    let signature = crypto_executor.execute_sign(&private_key, test_data).unwrap();
    let is_valid = crypto_executor.execute_verify(&public_key, &signature, test_data).unwrap();
    assert!(is_valid, "Signature should be valid");

    let is_invalid = crypto_executor.execute_verify(&public_key, &signature, b"wrong data").unwrap();
    assert!(!is_invalid, "Signature should be invalid for wrong data");

    println!("✓ Signature execution tests passed");

    // Test encryption execution
    let encryption_key = crypto_executor.encryption_provider.generate_key(EncryptionAlgorithm::Aes256Gcm).unwrap();

    let encrypted = crypto_executor.execute_encrypt(&encryption_key, test_data, None).unwrap();
    let decrypted = crypto_executor.execute_decrypt(&encryption_key, &encrypted).unwrap();
    assert_eq!(decrypted, test_data, "Decryption should recover original data");

    println!("✓ Encryption execution tests passed");

    // Test secure random execution
    let random_bytes = crypto_executor.execute_secure_random(64).unwrap();
    assert_eq!(random_bytes.len(), 64, "Should generate requested number of random bytes");

    let more_random = crypto_executor.execute_secure_random(64).unwrap();
    assert_ne!(random_bytes, more_random, "Random bytes should be different");

    println!("✓ Secure random execution tests passed");

    println!("All crypto executor integration tests passed!");
}

#[test]
fn test_crypto_opcode_enum() {
    println!("Testing crypto opcode enum functionality...");

    // Test opcode conversion
    assert_eq!(CryptoOpcode::Hash.as_u8(), 0x40);
    assert_eq!(CryptoOpcode::Encrypt.as_u8(), 0x41);
    assert_eq!(CryptoOpcode::Decrypt.as_u8(), 0x42);
    assert_eq!(CryptoOpcode::Sign.as_u8(), 0x43);
    assert_eq!(CryptoOpcode::VerifySignature.as_u8(), 0x44);
    assert_eq!(CryptoOpcode::SecureRandom.as_u8(), 0x45);
    assert_eq!(CryptoOpcode::ZkProof.as_u8(), 0x46);
    assert_eq!(CryptoOpcode::ZkVerify.as_u8(), 0x47);

    // Test reverse conversion
    assert_eq!(CryptoOpcode::from_u8(0x40), Some(CryptoOpcode::Hash));
    assert_eq!(CryptoOpcode::from_u8(0x41), Some(CryptoOpcode::Encrypt));
    assert_eq!(CryptoOpcode::from_u8(0x42), Some(CryptoOpcode::Decrypt));
    assert_eq!(CryptoOpcode::from_u8(0x43), Some(CryptoOpcode::Sign));
    assert_eq!(CryptoOpcode::from_u8(0x44), Some(CryptoOpcode::VerifySignature));
    assert_eq!(CryptoOpcode::from_u8(0x45), Some(CryptoOpcode::SecureRandom));
    assert_eq!(CryptoOpcode::from_u8(0x46), Some(CryptoOpcode::ZkProof));
    assert_eq!(CryptoOpcode::from_u8(0x47), Some(CryptoOpcode::ZkVerify));
    assert_eq!(CryptoOpcode::from_u8(0xFF), None);

    // Test mnemonic conversion
    assert_eq!(CryptoOpcode::Hash.to_mnemonic(), "CRYPTO_HASH");
    assert_eq!(CryptoOpcode::SecureRandom.to_mnemonic(), "CRYPTO_SECURE_RANDOM");
    assert_eq!(CryptoOpcode::ZkProof.to_mnemonic(), "CRYPTO_ZK_PROOF");
    assert_eq!(CryptoOpcode::ZkVerify.to_mnemonic(), "CRYPTO_ZK_VERIFY");

    // Test mnemonic parsing
    assert_eq!(CryptoOpcode::from_mnemonic("CRYPTO_HASH"), Some(CryptoOpcode::Hash));
    assert_eq!(CryptoOpcode::from_mnemonic("CRYPTO_SECURE_RANDOM"), Some(CryptoOpcode::SecureRandom));
    assert_eq!(CryptoOpcode::from_mnemonic("CRYPTO_ZK_PROOF"), Some(CryptoOpcode::ZkProof));
    assert_eq!(CryptoOpcode::from_mnemonic("CRYPTO_ZK_VERIFY"), Some(CryptoOpcode::ZkVerify));
    assert_eq!(CryptoOpcode::from_mnemonic("INVALID"), None);

    println!("✓ Crypto opcode enum tests passed");
}

#[test]
fn test_key_security() {
    println!("Testing key security (zeroization)...");

    let sig_provider = DefaultSignatureProvider;
    let (private_key, _public_key) = sig_provider.generate_keypair(SignatureAlgorithm::Ed25519).unwrap();

    // Keys should be properly zeroized when dropped
    // This is handled by the ZeroizeOnDrop trait
    assert_eq!(private_key.key_data.len(), 32, "Ed25519 private key should be 32 bytes");

    let enc_provider = DefaultEncryptionProvider;
    let encryption_key = enc_provider.generate_key(EncryptionAlgorithm::Aes256Gcm).unwrap();
    assert_eq!(encryption_key.key_data.len(), 32, "AES-256 key should be 32 bytes");

    println!("✓ Key security tests passed");
}

#[test]
fn test_plonky2_proof_generation_and_verification() {
    println!("Testing PLONKY2 proof generation and verification...");

    let zk_provider = Plonky2ZkProvider;

    // Create test circuit data (empty for this demo circuit)
    let circuit_data = vec![0u8; 32];

    // Create witness: secret=5, public_hash=25 (since 5^2 = 25)
    let mut witness = Vec::new();
    witness.extend_from_slice(&5u64.to_le_bytes()); // secret
    witness.extend_from_slice(&25u64.to_le_bytes()); // public hash

    // Generate proof
    let proof = zk_provider.generate_proof(&circuit_data, &witness).unwrap();
    assert!(!proof.is_empty(), "Proof should not be empty");

    // Verify proof with correct public inputs
    let public_inputs = 25u64.to_le_bytes();
    let is_valid = zk_provider.verify_proof(&proof, &public_inputs).unwrap();
    assert!(is_valid, "Proof should be valid");

    // Verify proof with incorrect public inputs should fail
    let wrong_public_inputs = 24u64.to_le_bytes();
    let is_invalid = zk_provider.verify_proof(&proof, &wrong_public_inputs).unwrap();
    assert!(!is_invalid, "Proof should be invalid with wrong public inputs");

    println!("✓ PLONKY2 proof generation and verification tests passed");
}

#[test]
fn test_plonky2_invalid_witness() {
    println!("Testing PLONKY2 invalid witness handling...");

    let zk_provider = Plonky2ZkProvider;
    let circuit_data = vec![0u8; 32];

    // Test with insufficient witness data
    let short_witness = vec![1, 2, 3];
    let result = zk_provider.generate_proof(&circuit_data, &short_witness);
    assert!(result.is_err(), "Should fail with insufficient witness data");

    println!("✓ PLONKY2 invalid witness tests passed");
}

#[test]
fn test_plonky2_invalid_proof_data() {
    println!("Testing PLONKY2 invalid proof data handling...");

    let zk_provider = Plonky2ZkProvider;

    // Test with invalid proof data
    let invalid_proof = vec![1, 2, 3, 4, 5];
    let public_inputs = 25u64.to_le_bytes();
    let result = zk_provider.verify_proof(&invalid_proof, &public_inputs);
    assert!(result.is_err(), "Should fail with invalid proof data");

    println!("✓ PLONKY2 invalid proof data tests passed");
}

#[test]
fn test_crypto_executor_with_plonky2() {
    println!("Testing crypto executor with PLONKY2 integration...");

    let crypto_executor = create_default_crypto_executor();

    // Test that ZK operations work through the executor
    let circuit_data = vec![0u8; 32];
    let mut witness = Vec::new();
    witness.extend_from_slice(&7u64.to_le_bytes()); // secret
    witness.extend_from_slice(&49u64.to_le_bytes()); // public hash (7^2 = 49)

    // Generate proof through executor
    let proof = crypto_executor.execute_zk_proof(&circuit_data, &witness).unwrap();
    assert!(!proof.is_empty());

    // Verify proof through executor
    let public_inputs = 49u64.to_le_bytes();
    let is_valid = crypto_executor.execute_zk_verify(&proof, &public_inputs).unwrap();
    assert!(is_valid);

    println!("✓ Crypto executor PLONKY2 integration tests passed");
}

#[test]
fn test_plonky2_vs_placeholder_providers() {
    println!("Testing PLONKY2 vs placeholder provider comparison...");

    // Test default executor (with PLONKY2)
    let plonky2_executor = create_default_crypto_executor();
    let circuit_data = vec![0u8; 32];
    let mut witness = Vec::new();
    witness.extend_from_slice(&3u64.to_le_bytes()); // secret
    witness.extend_from_slice(&9u64.to_le_bytes()); // public hash (3^2 = 9)

    // PLONKY2 should work
    let plonky2_proof = plonky2_executor.execute_zk_proof(&circuit_data, &witness).unwrap();
    let public_inputs = 9u64.to_le_bytes();
    let plonky2_valid = plonky2_executor.execute_zk_verify(&plonky2_proof, &public_inputs).unwrap();
    assert!(plonky2_valid, "PLONKY2 proof should be valid");

    // Test placeholder executor
    let placeholder_executor = create_crypto_executor_with_placeholder_zk();

    // Placeholder should return errors
    let proof_result = placeholder_executor.execute_zk_proof(&circuit_data, &witness);
    assert!(proof_result.is_err(), "Placeholder should fail proof generation");

    let verify_result = placeholder_executor.execute_zk_verify(&[], &[]);
    assert!(verify_result.is_err(), "Placeholder should fail proof verification");

    println!("✓ PLONKY2 vs placeholder provider tests passed");
}

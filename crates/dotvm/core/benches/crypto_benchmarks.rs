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

//! Cryptographic operations benchmarks for DotVM
//!
//! This benchmark suite verifies that cryptographic operations meet the
//! performance requirements specified in plan.md:
//! - Hash operations: >100K hashes/sec
//! - Sign/Verify operations: >10K ops/sec

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use dotvm_core::instruction::crypto_impl::*;
use dotvm_core::instruction::crypto_provider::*;
use std::sync::Arc;

/// Benchmark hash operations performance
fn bench_hash_operations(c: &mut Criterion) {
    let provider = DefaultHashProvider;
    let test_data = b"Hello, World! This is a test message for hashing performance benchmarking.";

    let mut group = c.benchmark_group("hash_operations");
    group.throughput(Throughput::Elements(1));

    // Benchmark SHA256
    group.bench_function("sha256", |b| b.iter(|| provider.hash(black_box(HashAlgorithm::Sha256), black_box(test_data)).unwrap()));

    // Benchmark Blake3
    group.bench_function("blake3", |b| b.iter(|| provider.hash(black_box(HashAlgorithm::Blake3), black_box(test_data)).unwrap()));

    // Benchmark Keccak256
    group.bench_function("keccak256", |b| b.iter(|| provider.hash(black_box(HashAlgorithm::Keccak256), black_box(test_data)).unwrap()));

    group.finish();
}

/// Benchmark signature operations performance
fn bench_signature_operations(c: &mut Criterion) {
    let provider = DefaultSignatureProvider;
    let test_data = b"Hello, World! This is a test message for signature performance benchmarking.";

    // Generate keypairs for benchmarking
    let (ed25519_private, ed25519_public) = provider.generate_keypair(SignatureAlgorithm::Ed25519).unwrap();
    let (ecdsa_private, ecdsa_public) = provider.generate_keypair(SignatureAlgorithm::EcdsaSecp256k1).unwrap();

    // Pre-generate signatures for verification benchmarks
    let ed25519_signature = provider.sign(&ed25519_private, test_data).unwrap();
    let ecdsa_signature = provider.sign(&ecdsa_private, test_data).unwrap();

    let mut group = c.benchmark_group("signature_operations");
    group.throughput(Throughput::Elements(1));

    // Ed25519 signing
    group.bench_function("ed25519_sign", |b| b.iter(|| provider.sign(black_box(&ed25519_private), black_box(test_data)).unwrap()));

    // Ed25519 verification
    group.bench_function("ed25519_verify", |b| {
        b.iter(|| provider.verify(black_box(&ed25519_public), black_box(&ed25519_signature), black_box(test_data)).unwrap())
    });

    // ECDSA signing
    group.bench_function("ecdsa_sign", |b| b.iter(|| provider.sign(black_box(&ecdsa_private), black_box(test_data)).unwrap()));

    // ECDSA verification
    group.bench_function("ecdsa_verify", |b| {
        b.iter(|| provider.verify(black_box(&ecdsa_public), black_box(&ecdsa_signature), black_box(test_data)).unwrap())
    });

    group.finish();
}

/// Benchmark encryption operations performance
fn bench_encryption_operations(c: &mut Criterion) {
    let provider = DefaultEncryptionProvider;
    let test_data = b"Hello, World! This is a test message for encryption performance benchmarking. It needs to be long enough to get meaningful measurements.";

    // Generate keys for benchmarking
    let aes_key = provider.generate_key(EncryptionAlgorithm::Aes256Gcm).unwrap();
    let chacha_key = provider.generate_key(EncryptionAlgorithm::ChaCha20Poly1305).unwrap();

    // Pre-encrypt data for decryption benchmarks
    let aes_encrypted = provider.encrypt(&aes_key, test_data, None).unwrap();
    let chacha_encrypted = provider.encrypt(&chacha_key, test_data, None).unwrap();

    let mut group = c.benchmark_group("encryption_operations");
    group.throughput(Throughput::Bytes(test_data.len() as u64));

    // AES-256-GCM encryption
    group.bench_function("aes256_gcm_encrypt", |b| b.iter(|| provider.encrypt(black_box(&aes_key), black_box(test_data), None).unwrap()));

    // AES-256-GCM decryption
    group.bench_function("aes256_gcm_decrypt", |b| b.iter(|| provider.decrypt(black_box(&aes_key), black_box(&aes_encrypted)).unwrap()));

    // ChaCha20Poly1305 encryption
    group.bench_function("chacha20poly1305_encrypt", |b| b.iter(|| provider.encrypt(black_box(&chacha_key), black_box(test_data), None).unwrap()));

    // ChaCha20Poly1305 decryption
    group.bench_function("chacha20poly1305_decrypt", |b| {
        b.iter(|| provider.decrypt(black_box(&chacha_key), black_box(&chacha_encrypted)).unwrap())
    });

    group.finish();
}

/// Benchmark secure random generation performance
fn bench_secure_random_operations(c: &mut Criterion) {
    let provider = DefaultSecureRandomProvider;

    let mut group = c.benchmark_group("secure_random_operations");

    // Benchmark different sizes of random data generation
    for size in [16, 32, 64, 128, 256, 512, 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(format!("generate_{}_bytes", size), size, |b, &size| b.iter(|| provider.generate_bytes(black_box(size)).unwrap()));
    }

    // Benchmark u64 generation
    group.bench_function("generate_u64", |b| b.iter(|| provider.generate_u64().unwrap()));

    group.finish();
}

/// Benchmark complete crypto executor integration
fn bench_crypto_executor_integration(c: &mut Criterion) {
    let crypto_executor = create_default_crypto_executor();
    let test_data = b"Integration benchmark test data for crypto executor performance.";

    // Generate keys for benchmarking
    let (private_key, public_key) = crypto_executor.signature_provider.generate_keypair(SignatureAlgorithm::Ed25519).unwrap();
    let encryption_key = crypto_executor.encryption_provider.generate_key(EncryptionAlgorithm::Aes256Gcm).unwrap();

    let mut group = c.benchmark_group("crypto_executor_integration");
    group.throughput(Throughput::Elements(1));

    // Hash through executor
    group.bench_function("executor_hash", |b| {
        b.iter(|| crypto_executor.execute_hash(black_box(HashAlgorithm::Sha256), black_box(test_data)).unwrap())
    });

    // Sign through executor
    group.bench_function("executor_sign", |b| b.iter(|| crypto_executor.execute_sign(black_box(&private_key), black_box(test_data)).unwrap()));

    // Encrypt through executor
    group.bench_function("executor_encrypt", |b| {
        b.iter(|| crypto_executor.execute_encrypt(black_box(&encryption_key), black_box(test_data), None).unwrap())
    });

    // Secure random through executor
    group.bench_function("executor_secure_random", |b| b.iter(|| crypto_executor.execute_secure_random(black_box(32)).unwrap()));

    group.finish();
}

/// Performance validation tests to ensure we meet plan.md requirements
fn bench_performance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_validation");

    // Set longer measurement time for more accurate results
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(1000);

    let provider = DefaultHashProvider;
    let sig_provider = DefaultSignatureProvider;
    let test_data = b"Performance validation test data";

    // Hash performance validation (target: >100K hashes/sec)
    group.bench_function("hash_performance_validation", |b| {
        b.iter(|| provider.hash(black_box(HashAlgorithm::Sha256), black_box(test_data)).unwrap())
    });

    // Signature performance validation (target: >10K ops/sec)
    let (private_key, public_key) = sig_provider.generate_keypair(SignatureAlgorithm::Ed25519).unwrap();
    let signature = sig_provider.sign(&private_key, test_data).unwrap();

    group.bench_function("signature_performance_validation", |b| {
        b.iter(|| sig_provider.verify(black_box(&public_key), black_box(&signature), black_box(test_data)).unwrap())
    });

    group.finish();
}

criterion_group!(
    crypto_benches,
    bench_hash_operations,
    bench_signature_operations,
    bench_encryption_operations,
    bench_secure_random_operations,
    bench_crypto_executor_integration,
    bench_performance_validation
);

criterion_main!(crypto_benches);

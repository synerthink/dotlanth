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

use crate::instruction::crypto_provider::*;
use crate::instruction::instruction::{ExecutorInterface, Instruction};
use crate::vm::errors::VMError;
use std::sync::Arc;

pub struct HashInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
    algorithm: HashAlgorithm,
}

impl HashInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>, algorithm: HashAlgorithm) -> Self {
        Self { crypto_executor, algorithm }
    }
}

impl Instruction for HashInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [data_length, data_ptr] -> [hash_length, hash_ptr]
        let data_length = executor.pop_operand()? as usize;
        let data_ptr = executor.pop_operand()? as usize;

        // Read data from memory
        let mut data = Vec::with_capacity(data_length);
        let memory_manager = executor.get_memory_manager_mut();
        for i in 0..data_length {
            let byte = memory_manager.load(data_ptr + i)?;
            data.push(byte);
        }

        // Compute hash
        let hash_result = self.crypto_executor.execute_hash(self.algorithm, &data)?;

        // Allocate memory for result
        let result_handle = memory_manager.allocate(hash_result.len())?;
        let result_ptr = result_handle.address();

        // Store hash result in memory
        for (i, &byte) in hash_result.iter().enumerate() {
            memory_manager.store(result_ptr + i, byte)?;
        }

        // Push result pointer and length onto stack
        executor.push_operand(result_ptr as f64);
        executor.push_operand(hash_result.len() as f64);

        Ok(())
    }
}

pub struct EncryptInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
    algorithm: EncryptionAlgorithm,
}

impl EncryptInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>, algorithm: EncryptionAlgorithm) -> Self {
        Self { crypto_executor, algorithm }
    }
}

impl Instruction for EncryptInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [key_ptr, key_len, data_ptr, data_len, nonce_ptr, nonce_len] -> [result_ptr, result_len]
        let nonce_len = executor.pop_operand()? as usize;
        let nonce_ptr = executor.pop_operand()? as usize;
        let data_len = executor.pop_operand()? as usize;
        let data_ptr = executor.pop_operand()? as usize;
        let key_len = executor.pop_operand()? as usize;
        let key_ptr = executor.pop_operand()? as usize;

        let memory_manager = executor.get_memory_manager_mut();

        // Read key from memory
        let mut key_data = Vec::with_capacity(key_len);
        for i in 0..key_len {
            key_data.push(memory_manager.load(key_ptr + i)?);
        }
        let encryption_key = EncryptionKey::new(self.algorithm, key_data);

        // Read data from memory
        let mut data = Vec::with_capacity(data_len);
        for i in 0..data_len {
            data.push(memory_manager.load(data_ptr + i)?);
        }

        // Read nonce if provided
        let nonce = if nonce_len > 0 {
            let mut nonce_data = Vec::with_capacity(nonce_len);
            for i in 0..nonce_len {
                nonce_data.push(memory_manager.load(nonce_ptr + i)?);
            }
            Some(nonce_data)
        } else {
            None
        };

        // Encrypt data
        let encrypted_result = self.crypto_executor.execute_encrypt(&encryption_key, &data, nonce.as_deref())?;

        // Allocate memory for result
        let result_handle = memory_manager.allocate(encrypted_result.len())?;
        let result_ptr = result_handle.address();

        // Store encrypted result in memory
        for (i, &byte) in encrypted_result.iter().enumerate() {
            memory_manager.store(result_ptr + i, byte)?;
        }

        // Push result pointer and length onto stack
        executor.push_operand(result_ptr as f64);
        executor.push_operand(encrypted_result.len() as f64);

        Ok(())
    }
}

pub struct DecryptInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
    algorithm: EncryptionAlgorithm,
}

impl DecryptInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>, algorithm: EncryptionAlgorithm) -> Self {
        Self { crypto_executor, algorithm }
    }
}

impl Instruction for DecryptInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [key_ptr, key_len, data_ptr, data_len] -> [result_ptr, result_len]
        let data_len = executor.pop_operand()? as usize;
        let data_ptr = executor.pop_operand()? as usize;
        let key_len = executor.pop_operand()? as usize;
        let key_ptr = executor.pop_operand()? as usize;

        let memory_manager = executor.get_memory_manager_mut();

        // Read key from memory
        let mut key_data = Vec::with_capacity(key_len);
        for i in 0..key_len {
            key_data.push(memory_manager.load(key_ptr + i)?);
        }
        let encryption_key = EncryptionKey::new(self.algorithm, key_data);

        // Read encrypted data from memory
        let mut encrypted_data = Vec::with_capacity(data_len);
        for i in 0..data_len {
            encrypted_data.push(memory_manager.load(data_ptr + i)?);
        }

        // Decrypt data
        let decrypted_result = self.crypto_executor.execute_decrypt(&encryption_key, &encrypted_data)?;

        // Allocate memory for result
        let result_handle = memory_manager.allocate(decrypted_result.len())?;
        let result_ptr = result_handle.address();

        // Store decrypted result in memory
        for (i, &byte) in decrypted_result.iter().enumerate() {
            memory_manager.store(result_ptr + i, byte)?;
        }

        // Push result pointer and length onto stack
        executor.push_operand(result_ptr as f64);
        executor.push_operand(decrypted_result.len() as f64);

        Ok(())
    }
}

pub struct SignInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
    algorithm: SignatureAlgorithm,
}

impl SignInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>, algorithm: SignatureAlgorithm) -> Self {
        Self { crypto_executor, algorithm }
    }
}

impl Instruction for SignInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [private_key_ptr, private_key_len, data_ptr, data_len] -> [signature_ptr, signature_len]
        let data_len = executor.pop_operand()? as usize;
        let data_ptr = executor.pop_operand()? as usize;
        let key_len = executor.pop_operand()? as usize;
        let key_ptr = executor.pop_operand()? as usize;

        let memory_manager = executor.get_memory_manager_mut();

        // Read private key from memory
        let mut key_data = Vec::with_capacity(key_len);
        for i in 0..key_len {
            key_data.push(memory_manager.load(key_ptr + i)?);
        }
        let private_key = CryptoKey::new(self.algorithm, key_data);

        // Read data from memory
        let mut data = Vec::with_capacity(data_len);
        for i in 0..data_len {
            data.push(memory_manager.load(data_ptr + i)?);
        }

        // Sign data
        let signature = self.crypto_executor.execute_sign(&private_key, &data)?;

        // Allocate memory for signature
        let result_handle = memory_manager.allocate(signature.len())?;
        let result_ptr = result_handle.address();

        // Store signature in memory
        for (i, &byte) in signature.iter().enumerate() {
            memory_manager.store(result_ptr + i, byte)?;
        }

        // Push result pointer and length onto stack
        executor.push_operand(result_ptr as f64);
        executor.push_operand(signature.len() as f64);

        Ok(())
    }
}

pub struct VerifySignatureInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
    algorithm: SignatureAlgorithm,
}

impl VerifySignatureInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>, algorithm: SignatureAlgorithm) -> Self {
        Self { crypto_executor, algorithm }
    }
}

impl Instruction for VerifySignatureInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [public_key_ptr, public_key_len, data_ptr, data_len, signature_ptr, signature_len] -> [valid]
        let signature_len = executor.pop_operand()? as usize;
        let signature_ptr = executor.pop_operand()? as usize;
        let data_len = executor.pop_operand()? as usize;
        let data_ptr = executor.pop_operand()? as usize;
        let key_len = executor.pop_operand()? as usize;
        let key_ptr = executor.pop_operand()? as usize;

        let memory_manager = executor.get_memory_manager_mut();

        // Read public key from memory
        let mut key_data = Vec::with_capacity(key_len);
        for i in 0..key_len {
            key_data.push(memory_manager.load(key_ptr + i)?);
        }
        let public_key = CryptoKey::new(self.algorithm, key_data);

        // Read data from memory
        let mut data = Vec::with_capacity(data_len);
        for i in 0..data_len {
            data.push(memory_manager.load(data_ptr + i)?);
        }

        // Read signature from memory
        let mut signature = Vec::with_capacity(signature_len);
        for i in 0..signature_len {
            signature.push(memory_manager.load(signature_ptr + i)?);
        }

        // Verify signature
        let is_valid = self.crypto_executor.execute_verify(&public_key, &signature, &data)?;

        // Push result onto stack (1.0 for valid, 0.0 for invalid)
        executor.push_operand(if is_valid { 1.0 } else { 0.0 });

        Ok(())
    }
}

// New instructions for SecureRandom and ZK operations

pub struct SecureRandomInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
}

impl SecureRandomInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>) -> Self {
        Self { crypto_executor }
    }
}

impl Instruction for SecureRandomInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [byte_count] -> [random_ptr, random_len]
        let byte_count = executor.pop_operand()? as usize;

        // Generate secure random bytes
        let random_bytes = self.crypto_executor.execute_secure_random(byte_count)?;

        // Allocate memory for random data
        let memory_manager = executor.get_memory_manager_mut();
        let result_handle = memory_manager.allocate(random_bytes.len())?;
        let result_ptr = result_handle.address();

        // Store random bytes in memory
        for (i, &byte) in random_bytes.iter().enumerate() {
            memory_manager.store(result_ptr + i, byte)?;
        }

        // Push result pointer and length onto stack
        executor.push_operand(result_ptr as f64);
        executor.push_operand(random_bytes.len() as f64);

        Ok(())
    }
}

pub struct ZkProofInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
}

impl ZkProofInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>) -> Self {
        Self { crypto_executor }
    }
}

impl Instruction for ZkProofInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [circuit_ptr, circuit_len, witness_ptr, witness_len] -> [proof_ptr, proof_len]
        let witness_len = executor.pop_operand()? as usize;
        let witness_ptr = executor.pop_operand()? as usize;
        let circuit_len = executor.pop_operand()? as usize;
        let circuit_ptr = executor.pop_operand()? as usize;

        let memory_manager = executor.get_memory_manager_mut();

        // Read circuit data from memory
        let mut circuit_data = Vec::with_capacity(circuit_len);
        for i in 0..circuit_len {
            circuit_data.push(memory_manager.load(circuit_ptr + i)?);
        }

        // Read witness data from memory
        let mut witness_data = Vec::with_capacity(witness_len);
        for i in 0..witness_len {
            witness_data.push(memory_manager.load(witness_ptr + i)?);
        }

        // Generate ZK proof
        let proof = self.crypto_executor.execute_zk_proof(&circuit_data, &witness_data)?;

        // Allocate memory for proof
        let result_handle = memory_manager.allocate(proof.len())?;
        let result_ptr = result_handle.address();

        // Store proof in memory
        for (i, &byte) in proof.iter().enumerate() {
            memory_manager.store(result_ptr + i, byte)?;
        }

        // Push result pointer and length onto stack
        executor.push_operand(result_ptr as f64);
        executor.push_operand(proof.len() as f64);

        Ok(())
    }
}

pub struct ZkVerifyInstruction {
    crypto_executor: Arc<CryptographicOpcodeExecutor>,
}

impl ZkVerifyInstruction {
    pub fn new(crypto_executor: Arc<CryptographicOpcodeExecutor>) -> Self {
        Self { crypto_executor }
    }
}

impl Instruction for ZkVerifyInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // Stack: [proof_ptr, proof_len, public_inputs_ptr, public_inputs_len] -> [valid]
        let public_inputs_len = executor.pop_operand()? as usize;
        let public_inputs_ptr = executor.pop_operand()? as usize;
        let proof_len = executor.pop_operand()? as usize;
        let proof_ptr = executor.pop_operand()? as usize;

        let memory_manager = executor.get_memory_manager_mut();

        // Read proof from memory
        let mut proof = Vec::with_capacity(proof_len);
        for i in 0..proof_len {
            proof.push(memory_manager.load(proof_ptr + i)?);
        }

        // Read public inputs from memory
        let mut public_inputs = Vec::with_capacity(public_inputs_len);
        for i in 0..public_inputs_len {
            public_inputs.push(memory_manager.load(public_inputs_ptr + i)?);
        }

        // Verify ZK proof
        let is_valid = self.crypto_executor.execute_zk_verify(&proof, &public_inputs)?;

        // Push result onto stack (1.0 for valid, 0.0 for invalid)
        executor.push_operand(if is_valid { 1.0 } else { 0.0 });

        Ok(())
    }
}

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

use crate::instruction::instruction::{ExecutorInterface, Instruction};
use crate::vm::errors::VMError;

use sha2::{Digest, Sha256};

pub struct HashInstruction;

impl Default for HashInstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl HashInstruction {
    pub fn new() -> Self {
        HashInstruction
    }
}

impl Instruction for HashInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // In this demonstration, we assume a hardcoded input.
        // In a complete implementation the input could be read from memory.
        let input = String::from("sample input");
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        // Convert the first 8 bytes of the hash to a u64 and push it as a f64.
        let hash_val = u64::from_be_bytes(result[0..8].try_into().unwrap());
        executor.push_operand(hash_val as f64);
        Ok(())
    }
}

pub struct EncryptInstruction;

impl Default for EncryptInstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl EncryptInstruction {
    pub fn new() -> Self {
        EncryptInstruction
    }
}

impl Instruction for EncryptInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // For demonstration, perform a simple XOR encryption.
        // The operator expects two numbers: key and plaintext.
        let plaintext = executor.pop_operand()? as u64;
        let key = executor.pop_operand()? as u64;
        // XOR provides an extremely simple and insecure “encryption.”
        let ciphertext = plaintext ^ key;
        executor.push_operand(ciphertext as f64);
        Ok(())
    }
}

pub struct DecryptInstruction;

impl Default for DecryptInstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl DecryptInstruction {
    pub fn new() -> Self {
        DecryptInstruction
    }
}

impl Instruction for DecryptInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // For demonstration, decryption is identical to encryption with XOR.
        let ciphertext = executor.pop_operand()? as u64;
        let key = executor.pop_operand()? as u64;
        let plaintext = ciphertext ^ key;
        executor.push_operand(plaintext as f64);
        Ok(())
    }
}

pub struct SignInstruction;

impl Default for SignInstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl SignInstruction {
    pub fn new() -> Self {
        SignInstruction
    }
}

impl Instruction for SignInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // For demonstration, use a dummy signature algorithm.
        // Expect that the operand stack holds [private_key, message].
        let message = executor.pop_operand()? as u64;
        let private_key = executor.pop_operand()? as u64;
        // A dummy “signature” computed by multiplying the message and private key.
        let signature = message.wrapping_mul(private_key);
        executor.push_operand(signature as f64);
        Ok(())
    }
}

pub struct VerifySignatureInstruction;

impl Default for VerifySignatureInstruction {
    fn default() -> Self {
        Self::new()
    }
}

impl VerifySignatureInstruction {
    pub fn new() -> Self {
        VerifySignatureInstruction
    }
}

impl Instruction for VerifySignatureInstruction {
    fn execute(&self, executor: &mut dyn ExecutorInterface) -> Result<(), VMError> {
        // For demonstration, assume the operand stack holds [public_key, message, signature].
        let signature = executor.pop_operand()? as u64;
        let message = executor.pop_operand()? as u64;
        let public_key = executor.pop_operand()? as u64;
        // Dummy verification: if (message * public_key) equals the signature, the signature is valid.
        let valid = public_key != 0 && message.wrapping_mul(public_key) == signature;
        // Push 1.0 for true (valid) or 0.0 for false.
        executor.push_operand(if valid { 1.0 } else { 0.0 });
        Ok(())
    }
}

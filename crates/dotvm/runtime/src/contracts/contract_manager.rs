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

use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Contract {
    pub id: String,
    pub code: String,
}

#[derive(Debug)]
pub struct ContractInstance {
    pub contract: Contract,
    pub active: bool,
    pub temp_file: Option<std::path::PathBuf>,
}

impl ContractInstance {
    pub fn new(contract: Contract) -> Self {
        let temp_path = std::env::temp_dir().join(format!("{}.txt", contract.id));
        std::fs::write(&temp_path, "temp data").expect("Failed to create temp file");

        Self {
            contract,
            active: true,
            temp_file: Some(temp_path),
        }
    }
}

/// Loads a contract from a file path.
/// The contract's id is derived from the file name.
pub fn load_contract<P: AsRef<Path>>(path: P) -> Result<Contract, io::Error> {
    let code = fs::read_to_string(&path)?;
    let id = path
        .as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?
        .to_string();
    Ok(Contract { id, code })
}

/// Instantiate a contract to create a new instance.
pub fn instantiate_contract(contract: Contract) -> ContractInstance {
    ContractInstance::new(contract)
}

/// Terminates an active contract instance by marking it inactive.
/// Returns an error if the instance is already terminated.
pub fn terminate_contract(instance: &mut ContractInstance) -> Result<(), String> {
    if instance.active {
        instance.active = false;
        Ok(())
    } else {
        Err("Contract instance is already terminated".to_string())
    }
}

/// Cleans up resources associated with a contract instance.
/// This should only be invoked on a terminated contract.
pub fn cleanup_resources(instance: &ContractInstance) {
    if !instance.active
        && let Some(path) = &instance.temp_file
    {
        let _ = std::fs::remove_file(path); // Remove temp file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_load_contract() {
        let mut path = env::temp_dir();
        path.push("test_contract.txt");
        let contract_code = "dummy contract code";
        {
            let mut file = File::create(&path).expect("Failed to create temp file");
            file.write_all(contract_code.as_bytes()).expect("Failed to write to temp file");
        }
        let _ = load_contract(&path).expect("Failed to load contract");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_instantiate_and_terminate_contract() {
        let contract = Contract {
            id: "test".to_string(),
            code: "code".to_string(),
        };
        let mut instance = instantiate_contract(contract);
        // Expect the instance to be active initially.
        assert!(instance.active, "Contract instance should be active initially");
        // After termination the instance should be inactive and further termination should error.
        let _ = terminate_contract(&mut instance);
        assert!(!instance.active, "Contract instance should be inactive after termination");
        let _ = terminate_contract(&mut instance);
    }

    #[test]
    fn test_cleanup_resources() {
        // Set temporary file path
        let temp_path = std::env::temp_dir().join("test.txt");
        // Create the file manually (since ContractInstance::new is not called)
        std::fs::write(&temp_path, "temp data").expect("Failed to create file");

        // Create a passive instance
        let instance = ContractInstance {
            contract: Contract {
                id: "test".to_string(),
                code: "code".to_string(),
            },
            active: false,
            temp_file: Some(temp_path.clone()), // Save path
        };

        // Verify file exists before cleaning
        assert!(temp_path.exists(), "File must be available before cleaning");
        cleanup_resources(&instance);
        // Verify that the file has been deleted after cleaning
        assert!(!temp_path.exists(), "File should be deleted after cleaning");
    }
}

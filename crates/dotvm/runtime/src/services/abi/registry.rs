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

//! ABI registry - stores and manages ABI versions

use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;
use tracing::{error, info, instrument};

use crate::proto::vm_service::{DotAbi, GetDotAbiRequest, GetDotAbiResponse, RegisterAbiRequest, RegisterAbiResponse};

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("ABI not found: {0}")]
    AbiNotFound(String),
    #[error("ABI already exists: {0}")]
    AbiAlreadyExists(String),
    #[error("Invalid ABI version: {0}")]
    InvalidVersion(String),
}

/// ABI registry stores and manages ABI versions
pub struct AbiRegistry {
    abis: RwLock<HashMap<String, HashMap<String, StoredAbi>>>, // dot_id -> version -> abi
}

#[derive(Clone, Debug)]
struct StoredAbi {
    abi: DotAbi,
    registered_at: u64,
    registrar_id: String,
}

impl AbiRegistry {
    pub fn new() -> Self {
        Self { abis: RwLock::new(HashMap::new()) }
    }

    #[instrument(skip(self))]
    pub async fn get_abi(&self, dot_id: &str, version: Option<&str>) -> Result<GetDotAbiResponse, RegistryError> {
        info!("Getting ABI for dot: {} version: {:?}", dot_id, version);

        let abis = self.abis.read().unwrap();

        let dot_abis = abis.get(dot_id).ok_or_else(|| RegistryError::AbiNotFound(dot_id.to_string()))?;

        let stored_abi = if let Some(version) = version {
            dot_abis.get(version).ok_or_else(|| RegistryError::AbiNotFound(format!("{}:{}", dot_id, version)))?
        } else {
            // Get latest version
            dot_abis.values().max_by_key(|abi| abi.registered_at).ok_or_else(|| RegistryError::AbiNotFound(dot_id.to_string()))?
        };

        Ok(GetDotAbiResponse {
            success: true,
            abi: Some(stored_abi.abi.clone()),
            error_message: String::new(),
        })
    }

    #[instrument(skip(self, request))]
    pub async fn register_abi(&self, request: RegisterAbiRequest) -> Result<RegisterAbiResponse, RegistryError> {
        info!("Registering ABI for dot: {}", request.dot_id);

        let abi = request.abi.ok_or_else(|| RegistryError::AbiNotFound("No ABI provided".to_string()))?;

        // Validate version format
        if !self.is_valid_version(&abi.version) {
            return Err(RegistryError::InvalidVersion(abi.version.clone()));
        }

        let stored_abi = StoredAbi {
            abi: abi.clone(),
            registered_at: chrono::Utc::now().timestamp() as u64,
            registrar_id: request.registrar_id,
        };

        // Store ABI
        {
            let mut abis = self.abis.write().unwrap();
            let dot_abis = abis.entry(request.dot_id.clone()).or_insert_with(HashMap::new);

            // Check if version already exists
            if dot_abis.contains_key(&abi.version) {
                return Err(RegistryError::AbiAlreadyExists(format!("{}:{}", request.dot_id, abi.version)));
            }

            dot_abis.insert(abi.version.clone(), stored_abi);
        }

        info!("Successfully registered ABI for dot: {} version: {}", request.dot_id, abi.version);

        Ok(RegisterAbiResponse {
            success: true,
            abi_version: abi.version,
            error_message: String::new(),
        })
    }

    /// Get all versions of an ABI for a dot
    pub async fn get_abi_versions(&self, dot_id: &str) -> Result<Vec<String>, RegistryError> {
        let abis = self.abis.read().unwrap();

        let dot_abis = abis.get(dot_id).ok_or_else(|| RegistryError::AbiNotFound(dot_id.to_string()))?;

        let mut versions: Vec<String> = dot_abis.keys().cloned().collect();
        versions.sort_by(|a, b| self.compare_versions(a, b));

        Ok(versions)
    }

    /// Check if an ABI exists
    pub async fn abi_exists(&self, dot_id: &str, version: Option<&str>) -> bool {
        let abis = self.abis.read().unwrap();

        if let Some(dot_abis) = abis.get(dot_id) {
            if let Some(version) = version { dot_abis.contains_key(version) } else { !dot_abis.is_empty() }
        } else {
            false
        }
    }

    /// Update an existing ABI (creates new version)
    pub async fn update_abi(&self, dot_id: &str, new_abi: DotAbi, registrar_id: String) -> Result<RegisterAbiResponse, RegistryError> {
        // This is essentially the same as register_abi but with version increment logic
        let request = RegisterAbiRequest {
            dot_id: dot_id.to_string(),
            abi: Some(new_abi),
            registrar_id,
        };

        self.register_abi(request).await
    }

    // Private helper methods
    fn is_valid_version(&self, version: &str) -> bool {
        // Simple semantic version validation
        let parts: Vec<&str> = version.split('.').collect();
        parts.len() == 3 && parts.iter().all(|part| part.parse::<u32>().is_ok())
    }

    fn compare_versions(&self, a: &str, b: &str) -> std::cmp::Ordering {
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

        for i in 0..3 {
            let a_part = a_parts.get(i).unwrap_or(&0);
            let b_part = b_parts.get(i).unwrap_or(&0);

            match a_part.cmp(b_part) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            }
        }

        std::cmp::Ordering::Equal
    }
}

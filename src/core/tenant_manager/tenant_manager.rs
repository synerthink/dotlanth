use std::collections::HashMap;

use crate::core::contract_manager::contract::Contract;

use super::tenant_vm::TenantVM;

/// The main structure for managing tenants and their VMs.
pub struct TenantManager {
    tenants: HashMap<String, TenantVM>,
}

impl TenantManager {
    /// Creates a new instance of the TenantManager.
    pub fn new() -> Self {
        Self {
            tenants: HashMap::new(),
        }
    }

    /// Creates a new tenant VM.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - A string slice that holds the tenant's unique identifier
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the operation
    pub fn create_tenant(&mut self, tenant_id: String) -> Result<(), String> {
        if self.tenants.contains_key(&tenant_id) {
            return Err(format!("Tenant '{}' already exists", tenant_id));
        }
        let tenant_vm = TenantVM {
            tenant_id: tenant_id.clone(),
            contracts: Vec::new(),
        };
        self.tenants.insert(tenant_id, tenant_vm);
        Ok(())
    }

    /// Adds a contract to a tenant's VM.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - A string slice that holds the tenant's unique identifier
    /// * `contract` - The Contract to be added to the tenant's VM
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the operation
    pub fn add_contract(&mut self, tenant_id: &str, contract: Contract) -> Result<(), String> {
        match self.tenants.get_mut(tenant_id) {
            Some(tenant_vm) => {
                tenant_vm.contracts.push(contract);
                Ok(())
            }
            None => Err(format!("Tenant '{}' not found", tenant_id)),
        }
    }

    /// Retrieves a reference to a tenant's VM.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - A string slice that holds the tenant's unique identifier
    ///
    /// # Returns
    ///
    /// An Option containing a reference to the TenantVM if found, or None if not found
    pub fn get_tenant_vm(&self, tenant_id: &str) -> Option<&TenantVM> {
        self.tenants.get(tenant_id)
    }

    /// Removes a tenant and its associated VM.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - A string slice that holds the tenant's unique identifier
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure of the operation
    pub fn remove_tenant(&mut self, tenant_id: &str) -> Result<(), String> {
        if self.tenants.remove(tenant_id).is_some() {
            Ok(())
        } else {
            Err(format!("Tenant '{}' not found", tenant_id))
        }
    }
}

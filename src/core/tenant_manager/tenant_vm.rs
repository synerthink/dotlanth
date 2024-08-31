use crate::core::contract_manager::contract::Contract;

/// Represents a tenant-specific virtual machine.
#[derive(Debug)]
pub struct TenantVM {
    pub tenant_id: String,
    pub contracts: Vec<Contract>,
}

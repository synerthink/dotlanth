#[cfg(test)]
mod tests {
    use crate::core::{
        contract_manager::{contract::Contract, contract_state::ContractState},
        tenant_manager::tenant_manager::TenantManager,
    };

    #[test]
    fn test_tenant_manager() {
        let mut manager = TenantManager::new();

        // Test creating a tenant
        assert!(manager.create_tenant("tenant1".to_string()).is_ok());
        assert!(manager.create_tenant("tenant1".to_string()).is_err());

        // Test adding a contract
        let contract = Contract {
            name: "contract1".to_string(),
            state: ContractState::Active,
        };
        assert!(manager.add_contract("tenant1", contract).is_ok());
        assert!(manager
            .add_contract(
                "tenant2",
                Contract {
                    name: "contract2".to_string(),
                    state: ContractState::Active
                }
            )
            .is_err());

        // Test retrieving a tenant VM
        let tenant_vm = manager.get_tenant_vm("tenant1").unwrap();
        assert_eq!(tenant_vm.tenant_id, "tenant1");
        assert_eq!(tenant_vm.contracts.len(), 1);
        assert!(manager.get_tenant_vm("tenant2").is_none());

        // Test removing a tenant
        assert!(manager.remove_tenant("tenant1").is_ok());
        assert!(manager.remove_tenant("tenant1").is_err());
    }
}

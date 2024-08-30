#[cfg(test)]
mod tests {
    use crate::core::contract_manager::{
        contract_manager::ContractManager, contract_state::ContractState,
    };

    #[test]
    fn test_create_contract() {
        let mut manager = ContractManager::new();
        let contract = manager.create_contract("Test Contract".to_string());
        assert_eq!(contract.name, "Test Contract");
        assert_eq!(contract.state, ContractState::Created);
    }

    #[test]
    fn test_validate_contract() {
        let mut manager = ContractManager::new();
        manager.create_contract("Test Contract".to_string());
        let validated = manager.validate_contract("Test Contract");
        assert!(validated);
        let contract = manager
            .contracts
            .iter()
            .find(|&c| c.name == "Test Contract")
            .unwrap();
        assert_eq!(contract.state, ContractState::Validated);
    }

    #[test]
    fn test_activate_contract() {
        let mut manager = ContractManager::new();
        manager.create_contract("Test Contract".to_string());
        let activated = manager.activate_contract("Test Contract");
        assert!(activated);
        let contract = manager
            .contracts
            .iter()
            .find(|&c| c.name == "Test Contract")
            .unwrap();
        assert_eq!(contract.state, ContractState::Active);
    }

    #[test]
    fn test_suspend_contract() {
        let mut manager = ContractManager::new();
        manager.create_contract("Test Contract".to_string());
        let suspended = manager.suspend_contract("Test Contract");
        assert!(suspended);
        let contract = manager
            .contracts
            .iter()
            .find(|&c| c.name == "Test Contract")
            .unwrap();
        assert_eq!(contract.state, ContractState::Suspended);
    }
}

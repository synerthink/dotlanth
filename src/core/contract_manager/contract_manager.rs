use super::{contract::Contract, contract_state::ContractState};

pub struct ContractManager {
    pub contracts: Vec<Contract>,
}

impl ContractManager {
    /// Creates a new `ContractManager`.
    ///
    /// # Returns
    ///
    /// A new instance of `ContractManager` with an empty list of contracts.
    pub fn new() -> Self {
        Self {
            contracts: Vec::new(),
        }
    }

    /// Creates a new contract and adds it to the manager.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the contract.
    ///
    /// # Returns
    ///
    /// The newly created `Contract`.
    pub fn create_contract(&mut self, name: String) -> Contract {
        let contract = Contract {
            name,
            state: ContractState::Created,
        };
        self.contracts.push(contract.clone());
        contract
    }

    /// Validates a contract by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the contract to validate.
    ///
    /// # Returns
    ///
    /// `true` if the contract was found and validated, `false` otherwise.
    pub fn validate_contract(&mut self, name: &str) -> bool {
        if let Some(contract) = self.contracts.iter_mut().find(|c| c.name == name) {
            contract.state = ContractState::Validated;
            return true;
        }
        false
    }

    /// Activates a contract by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the contract to activate.
    ///
    /// # Returns
    ///
    /// `true` if the contract was found and activated, `false` otherwise.
    pub fn activate_contract(&mut self, name: &str) -> bool {
        if let Some(contract) = self.contracts.iter_mut().find(|c| c.name == name) {
            contract.state = ContractState::Active;
            return true;
        }
        false
    }

    /// Suspends a contract by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the contract to suspend.
    ///
    /// # Returns
    ///
    /// `true` if the contract was found and suspended, `false` otherwise.
    pub fn suspend_contract(&mut self, name: &str) -> bool {
        if let Some(contract) = self.contracts.iter_mut().find(|c| c.name == name) {
            contract.state = ContractState::Suspended;
            return true;
        }
        false
    }
}

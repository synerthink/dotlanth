use super::contract_state::ContractState;

/// Represents a contract with a name and state.
#[derive(Clone, Debug)]
pub struct Contract {
    pub name: String,
    pub state: ContractState,
}

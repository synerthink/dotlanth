#[derive(Clone, Debug, PartialEq)]
pub enum ContractState {
    ContractState,
    Created,
    Validated,
    Active,
    Suspended,
}

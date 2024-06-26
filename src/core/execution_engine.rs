use crate::contracts::SimpleAdditionContract;
use log::{debug, info};

pub struct ExecutionEngine;

impl ExecutionEngine {
    pub fn execute_contract(contract: &SimpleAdditionContract) -> i32 {
        debug!("Input parameters: a={}, b={}", contract.a, contract.b);
        let result = contract.a + contract.b;
        debug!("Executing instruction: ADD");
        info!("Execution result: {}", result);
        result
    }
}

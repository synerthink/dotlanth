mod contracts;
mod core;
mod logging;

use contracts::SimpleAdditionContract;
use core::{
    execution_engine::ExecutionEngine, protocol_manager::protocol_manager::ProtocolManager,
};
use logging::setup_logging;

use log::info;

fn main() {
    setup_logging();
    info!("Starting dotVM...");

    // Test Execution Engine
    let contract = SimpleAdditionContract { a: 5, b: 3 };
    let result = ExecutionEngine::execute_contract(&contract);

    match result {
        Ok(res) => println!("Execution result: {:?}", res),
        Err(err) => println!("Execution error: {:?}", err),
    }

    // Test Protocol Manager
    let mut protocol_manager = ProtocolManager::new();
    let protocol = protocol_manager.create_protocol(
        "ExpenseApproval".to_string(),
        "Protocol for approving expenses".to_string(),
    );

    println!("{:?}", protocol)
}

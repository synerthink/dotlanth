mod contracts;
mod core;
mod logging;

use contracts::SimpleAdditionContract;
use core::
    execution_engine::ExecutionEngine
;
use logging::setup_logging;

use log::info;

fn main() {
    setup_logging();
    info!("Starting dotVM...");

    // Create a SimpleAdditionContract with the source code
    let contract = SimpleAdditionContract::new();

    // Execute the contract
    let result = ExecutionEngine::execute_contract(&contract);

    match result {
        Ok(res) => println!("Execution result: {:?}", res),
        Err(err) => println!("Execution error: {:?}", err),
    }
}

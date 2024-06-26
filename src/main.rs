mod contracts;
mod core;
mod logging;
mod protocols;

use contracts::SimpleAdditionContract;
use core::execution_engine::ExecutionEngine;
use core::protocol_manager::ProtocolManager;
use logging::setup_logging;
use protocols::ExpenseApprovalProtocol;

use log::info;

fn main() {
    setup_logging();
    info!("Starting dotVM...");

    // Test Execution Engine
    let contract = SimpleAdditionContract { a: 5, b: 3 };
    let result = ExecutionEngine::execute_contract(&contract);
    info!("SimpleAdditionContract result: {}", result);

    // Test Protocol Manager
    let protocol = ExpenseApprovalProtocol {
        amount: 1200,
        limit: 1000,
    };
    let approval_required = ProtocolManager::enforce_protocol(&protocol);
    info!(
        "ExpenseApprovalProtocol approval required: {}",
        approval_required
    );
}

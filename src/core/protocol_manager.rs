use crate::protocols::ExpenseApprovalProtocol;
use log::{debug, info};

pub struct ProtocolManager;

impl ProtocolManager {
    pub fn enforce_protocol(protocol: &ExpenseApprovalProtocol) -> bool {
        debug!(
            "Expense amount: {}, Limit: {}",
            protocol.amount, protocol.limit
        );
        if protocol.amount > protocol.limit {
            info!("Approval required for expense.");
            true
        } else {
            info!("No approval required for expense");
            false
        }
    }
}

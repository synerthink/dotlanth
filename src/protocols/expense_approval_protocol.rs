use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExpenseApprovalProtocol {
    pub amount: i32,
    pub limit: i32,
}

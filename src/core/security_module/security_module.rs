use std::collections::HashMap;

use super::clearance_level::ClearanceLevel;

pub struct SecurityModule {
    user_clearences: HashMap<String, ClearanceLevel>,
}

impl SecurityModule {
    pub fn new() -> Self {
        Self {
            user_clearences: HashMap::new(),
        }
    }

    pub fn set_user_clearance(&mut self, user_id: String, clearence: ClearanceLevel) {
        self.user_clearences.insert(user_id, clearence);
    }

    pub fn get_user_clearance(&self, user_id: &str) -> ClearanceLevel {
        *self
            .user_clearences
            .get(user_id)
            .unwrap_or(&ClearanceLevel::Employee)
    }

    pub fn check_access(&self, user_id: &str, required_clearence: ClearanceLevel) -> bool {
        let user_clearence = self.get_user_clearance(user_id);
        user_clearence >= required_clearence
    }
}

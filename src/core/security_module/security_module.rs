use std::collections::HashMap;

use super::clearance_level::ClearanceLevel;
/// The main structure for managing security clearances and access checks.
pub struct SecurityModule {
    user_clearences: HashMap<String, ClearanceLevel>,
}

impl SecurityModule {
    /// Creates a new instance of the SecurityModule.
    pub fn new() -> Self {
        Self {
            user_clearences: HashMap::new(),
        }
    }

    /// Sets the clearance level for a specific user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string slice that holds the user's unique identifier
    /// * `clearance` - The ClearanceLevel to be assigned to the user
    ///
    pub fn set_user_clearance(&mut self, user_id: String, clearence: ClearanceLevel) {
        self.user_clearences.insert(user_id, clearence);
    }

    /// Retrieves the clearance level for a specific user.
    ///
    /// If the user is not found, it returns the default clearance level of Employee.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string slice that holds the user's unique identifier
    ///
    /// # Returns
    ///
    /// The ClearanceLevel of the user
    pub fn get_user_clearance(&self, user_id: &str) -> ClearanceLevel {
        *self
            .user_clearences
            .get(user_id)
            .unwrap_or(&ClearanceLevel::Employee)
    }

    /// Checks if a user has sufficient clearance for a required level.
    ///
    /// # Arguments
    ///
    /// * `user_id` - A string slice that holds the user's unique identifier
    /// * `required_clearance` - The ClearanceLevel required for the operation
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the user has sufficient clearance
    pub fn check_access(&self, user_id: &str, required_clearence: ClearanceLevel) -> bool {
        let user_clearence = self.get_user_clearance(user_id);
        user_clearence >= required_clearence
    }
}

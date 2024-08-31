#[cfg(test)]
mod tests {
    use crate::core::security_module::{
        clearance_level::ClearanceLevel, security_module::SecurityModule,
    };

    #[test]
    fn test_security_module() {
        let mut security = SecurityModule::new();

        // Test setting and getting user clearance
        security.set_user_clearance("user1".to_string(), ClearanceLevel::Manager);
        assert_eq!(
            security.get_user_clearance("user1"),
            ClearanceLevel::Manager
        );

        // Test access checks
        assert!(security.check_access("user1", ClearanceLevel::Employee));
        assert!(security.check_access("user1", ClearanceLevel::Manager));
        assert!(!security.check_access("user1", ClearanceLevel::Executive));

        // Test default clearance
        assert_eq!(
            security.get_user_clearance("user2"),
            ClearanceLevel::Employee
        );
    }
}

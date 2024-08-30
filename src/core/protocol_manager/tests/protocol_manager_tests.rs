#[cfg(test)]
mod tests {
    use crate::core::protocol_manager::protocol_manager::ProtocolManager;

    #[test]
    fn test_create_protocol() {
        let mut manager = ProtocolManager::new();
        let protocol =
            manager.create_protocol("Test Protocol".to_string(), "Description".to_string());
        assert_eq!(protocol.name, "Test Protocol");
    }

    #[test]
    fn test_get_protocol() {
        let mut manager = ProtocolManager::new();
        manager.create_protocol("Test Protocol".to_string(), "Description".to_string());
        let protocol = manager.get_protocol("Test Protocol").unwrap();
        assert_eq!(protocol.name, "Test Protocol");
    }

    #[test]
    fn test_update_protocol() {
        let mut manager = ProtocolManager::new();
        manager.create_protocol("Test Protocol".to_string(), "Description".to_string());
        manager.update_protocol("Test Protocol", "New Description".to_string());
        let protocol = manager.get_protocol("Test Protocol").unwrap();
        assert_eq!(protocol.description, "New Description");
    }

    #[test]
    fn test_delete_protocol() {
        let mut manager = ProtocolManager::new();
        manager.create_protocol("Test Protocol".to_string(), "Description".to_string());
        let deleted = manager.delete_protocol("Test Protocol");
        assert!(deleted);
        assert!(manager.get_protocol("Test Protocol").is_none());
    }
}

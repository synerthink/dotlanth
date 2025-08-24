//! Tests for user management system

#[cfg(test)]
mod tests {
    use crate::user_management::models::{User, UserActivity, UserPreferences, UserProfile, UserStatus};
    use crate::user_management::store::{ActivityStore, ActivityStoreTrait, UserStore, UserStoreTrait};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::SystemTime;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_user_creation() {
        let user_id = uuid::Uuid::new_v4().to_string();
        let user = User::new(user_id.clone(), "test@example.com".to_string(), "testuser".to_string());

        assert_eq!(user.id, user_id);
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, "testuser");
        assert_eq!(user.status, UserStatus::PendingVerification);
        assert!(user.roles.contains(&"user".to_string()));
    }

    #[tokio::test]
    async fn test_user_status_update() {
        let mut user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());

        // Test status update
        user.update_status(UserStatus::Active).unwrap();
        assert_eq!(user.status, UserStatus::Active);
        assert!(user.is_active());
        assert!(user.can_login());
    }

    #[tokio::test]
    async fn test_user_store_operations() {
        let db_path = std::env::temp_dir().join(format!("dotlanth/dotdb/test/{}", uuid::Uuid::new_v4()));
        // Clean up any existing test database
        let _ = std::fs::remove_dir_all(&db_path);
        let store = UserStore::new_persistent(&db_path).unwrap();
        store.initialize().await.unwrap();

        let user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());

        // Test create
        let created_user = store.create_user(user.clone()).await.unwrap();
        assert_eq!(created_user.id, user.id);

        // Test get by ID
        let retrieved_user = store.get_user(&user.id).await.unwrap().unwrap();
        assert_eq!(retrieved_user.id, user.id);

        // Test get by email
        let user_by_email = store.get_user_by_email("test@example.com").await.unwrap().unwrap();
        assert_eq!(user_by_email.email, "test@example.com");

        // Test get by username
        let user_by_username = store.get_user_by_username("testuser").await.unwrap().unwrap();
        assert_eq!(user_by_username.username, "testuser");

        // Test duplicate email prevention
        let duplicate_user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser2".to_string());

        let result = store.create_user(duplicate_user).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_user_preferences() {
        let preferences = UserPreferences::default();

        assert_eq!(preferences.theme, "auto");
        assert!(preferences.notifications.email_enabled);
        assert_eq!(preferences.privacy.profile_visibility, "public");
        assert_eq!(preferences.dashboard.default_view, "overview");
    }

    #[tokio::test]
    async fn test_user_summary() {
        let user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());

        let summary = user.to_summary(true);
        assert!(!summary.id.is_empty());
        assert_eq!(summary.username, "testuser");
        assert_eq!(summary.email, Some("test@example.com".to_string()));

        let summary_no_email = user.to_summary(false);
        assert_eq!(summary_no_email.email, None);
    }

    #[tokio::test]
    async fn test_user_retrieval() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = UserStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        let user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());

        store.create_user(user.clone()).await.unwrap();

        // Test get by ID
        let retrieved_user = store.get_user(&user.id).await.unwrap().unwrap();
        assert_eq!(retrieved_user.id, user.id);

        // Test get by email
        let user_by_email = store.get_user_by_email("test@example.com").await.unwrap().unwrap();
        assert_eq!(user_by_email.email, user.email);

        // Test get by username
        let user_by_username = store.get_user_by_username("testuser").await.unwrap().unwrap();
        assert_eq!(user_by_username.username, user.username);
    }

    #[tokio::test]
    async fn test_user_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = UserStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        let mut user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());
        let user_id = user.id.clone();

        store.create_user(user.clone()).await.unwrap();

        // Update user
        user.profile.display_name = Some("Test User".to_string());
        user.email = "newemail@example.com".to_string();

        let updated_user = store.update_user(user).await.unwrap();
        assert_eq!(updated_user.profile.display_name, Some("Test User".to_string()));
        assert_eq!(updated_user.email, "newemail@example.com");

        // Verify the update persisted
        let retrieved_user = store.get_user(&user_id).await.unwrap().unwrap();
        assert_eq!(retrieved_user.email, "newemail@example.com");
    }

    #[tokio::test]
    async fn test_duplicate_prevention() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = UserStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        let user1 = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());

        store.create_user(user1).await.unwrap();

        // Try to create user with same email
        let user2 = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser2".to_string());

        let result = store.create_user(user2).await;
        assert!(result.is_err());

        // Try to create user with same username
        let user3 = User::new(uuid::Uuid::new_v4().to_string(), "test2@example.com".to_string(), "testuser".to_string());

        let result = store.create_user(user3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_user_search() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = UserStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        // Create multiple users
        for i in 1..=5 {
            let user = User::new(uuid::Uuid::new_v4().to_string(), format!("user{}@example.com", i), format!("user{}", i));
            store.create_user(user).await.unwrap();
        }

        // Search users
        let query = crate::user_management::models::UserSearchQuery {
            query: Some("user".to_string()),
            status: Some(UserStatus::PendingVerification),
            roles: None,
            created_after: None,
            created_before: None,
            last_login_after: None,
            sort_by: Some("username".to_string()),
            sort_direction: Some("asc".to_string()),
            page: Some(1),
            page_size: Some(10),
        };

        let results = store.search_users(&query).await.unwrap();
        assert_eq!(results.users.len(), 5);
        assert_eq!(results.total_count, 5);
    }

    #[tokio::test]
    async fn test_activity_logging() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let store = ActivityStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: "test_user".to_string(),
            activity_type: "login".to_string(),
            description: "User logged in".to_string(),
            metadata: HashMap::new(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            timestamp: SystemTime::now(),
        };

        store.log_activity(activity.clone()).await.unwrap();

        // Retrieve activities
        let activities = store.get_user_activities("test_user", Some(10), None).await.unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].activity_type, "login");

        // Get activities by type
        let login_activities = store.get_activities_by_type("login", Some(10)).await.unwrap();
        assert_eq!(login_activities.len(), 1);
    }

    #[tokio::test]
    async fn test_user_count() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = UserStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        // Initially no users
        let count = store.get_user_count().await.unwrap();
        assert_eq!(count, 0);

        // Add some users
        for i in 1..=3 {
            let user = User::new(uuid::Uuid::new_v4().to_string(), format!("user{}@example.com", i), format!("user{}", i));
            store.create_user(user).await.unwrap();
        }

        let count = store.get_user_count().await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_user_deletion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = UserStore::new_persistent(temp_dir.path()).unwrap();
        store.initialize().await.unwrap();

        let user = User::new(uuid::Uuid::new_v4().to_string(), "test@example.com".to_string(), "testuser".to_string());
        let user_id = user.id.clone();

        store.create_user(user).await.unwrap();

        // Delete user (soft delete)
        store.delete_user(&user_id).await.unwrap();

        // User should still exist but be marked as deleted
        let deleted_user = store.get_user(&user_id).await.unwrap().unwrap();
        assert_eq!(deleted_user.status, UserStatus::Deleted);
    }
}

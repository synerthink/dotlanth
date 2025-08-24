//! User data storage layer

use crate::user_management::models::{User, UserActivity, UserError, UserSearchQuery, UserSearchResults, UserStatus, UserSummary};
use async_trait::async_trait;
use dotdb_core::document::{CollectionManager, DocumentId, DocumentResult};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

/// User storage trait
#[async_trait]
pub trait UserStoreTrait: Send + Sync {
    /// Get the trait object as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
    /// Create a new user
    async fn create_user(&self, user: User) -> Result<User, UserError>;

    /// Get user by ID
    async fn get_user(&self, user_id: &str) -> Result<Option<User>, UserError>;

    /// Get user by email
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, UserError>;

    /// Get user by username
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, UserError>;

    /// Update user
    async fn update_user(&self, user: User) -> Result<User, UserError>;

    /// Delete user (soft delete)
    async fn delete_user(&self, user_id: &str) -> Result<(), UserError>;

    /// Search users
    async fn search_users(&self, query: &UserSearchQuery) -> Result<UserSearchResults, UserError>;

    /// Get users by role
    async fn get_users_by_role(&self, role: &str) -> Result<Vec<User>, UserError>;

    /// Get user count
    async fn get_user_count(&self) -> Result<u64, UserError>;

    /// Check if email exists
    async fn email_exists(&self, email: &str) -> Result<bool, UserError>;

    /// Check if username exists
    async fn username_exists(&self, username: &str) -> Result<bool, UserError>;
}

/// Activity storage trait
#[async_trait]
pub trait ActivityStoreTrait: Send + Sync {
    /// Log user activity
    async fn log_activity(&self, activity: UserActivity) -> Result<(), UserError>;

    /// Get user activities
    async fn get_user_activities(&self, user_id: &str, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<UserActivity>, UserError>;

    /// Get activities by type
    async fn get_activities_by_type(&self, activity_type: &str, limit: Option<u32>) -> Result<Vec<UserActivity>, UserError>;

    /// Clean old activities
    async fn clean_old_activities(&self, older_than: SystemTime) -> Result<u64, UserError>;
}

/// User store implementation using DotDB
pub struct UserStore {
    collection_manager: Arc<CollectionManager>,
    users_collection: String,
    email_index_collection: String,
    username_index_collection: String,
}

impl UserStore {
    /// Create a new DotDB user store
    pub fn new(collection_manager: Arc<CollectionManager>) -> Self {
        Self {
            collection_manager,
            users_collection: "users".to_string(),
            email_index_collection: "user_email_index".to_string(),
            username_index_collection: "user_username_index".to_string(),
        }
    }

    /// Create a new DotDB user store with in-memory storage

    /// Create a new DotDB user store with persistent storage
    pub fn new_persistent<P: AsRef<std::path::Path>>(path: P) -> Result<Self, UserError> {
        let collection_manager = Arc::new(dotdb_core::document::collection::create_persistent_collection_manager(path, None).map_err(|e| UserError::StorageError {
            message: format!("Failed to create collection manager: {}", e),
        })?);
        Ok(Self::new(collection_manager))
    }

    /// Initialize collections
    pub async fn initialize(&self) -> Result<(), UserError> {
        // For file-based storage, collections are logical groupings
        // The actual initialization happens when the database files are created
        // No additional collection creation needed for DotDB file storage
        Ok(())
    }

    /// Add default admin user
    pub async fn add_default_admin(&self, password_hash: String) -> Result<(), UserError> {
        let admin_user = User {
            id: "admin".to_string(),
            email: "admin@dotlanth.com".to_string(),
            username: "admin".to_string(),
            profile: crate::user_management::models::UserProfile {
                display_name: Some("Administrator".to_string()),
                first_name: Some("System".to_string()),
                last_name: Some("Administrator".to_string()),
                ..Default::default()
            },
            roles: vec!["super_admin".to_string(), "admin".to_string(), "user".to_string()],
            status: UserStatus::Active,
            preferences: Default::default(),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            last_login: None,
        };

        self.create_user(admin_user).await?;
        Ok(())
    }

    /// Convert User to JSON Value
    fn user_to_json(&self, user: &User) -> Result<Value, UserError> {
        serde_json::to_value(user).map_err(|e| UserError::StorageError {
            message: format!("Failed to serialize user: {}", e),
        })
    }

    /// Convert JSON Value to User
    fn json_to_user(&self, value: &Value) -> Result<User, UserError> {
        serde_json::from_value(value.clone()).map_err(|e| UserError::StorageError {
            message: format!("Failed to deserialize user: {}", e),
        })
    }

    /// Create index entry for email
    fn create_email_index(&self, email: &str, user_id: &str) -> Result<(), UserError> {
        let index_value = json!({
            "email": email,
            "user_id": user_id
        });
        self.collection_manager.insert_value(&self.email_index_collection, index_value).map_err(|e| UserError::StorageError {
            message: format!("Failed to create email index: {}", e),
        })?;
        Ok(())
    }

    /// Create index entry for username
    fn create_username_index(&self, username: &str, user_id: &str) -> Result<(), UserError> {
        let index_value = json!({
            "username": username,
            "user_id": user_id
        });
        self.collection_manager
            .insert_value(&self.username_index_collection, index_value)
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to create username index: {}", e),
            })?;
        Ok(())
    }

    /// Find user ID by email
    fn find_user_id_by_email(&self, email: &str) -> Result<Option<String>, UserError> {
        let results = self
            .collection_manager
            .find_by_field(&self.email_index_collection, "email", &json!(email))
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to search email index: {}", e),
            })?;

        if let Some((_, value)) = results.first() {
            if let Some(user_id) = value.get("user_id").and_then(|v| v.as_str()) {
                return Ok(Some(user_id.to_string()));
            }
        }

        Ok(None)
    }

    /// Find user ID by username
    fn find_user_id_by_username(&self, username: &str) -> Result<Option<String>, UserError> {
        let results = self
            .collection_manager
            .find_by_field(&self.username_index_collection, "username", &json!(username))
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to search username index: {}", e),
            })?;

        if let Some((_, value)) = results.first() {
            if let Some(user_id) = value.get("user_id").and_then(|v| v.as_str()) {
                return Ok(Some(user_id.to_string()));
            }
        }

        Ok(None)
    }

    /// Remove email index entry
    fn remove_email_index(&self, email: &str) -> Result<(), UserError> {
        let results = self
            .collection_manager
            .find_by_field(&self.email_index_collection, "email", &json!(email))
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to search email index: {}", e),
            })?;

        for (doc_id, _) in results {
            self.collection_manager.delete(&self.email_index_collection, &doc_id).map_err(|e| UserError::StorageError {
                message: format!("Failed to remove email index: {}", e),
            })?;
        }

        Ok(())
    }

    /// Remove username index entry
    fn remove_username_index(&self, username: &str) -> Result<(), UserError> {
        let results = self
            .collection_manager
            .find_by_field(&self.username_index_collection, "username", &json!(username))
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to search username index: {}", e),
            })?;

        for (doc_id, _) in results {
            self.collection_manager.delete(&self.username_index_collection, &doc_id).map_err(|e| UserError::StorageError {
                message: format!("Failed to remove username index: {}", e),
            })?;
        }

        Ok(())
    }
}

#[async_trait]
impl UserStoreTrait for UserStore {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    async fn create_user(&self, user: User) -> Result<User, UserError> {
        // Check if user ID already exists
        let user_doc_id = DocumentId::from_string(&user.id).map_err(|e| UserError::InvalidData {
            message: format!("Invalid user ID: {}", e),
        })?;

        if self.collection_manager.exists(&self.users_collection, &user_doc_id).map_err(|e| UserError::StorageError {
            message: format!("Failed to check user existence: {}", e),
        })? {
            return Err(UserError::InvalidData {
                message: format!("User ID already exists: {}", user.id),
            });
        }

        // Check if email already exists
        if self.find_user_id_by_email(&user.email)?.is_some() {
            return Err(UserError::EmailExists { email: user.email.clone() });
        }

        // Check if username already exists
        if self.find_user_id_by_username(&user.username)?.is_some() {
            return Err(UserError::UsernameExists { username: user.username.clone() });
        }

        // Store user document using insert_value and then create a mapping
        let user_json = self.user_to_json(&user)?;

        // Insert the document (DotDB will generate its own document ID)
        let doc_id = self.collection_manager.insert_value(&self.users_collection, user_json).map_err(|e| UserError::StorageError {
            message: format!("Failed to store user: {}", e),
        })?;

        // Create a mapping from user UUID to document ID for retrieval
        let mapping_json = serde_json::json!({
            "user_id": user.id,
            "doc_id": doc_id.to_string()
        });

        self.collection_manager.insert_value("user_id_mappings", mapping_json).map_err(|e| UserError::StorageError {
            message: format!("Failed to store user ID mapping: {}", e),
        })?;

        // Create indices
        self.create_email_index(&user.email, &user.id)?;
        self.create_username_index(&user.username, &user.id)?;

        Ok(user)
    }

    async fn get_user(&self, user_id: &str) -> Result<Option<User>, UserError> {
        // First, find the document ID from the user ID mapping
        let mappings = self
            .collection_manager
            .find_by_field("user_id_mappings", "user_id", &json!(user_id))
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to search user ID mappings: {}", e),
            })?;

        if let Some((_, mapping_value)) = mappings.first() {
            if let Some(doc_id_str) = mapping_value.get("doc_id").and_then(|v| v.as_str()) {
                let doc_id = DocumentId::from_string(doc_id_str).map_err(|e| UserError::StorageError {
                    message: format!("Invalid document ID in mapping: {}", e),
                })?;

                match self.collection_manager.get_value(&self.users_collection, &doc_id).map_err(|e| UserError::StorageError {
                    message: format!("Failed to get user: {}", e),
                })? {
                    Some(value) => Ok(Some(self.json_to_user(&value)?)),
                    None => Ok(None),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
        if let Some(user_id) = self.find_user_id_by_email(email)? {
            self.get_user(&user_id).await
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, UserError> {
        if let Some(user_id) = self.find_user_id_by_username(username)? {
            self.get_user(&user_id).await
        } else {
            Ok(None)
        }
    }

    async fn update_user(&self, mut user: User) -> Result<User, UserError> {
        // Check if user exists
        let existing_user = self.get_user(&user.id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user.id.clone() })?;

        // Check if email changed and new email already exists
        if user.email != existing_user.email {
            if self.find_user_id_by_email(&user.email)?.is_some() {
                return Err(UserError::EmailExists { email: user.email.clone() });
            }
            // Update email index
            self.remove_email_index(&existing_user.email)?;
            self.create_email_index(&user.email, &user.id)?;
        }

        // Check if username changed and new username already exists
        if user.username != existing_user.username {
            if self.find_user_id_by_username(&user.username)?.is_some() {
                return Err(UserError::UsernameExists { username: user.username.clone() });
            }
            // Update username index
            self.remove_username_index(&existing_user.username)?;
            self.create_username_index(&user.username, &user.id)?;
        }

        // Update timestamp
        user.updated_at = SystemTime::now();

        // Update user document - first find the document ID from the mapping
        let mappings = self
            .collection_manager
            .find_by_field("user_id_mappings", "user_id", &json!(user.id))
            .map_err(|e| UserError::StorageError {
                message: format!("Failed to search user ID mappings: {}", e),
            })?;

        if let Some((_, mapping_value)) = mappings.first() {
            if let Some(doc_id_str) = mapping_value.get("doc_id").and_then(|v| v.as_str()) {
                let doc_id = DocumentId::from_string(doc_id_str).map_err(|e| UserError::StorageError {
                    message: format!("Invalid document ID in mapping: {}", e),
                })?;

                let user_json = self.user_to_json(&user)?;
                self.collection_manager.update_value(&self.users_collection, &doc_id, user_json).map_err(|e| UserError::StorageError {
                    message: format!("Failed to update user: {}", e),
                })?;
            } else {
                return Err(UserError::UserNotFound { user_id: user.id.clone() });
            }
        } else {
            return Err(UserError::UserNotFound { user_id: user.id.clone() });
        }

        Ok(user)
    }

    async fn delete_user(&self, user_id: &str) -> Result<(), UserError> {
        let user = self.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        let mut updated_user = user;
        updated_user.status = UserStatus::Deleted;
        updated_user.updated_at = SystemTime::now();

        self.update_user(updated_user).await?;
        Ok(())
    }

    async fn search_users(&self, query: &UserSearchQuery) -> Result<UserSearchResults, UserError> {
        // Get all users from the collection
        let all_users = self.collection_manager.get_all_values(&self.users_collection).map_err(|e| UserError::StorageError {
            message: format!("Failed to get all users: {}", e),
        })?;

        let mut users: Vec<User> = all_users.into_iter().filter_map(|(_, value)| self.json_to_user(&value).ok()).collect();

        // Apply filters
        if let Some(ref search_term) = query.query {
            let search_term = search_term.to_lowercase();
            users.retain(|user| {
                user.username.to_lowercase().contains(&search_term)
                    || user.email.to_lowercase().contains(&search_term)
                    || user.profile.display_name.as_ref().map_or(false, |name| name.to_lowercase().contains(&search_term))
            });
        }

        if let Some(ref status) = query.status {
            users.retain(|user| &user.status == status);
        }

        if let Some(ref roles) = query.roles {
            users.retain(|user| roles.iter().any(|role| user.roles.contains(role)));
        }

        // Apply sorting
        let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
        let sort_desc = query.sort_direction.as_deref() == Some("desc");

        users.sort_by(|a, b| {
            let ordering = match sort_by {
                "username" => a.username.cmp(&b.username),
                "email" => a.email.cmp(&b.email),
                "created_at" => a.created_at.cmp(&b.created_at),
                "last_login" => a.last_login.cmp(&b.last_login),
                _ => a.created_at.cmp(&b.created_at),
            };

            if sort_desc { ordering.reverse() } else { ordering }
        });

        let total_count = users.len() as u64;

        // Apply pagination
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).min(100);
        let offset = ((page - 1) * page_size) as usize;

        let paginated_users: Vec<User> = users.into_iter().skip(offset).take(page_size as usize).collect();

        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as u32;

        // Convert to summaries
        let user_summaries: Vec<UserSummary> = paginated_users
            .into_iter()
            .map(|user| user.to_summary(true)) // Include email for admin searches
            .collect();

        Ok(UserSearchResults {
            users: user_summaries,
            total_count,
            page,
            page_size,
            total_pages,
        })
    }

    async fn get_users_by_role(&self, role: &str) -> Result<Vec<User>, UserError> {
        let all_users = self.collection_manager.get_all_values(&self.users_collection).map_err(|e| UserError::StorageError {
            message: format!("Failed to get all users: {}", e),
        })?;

        let users: Vec<User> = all_users
            .into_iter()
            .filter_map(|(_, value)| self.json_to_user(&value).ok())
            .filter(|user| user.roles.contains(&role.to_string()))
            .collect();

        Ok(users)
    }

    async fn get_user_count(&self) -> Result<u64, UserError> {
        let count = self.collection_manager.count(&self.users_collection).map_err(|e| UserError::StorageError {
            message: format!("Failed to count users: {}", e),
        })?;

        Ok(count as u64)
    }

    async fn email_exists(&self, email: &str) -> Result<bool, UserError> {
        Ok(self.find_user_id_by_email(email)?.is_some())
    }

    async fn username_exists(&self, username: &str) -> Result<bool, UserError> {
        Ok(self.find_user_id_by_username(username)?.is_some())
    }
}

/// Activity store implementation using DotDB
pub struct ActivityStore {
    collection_manager: Arc<CollectionManager>,
    activities_collection: String,
}

impl ActivityStore {
    /// Create a new DotDB activity store
    pub fn new(collection_manager: Arc<CollectionManager>) -> Self {
        Self {
            collection_manager,
            activities_collection: "user_activities".to_string(),
        }
    }

    /// Create a new DotDB activity store with persistent storage
    pub fn new_persistent<P: AsRef<std::path::Path>>(path: P) -> Result<Self, UserError> {
        // Ensure the directory exists
        std::fs::create_dir_all(&path).map_err(|e| UserError::StorageError {
            message: format!("Failed to create database directory: {}", e),
        })?;

        let collection_manager = Arc::new(dotdb_core::document::collection::create_persistent_collection_manager(path, None).map_err(|e| UserError::StorageError {
            message: format!("Failed to create collection manager: {}", e),
        })?);
        Ok(Self::new(collection_manager))
    }

    /// Initialize collections
    pub async fn initialize(&self) -> Result<(), UserError> {
        // For file-based storage, collections are logical groupings
        // No additional collection creation needed for DotDB file storage
        Ok(())
    }

    /// Convert UserActivity to JSON Value
    fn activity_to_json(&self, activity: &UserActivity) -> Result<Value, UserError> {
        serde_json::to_value(activity).map_err(|e| UserError::StorageError {
            message: format!("Failed to serialize activity: {}", e),
        })
    }

    /// Convert JSON Value to UserActivity
    fn json_to_activity(&self, value: &Value) -> Result<UserActivity, UserError> {
        serde_json::from_value(value.clone()).map_err(|e| UserError::StorageError {
            message: format!("Failed to deserialize activity: {}", e),
        })
    }
}

#[async_trait]
impl ActivityStoreTrait for ActivityStore {
    async fn log_activity(&self, activity: UserActivity) -> Result<(), UserError> {
        let activity_json = self.activity_to_json(&activity)?;
        self.collection_manager.insert_value(&self.activities_collection, activity_json).map_err(|e| UserError::StorageError {
            message: format!("Failed to log activity: {}", e),
        })?;
        Ok(())
    }

    async fn get_user_activities(&self, user_id: &str, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<UserActivity>, UserError> {
        let all_activities = self.collection_manager.get_all_values(&self.activities_collection).map_err(|e| UserError::StorageError {
            message: format!("Failed to get activities: {}", e),
        })?;

        let mut user_activities: Vec<UserActivity> = all_activities
            .into_iter()
            .filter_map(|(_, value)| self.json_to_activity(&value).ok())
            .filter(|activity| activity.user_id == user_id)
            .collect();

        // Sort by timestamp (newest first)
        user_activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let offset = offset.unwrap_or(0) as usize;
        let limit = limit.unwrap_or(100) as usize;

        let result = user_activities.into_iter().skip(offset).take(limit).collect();

        Ok(result)
    }

    async fn get_activities_by_type(&self, activity_type: &str, limit: Option<u32>) -> Result<Vec<UserActivity>, UserError> {
        let all_activities = self.collection_manager.get_all_values(&self.activities_collection).map_err(|e| UserError::StorageError {
            message: format!("Failed to get activities: {}", e),
        })?;

        let mut filtered_activities: Vec<UserActivity> = all_activities
            .into_iter()
            .filter_map(|(_, value)| self.json_to_activity(&value).ok())
            .filter(|activity| activity.activity_type == activity_type)
            .collect();

        // Sort by timestamp (newest first)
        filtered_activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let limit = limit.unwrap_or(100) as usize;
        filtered_activities.truncate(limit);

        Ok(filtered_activities)
    }

    async fn clean_old_activities(&self, older_than: SystemTime) -> Result<u64, UserError> {
        let all_activities = self.collection_manager.get_all_values(&self.activities_collection).map_err(|e| UserError::StorageError {
            message: format!("Failed to get activities: {}", e),
        })?;

        let mut removed_count = 0u64;

        for (doc_id, value) in all_activities {
            if let Ok(activity) = self.json_to_activity(&value) {
                if activity.timestamp < older_than {
                    self.collection_manager.delete(&self.activities_collection, &doc_id).map_err(|e| UserError::StorageError {
                        message: format!("Failed to delete old activity: {}", e),
                    })?;
                    removed_count += 1;
                }
            }
        }

        Ok(removed_count)
    }
}

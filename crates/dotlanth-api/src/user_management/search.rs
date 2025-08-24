//! User search functionality

use crate::user_management::models::{UserError, UserSearchQuery, UserSearchResults, UserStatus, UserSummary};
use crate::user_management::store::UserStoreTrait;
use std::sync::Arc;

/// User search service
pub struct UserSearchService {
    user_store: Arc<dyn UserStoreTrait>,
}

impl UserSearchService {
    /// Create a new user search service
    pub fn new(user_store: Arc<dyn UserStoreTrait>) -> Self {
        Self { user_store }
    }

    /// Search users with advanced filtering
    pub async fn search(&self, query: &UserSearchQuery) -> Result<UserSearchResults, UserError> {
        // Validate search parameters
        self.validate_search_query(query)?;

        // Perform search using the user store
        self.user_store.search_users(query).await
    }

    /// Search users by username prefix
    pub async fn search_by_username_prefix(&self, prefix: &str, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let query = UserSearchQuery {
            query: Some(prefix.to_string()),
            status: Some(UserStatus::Active),
            roles: None,
            created_after: None,
            created_before: None,
            last_login_after: None,
            sort_by: Some("username".to_string()),
            sort_direction: Some("asc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Search users by email domain
    pub async fn search_by_email_domain(&self, domain: &str, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let query = UserSearchQuery {
            query: Some(format!("@{}", domain)),
            status: None,
            roles: None,
            created_after: None,
            created_before: None,
            last_login_after: None,
            sort_by: Some("email".to_string()),
            sort_direction: Some("asc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Get recently active users
    pub async fn get_recently_active_users(&self, days: u32, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let since = std::time::SystemTime::now() - std::time::Duration::from_secs(days as u64 * 24 * 60 * 60);
        let since_chrono = chrono::DateTime::<chrono::Utc>::from(since);

        let query = UserSearchQuery {
            query: None,
            status: Some(UserStatus::Active),
            roles: None,
            created_after: None,
            created_before: None,
            last_login_after: Some(since_chrono),
            sort_by: Some("last_login".to_string()),
            sort_direction: Some("desc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Get users by role
    pub async fn get_users_by_role(&self, role: &str, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let query = UserSearchQuery {
            query: None,
            status: Some(UserStatus::Active),
            roles: Some(vec![role.to_string()]),
            created_after: None,
            created_before: None,
            last_login_after: None,
            sort_by: Some("username".to_string()),
            sort_direction: Some("asc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Get new users (registered recently)
    pub async fn get_new_users(&self, days: u32, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let since = std::time::SystemTime::now() - std::time::Duration::from_secs(days as u64 * 24 * 60 * 60);
        let since_chrono = chrono::DateTime::<chrono::Utc>::from(since);

        let query = UserSearchQuery {
            query: None,
            status: None,
            roles: None,
            created_after: Some(since_chrono),
            created_before: None,
            last_login_after: None,
            sort_by: Some("created_at".to_string()),
            sort_direction: Some("desc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Get suspended users
    pub async fn get_suspended_users(&self, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let query = UserSearchQuery {
            query: None,
            status: Some(UserStatus::Suspended),
            roles: None,
            created_after: None,
            created_before: None,
            last_login_after: None,
            sort_by: Some("updated_at".to_string()),
            sort_direction: Some("desc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Get users pending verification
    pub async fn get_pending_verification_users(&self, limit: Option<u32>) -> Result<Vec<UserSummary>, UserError> {
        let query = UserSearchQuery {
            query: None,
            status: Some(UserStatus::PendingVerification),
            roles: None,
            created_after: None,
            created_before: None,
            last_login_after: None,
            sort_by: Some("created_at".to_string()),
            sort_direction: Some("asc".to_string()),
            page: Some(1),
            page_size: limit,
        };

        let results = self.search(&query).await?;
        Ok(results.users)
    }

    /// Validate search query parameters
    fn validate_search_query(&self, query: &UserSearchQuery) -> Result<(), UserError> {
        // Validate page size
        if let Some(page_size) = query.page_size {
            if page_size == 0 || page_size > 1000 {
                return Err(UserError::ValidationError {
                    message: "Page size must be between 1 and 1000".to_string(),
                });
            }
        }

        // Validate page number
        if let Some(page) = query.page {
            if page == 0 {
                return Err(UserError::ValidationError {
                    message: "Page number must be greater than 0".to_string(),
                });
            }
        }

        // Validate sort direction
        if let Some(ref direction) = query.sort_direction {
            if !["asc", "desc"].contains(&direction.as_str()) {
                return Err(UserError::ValidationError {
                    message: "Sort direction must be 'asc' or 'desc'".to_string(),
                });
            }
        }

        // Validate sort field
        if let Some(ref sort_by) = query.sort_by {
            let valid_fields = ["username", "email", "created_at", "updated_at", "last_login"];
            if !valid_fields.contains(&sort_by.as_str()) {
                return Err(UserError::ValidationError {
                    message: format!("Invalid sort field. Must be one of: {}", valid_fields.join(", ")),
                });
            }
        }

        // Validate date ranges
        if let (Some(after), Some(before)) = (&query.created_after, &query.created_before) {
            if after >= before {
                return Err(UserError::ValidationError {
                    message: "created_after must be before created_before".to_string(),
                });
            }
        }

        Ok(())
    }
}

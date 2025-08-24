//! User management service

use crate::auth::JWTAuthSystem;
use crate::error::{ApiError, ApiResult};
use crate::rbac::system::RBACSystem;
use crate::user_management::audit::AuditLogger;
use crate::user_management::models::{User, UserError, UserRegistration, UserSearchQuery, UserSearchResults, UserStatus, UserUpdates};
use crate::user_management::store::{ActivityStoreTrait, UserStore, UserStoreTrait};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Notification service trait for sending notifications
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// Send email verification
    async fn send_email_verification(&self, user_id: &str, email: &str, verification_token: &str) -> Result<(), UserError>;

    /// Send welcome email
    async fn send_welcome_email(&self, user_id: &str, email: &str) -> Result<(), UserError>;

    /// Send password reset email
    async fn send_password_reset(&self, user_id: &str, email: &str, reset_token: &str) -> Result<(), UserError>;

    /// Send account suspension notification
    async fn send_suspension_notification(&self, user_id: &str, email: &str, reason: &str) -> Result<(), UserError>;

    /// Send account reactivation notification
    async fn send_reactivation_notification(&self, user_id: &str, email: &str) -> Result<(), UserError>;
}

/// User manager for comprehensive user management
pub struct UserManager {
    user_store: Arc<dyn UserStoreTrait>,
    role_manager: Arc<RBACSystem>,
    audit_logger: Arc<AuditLogger>,
    notification_service: Arc<dyn NotificationService>,
    jwt_auth: Arc<JWTAuthSystem>,
    verification_tokens: Arc<RwLock<std::collections::HashMap<String, String>>>, // token -> user_id
}

impl UserManager {
    /// Create a new user manager
    pub fn new(
        user_store: Arc<dyn UserStoreTrait>,
        role_manager: Arc<RBACSystem>,
        audit_logger: Arc<AuditLogger>,
        notification_service: Arc<dyn NotificationService>,
        jwt_auth: Arc<JWTAuthSystem>,
    ) -> Self {
        Self {
            user_store,
            role_manager,
            audit_logger,
            notification_service,
            jwt_auth,
            verification_tokens: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Create a new user manager with DotDB persistent storage
    pub fn new_with_dotdb_persistent<P: AsRef<std::path::Path>>(
        path: P,
        role_manager: Arc<RBACSystem>,
        audit_logger: Arc<AuditLogger>,
        notification_service: Arc<dyn NotificationService>,
        jwt_auth: Arc<JWTAuthSystem>,
    ) -> Result<Self, UserError> {
        let user_store = Arc::new(UserStore::new_persistent(path)?) as Arc<dyn UserStoreTrait>;
        Ok(Self::new(user_store, role_manager, audit_logger, notification_service, jwt_auth))
    }

    /// Initialize the user store (create collections, etc.)
    pub async fn initialize(&self) -> Result<(), UserError> {
        // Try to downcast to UserStoreTrait to call initialize
        if let Some(dotdb_store) = self.user_store.as_any().downcast_ref::<UserStore>() {
            dotdb_store.initialize().await?;
        }
        Ok(())
    }

    /// Create a new user
    pub async fn create_user(&self, registration: UserRegistration) -> Result<User, UserError> {
        // Validate registration data
        self.validate_registration(&registration).await?;

        // Check if email or username already exists
        if self.user_store.email_exists(&registration.email).await? {
            return Err(UserError::EmailExists { email: registration.email });
        }

        if self.user_store.username_exists(&registration.username).await? {
            return Err(UserError::UsernameExists { username: registration.username });
        }

        // Hash password
        let password_hash = self.jwt_auth.hash_password(&registration.password).map_err(|e| UserError::ValidationError {
            message: format!("Password hashing failed: {}", e),
        })?;

        // Create user
        let user_id = Uuid::new_v4().to_string();
        let mut user = User::new(user_id.clone(), registration.email.clone(), registration.username.clone());

        // Set profile if provided
        if let Some(profile) = registration.profile {
            user.profile = profile;
        }

        // Set preferences if provided
        if let Some(preferences) = registration.preferences {
            user.preferences = preferences;
        }

        // Set initial status
        user.status = if registration.require_verification.unwrap_or(true) {
            UserStatus::PendingVerification
        } else {
            UserStatus::Active
        };

        // Store user
        let created_user = self.user_store.create_user(user).await?;

        // Log registration
        self.audit_logger.log_registration(&user_id, None, None).await?;

        // Send verification email if required
        if created_user.status == UserStatus::PendingVerification {
            let verification_token = Uuid::new_v4().to_string();
            {
                let mut tokens = self.verification_tokens.write().await;
                tokens.insert(verification_token.clone(), user_id.clone());
            }

            self.notification_service.send_email_verification(&user_id, &created_user.email, &verification_token).await?;
        } else {
            // Send welcome email for active users
            self.notification_service.send_welcome_email(&user_id, &created_user.email).await?;
        }

        Ok(created_user)
    }

    /// Update user
    pub async fn update_user(&self, user_id: &str, updates: UserUpdates) -> Result<User, UserError> {
        // Get existing user
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        let mut updated_fields = Vec::new();

        // Update fields
        if let Some(email) = updates.email {
            if email != user.email {
                // Check if new email already exists
                if self.user_store.email_exists(&email).await? {
                    return Err(UserError::EmailExists { email });
                }
                user.email = email;
                updated_fields.push("email".to_string());
                // Reset verification status if email changed
                if user.status == UserStatus::Active {
                    user.status = UserStatus::PendingVerification;
                }
            }
        }

        if let Some(username) = updates.username {
            if username != user.username {
                // Check if new username already exists
                if self.user_store.username_exists(&username).await? {
                    return Err(UserError::UsernameExists { username });
                }
                user.username = username;
                updated_fields.push("username".to_string());
            }
        }

        if let Some(profile) = updates.profile {
            user.profile = profile;
            updated_fields.push("profile".to_string());
        }

        if let Some(preferences) = updates.preferences {
            user.preferences = preferences;
            updated_fields.push("preferences".to_string());
        }

        if let Some(status) = updates.status {
            user.update_status(status)?;
            updated_fields.push("status".to_string());
        }

        // Update user
        let updated_user = self.user_store.update_user(user).await?;

        // Log update
        self.audit_logger.log_profile_update(user_id, updated_fields, user_id).await?;

        Ok(updated_user)
    }

    /// Assign role to user
    pub async fn assign_role(&self, user_id: &str, role_id: &str) -> Result<(), UserError> {
        // Check if user exists
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Check if role already assigned
        if user.roles.contains(&role_id.to_string()) {
            return Ok(()); // Already assigned
        }

        // Add role
        user.roles.push(role_id.to_string());

        // Update user
        self.user_store.update_user(user).await?;

        // Assign role in RBAC system
        self.role_manager.assign_role(user_id, role_id, "system").await.map_err(|e| UserError::StorageError {
            message: format!("RBAC role assignment failed: {}", e),
        })?;

        // Log role assignment
        self.audit_logger.log_role_assignment(user_id, role_id, "system").await?;

        Ok(())
    }

    /// Revoke role from user
    pub async fn revoke_role(&self, user_id: &str, role_id: &str) -> Result<(), UserError> {
        // Check if user exists
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Remove role
        user.roles.retain(|r| r != role_id);

        // Update user
        self.user_store.update_user(user).await?;

        // Revoke role in RBAC system
        self.role_manager.revoke_role(user_id, role_id, "system").await.map_err(|e| UserError::StorageError {
            message: format!("RBAC role revocation failed: {}", e),
        })?;

        // Log role revocation
        self.audit_logger.log_role_revocation(user_id, role_id, "system").await?;

        Ok(())
    }

    /// Suspend user account
    pub async fn suspend_user(&self, user_id: &str, reason: &str) -> Result<(), UserError> {
        // Get user
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Update status
        user.update_status(UserStatus::Suspended)?;

        // Update user
        self.user_store.update_user(user.clone()).await?;

        // Log suspension
        self.audit_logger.log_account_suspension(user_id, reason, "system").await?;

        // Send notification
        self.notification_service.send_suspension_notification(user_id, &user.email, reason).await?;

        Ok(())
    }

    /// Reactivate user account
    pub async fn reactivate_user(&self, user_id: &str) -> Result<(), UserError> {
        // Get user
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Update status
        user.update_status(UserStatus::Active)?;

        // Update user
        self.user_store.update_user(user.clone()).await?;

        // Log reactivation
        self.audit_logger.log_account_reactivation(user_id, "system").await?;

        // Send notification
        self.notification_service.send_reactivation_notification(user_id, &user.email).await?;

        Ok(())
    }

    /// Delete user account (soft delete)
    pub async fn delete_user(&self, user_id: &str) -> Result<(), UserError> {
        // Check if user exists
        let user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Soft delete
        self.user_store.delete_user(user_id).await?;

        // Log deletion
        self.audit_logger.log_account_deletion(user_id, "system", None).await?;

        Ok(())
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<Option<User>, UserError> {
        self.user_store.get_user(user_id).await
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, UserError> {
        self.user_store.get_user_by_email(email).await
    }

    /// Get user by username
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, UserError> {
        self.user_store.get_user_by_username(username).await
    }

    /// Search users
    pub async fn search_users(&self, query: &UserSearchQuery) -> Result<UserSearchResults, UserError> {
        self.user_store.search_users(query).await
    }

    /// Verify email address
    pub async fn verify_email(&self, verification_token: &str) -> Result<(), UserError> {
        // Get user ID from token
        let user_id = {
            let tokens = self.verification_tokens.read().await;
            tokens.get(verification_token).cloned()
        };

        let user_id = user_id.ok_or_else(|| UserError::ValidationError {
            message: "Invalid verification token".to_string(),
        })?;

        // Get user
        let mut user = self.user_store.get_user(&user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.clone() })?;

        // Update status to active
        user.update_status(UserStatus::Active)?;

        // Update user
        self.user_store.update_user(user.clone()).await?;

        // Remove verification token
        {
            let mut tokens = self.verification_tokens.write().await;
            tokens.remove(verification_token);
        }

        // Log verification
        self.audit_logger.log_email_verification(&user_id, None).await?;

        // Send welcome email
        self.notification_service.send_welcome_email(&user_id, &user.email).await?;

        Ok(())
    }

    /// Get user count
    pub async fn get_user_count(&self) -> Result<u64, UserError> {
        self.user_store.get_user_count().await
    }

    /// Get users by role
    pub async fn get_users_by_role(&self, role: &str) -> Result<Vec<User>, UserError> {
        self.user_store.get_users_by_role(role).await
    }

    /// Validate registration data
    async fn validate_registration(&self, registration: &UserRegistration) -> Result<(), UserError> {
        // Validate email format
        if !self.is_valid_email(&registration.email) {
            return Err(UserError::ValidationError {
                message: "Invalid email format".to_string(),
            });
        }

        // Validate username
        if registration.username.len() < 3 || registration.username.len() > 50 {
            return Err(UserError::ValidationError {
                message: "Username must be between 3 and 50 characters".to_string(),
            });
        }

        // Validate password strength
        if registration.password.len() < 8 {
            return Err(UserError::ValidationError {
                message: "Password must be at least 8 characters long".to_string(),
            });
        }

        Ok(())
    }

    /// Simple email validation
    fn is_valid_email(&self, email: &str) -> bool {
        email.contains('@') && email.contains('.') && email.len() > 5
    }
}

/// Mock notification service for testing
pub struct MockNotificationService;

#[async_trait]
impl NotificationService for MockNotificationService {
    async fn send_email_verification(&self, _user_id: &str, _email: &str, _verification_token: &str) -> Result<(), UserError> {
        // Mock implementation - in production this would send actual emails
        Ok(())
    }

    async fn send_welcome_email(&self, _user_id: &str, _email: &str) -> Result<(), UserError> {
        // Mock implementation
        Ok(())
    }

    async fn send_password_reset(&self, _user_id: &str, _email: &str, _reset_token: &str) -> Result<(), UserError> {
        // Mock implementation
        Ok(())
    }

    async fn send_suspension_notification(&self, _user_id: &str, _email: &str, _reason: &str) -> Result<(), UserError> {
        // Mock implementation
        Ok(())
    }

    async fn send_reactivation_notification(&self, _user_id: &str, _email: &str) -> Result<(), UserError> {
        // Mock implementation
        Ok(())
    }
}

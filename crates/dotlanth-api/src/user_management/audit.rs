//! User activity audit logging

use crate::user_management::models::{UserActivity, UserError};
use crate::user_management::store::ActivityStoreTrait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// Audit logger for user activities
pub struct AuditLogger {
    activity_store: Arc<dyn ActivityStoreTrait>,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(activity_store: Arc<dyn ActivityStoreTrait>) -> Self {
        Self { activity_store }
    }

    /// Log user login
    pub async fn log_login(&self, user_id: &str, ip_address: Option<String>, user_agent: Option<String>) -> Result<(), UserError> {
        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "login".to_string(),
            description: "User logged in".to_string(),
            metadata: HashMap::new(),
            ip_address,
            user_agent,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log user logout
    pub async fn log_logout(&self, user_id: &str, ip_address: Option<String>) -> Result<(), UserError> {
        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "logout".to_string(),
            description: "User logged out".to_string(),
            metadata: HashMap::new(),
            ip_address,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log user registration
    pub async fn log_registration(&self, user_id: &str, ip_address: Option<String>, user_agent: Option<String>) -> Result<(), UserError> {
        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "registration".to_string(),
            description: "User account created".to_string(),
            metadata: HashMap::new(),
            ip_address,
            user_agent,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log profile update
    pub async fn log_profile_update(&self, user_id: &str, updated_fields: Vec<String>, updated_by: &str) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("updated_fields".to_string(), json!(updated_fields));
        metadata.insert("updated_by".to_string(), json!(updated_by));

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "profile_update".to_string(),
            description: "User profile updated".to_string(),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log role assignment
    pub async fn log_role_assignment(&self, user_id: &str, role_id: &str, assigned_by: &str) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("role_id".to_string(), json!(role_id));
        metadata.insert("assigned_by".to_string(), json!(assigned_by));

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "role_assignment".to_string(),
            description: format!("Role '{}' assigned to user", role_id),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log role revocation
    pub async fn log_role_revocation(&self, user_id: &str, role_id: &str, revoked_by: &str) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("role_id".to_string(), json!(role_id));
        metadata.insert("revoked_by".to_string(), json!(revoked_by));

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "role_revocation".to_string(),
            description: format!("Role '{}' revoked from user", role_id),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log account suspension
    pub async fn log_account_suspension(&self, user_id: &str, reason: &str, suspended_by: &str) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("reason".to_string(), json!(reason));
        metadata.insert("suspended_by".to_string(), json!(suspended_by));

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "account_suspension".to_string(),
            description: "User account suspended".to_string(),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log account reactivation
    pub async fn log_account_reactivation(&self, user_id: &str, reactivated_by: &str) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("reactivated_by".to_string(), json!(reactivated_by));

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "account_reactivation".to_string(),
            description: "User account reactivated".to_string(),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log account deletion
    pub async fn log_account_deletion(&self, user_id: &str, deleted_by: &str, reason: Option<String>) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("deleted_by".to_string(), json!(deleted_by));
        if let Some(reason) = reason {
            metadata.insert("reason".to_string(), json!(reason));
        }

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "account_deletion".to_string(),
            description: "User account deleted".to_string(),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log password change
    pub async fn log_password_change(&self, user_id: &str, ip_address: Option<String>) -> Result<(), UserError> {
        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "password_change".to_string(),
            description: "User password changed".to_string(),
            metadata: HashMap::new(),
            ip_address,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log email verification
    pub async fn log_email_verification(&self, user_id: &str, ip_address: Option<String>) -> Result<(), UserError> {
        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "email_verification".to_string(),
            description: "Email address verified".to_string(),
            metadata: HashMap::new(),
            ip_address,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log data export request
    pub async fn log_data_export(&self, user_id: &str, export_type: &str, requested_by: &str) -> Result<(), UserError> {
        let mut metadata = HashMap::new();
        metadata.insert("export_type".to_string(), json!(export_type));
        metadata.insert("requested_by".to_string(), json!(requested_by));

        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: "data_export".to_string(),
            description: "User data export requested".to_string(),
            metadata,
            ip_address: None,
            user_agent: None,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Log custom activity
    pub async fn log_custom_activity(
        &self,
        user_id: &str,
        activity_type: &str,
        description: &str,
        metadata: HashMap<String, serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<(), UserError> {
        let activity = UserActivity {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            activity_type: activity_type.to_string(),
            description: description.to_string(),
            metadata,
            ip_address,
            user_agent,
            timestamp: SystemTime::now(),
        };

        self.activity_store.log_activity(activity).await
    }

    /// Get user activities
    pub async fn get_user_activities(&self, user_id: &str, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<UserActivity>, UserError> {
        self.activity_store.get_user_activities(user_id, limit, offset).await
    }

    /// Get activities by type
    pub async fn get_activities_by_type(&self, activity_type: &str, limit: Option<u32>) -> Result<Vec<UserActivity>, UserError> {
        self.activity_store.get_activities_by_type(activity_type, limit).await
    }

    /// Clean old activities
    pub async fn clean_old_activities(&self, older_than: SystemTime) -> Result<u64, UserError> {
        self.activity_store.clean_old_activities(older_than).await
    }
}

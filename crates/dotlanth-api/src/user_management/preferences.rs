//! User preferences management

use crate::user_management::models::{DashboardConfig, NotificationPreferences, PrivacySettings, UserError, UserPreferences};
use crate::user_management::store::UserStoreTrait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// User preferences manager
pub struct PreferencesManager {
    user_store: Arc<dyn UserStoreTrait>,
}

impl PreferencesManager {
    /// Create a new preferences manager
    pub fn new(user_store: Arc<dyn UserStoreTrait>) -> Self {
        Self { user_store }
    }

    /// Get user preferences
    pub async fn get_preferences(&self, user_id: &str) -> Result<UserPreferences, UserError> {
        let user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        Ok(user.preferences)
    }

    /// Update user preferences
    pub async fn update_preferences(&self, user_id: &str, preferences: UserPreferences) -> Result<UserPreferences, UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Validate preferences
        self.validate_preferences(&preferences)?;

        user.preferences = preferences;
        let updated_user = self.user_store.update_user(user).await?;

        Ok(updated_user.preferences)
    }

    /// Update theme preference
    pub async fn update_theme(&self, user_id: &str, theme: String) -> Result<(), UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Validate theme
        if !["light", "dark", "auto"].contains(&theme.as_str()) {
            return Err(UserError::ValidationError {
                message: "Invalid theme. Must be 'light', 'dark', or 'auto'".to_string(),
            });
        }

        user.preferences.theme = theme;
        self.user_store.update_user(user).await?;

        Ok(())
    }

    /// Update notification preferences
    pub async fn update_notification_preferences(&self, user_id: &str, notifications: NotificationPreferences) -> Result<(), UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        user.preferences.notifications = notifications;
        self.user_store.update_user(user).await?;

        Ok(())
    }

    /// Update privacy settings
    pub async fn update_privacy_settings(&self, user_id: &str, privacy: PrivacySettings) -> Result<(), UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Validate privacy settings
        if !["public", "private", "friends"].contains(&privacy.profile_visibility.as_str()) {
            return Err(UserError::ValidationError {
                message: "Invalid profile visibility. Must be 'public', 'private', or 'friends'".to_string(),
            });
        }

        user.preferences.privacy = privacy;
        self.user_store.update_user(user).await?;

        Ok(())
    }

    /// Update dashboard configuration
    pub async fn update_dashboard_config(&self, user_id: &str, dashboard: DashboardConfig) -> Result<(), UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Validate dashboard config
        if dashboard.refresh_interval < 5 || dashboard.refresh_interval > 3600 {
            return Err(UserError::ValidationError {
                message: "Refresh interval must be between 5 and 3600 seconds".to_string(),
            });
        }

        user.preferences.dashboard = dashboard;
        self.user_store.update_user(user).await?;

        Ok(())
    }

    /// Set custom preference
    pub async fn set_custom_preference(&self, user_id: &str, key: String, value: Value) -> Result<(), UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        user.preferences.custom.insert(key, value);
        self.user_store.update_user(user).await?;

        Ok(())
    }

    /// Get custom preference
    pub async fn get_custom_preference(&self, user_id: &str, key: &str) -> Result<Option<Value>, UserError> {
        let user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        Ok(user.preferences.custom.get(key).cloned())
    }

    /// Remove custom preference
    pub async fn remove_custom_preference(&self, user_id: &str, key: &str) -> Result<(), UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        user.preferences.custom.remove(key);
        self.user_store.update_user(user).await?;

        Ok(())
    }

    /// Reset preferences to default
    pub async fn reset_preferences(&self, user_id: &str) -> Result<UserPreferences, UserError> {
        let mut user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        user.preferences = UserPreferences::default();
        let updated_user = self.user_store.update_user(user).await?;

        Ok(updated_user.preferences)
    }

    /// Validate preferences
    fn validate_preferences(&self, preferences: &UserPreferences) -> Result<(), UserError> {
        // Validate theme
        if !["light", "dark", "auto"].contains(&preferences.theme.as_str()) {
            return Err(UserError::ValidationError { message: "Invalid theme".to_string() });
        }

        // Validate privacy settings
        if !["public", "private", "friends"].contains(&preferences.privacy.profile_visibility.as_str()) {
            return Err(UserError::ValidationError {
                message: "Invalid profile visibility".to_string(),
            });
        }

        // Validate dashboard refresh interval
        if preferences.dashboard.refresh_interval < 5 || preferences.dashboard.refresh_interval > 3600 {
            return Err(UserError::ValidationError {
                message: "Invalid refresh interval".to_string(),
            });
        }

        Ok(())
    }
}

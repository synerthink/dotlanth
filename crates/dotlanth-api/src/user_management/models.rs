//! User management data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// User entity with comprehensive profile information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    /// Unique user identifier
    pub id: String,

    /// User's email address (unique)
    pub email: String,

    /// Username (unique)
    pub username: String,

    /// User profile information
    pub profile: UserProfile,

    /// Assigned roles
    pub roles: Vec<String>,

    /// Current account status
    pub status: UserStatus,

    /// User preferences and settings
    pub preferences: UserPreferences,

    /// Account creation timestamp
    pub created_at: SystemTime,

    /// Last update timestamp
    pub updated_at: SystemTime,

    /// Last login timestamp
    pub last_login: Option<SystemTime>,
}

/// User account status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    /// Account is active and can be used
    Active,

    /// Account is suspended (temporarily disabled)
    Suspended,

    /// Account is pending email verification
    PendingVerification,

    /// Account is soft-deleted (can be recovered)
    Deleted,
}

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    /// Display name
    pub display_name: Option<String>,

    /// First name
    pub first_name: Option<String>,

    /// Last name
    pub last_name: Option<String>,

    /// Profile picture URL
    pub avatar_url: Option<String>,

    /// User bio/description
    pub bio: Option<String>,

    /// User location
    pub location: Option<String>,

    /// User website
    pub website: Option<String>,

    /// User timezone
    pub timezone: Option<String>,

    /// User language preference
    pub language: Option<String>,

    /// Additional profile metadata
    pub metadata: HashMap<String, String>,
}

/// User preferences and settings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPreferences {
    /// Theme preference (light, dark, auto)
    pub theme: String,

    /// Notification preferences
    pub notifications: NotificationPreferences,

    /// Privacy settings
    pub privacy: PrivacySettings,

    /// Dashboard configuration
    pub dashboard: DashboardConfig,

    /// Additional custom preferences
    pub custom: HashMap<String, serde_json::Value>,
}

/// Notification preferences
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NotificationPreferences {
    /// Email notifications enabled
    pub email_enabled: bool,

    /// Push notifications enabled
    pub push_enabled: bool,

    /// SMS notifications enabled
    pub sms_enabled: bool,

    /// Notification types to receive
    pub types: Vec<String>,

    /// Quiet hours configuration
    pub quiet_hours: Option<QuietHours>,
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuietHours {
    /// Start time (24-hour format, e.g., "22:00")
    pub start: String,

    /// End time (24-hour format, e.g., "08:00")
    pub end: String,

    /// Timezone for quiet hours
    pub timezone: String,
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrivacySettings {
    /// Profile visibility (public, private, friends)
    pub profile_visibility: String,

    /// Email visibility
    pub email_visible: bool,

    /// Activity tracking enabled
    pub activity_tracking: bool,

    /// Data collection consent
    pub data_collection_consent: bool,

    /// Marketing emails consent
    pub marketing_consent: bool,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardConfig {
    /// Widget layout
    pub layout: Vec<DashboardWidget>,

    /// Default view
    pub default_view: String,

    /// Refresh interval in seconds
    pub refresh_interval: u32,
}

/// Dashboard widget configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardWidget {
    /// Widget ID
    pub id: String,

    /// Widget type
    pub widget_type: String,

    /// Widget position
    pub position: WidgetPosition,

    /// Widget configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Widget position on dashboard
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WidgetPosition {
    /// X coordinate
    pub x: u32,

    /// Y coordinate
    pub y: u32,

    /// Width
    pub width: u32,

    /// Height
    pub height: u32,
}

/// User registration request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserRegistration {
    /// Email address
    pub email: String,

    /// Username
    pub username: String,

    /// Password
    pub password: String,

    /// Initial profile information
    pub profile: Option<UserProfile>,

    /// Initial preferences
    pub preferences: Option<UserPreferences>,

    /// Email verification required
    pub require_verification: Option<bool>,
}

/// User update request
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserUpdates {
    /// Updated email
    pub email: Option<String>,

    /// Updated username
    pub username: Option<String>,

    /// Updated profile
    pub profile: Option<UserProfile>,

    /// Updated preferences
    pub preferences: Option<UserPreferences>,

    /// Updated status
    pub status: Option<UserStatus>,
}

/// User search query
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserSearchQuery {
    /// Search term (searches username, email, display name)
    pub query: Option<String>,

    /// Filter by status
    pub status: Option<UserStatus>,

    /// Filter by roles
    pub roles: Option<Vec<String>>,

    /// Created after date
    pub created_after: Option<DateTime<Utc>>,

    /// Created before date
    pub created_before: Option<DateTime<Utc>>,

    /// Last login after date
    pub last_login_after: Option<DateTime<Utc>>,

    /// Sort field
    pub sort_by: Option<String>,

    /// Sort direction (asc, desc)
    pub sort_direction: Option<String>,

    /// Page number (1-based)
    pub page: Option<u32>,

    /// Page size
    pub page_size: Option<u32>,
}

/// User search results
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserSearchResults {
    /// Found users
    pub users: Vec<UserSummary>,

    /// Total count
    pub total_count: u64,

    /// Current page
    pub page: u32,

    /// Page size
    pub page_size: u32,

    /// Total pages
    pub total_pages: u32,
}

/// User summary for search results
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserSummary {
    /// User ID
    pub id: String,

    /// Username
    pub username: String,

    /// Email (may be hidden based on privacy settings)
    pub email: Option<String>,

    /// Display name
    pub display_name: Option<String>,

    /// Avatar URL
    pub avatar_url: Option<String>,

    /// Account status
    pub status: UserStatus,

    /// Roles
    pub roles: Vec<String>,

    /// Created at
    pub created_at: SystemTime,

    /// Last login
    pub last_login: Option<SystemTime>,
}

/// User activity log entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserActivity {
    /// Activity ID
    pub id: String,

    /// User ID
    pub user_id: String,

    /// Activity type
    pub activity_type: String,

    /// Activity description
    pub description: String,

    /// Activity metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// IP address
    pub ip_address: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Timestamp
    pub timestamp: SystemTime,
}

/// User data export request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDataExportRequest {
    /// Export format (json, csv, xml)
    pub format: String,

    /// Data types to include
    pub include_data: Vec<String>,

    /// Date range start
    pub date_from: Option<DateTime<Utc>>,

    /// Date range end
    pub date_to: Option<DateTime<Utc>>,
}

/// User data export response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDataExport {
    /// Export ID
    pub export_id: String,

    /// Export status
    pub status: ExportStatus,

    /// Download URL (when ready)
    pub download_url: Option<String>,

    /// Export size in bytes
    pub size_bytes: Option<u64>,

    /// Created at
    pub created_at: SystemTime,

    /// Expires at
    pub expires_at: SystemTime,
}

/// Export status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    /// Export is being processed
    Processing,

    /// Export is ready for download
    Ready,

    /// Export failed
    Failed,

    /// Export expired
    Expired,
}

/// User error types
#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("User not found: {user_id}")]
    UserNotFound { user_id: String },

    #[error("Username already exists: {username}")]
    UsernameExists { username: String },

    #[error("Email already exists: {email}")]
    EmailExists { email: String },

    #[error("Invalid user status transition from {from:?} to {to:?}")]
    InvalidStatusTransition { from: UserStatus, to: UserStatus },

    #[error("User account is suspended: {reason}")]
    AccountSuspended { reason: String },

    #[error("User account requires verification")]
    VerificationRequired,

    #[error("Invalid user data: {message}")]
    InvalidData { message: String },

    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Validation error: {message}")]
    ValidationError { message: String },
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: None,
            first_name: None,
            last_name: None,
            avatar_url: None,
            bio: None,
            location: None,
            website: None,
            timezone: None,
            language: Some("en".to_string()),
            metadata: HashMap::new(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "auto".to_string(),
            notifications: NotificationPreferences::default(),
            privacy: PrivacySettings::default(),
            dashboard: DashboardConfig::default(),
            custom: HashMap::new(),
        }
    }
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_enabled: true,
            push_enabled: true,
            sms_enabled: false,
            types: vec!["security".to_string(), "system".to_string(), "updates".to_string()],
            quiet_hours: None,
        }
    }
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            profile_visibility: "public".to_string(),
            email_visible: false,
            activity_tracking: true,
            data_collection_consent: false,
            marketing_consent: false,
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            layout: vec![],
            default_view: "overview".to_string(),
            refresh_interval: 30,
        }
    }
}

impl User {
    /// Create a new user
    pub fn new(id: String, email: String, username: String) -> Self {
        let now = SystemTime::now();

        Self {
            id,
            email,
            username,
            profile: UserProfile::default(),
            roles: vec!["user".to_string()],
            status: UserStatus::PendingVerification,
            preferences: UserPreferences::default(),
            created_at: now,
            updated_at: now,
            last_login: None,
        }
    }

    /// Check if user is active
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// Check if user can login
    pub fn can_login(&self) -> bool {
        matches!(self.status, UserStatus::Active | UserStatus::PendingVerification)
    }

    /// Update last login timestamp
    pub fn update_last_login(&mut self) {
        self.last_login = Some(SystemTime::now());
        self.updated_at = SystemTime::now();
    }

    /// Update user status
    pub fn update_status(&mut self, status: UserStatus) -> Result<(), UserError> {
        // Validate status transition
        match (&self.status, &status) {
            (UserStatus::Deleted, _) => {
                return Err(UserError::InvalidStatusTransition {
                    from: self.status.clone(),
                    to: status,
                });
            }
            _ => {}
        }

        self.status = status;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    /// Convert to user summary
    pub fn to_summary(&self, include_email: bool) -> UserSummary {
        UserSummary {
            id: self.id.clone(),
            username: self.username.clone(),
            email: if include_email { Some(self.email.clone()) } else { None },
            display_name: self.profile.display_name.clone(),
            avatar_url: self.profile.avatar_url.clone(),
            status: self.status.clone(),
            roles: self.roles.clone(),
            created_at: self.created_at,
            last_login: self.last_login,
        }
    }
}

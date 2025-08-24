//! User data export functionality for GDPR compliance

use crate::user_management::audit::AuditLogger;
use crate::user_management::models::{ExportStatus, User, UserActivity, UserDataExport, UserDataExportRequest, UserError};
use crate::user_management::store::{ActivityStoreTrait, UserStoreTrait};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

/// User data export service for GDPR compliance
pub struct UserDataExportService {
    user_store: Arc<dyn UserStoreTrait>,
    activity_store: Arc<dyn ActivityStoreTrait>,
    audit_logger: Arc<AuditLogger>,
    exports: Arc<RwLock<HashMap<String, UserDataExport>>>,
}

impl UserDataExportService {
    /// Create a new user data export service
    pub fn new(user_store: Arc<dyn UserStoreTrait>, activity_store: Arc<dyn ActivityStoreTrait>, audit_logger: Arc<AuditLogger>) -> Self {
        Self {
            user_store,
            activity_store,
            audit_logger,
            exports: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request user data export
    pub async fn request_export(&self, user_id: &str, request: UserDataExportRequest) -> Result<UserDataExport, UserError> {
        // Validate request
        self.validate_export_request(&request)?;

        // Check if user exists
        let user = self.user_store.get_user(user_id).await?.ok_or_else(|| UserError::UserNotFound { user_id: user_id.to_string() })?;

        // Create export record
        let export_id = Uuid::new_v4().to_string();
        let now = SystemTime::now();
        let expires_at = now + std::time::Duration::from_secs(7 * 24 * 60 * 60); // 7 days

        let export = UserDataExport {
            export_id: export_id.clone(),
            status: ExportStatus::Processing,
            download_url: None,
            size_bytes: None,
            created_at: now,
            expires_at,
        };

        // Store export record
        {
            let mut exports = self.exports.write().await;
            exports.insert(export_id.clone(), export.clone());
        }

        // Log export request
        self.audit_logger.log_data_export(user_id, &request.format, user_id).await?;

        // Process export asynchronously (in a real implementation, this would be queued)
        let export_service = self.clone();
        let user_clone = user.clone();
        let request_clone = request.clone();
        tokio::spawn(async move {
            if let Err(e) = export_service.process_export(&export_id, &user_clone, &request_clone).await {
                tracing::error!("Failed to process export {}: {}", export_id, e);
                // Mark export as failed
                let mut exports = export_service.exports.write().await;
                if let Some(export) = exports.get_mut(&export_id) {
                    export.status = ExportStatus::Failed;
                }
            }
        });

        Ok(export)
    }

    /// Get export status
    pub async fn get_export_status(&self, export_id: &str) -> Result<Option<UserDataExport>, UserError> {
        let exports = self.exports.read().await;
        Ok(exports.get(export_id).cloned())
    }

    /// Get user exports
    pub async fn get_user_exports(&self, user_id: &str) -> Result<Vec<UserDataExport>, UserError> {
        let exports = self.exports.read().await;
        let user_exports: Vec<UserDataExport> = exports
            .values()
            .filter(|export| {
                // In a real implementation, we'd store user_id with the export
                // For now, we'll return all exports
                true
            })
            .cloned()
            .collect();

        Ok(user_exports)
    }

    /// Delete expired exports
    pub async fn cleanup_expired_exports(&self) -> Result<u64, UserError> {
        let mut exports = self.exports.write().await;
        let now = SystemTime::now();
        let original_count = exports.len();

        exports.retain(|_, export| export.expires_at > now);

        let removed_count = original_count - exports.len();
        Ok(removed_count as u64)
    }

    /// Process export (generate the actual export data)
    async fn process_export(&self, export_id: &str, user: &User, request: &UserDataExportRequest) -> Result<(), UserError> {
        // Collect user data based on request
        let mut export_data = json!({
            "user_id": user.id,
            "export_id": export_id,
            "generated_at": Utc::now().to_rfc3339(),
            "format": request.format,
            "data": {}
        });

        // Include basic user data
        if request.include_data.contains(&"profile".to_string()) || request.include_data.contains(&"all".to_string()) {
            export_data["data"]["profile"] = json!({
                "id": user.id,
                "username": user.username,
                "email": user.email,
                "profile": user.profile,
                "status": user.status,
                "created_at": user.created_at,
                "updated_at": user.updated_at,
                "last_login": user.last_login,
            });
        }

        // Include preferences
        if request.include_data.contains(&"preferences".to_string()) || request.include_data.contains(&"all".to_string()) {
            export_data["data"]["preferences"] = json!(user.preferences);
        }

        // Include roles
        if request.include_data.contains(&"roles".to_string()) || request.include_data.contains(&"all".to_string()) {
            export_data["data"]["roles"] = json!(user.roles);
        }

        // Include activity logs
        if request.include_data.contains(&"activity".to_string()) || request.include_data.contains(&"all".to_string()) {
            let activities = self.activity_store.get_user_activities(&user.id, None, None).await?;

            // Filter by date range if specified
            let filtered_activities: Vec<&UserActivity> = activities
                .iter()
                .filter(|activity| {
                    if let Some(from) = request.date_from {
                        let activity_time = DateTime::<Utc>::from(activity.timestamp);
                        if activity_time < from {
                            return false;
                        }
                    }

                    if let Some(to) = request.date_to {
                        let activity_time = DateTime::<Utc>::from(activity.timestamp);
                        if activity_time > to {
                            return false;
                        }
                    }

                    true
                })
                .collect();

            export_data["data"]["activity"] = json!(filtered_activities);
        }

        // Generate export file based on format
        let export_content = match request.format.as_str() {
            "json" => self.generate_json_export(&export_data)?,
            "csv" => self.generate_csv_export(&export_data)?,
            "xml" => self.generate_xml_export(&export_data)?,
            _ => {
                return Err(UserError::ValidationError {
                    message: "Unsupported export format".to_string(),
                });
            }
        };

        let size_bytes = export_content.len() as u64;

        // In a real implementation, we would:
        // 1. Store the export file in a secure location (S3, etc.)
        // 2. Generate a secure download URL
        // 3. Set up automatic cleanup

        let download_url = format!("/api/v1/users/exports/{}/download", export_id);

        // Update export status
        {
            let mut exports = self.exports.write().await;
            if let Some(export) = exports.get_mut(export_id) {
                export.status = ExportStatus::Ready;
                export.download_url = Some(download_url);
                export.size_bytes = Some(size_bytes);
            }
        }

        Ok(())
    }

    /// Generate JSON export
    fn generate_json_export(&self, data: &Value) -> Result<Vec<u8>, UserError> {
        serde_json::to_vec_pretty(data).map_err(|e| UserError::ValidationError {
            message: format!("Failed to generate JSON export: {}", e),
        })
    }

    /// Generate CSV export
    fn generate_csv_export(&self, data: &Value) -> Result<Vec<u8>, UserError> {
        // Simplified CSV generation - in a real implementation, this would be more sophisticated
        let mut csv_content = String::new();

        // Add headers
        csv_content.push_str("field,value\n");

        // Flatten the JSON data into CSV rows
        self.flatten_json_to_csv(data, "", &mut csv_content);

        Ok(csv_content.into_bytes())
    }

    /// Generate XML export
    fn generate_xml_export(&self, data: &Value) -> Result<Vec<u8>, UserError> {
        // Simplified XML generation
        let mut xml_content = String::new();
        xml_content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml_content.push_str("<user_data_export>\n");

        self.json_to_xml(data, &mut xml_content, 1);

        xml_content.push_str("</user_data_export>\n");

        Ok(xml_content.into_bytes())
    }

    /// Helper function to flatten JSON to CSV
    fn flatten_json_to_csv(&self, value: &Value, prefix: &str, output: &mut String) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let new_prefix = if prefix.is_empty() { key.clone() } else { format!("{}.{}", prefix, key) };
                    self.flatten_json_to_csv(val, &new_prefix, output);
                }
            }
            Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let new_prefix = format!("{}[{}]", prefix, i);
                    self.flatten_json_to_csv(val, &new_prefix, output);
                }
            }
            _ => {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => value.to_string(),
                };
                output.push_str(&format!("{},{}\n", prefix, value_str));
            }
        }
    }

    /// Helper function to convert JSON to XML
    fn json_to_xml(&self, value: &Value, output: &mut String, indent: usize) {
        let indent_str = "  ".repeat(indent);

        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    output.push_str(&format!("{}<{}>\n", indent_str, key));
                    self.json_to_xml(val, output, indent + 1);
                    output.push_str(&format!("{}</{}>\n", indent_str, key));
                }
            }
            Value::Array(arr) => {
                for val in arr {
                    output.push_str(&format!("{}<item>\n", indent_str));
                    self.json_to_xml(val, output, indent + 1);
                    output.push_str(&format!("{}</item>\n", indent_str));
                }
            }
            _ => {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    _ => value.to_string(),
                };
                output.push_str(&format!("{}{}\n", indent_str, value_str));
            }
        }
    }

    /// Validate export request
    fn validate_export_request(&self, request: &UserDataExportRequest) -> Result<(), UserError> {
        // Validate format
        if !["json", "csv", "xml"].contains(&request.format.as_str()) {
            return Err(UserError::ValidationError {
                message: "Invalid export format. Must be 'json', 'csv', or 'xml'".to_string(),
            });
        }

        // Validate data types
        let valid_data_types = ["profile", "preferences", "roles", "activity", "all"];
        for data_type in &request.include_data {
            if !valid_data_types.contains(&data_type.as_str()) {
                return Err(UserError::ValidationError {
                    message: format!("Invalid data type: {}. Must be one of: {}", data_type, valid_data_types.join(", ")),
                });
            }
        }

        // Validate date range
        if let (Some(from), Some(to)) = (request.date_from, request.date_to) {
            if from >= to {
                return Err(UserError::ValidationError {
                    message: "date_from must be before date_to".to_string(),
                });
            }
        }

        Ok(())
    }
}

impl Clone for UserDataExportService {
    fn clone(&self) -> Self {
        Self {
            user_store: Arc::clone(&self.user_store),
            activity_store: Arc::clone(&self.activity_store),
            audit_logger: Arc::clone(&self.audit_logger),
            exports: Arc::clone(&self.exports),
        }
    }
}

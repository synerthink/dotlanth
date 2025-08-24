// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Audit logging for RBAC operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AuditEventType {
    /// Permission check performed
    PermissionCheck,
    /// Dot permission check performed
    DotPermissionCheck,
    /// Role created
    RoleCreated,
    /// Role updated
    RoleUpdated,
    /// Role deleted
    RoleDeleted,
    /// Role assigned to user
    RoleAssigned,
    /// Temporary role assigned to user
    TemporaryRoleAssigned,
    /// Role revoked from user
    RoleRevoked,
    /// Permission granted
    PermissionGranted,
    /// Permission denied
    PermissionDenied,
    /// Authentication attempt
    AuthenticationAttempt,
    /// Authorization failure
    AuthorizationFailure,
}

/// Audit event result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditResult {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
    /// Operation was denied
    Denied,
}

/// Audit event entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,

    /// Event type
    pub event_type: AuditEventType,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// User who performed the action
    pub actor: String,

    /// Target user (if applicable)
    pub target_user: Option<String>,

    /// Resource being accessed
    pub resource: Option<String>,

    /// Action being performed
    pub action: Option<String>,

    /// Event result
    pub result: AuditResult,

    /// Client IP address
    pub client_ip: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Additional event details
    pub details: HashMap<String, String>,

    /// Request ID for correlation
    pub request_id: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType, actor: String, result: AuditResult) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            actor,
            target_user: None,
            resource: None,
            action: None,
            result,
            client_ip: None,
            user_agent: None,
            details: HashMap::new(),
            request_id: None,
        }
    }

    /// Set target user
    pub fn with_target_user(mut self, target_user: String) -> Self {
        self.target_user = Some(target_user);
        self
    }

    /// Set resource and action
    pub fn with_resource_action(mut self, resource: String, action: String) -> Self {
        self.resource = Some(resource);
        self.action = Some(action);
        self
    }

    /// Set client information
    pub fn with_client_info(mut self, client_ip: Option<String>, user_agent: Option<String>) -> Self {
        self.client_ip = client_ip;
        self.user_agent = user_agent;
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Add detail
    pub fn with_detail(mut self, key: String, value: String) -> Self {
        self.details.insert(key, value);
        self
    }

    /// Add multiple details
    pub fn with_details(mut self, details: HashMap<String, String>) -> Self {
        self.details.extend(details);
        self
    }
}

/// Audit logger for RBAC operations
#[derive(Debug)]
pub struct AuditLogger {
    /// In-memory audit log (in production, this would be a persistent store)
    events: Arc<RwLock<Vec<AuditEvent>>>,

    /// Maximum number of events to keep in memory
    max_events: usize,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events: 10000, // Keep last 10k events in memory
        }
    }

    /// Create audit logger with custom max events
    pub fn with_max_events(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events,
        }
    }

    /// Log an audit event
    pub async fn log_event(&self, event: AuditEvent) {
        // Log to structured logging
        match event.result {
            AuditResult::Success => {
                info!(
                    event_type = ?event.event_type,
                    actor = %event.actor,
                    target_user = ?event.target_user,
                    resource = ?event.resource,
                    action = ?event.action,
                    client_ip = ?event.client_ip,
                    request_id = ?event.request_id,
                    "Audit event: {:?}", event.event_type
                );
            }
            AuditResult::Failure | AuditResult::Denied => {
                warn!(
                    event_type = ?event.event_type,
                    actor = %event.actor,
                    target_user = ?event.target_user,
                    resource = ?event.resource,
                    action = ?event.action,
                    result = ?event.result,
                    client_ip = ?event.client_ip,
                    request_id = ?event.request_id,
                    "Audit event: {:?} - {:?}", event.event_type, event.result
                );
            }
        }

        // Store in memory
        let mut events = self.events.write().await;
        events.push(event);

        // Trim if necessary
        if events.len() > self.max_events {
            let excess = events.len() - self.max_events;
            events.drain(0..excess);
        }
    }

    /// Log permission check
    pub async fn log_permission_check(&self, user_id: &str, resource: &str, action: &str, granted: bool, client_ip: Option<String>, request_id: Option<String>) {
        let result = if granted { AuditResult::Success } else { AuditResult::Denied };

        let mut event = AuditEvent::new(AuditEventType::PermissionCheck, user_id.to_string(), result)
            .with_resource_action(resource.to_string(), action.to_string())
            .with_client_info(client_ip, None);

        if let Some(req_id) = request_id {
            event = event.with_request_id(req_id);
        }

        self.log_event(event).await;
    }

    /// Log dot permission check
    pub async fn log_dot_permission_check(&self, user_id: &str, dot_id: &str, operation: &str, granted: bool, client_ip: Option<String>, request_id: Option<String>) {
        let result = if granted { AuditResult::Success } else { AuditResult::Denied };

        let mut event = AuditEvent::new(AuditEventType::DotPermissionCheck, user_id.to_string(), result)
            .with_resource_action(format!("dot:{}", dot_id), operation.to_string())
            .with_client_info(client_ip, None);

        if let Some(req_id) = request_id {
            event = event.with_request_id(req_id);
        }

        self.log_event(event).await;
    }

    /// Log role creation
    pub async fn log_role_created(&self, role_id: &str, created_by: &str) {
        let event = AuditEvent::new(AuditEventType::RoleCreated, created_by.to_string(), AuditResult::Success).with_detail("role_id".to_string(), role_id.to_string());

        self.log_event(event).await;
    }

    /// Log role update
    pub async fn log_role_updated(&self, role_id: &str, updated_by: &str) {
        let event = AuditEvent::new(AuditEventType::RoleUpdated, updated_by.to_string(), AuditResult::Success).with_detail("role_id".to_string(), role_id.to_string());

        self.log_event(event).await;
    }

    /// Log role deletion
    pub async fn log_role_deleted(&self, role_id: &str, deleted_by: &str) {
        let event = AuditEvent::new(AuditEventType::RoleDeleted, deleted_by.to_string(), AuditResult::Success).with_detail("role_id".to_string(), role_id.to_string());

        self.log_event(event).await;
    }

    /// Log role assignment
    pub async fn log_role_assigned(&self, user_id: &str, role_id: &str, assigned_by: &str) {
        let event = AuditEvent::new(AuditEventType::RoleAssigned, assigned_by.to_string(), AuditResult::Success)
            .with_target_user(user_id.to_string())
            .with_detail("role_id".to_string(), role_id.to_string());

        self.log_event(event).await;
    }

    /// Log temporary role assignment
    pub async fn log_temporary_role_assigned(&self, user_id: &str, role_id: &str, expires_at: DateTime<Utc>, assigned_by: &str) {
        let event = AuditEvent::new(AuditEventType::TemporaryRoleAssigned, assigned_by.to_string(), AuditResult::Success)
            .with_target_user(user_id.to_string())
            .with_detail("role_id".to_string(), role_id.to_string())
            .with_detail("expires_at".to_string(), expires_at.to_rfc3339());

        self.log_event(event).await;
    }

    /// Log role revocation
    pub async fn log_role_revoked(&self, user_id: &str, role_id: &str, revoked_by: &str) {
        let event = AuditEvent::new(AuditEventType::RoleRevoked, revoked_by.to_string(), AuditResult::Success)
            .with_target_user(user_id.to_string())
            .with_detail("role_id".to_string(), role_id.to_string());

        self.log_event(event).await;
    }

    /// Log authentication attempt
    pub async fn log_authentication_attempt(&self, user_id: &str, success: bool, client_ip: Option<String>, user_agent: Option<String>, request_id: Option<String>) {
        let result = if success { AuditResult::Success } else { AuditResult::Failure };

        let mut event = AuditEvent::new(AuditEventType::AuthenticationAttempt, user_id.to_string(), result).with_client_info(client_ip, user_agent);

        if let Some(req_id) = request_id {
            event = event.with_request_id(req_id);
        }

        self.log_event(event).await;
    }

    /// Log authorization failure
    pub async fn log_authorization_failure(&self, user_id: &str, resource: &str, action: &str, reason: &str, client_ip: Option<String>, request_id: Option<String>) {
        let mut event = AuditEvent::new(AuditEventType::AuthorizationFailure, user_id.to_string(), AuditResult::Denied)
            .with_resource_action(resource.to_string(), action.to_string())
            .with_detail("reason".to_string(), reason.to_string())
            .with_client_info(client_ip, None);

        if let Some(req_id) = request_id {
            event = event.with_request_id(req_id);
        }

        self.log_event(event).await;
    }

    /// Get audit events (for admin interface)
    pub async fn get_events(&self, limit: Option<usize>) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        if let Some(limit) = limit {
            events.iter().rev().take(limit).cloned().collect()
        } else {
            events.iter().rev().cloned().collect()
        }
    }

    /// Get events for a specific user
    pub async fn get_user_events(&self, user_id: &str, limit: Option<usize>) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        let user_events: Vec<AuditEvent> = events
            .iter()
            .filter(|event| event.actor == user_id || event.target_user.as_ref().map(|u| u == user_id).unwrap_or(false))
            .rev()
            .cloned()
            .collect();

        if let Some(limit) = limit { user_events.into_iter().take(limit).collect() } else { user_events }
    }

    /// Get events by type
    pub async fn get_events_by_type(&self, event_type: AuditEventType, limit: Option<usize>) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        let filtered_events: Vec<AuditEvent> = events.iter().filter(|event| event.event_type == event_type).rev().cloned().collect();

        if let Some(limit) = limit {
            filtered_events.into_iter().take(limit).collect()
        } else {
            filtered_events
        }
    }

    /// Get events within a time range
    pub async fn get_events_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>, limit: Option<usize>) -> Vec<AuditEvent> {
        let events = self.events.read().await;

        let filtered_events: Vec<AuditEvent> = events.iter().filter(|event| event.timestamp >= start && event.timestamp <= end).rev().cloned().collect();

        if let Some(limit) = limit {
            filtered_events.into_iter().take(limit).collect()
        } else {
            filtered_events
        }
    }

    /// Clear all audit events (admin only)
    pub async fn clear_events(&self) {
        let mut events = self.events.write().await;
        events.clear();

        info!("Audit log cleared");
    }

    /// Get audit statistics
    pub async fn get_statistics(&self) -> AuditStatistics {
        let events = self.events.read().await;

        let mut stats = AuditStatistics::default();
        stats.total_events = events.len();

        for event in events.iter() {
            match event.result {
                AuditResult::Success => stats.successful_events += 1,
                AuditResult::Failure => stats.failed_events += 1,
                AuditResult::Denied => stats.denied_events += 1,
            }

            *stats.events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        stats
    }
}

/// Audit statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuditStatistics {
    /// Total number of events
    pub total_events: usize,

    /// Number of successful events
    pub successful_events: usize,

    /// Number of failed events
    pub failed_events: usize,

    /// Number of denied events
    pub denied_events: usize,

    /// Events by type
    pub events_by_type: HashMap<AuditEventType, usize>,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::PermissionCheck, "user123".to_string(), AuditResult::Success)
            .with_resource_action("dots".to_string(), "read".to_string())
            .with_detail("test".to_string(), "value".to_string());

        assert_eq!(event.event_type, AuditEventType::PermissionCheck);
        assert_eq!(event.actor, "user123");
        assert_eq!(event.result, AuditResult::Success);
        assert_eq!(event.resource, Some("dots".to_string()));
        assert_eq!(event.action, Some("read".to_string()));
        assert_eq!(event.details.get("test"), Some(&"value".to_string()));
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let logger = AuditLogger::new();

        // Log some events
        logger.log_permission_check("user123", "dots", "read", true, None, None).await;
        logger.log_role_assigned("user123", "admin", "system").await;

        // Check events
        let events = logger.get_events(None).await;
        assert_eq!(events.len(), 2);

        // Check user events
        let user_events = logger.get_user_events("user123", None).await;
        assert_eq!(user_events.len(), 2);

        // Check statistics
        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.successful_events, 2);
    }

    #[tokio::test]
    async fn test_audit_logger_limits() {
        let logger = AuditLogger::with_max_events(2);

        // Log more events than the limit
        for i in 0..5 {
            logger.log_permission_check(&format!("user{}", i), "dots", "read", true, None, None).await;
        }

        // Should only keep the last 2 events
        let events = logger.get_events(None).await;
        assert_eq!(events.len(), 2);
    }
}

// Dotlanth
// Copyright (C) 2025 Synerthink

use crate::versioning::{ApiVersion, ProtocolType, ServiceType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Deprecation management errors
#[derive(Error, Debug)]
pub enum DeprecationError {
    #[error("Feature already deprecated: {0}")]
    AlreadyDeprecated(String),
    #[error("Invalid deprecation schedule: {0}")]
    InvalidSchedule(String),
    #[error("Deprecation not found: {0}")]
    NotFound(String),
}

/// Deprecation timeline phase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeprecationPhase {
    /// Feature is active and not deprecated
    Active,
    /// Feature is deprecated but still supported
    Deprecated,
    /// Feature is in sunset phase (will be removed soon)
    Sunset,
    /// Feature has been removed
    Removed,
}

/// Deprecation policy for managing API lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationPolicy {
    /// Minimum time a feature must be deprecated before removal
    pub min_deprecation_period: chrono::Duration,
    /// Notification periods before removal
    pub notification_periods: Vec<chrono::Duration>,
    /// Whether to enforce strict deprecation rules
    pub strict_enforcement: bool,
    /// Whether to allow extending deprecation timelines
    pub allow_extensions: bool,
}

impl Default for DeprecationPolicy {
    fn default() -> Self {
        Self {
            min_deprecation_period: chrono::Duration::days(180), // 6 months
            notification_periods: vec![
                chrono::Duration::days(90), // 3 months before
                chrono::Duration::days(30), // 1 month before
                chrono::Duration::days(7),  // 1 week before
            ],
            strict_enforcement: true,
            allow_extensions: true,
        }
    }
}

/// Deprecation notice for a specific feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationNotice {
    /// Unique identifier for the deprecation
    pub id: String,
    /// Protocol this deprecation applies to
    pub protocol: ProtocolType,
    /// Service this deprecation applies to
    pub service: ServiceType,
    /// Component being deprecated
    pub component: String,
    /// Version in which deprecation was announced
    pub deprecated_in: ApiVersion,
    /// Version in which component will be removed
    pub removal_version: ApiVersion,
    /// Date when deprecation was announced
    pub deprecation_date: DateTime<Utc>,
    /// Planned removal date
    pub removal_date: DateTime<Utc>,
    /// Reason for deprecation
    pub reason: String,
    /// Migration instructions
    pub migration_guide: String,
    /// Replacement component (if any)
    pub replacement: Option<String>,
    /// Current phase of deprecation
    pub phase: DeprecationPhase,
    /// Whether this is a breaking change
    pub is_breaking: bool,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Deprecation event for tracking lifecycle changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationEvent {
    pub notice_id: String,
    pub event_type: DeprecationEventType,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    pub metadata: HashMap<String, String>,
}

/// Types of deprecation events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeprecationEventType {
    /// Deprecation announced
    Announced,
    /// Sunset phase started
    SunsetStarted,
    /// Removal warning sent
    RemovalWarning,
    /// Component removed
    Removed,
    /// Deprecation timeline extended
    TimelineExtended,
    /// Deprecation cancelled
    Cancelled,
}

/// Deprecation manager for handling API lifecycle
#[derive(Debug, Clone)]
pub struct DeprecationManager {
    /// Active deprecation notices
    notices: HashMap<String, DeprecationNotice>,
    /// Deprecation events history
    events: Vec<DeprecationEvent>,
    /// Deprecation policy
    policy: DeprecationPolicy,
}

impl Default for DeprecationManager {
    fn default() -> Self {
        Self {
            notices: HashMap::new(),
            events: Vec::new(),
            policy: DeprecationPolicy::default(),
        }
    }
}

impl DeprecationManager {
    /// Create a new deprecation manager
    pub fn new(policy: DeprecationPolicy) -> Self {
        Self {
            notices: HashMap::new(),
            events: Vec::new(),
            policy,
        }
    }

    /// Add a new deprecation notice
    pub fn deprecate_feature(
        &mut self,
        protocol: ProtocolType,
        service: ServiceType,
        component: String,
        deprecated_in: ApiVersion,
        removal_version: ApiVersion,
        reason: String,
        migration_guide: String,
        replacement: Option<String>,
    ) -> Result<String, DeprecationError> {
        let id = format!("{}-{}-{}-{}", protocol, service, component, deprecated_in);

        // Check if already deprecated
        if self.notices.contains_key(&id) {
            return Err(DeprecationError::AlreadyDeprecated(component));
        }

        // Validate timeline
        if removal_version <= deprecated_in {
            return Err(DeprecationError::InvalidSchedule("Removal version must be after deprecation version".to_string()));
        }

        let now = Utc::now();
        let removal_date = now + self.policy.min_deprecation_period;

        let notice = DeprecationNotice {
            id: id.clone(),
            protocol,
            service,
            component: component.clone(),
            deprecated_in: deprecated_in.clone(),
            removal_version: removal_version.clone(),
            deprecation_date: now,
            removal_date,
            reason,
            migration_guide,
            replacement,
            phase: DeprecationPhase::Deprecated,
            is_breaking: removal_version.is_breaking_change_from(&deprecated_in),
            metadata: HashMap::new(),
        };

        // Add deprecation notice
        self.notices.insert(id.clone(), notice);

        // Record event
        self.record_event(DeprecationEvent {
            notice_id: id.clone(),
            event_type: DeprecationEventType::Announced,
            timestamp: now,
            description: format!("Component '{}' deprecated", component),
            metadata: HashMap::new(),
        });

        Ok(id)
    }

    /// Update deprecation phase
    pub fn update_phase(&mut self, notice_id: &str, phase: DeprecationPhase) -> Result<(), DeprecationError> {
        let notice = self.notices.get_mut(notice_id).ok_or_else(|| DeprecationError::NotFound(notice_id.to_string()))?;

        let old_phase = notice.phase.clone();
        notice.phase = phase.clone();

        // Record phase change event
        let event_type = match phase {
            DeprecationPhase::Sunset => DeprecationEventType::SunsetStarted,
            DeprecationPhase::Removed => DeprecationEventType::Removed,
            _ => return Ok(()),
        };

        self.record_event(DeprecationEvent {
            notice_id: notice_id.to_string(),
            event_type,
            timestamp: Utc::now(),
            description: format!("Phase changed from {:?} to {:?}", old_phase, phase),
            metadata: HashMap::new(),
        });

        Ok(())
    }

    /// Extend deprecation timeline
    pub fn extend_deprecation(&mut self, notice_id: &str, new_removal_date: DateTime<Utc>, reason: String) -> Result<(), DeprecationError> {
        if !self.policy.allow_extensions {
            return Err(DeprecationError::InvalidSchedule("Deprecation extensions not allowed by policy".to_string()));
        }

        let notice = self.notices.get_mut(notice_id).ok_or_else(|| DeprecationError::NotFound(notice_id.to_string()))?;

        let old_date = notice.removal_date;
        notice.removal_date = new_removal_date;

        // Record extension event
        self.record_event(DeprecationEvent {
            notice_id: notice_id.to_string(),
            event_type: DeprecationEventType::TimelineExtended,
            timestamp: Utc::now(),
            description: format!("Removal date extended from {} to {}: {}", old_date, new_removal_date, reason),
            metadata: HashMap::new(),
        });

        Ok(())
    }

    /// Cancel a deprecation
    pub fn cancel_deprecation(&mut self, notice_id: &str, reason: String) -> Result<(), DeprecationError> {
        let notice = self.notices.get_mut(notice_id).ok_or_else(|| DeprecationError::NotFound(notice_id.to_string()))?;

        notice.phase = DeprecationPhase::Active;

        // Record cancellation event
        self.record_event(DeprecationEvent {
            notice_id: notice_id.to_string(),
            event_type: DeprecationEventType::Cancelled,
            timestamp: Utc::now(),
            description: format!("Deprecation cancelled: {}", reason),
            metadata: HashMap::new(),
        });

        Ok(())
    }

    /// Get all deprecation notices for a protocol/service
    pub fn get_notices_for_service(&self, protocol: &ProtocolType, service: &ServiceType) -> Vec<&DeprecationNotice> {
        self.notices.values().filter(|notice| notice.protocol == *protocol && notice.service == *service).collect()
    }

    /// Get deprecation notice by component
    pub fn get_notice_for_component(&self, protocol: &ProtocolType, service: &ServiceType, component: &str) -> Option<&DeprecationNotice> {
        self.notices
            .values()
            .find(|notice| notice.protocol == *protocol && notice.service == *service && notice.component == component)
    }

    /// Check if a component is deprecated
    pub fn is_deprecated(&self, protocol: &ProtocolType, service: &ServiceType, component: &str) -> bool {
        self.get_notice_for_component(protocol, service, component)
            .map(|notice| matches!(notice.phase, DeprecationPhase::Deprecated | DeprecationPhase::Sunset))
            .unwrap_or(false)
    }

    /// Check if a component is in sunset phase
    pub fn is_in_sunset(&self, protocol: &ProtocolType, service: &ServiceType, component: &str) -> bool {
        self.get_notice_for_component(protocol, service, component)
            .map(|notice| notice.phase == DeprecationPhase::Sunset)
            .unwrap_or(false)
    }

    /// Get notices that need warnings
    pub fn get_notices_needing_warnings(&self) -> Vec<&DeprecationNotice> {
        let now = Utc::now();
        self.notices
            .values()
            .filter(|notice| {
                matches!(notice.phase, DeprecationPhase::Deprecated | DeprecationPhase::Sunset)
                    && self.policy.notification_periods.iter().any(|period| notice.removal_date - now <= *period && notice.removal_date > now)
            })
            .collect()
    }

    /// Process deprecation lifecycle (should be called periodically)
    pub fn process_lifecycle(&mut self) {
        let now = Utc::now();
        let mut notices_to_update = Vec::new();

        for (id, notice) in &self.notices {
            match notice.phase {
                DeprecationPhase::Deprecated => {
                    // Check if should move to sunset phase (e.g., 30 days before removal)
                    if notice.removal_date - now <= chrono::Duration::days(30) {
                        notices_to_update.push((id.clone(), DeprecationPhase::Sunset));
                    }
                }
                DeprecationPhase::Sunset => {
                    // Check if should be removed
                    if now >= notice.removal_date {
                        notices_to_update.push((id.clone(), DeprecationPhase::Removed));
                    }
                }
                _ => {}
            }
        }

        // Apply updates
        for (id, phase) in notices_to_update {
            let _ = self.update_phase(&id, phase);
        }
    }

    /// Generate deprecation warnings for a version
    pub fn generate_warnings(&self, protocol: &ProtocolType, service: &ServiceType, version: &ApiVersion, used_features: &[String]) -> Vec<String> {
        let mut warnings = Vec::new();

        for feature in used_features {
            if let Some(notice) = self.get_notice_for_component(protocol, service, feature) {
                if notice.deprecated_in <= *version {
                    let warning = match notice.phase {
                        DeprecationPhase::Deprecated => {
                            format!(
                                "Feature '{}' is deprecated since version {} and will be removed in version {}. {}",
                                feature, notice.deprecated_in, notice.removal_version, notice.migration_guide
                            )
                        }
                        DeprecationPhase::Sunset => {
                            format!(
                                "Feature '{}' is in sunset phase and will be removed soon (scheduled for {}). {}",
                                feature,
                                notice.removal_date.format("%Y-%m-%d"),
                                notice.migration_guide
                            )
                        }
                        _ => continue,
                    };
                    warnings.push(warning);
                }
            }
        }

        warnings
    }

    /// Record a deprecation event
    fn record_event(&mut self, event: DeprecationEvent) {
        self.events.push(event);
    }

    /// Get deprecation events for a notice
    pub fn get_events_for_notice(&self, notice_id: &str) -> Vec<&DeprecationEvent> {
        self.events.iter().filter(|event| event.notice_id == notice_id).collect()
    }

    /// Get all deprecation notices
    pub fn get_all_notices(&self) -> Vec<&DeprecationNotice> {
        self.notices.values().collect()
    }

    /// Get deprecation policy
    pub fn policy(&self) -> &DeprecationPolicy {
        &self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecation_lifecycle() {
        let mut manager = DeprecationManager::default();

        let notice_id = manager
            .deprecate_feature(
                ProtocolType::Rest,
                ServiceType::Vm,
                "old_execute".to_string(),
                ApiVersion::new(1, 0, 0),
                ApiVersion::new(2, 0, 0),
                "Replaced by new_execute".to_string(),
                "Use new_execute instead".to_string(),
                Some("new_execute".to_string()),
            )
            .unwrap();

        assert!(manager.is_deprecated(&ProtocolType::Rest, &ServiceType::Vm, "old_execute"));

        // Test phase update
        manager.update_phase(&notice_id, DeprecationPhase::Sunset).unwrap();
        assert!(manager.is_in_sunset(&ProtocolType::Rest, &ServiceType::Vm, "old_execute"));
    }

    #[test]
    fn test_warning_generation() {
        let mut manager = DeprecationManager::default();

        manager
            .deprecate_feature(
                ProtocolType::Rest,
                ServiceType::Vm,
                "deprecated_feature".to_string(),
                ApiVersion::new(1, 0, 0),
                ApiVersion::new(2, 0, 0),
                "Testing".to_string(),
                "Migrate to new feature".to_string(),
                None,
            )
            .unwrap();

        let warnings = manager.generate_warnings(&ProtocolType::Rest, &ServiceType::Vm, &ApiVersion::new(1, 1, 0), &["deprecated_feature".to_string()]);

        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("deprecated"));
    }
}

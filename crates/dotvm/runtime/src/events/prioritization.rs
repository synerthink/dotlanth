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

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::events::lib::{Event, Priority};

/// Manages per-event-type priorities with thread-safe interior mutability
pub struct PriorityManager {
    /// Custom priorities for specific event types
    type_priorities: RwLock<HashMap<TypeId, Priority>>,
    /// Default priority fallback for unregistered types
    default_priority: RwLock<Priority>,
}

impl Default for PriorityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityManager {
    /// Create a new priority manager with default settings
    pub fn new() -> Self {
        PriorityManager {
            type_priorities: RwLock::new(HashMap::new()),
            default_priority: RwLock::new(Priority::Normal),
        }
    }

    /// Set the default priority for all unregistered event types
    pub fn set_default_priority(&self, priority: Priority) {
        let mut dp = self.default_priority.write().unwrap_or_else(|e| e.into_inner());
        *dp = priority;
    }

    /// Assign a custom priority to a specific event type
    pub fn register_priority<T: Event + 'static>(&self, priority: Priority) {
        let mut map = self.type_priorities.write().unwrap_or_else(|e| e.into_inner());
        map.insert(TypeId::of::<T>(), priority);
    }

    /// Get the priority registered for `T`, or the current default if none
    pub fn get_priority<T: Event + 'static>(&self) -> Priority {
        let map = self.type_priorities.read().unwrap_or_else(|e| e.into_inner());
        map.get(&TypeId::of::<T>()).copied().unwrap_or_else(|| *self.default_priority.read().unwrap_or_else(|e| e.into_inner()))
    }

    /// Determine priority for a dynamic event:
    /// 1. If the event’s own `priority()` != Normal, use it
    /// 2. Else if a custom priority is registered, use it
    /// 3. Otherwise use the default
    pub fn get_event_priority(&self, event: &dyn Event) -> Priority {
        let ev_pr = event.priority();
        if ev_pr != Priority::Normal {
            return ev_pr;
        }
        let map = self.type_priorities.read().unwrap_or_else(|e| e.into_inner());
        let tid = (event as &dyn Any).type_id();
        map.get(&tid).copied().unwrap_or_else(|| *self.default_priority.read().unwrap_or_else(|e| e.into_inner()))
    }

    /// Change an existing registered priority, returning the old value if any
    pub fn adjust_priority<T: Event + 'static>(&self, new_priority: Priority) -> Option<Priority> {
        let mut map = self.type_priorities.write().unwrap_or_else(|e| e.into_inner());
        map.insert(TypeId::of::<T>(), new_priority)
    }

    /// Remove a custom priority for `T`, returning it if present
    pub fn remove_priority<T: Event + 'static>(&self) -> Option<Priority> {
        let mut map = self.type_priorities.write().unwrap_or_else(|e| e.into_inner());
        map.remove(&TypeId::of::<T>())
    }

    /// Clear all custom priority registrations
    pub fn reset_priorities(&self) {
        let mut map = self.type_priorities.write().unwrap_or_else(|e| e.into_inner());
        map.clear();
    }

    /// How many custom priorities are registered
    pub fn registered_count(&self) -> usize {
        let map = self.type_priorities.read().unwrap_or_else(|e| e.into_inner());
        map.len()
    }

    /// Check if `T` has a custom priority
    pub fn is_registered<T: Event + 'static>(&self) -> bool {
        let map = self.type_priorities.read().unwrap_or_else(|e| e.into_inner());
        map.contains_key(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestEvent1 {
        id: usize,
    }

    impl Event for TestEvent1 {}

    #[derive(Debug)]
    struct TestEvent2 {
        name: String,
    }

    impl Event for TestEvent2 {}

    #[derive(Debug)]
    struct PriorityOverrideEvent {
        value: usize,
    }

    impl Event for PriorityOverrideEvent {
        fn priority(&self) -> Priority {
            Priority::Critical
        }
    }

    #[test]
    fn test_register_and_get_priority() {
        let manager = PriorityManager::new();

        // Default priority should be Normal
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::Normal);

        // Register a priority
        manager.register_priority::<TestEvent1>(Priority::High);
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::High);

        // Different event types have different priorities
        assert_eq!(manager.get_priority::<TestEvent2>(), Priority::Normal);
        manager.register_priority::<TestEvent2>(Priority::Low);
        assert_eq!(manager.get_priority::<TestEvent2>(), Priority::Low);
    }

    #[test]
    fn test_adjust_priority() {
        let manager = PriorityManager::new();

        // Register initial priority
        manager.register_priority::<TestEvent1>(Priority::Normal);
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::Normal);

        // Adjust priority
        let old_priority = manager.adjust_priority::<TestEvent1>(Priority::High);
        assert_eq!(old_priority, Some(Priority::Normal));
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::High);

        // Adjust priority of unregistered type
        let old_priority = manager.adjust_priority::<TestEvent2>(Priority::Low);
        assert_eq!(old_priority, None);
        assert_eq!(manager.get_priority::<TestEvent2>(), Priority::Low);
    }

    #[test]
    fn test_remove_priority() {
        let manager = PriorityManager::new();

        // Register a priority
        manager.register_priority::<TestEvent1>(Priority::High);
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::High);

        // Remove priority
        let old_priority = manager.remove_priority::<TestEvent1>();
        assert_eq!(old_priority, Some(Priority::High));

        // Should return to default
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::Normal);
    }

    #[test]
    fn test_event_priority_override() {
        let manager = PriorityManager::new();

        // Register a priority that should be overridden
        manager.register_priority::<PriorityOverrideEvent>(Priority::Low);

        // Create an event instance
        let event = PriorityOverrideEvent { value: 42 };

        // Event's own priority method should take precedence
        assert_eq!(manager.get_event_priority(&event), Priority::Critical);
    }

    #[test]
    fn test_reset_priorities() {
        let manager = PriorityManager::new();

        // Register some priorities
        manager.register_priority::<TestEvent1>(Priority::High);
        manager.register_priority::<TestEvent2>(Priority::Low);

        assert_eq!(manager.registered_count(), 2);

        // Reset all priorities
        manager.reset_priorities();

        assert_eq!(manager.registered_count(), 0);
        assert_eq!(manager.get_priority::<TestEvent1>(), Priority::Normal);
        assert_eq!(manager.get_priority::<TestEvent2>(), Priority::Normal);
    }

    #[test]
    fn test_is_registered() {
        let manager = PriorityManager::new();

        assert!(!manager.is_registered::<TestEvent1>());

        manager.register_priority::<TestEvent1>(Priority::High);

        assert!(manager.is_registered::<TestEvent1>());
        assert!(!manager.is_registered::<TestEvent2>());
    }
}

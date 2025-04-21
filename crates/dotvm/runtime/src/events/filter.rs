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
use std::sync::{Arc, RwLock};

use crate::events::lib::{Event, event_downcast};

/// The result of applying a filter to an event
pub enum FilterResult {
    /// Allow the event to be dispatched
    Accept,
    /// Prevent the event from being dispatched
    Reject,
    /// Replace the original event with a modified one
    Replace(Box<dyn Event>),
}

/// A function type for event filters
pub type FilterFn = Arc<dyn Fn(&dyn Event) -> FilterResult + Send + Sync>;

/// The FilterManager manages event filters and their application
pub struct FilterManager {
    // Filters for specific event types (TypeId -> Filters)
    type_filters: RwLock<HashMap<TypeId, Vec<FilterFn>>>,
    // Global filters that apply to all event types
    global_filters: RwLock<Vec<FilterFn>>,
}

impl FilterManager {
    /// Create a new filter manager
    pub fn new() -> Self {
        FilterManager {
            type_filters: RwLock::new(HashMap::new()),
            global_filters: RwLock::new(Vec::new()),
        }
    }

    /// Add a filter for a specific event type
    pub fn add_filter<T: Event + 'static>(&self, filter: FilterFn) {
        let mut map = self.type_filters.write().unwrap_or_else(|e| e.into_inner());
        map.entry(TypeId::of::<T>()).or_default().push(filter);
    }

    /// Add a global filter that applies to all event types
    pub fn add_global_filter(&self, filter: FilterFn) {
        let mut vec = self.global_filters.write().unwrap_or_else(|e| e.into_inner());
        vec.push(filter);
    }

    /// Clear all filters for a specific event type
    pub fn clear_filters<T: Event + 'static>(&self) {
        let mut map = self.type_filters.write().unwrap_or_else(|e| e.into_inner());
        map.remove(&TypeId::of::<T>());
    }

    /// Clear all global filters
    pub fn clear_global_filters(&self) {
        let mut vec = self.global_filters.write().unwrap_or_else(|e| e.into_inner());
        vec.clear();
    }

    /// Reset all filters (both type-specific and global)
    pub fn reset_filters(&self) {
        self.clear_global_filters();
        let mut map = self.type_filters.write().unwrap_or_else(|e| e.into_inner());
        map.clear();
    }

    /// Apply all relevant filters to an event.
    /// Returns `Some(event)` if accepted (possibly modified), or `None` if rejected.
    pub fn apply_filters(&self, mut event: Box<dyn Event>) -> Option<Box<dyn Event>> {
        // Apply global filters first
        let globals = self.global_filters.read().unwrap_or_else(|e| e.into_inner());
        for filt in globals.iter() {
            match filt(&*event) {
                FilterResult::Accept => continue,
                FilterResult::Reject => return None,
                FilterResult::Replace(new_event) => event = new_event,
            }
        }

        // Then apply type-specific filters
        let type_id = (&*event as &dyn Any).type_id();
        let map = self.type_filters.read().unwrap_or_else(|e| e.into_inner());
        if let Some(filters) = map.get(&type_id) {
            for filt in filters.iter() {
                match filt(&*event) {
                    FilterResult::Accept => continue,
                    FilterResult::Reject => return None,
                    FilterResult::Replace(new_event) => event = new_event,
                }
            }
        }

        Some(event)
    }

    /// Number of filters registered for a specific event type
    pub fn type_filter_count<T: Event + 'static>(&self) -> usize {
        let map = self.type_filters.read().unwrap_or_else(|e| e.into_inner());
        map.get(&TypeId::of::<T>()).map_or(0, |v| v.len())
    }

    /// Number of global filters
    pub fn global_filter_count(&self) -> usize {
        let vec = self.global_filters.read().unwrap_or_else(|e| e.into_inner());
        vec.len()
    }

    /// Create a simple filter from a predicate (Accept if true)
    pub fn create_predicate_filter<F>(pred: F) -> FilterFn
    where
        F: Fn(&dyn Event) -> bool + Send + Sync + 'static,
    {
        Arc::new(move |e| if pred(e) { FilterResult::Accept } else { FilterResult::Reject })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::lib::Priority;

    #[derive(Debug, Clone)]
    struct TestEvent {
        id: usize,
        name: String,
    }
    impl Event for TestEvent {}

    #[derive(Debug)]
    struct OtherEvent {
        value: usize,
    }
    impl Event for OtherEvent {}

    #[test]
    fn test_type_filter() {
        let filter_manager = FilterManager::new();

        // Add a filter that rejects TestEvents with id < 10
        filter_manager.add_filter::<TestEvent>(Arc::new(|event| {
            if let Some(test_event) = event_downcast::<TestEvent>(event) {
                // Used event_downcast
                if test_event.id < 10 { FilterResult::Reject } else { FilterResult::Accept }
            } else {
                FilterResult::Accept
            }
        }));

        // This event should be rejected
        let rejected_event = Box::new(TestEvent { id: 5, name: "rejected".to_string() });
        assert!(filter_manager.apply_filters(rejected_event).is_none());

        // This event should pass
        let accepted_event = Box::new(TestEvent { id: 15, name: "accepted".to_string() });
        assert!(filter_manager.apply_filters(accepted_event).is_some());

        // OtherEvent should pass because the filter is type-specific
        let other_event = Box::new(OtherEvent { value: 5 });
        assert!(filter_manager.apply_filters(other_event).is_some());
    }

    #[test]
    fn test_global_filter() {
        let filter_manager = FilterManager::new();

        // Add a global filter that rejects all events with "reject" in their Debug output
        filter_manager.add_global_filter(Arc::new(|event| {
            let debug_str = format!("{:?}", event);
            if debug_str.contains("reject") { FilterResult::Reject } else { FilterResult::Accept }
        }));

        // This event should be rejected
        let rejected_event = Box::new(TestEvent { id: 1, name: "reject_me".to_string() });
        assert!(filter_manager.apply_filters(rejected_event).is_none());

        // This event should pass
        let accepted_event = Box::new(TestEvent { id: 2, name: "accept_me".to_string() });
        assert!(filter_manager.apply_filters(accepted_event).is_some());
    }

    #[test]
    fn test_event_modification() {
        let filter_manager = FilterManager::new();

        // Add a filter that modifies the event
        filter_manager.add_filter::<TestEvent>(Arc::new(|event| {
            if let Some(test_event) = event_downcast::<TestEvent>(event) {
                let mut new_event = test_event.clone();
                new_event.name = format!("modified_{}", new_event.name);
                FilterResult::Replace(Box::new(new_event))
            } else {
                FilterResult::Accept
            }
        }));

        // This event should be modified
        let original_event = Box::new(TestEvent { id: 1, name: "original".to_string() });
        let filtered_event = filter_manager.apply_filters(original_event).unwrap();

        // Downcast and check if it was modified
        let modified = event_downcast::<TestEvent>(&*filtered_event).unwrap();
        assert_eq!(modified.name, "modified_original");
    }

    #[test]
    fn test_multiple_filters() {
        let filter_manager = FilterManager::new();

        // Add a filter that modifies the id
        filter_manager.add_filter::<TestEvent>(Arc::new(|event| {
            if let Some(test_event) = event_downcast::<TestEvent>(event) {
                let mut new_event = test_event.clone();
                new_event.id *= 2;
                FilterResult::Replace(Box::new(new_event))
            } else {
                FilterResult::Accept
            }
        }));

        // Add another filter that rejects events with id > 15
        filter_manager.add_filter::<TestEvent>(Arc::new(|event| {
            if let Some(test_event) = event_downcast::<TestEvent>(event) {
                if test_event.id > 15 { FilterResult::Reject } else { FilterResult::Accept }
            } else {
                FilterResult::Accept
            }
        }));

        // This event should pass both filters (id becomes 10, still <= 15)
        let event1 = Box::new(TestEvent { id: 5, name: "test1".to_string() });
        let filtered = filter_manager.apply_filters(event1).unwrap();
        let result = event_downcast::<TestEvent>(&*filtered).unwrap();
        assert_eq!(result.id, 10);

        // This event should be rejected after modification (id becomes 16, > 15)
        let event2 = Box::new(TestEvent { id: 8, name: "test2".to_string() });
        assert!(filter_manager.apply_filters(event2).is_none());
    }

    #[test]
    fn test_clear_filters() {
        let filter_manager = FilterManager::new();

        // Add a type filter that rejects all TestEvents
        filter_manager.add_filter::<TestEvent>(Arc::new(|_| FilterResult::Reject));

        // Add a global filter that rejects all events
        filter_manager.add_global_filter(Arc::new(|_| FilterResult::Reject));

        // Both filters should be active
        let event1 = Box::new(TestEvent { id: 1, name: "test".to_string() });
        assert!(filter_manager.apply_filters(event1).is_none());

        // Clear the type-specific filter
        filter_manager.clear_filters::<TestEvent>();

        // Global filter should still reject
        let event2 = Box::new(TestEvent { id: 2, name: "test".to_string() });
        assert!(filter_manager.apply_filters(event2).is_none());

        // Clear global filters
        filter_manager.clear_global_filters();

        // Now events should pass
        let event3 = Box::new(TestEvent { id: 3, name: "test".to_string() });
        assert!(filter_manager.apply_filters(event3).is_some());
    }

    #[test]
    fn test_predicate_filter() {
        let filter_manager = FilterManager::new();

        // Create a predicate filter that only accepts events with high priority
        let high_priority_filter = FilterManager::create_predicate_filter(|event| event.priority() == Priority::High || event.priority() == Priority::Critical);

        filter_manager.add_global_filter(high_priority_filter);

        // Create events with different priorities
        #[derive(Debug)]
        struct PriorityEvent {
            priority: Priority,
        }

        impl Event for PriorityEvent {
            fn priority(&self) -> Priority {
                self.priority
            }
        }

        // Low priority should be rejected
        let low_event = Box::new(PriorityEvent { priority: Priority::Low });
        assert!(filter_manager.apply_filters(low_event).is_none());

        // Normal priority should be rejected
        let normal_event = Box::new(PriorityEvent { priority: Priority::Normal });
        assert!(filter_manager.apply_filters(normal_event).is_none());

        // High priority should be accepted
        let high_event = Box::new(PriorityEvent { priority: Priority::High });
        assert!(filter_manager.apply_filters(high_event).is_some());

        // Critical priority should be accepted
        let critical_event = Box::new(PriorityEvent { priority: Priority::Critical });
        assert!(filter_manager.apply_filters(critical_event).is_some());
    }
}

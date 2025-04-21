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

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::events::lib::{Event, EventHandler, HandlerId, event_downcast, generate_handler_id};
use crate::events::queue::EventQueue;

/// The EventDispatcher handles registration of event handlers
/// and dispatching events to registered handlers
pub struct EventDispatcher {
    handlers: RwLock<HashMap<TypeId, HashMap<HandlerId, Arc<dyn Fn(&dyn Event) + Send + Sync>>>>,
    queue: Arc<Mutex<EventQueue>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher with an event queue
    pub fn new(queue: Arc<Mutex<EventQueue>>) -> Self {
        EventDispatcher {
            handlers: RwLock::new(HashMap::new()),
            queue,
        }
    }

    /// Register a handler for a specific event type
    pub fn subscribe<T: Event + 'static>(&self, handler: Arc<dyn Fn(&T) + Send + Sync>) -> HandlerId {
        let handler_id = generate_handler_id();
        let type_id = TypeId::of::<T>();

        // Wrap to dyn Fn(&dyn Event)
        let wrapper: Arc<dyn Fn(&dyn Event) + Send + Sync> = Arc::new(move |event: &dyn Event| {
            if let Some(concrete) = event_downcast::<T>(event) {
                handler(concrete);
            }
        });

        // Acquire write lock, recover from poison if needed
        let mut handlers_map = self.handlers.write().unwrap_or_else(|e| e.into_inner());
        let type_handlers = handlers_map.entry(type_id).or_insert_with(HashMap::new);
        type_handlers.insert(handler_id, wrapper);

        handler_id
    }

    /// Unregister a handler by its ID and event type
    pub fn unsubscribe<T: Event + 'static>(&self, handler_id: HandlerId) -> bool {
        let type_id = TypeId::of::<T>();
        let mut handlers_map = self.handlers.write().unwrap_or_else(|e| e.into_inner());

        if let Some(type_handlers) = handlers_map.get_mut(&type_id) {
            let removed = type_handlers.remove(&handler_id).is_some();
            if type_handlers.is_empty() {
                handlers_map.remove(&type_id);
            }
            removed
        } else {
            false
        }
    }

    /// Publish an event to all registered handlers for its type (async)
    pub fn publish<T: Event + 'static>(&self, event: T) {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.enqueue(Box::new(event));
    }

    /// Process an event immediately (for synchronous dispatch)
    pub fn dispatch(&self, event: &dyn Event) {
        let handlers_map = self.handlers.read().unwrap_or_else(|e| e.into_inner());
        let type_id = (*event).type_id();
        if let Some(type_handlers) = handlers_map.get(&type_id) {
            for handler in type_handlers.values() {
                handler(event);
            }
        }
    }

    /// Process all events in the queue without holding the lock during dispatch
    pub fn process_queue(&self) {
        // Drain queue under lock
        let mut drained = Vec::new();
        {
            let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
            while let Some(event) = queue.dequeue() {
                drained.push(event);
            }
        }
        // Dispatch outside lock to avoid deadlocks
        for event in drained {
            self.dispatch(&*event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestEvent {
        id: usize,
    }

    impl Event for TestEvent {}

    #[test]
    fn test_subscribe_and_publish() {
        let queue = Arc::new(Mutex::new(EventQueue::new()));
        let dispatcher = EventDispatcher::new(queue);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Wrapped the closure in Arc::new()
        let handler_id = dispatcher.subscribe(Arc::new(move |event: &TestEvent| {
            counter_clone.fetch_add(event.id, Ordering::SeqCst);
        }));

        dispatcher.publish(TestEvent { id: 5 });
        dispatcher.process_queue();

        assert_eq!(counter.load(Ordering::SeqCst), 5);

        // Test unsubscribe
        assert!(dispatcher.unsubscribe::<TestEvent>(handler_id));

        dispatcher.publish(TestEvent { id: 10 });
        dispatcher.process_queue();

        // Counter should still be 5 since we unsubscribed
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_multiple_handlers() {
        let queue = Arc::new(Mutex::new(EventQueue::new()));
        let dispatcher = EventDispatcher::new(queue);

        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter1_clone = counter1.clone();

        let counter2 = Arc::new(AtomicUsize::new(0));
        let counter2_clone = counter2.clone();

        // Wrapped the closures in Arc::new()
        dispatcher.subscribe(Arc::new(move |event: &TestEvent| {
            counter1_clone.fetch_add(event.id, Ordering::SeqCst);
        }));

        // Wrapped the closures in Arc::new()
        dispatcher.subscribe(Arc::new(move |event: &TestEvent| {
            counter2_clone.fetch_add(event.id * 2, Ordering::SeqCst);
        }));

        dispatcher.publish(TestEvent { id: 5 });
        dispatcher.process_queue();

        assert_eq!(counter1.load(Ordering::SeqCst), 5);
        assert_eq!(counter2.load(Ordering::SeqCst), 10);
    }

    #[derive(Debug)]
    struct OtherEvent {
        value: String,
    }

    impl Event for OtherEvent {}

    #[test]
    fn test_different_event_types() {
        let queue = Arc::new(Mutex::new(EventQueue::new()));
        let dispatcher = EventDispatcher::new(queue);

        let int_counter = Arc::new(AtomicUsize::new(0));
        let int_counter_clone = int_counter.clone();

        let string_counter = Arc::new(AtomicUsize::new(0));
        let string_counter_clone = string_counter.clone();

        // Wrapped the closures in Arc::new()
        dispatcher.subscribe(Arc::new(move |event: &TestEvent| {
            int_counter_clone.fetch_add(event.id, Ordering::SeqCst);
        }));

        // Wrapped the closures in Arc::new()
        dispatcher.subscribe(Arc::new(move |event: &OtherEvent| {
            string_counter_clone.fetch_add(event.value.len(), Ordering::SeqCst);
        }));

        dispatcher.publish(TestEvent { id: 7 });
        dispatcher.publish(OtherEvent { value: "hello".to_string() });
        dispatcher.process_queue();

        assert_eq!(int_counter.load(Ordering::SeqCst), 7);
        assert_eq!(string_counter.load(Ordering::SeqCst), 5);
    }
}

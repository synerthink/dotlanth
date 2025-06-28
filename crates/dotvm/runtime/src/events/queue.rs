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

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::events::lib::Event;

/// Statistics for monitoring queue performance
///
/// All counters are atomic: safe to read concurrently from multiple threads,
/// but EventQueue itself is **not** internally synchronized and should be
/// wrapped in a Mutex or other lock for thread-safe use.
#[derive(Debug, Default)]
pub struct QueueStats {
    /// Total number of processed (dequeued) events
    pub processed_count: AtomicUsize,
    /// Total number of dropped events due to capacity or manual clear
    pub dropped_count: AtomicUsize,
    /// Moving average wait time in **microseconds**
    pub avg_wait_time: AtomicUsize,
}

/// A queued event with metadata
pub struct QueuedEvent {
    pub event: Box<dyn Event>,
    pub enqueue_time: Instant,
    pub delay_until: Option<Instant>,
}

/// A queue for buffering events before they are processed.
///
/// Not internally synchronized: users must wrap this in a Mutex or similar
/// if using from multiple threads.
pub struct EventQueue {
    queue: VecDeque<QueuedEvent>,
    capacity: usize,
    stats: QueueStats,
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EventQueue {
    /// Create a new event queue with default capacity of 1000
    pub fn new() -> Self {
        EventQueue::with_capacity(1_000)
    }

    /// Create a new event queue with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        EventQueue {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            stats: QueueStats::default(),
        }
    }

    /// Add an event to the queue immediately.
    /// Returns `true` if enqueued, `false` if dropped due to capacity.
    pub fn enqueue(&mut self, event: Box<dyn Event>) -> bool {
        self.enqueue_with_delay(event, None)
    }

    /// Add an event to the queue with an optional delay.
    /// `delay` is relative to now; event won't be dequeued until deadline.
    pub fn enqueue_with_delay(&mut self, event: Box<dyn Event>, delay: Option<Duration>) -> bool {
        // Drop if at capacity
        if self.queue.len() >= self.capacity {
            self.stats.dropped_count.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Single timestamp for both enqueue_time and delay calculation
        let now = Instant::now();
        let deadline = delay.map(|d| now + d);
        let priority = event.priority();

        let queued = QueuedEvent {
            event,
            enqueue_time: now,
            delay_until: deadline,
        };

        // Insert higher-priority items earlier
        if let Some(pos) = self.queue.iter().position(|e| e.event.priority() < priority) {
            self.queue.insert(pos, queued);
        } else {
            self.queue.push_back(queued);
        }

        true
    }

    /// Remove and return the next ready event (non-delayed).
    /// Updates statistics (processed_count, avg_wait_time).
    pub fn dequeue(&mut self) -> Option<Box<dyn Event>> {
        let now = Instant::now();
        if let Some(idx) = self.queue.iter().position(|e| e.delay_until.is_none_or(|d| now >= d)) {
            let queued = self.queue.remove(idx).unwrap();

            // Update stats
            let wait = queued.enqueue_time.elapsed().as_micros() as usize;
            let prev_count = self.stats.processed_count.fetch_add(1, Ordering::Relaxed);

            // Compute new average (simple running average)
            if prev_count > 0 {
                let old_avg = self.stats.avg_wait_time.load(Ordering::Relaxed);
                let new_avg = (old_avg * prev_count + wait) / (prev_count + 1);
                self.stats.avg_wait_time.store(new_avg, Ordering::Relaxed);
            } else {
                self.stats.avg_wait_time.store(wait, Ordering::Relaxed);
            }

            Some(queued.event)
        } else {
            None
        }
    }

    /// Adjust queue capacity; if shrinking, drops lowest-priority events
    /// until `len() <= new_capacity`, updating dropped_count.
    pub fn resize(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        while self.queue.len() > new_capacity {
            // Find lowest-priority index
            let (idx, _) = self.queue.iter().enumerate().min_by_key(|(_, e)| e.event.priority()).unwrap();
            self.queue.remove(idx);
            self.stats.dropped_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Number of queued events (including delayed)
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// True if no queued events
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Configured capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// View queue statistics
    pub fn stats(&self) -> &QueueStats {
        &self.stats
    }

    /// Clear all events, marking them dropped
    pub fn clear(&mut self) {
        let removed = self.queue.len();
        self.queue.clear();
        self.stats.dropped_count.fetch_add(removed, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[derive(Debug)]
    struct TestEvent {
        id: usize,
        priority: Priority,
    }

    impl Event for TestEvent {
        fn priority(&self) -> Priority {
            self.priority
        }
    }

    #[test]
    fn test_enqueue_dequeue() {
        let mut eq = EventQueue::new();
        assert!(eq.enqueue(Box::new(TestEvent { id: 1, priority: Priority::Normal })));
        assert_eq!(eq.len(), 1);
        let ev = eq.dequeue().unwrap();
        let te = event_downcast::<TestEvent>(&*ev).unwrap();
        assert_eq!(te.id, 1);
        assert_eq!(eq.stats().processed_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut eq = EventQueue::new();
        eq.enqueue(Box::new(TestEvent { id: 1, priority: Priority::Low }));
        eq.enqueue(Box::new(TestEvent { id: 2, priority: Priority::Normal }));
        eq.enqueue(Box::new(TestEvent { id: 3, priority: Priority::High }));
        eq.enqueue(Box::new(TestEvent { id: 4, priority: Priority::Critical }));

        let ids: Vec<usize> = (0..4).map(|_| event_downcast::<TestEvent>(&*eq.dequeue().unwrap()).unwrap().id).collect();
        assert_eq!(ids, vec![4, 3, 2, 1]);
    }

    #[test]
    fn test_capacity_limit() {
        let mut queue = EventQueue::with_capacity(2);

        assert!(queue.enqueue(Box::new(TestEvent { id: 1, priority: Priority::Normal })));
        assert!(queue.enqueue(Box::new(TestEvent { id: 2, priority: Priority::Normal })));
        // This should be dropped due to capacity
        assert!(!queue.enqueue(Box::new(TestEvent { id: 3, priority: Priority::Normal })));

        assert_eq!(queue.len(), 2);
        assert_eq!(queue.stats().dropped_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_delayed_events() {
        let mut eq = EventQueue::new();
        eq.enqueue_with_delay(Box::new(TestEvent { id: 5, priority: Priority::Normal }), Some(Duration::from_millis(50)));

        // Immediate dequeue should return None since the event is delayed
        assert!(eq.dequeue().is_none());

        // Wait for delay to pass
        thread::sleep(Duration::from_millis(60));

        // Now we should be able to dequeue
        let binding = eq.dequeue().unwrap();
        let te = event_downcast::<TestEvent>(&*binding).unwrap();
        assert_eq!(te.id, 5);
    }

    #[test]
    fn test_resize() {
        let mut queue = EventQueue::with_capacity(5);

        for i in 1..=5 {
            queue.enqueue(Box::new(TestEvent {
                id: i,
                priority: if i % 2 == 0 { Priority::High } else { Priority::Low },
            }));
        }

        assert_eq!(queue.len(), 5);

        // Resize to smaller capacity
        queue.resize(3);

        // Should have dropped the 2 lowest priority events
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.stats().dropped_count.load(Ordering::SeqCst), 2);

        // Check that we kept the high priority events
        let mut high_count = 0;
        while let Some(event) = queue.dequeue() {
            let downcast = event_downcast::<TestEvent>(&*event).unwrap();
            if downcast.priority == Priority::High {
                high_count += 1;
            }
        }

        // We had 2 high priority events (IDs 2 and 4)
        assert_eq!(high_count, 2);
    }

    #[test]
    fn test_resize_and_drops() {
        let mut eq = EventQueue::with_capacity(3);
        for &p in &[Priority::Low, Priority::Normal, Priority::High, Priority::Critical] {
            let _ = eq.enqueue(Box::new(TestEvent { id: 0, priority: p }));
        }
        // capacity=3, so one dropped
        assert_eq!(eq.len(), 3);
        assert_eq!(eq.stats().dropped_count.load(Ordering::SeqCst), 1);

        // force manual shrink
        eq.resize(2);
        assert_eq!(eq.len(), 2);
    }
}

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
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The priority level for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Base trait for all events
pub trait Event: Any + Send + Sync + Debug {
    /// Returns the priority of this event
    fn priority(&self) -> Priority {
        Priority::default()
    }
}

/// A unique identifier for an event handler
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandlerId(pub usize);

/// Type alias for event handlers
pub type EventHandler<T> = Arc<dyn Fn(&T) + Send + Sync>;

/// A helper method to cast an event to a specific type
pub fn event_downcast<T: Event + 'static>(event: &dyn Event) -> Option<&T> {
    // First convert to &dyn Any (since Event: Any), then use downcast_ref
    (event as &dyn Any).downcast_ref::<T>()
}

// Counter for generating unique handler IDs
static HANDLER_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generate a new unique handler ID
pub fn generate_handler_id() -> HandlerId {
    let id = HANDLER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    HandlerId(id)
}

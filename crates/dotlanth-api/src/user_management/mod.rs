//! User Management System
//!
//! This module provides comprehensive user management functionality including:
//! - User registration and profile management
//! - Role assignment and management
//! - Account lifecycle management
//! - User preferences and settings
//! - User search and discovery
//! - Activity tracking and audit logs
//! - Data export and GDPR compliance

pub mod audit;
pub mod export;
pub mod handlers;
pub mod manager;
pub mod models;
pub mod preferences;
pub mod search;
pub mod store;

#[cfg(test)]
pub mod tests;

pub use audit::*;
pub use export::*;
pub use handlers::*;
pub use manager::*;
pub use models::*;
pub use preferences::*;
pub use search::*;
pub use store::*;

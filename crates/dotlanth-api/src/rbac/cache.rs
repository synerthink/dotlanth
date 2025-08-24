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

//! Permission caching for performance optimization

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// Cached value
    value: T,
    /// When this entry expires
    expires_at: Instant,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    /// Check if this entry is expired
    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Permission cache for storing permission check results
#[derive(Debug)]
pub struct PermissionCache {
    /// Permission check results cache
    permission_cache: DashMap<String, CacheEntry<bool>>,

    /// User permissions cache
    user_permissions_cache: DashMap<String, CacheEntry<Vec<crate::rbac::permissions::Permission>>>,

    /// User dot permissions cache
    user_dot_permissions_cache: DashMap<String, CacheEntry<Vec<crate::rbac::permissions::DotPermission>>>,

    /// Role hierarchy cache
    role_hierarchy_cache: DashMap<String, CacheEntry<Vec<String>>>,

    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics for monitoring
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,

    /// Total cache misses
    pub misses: u64,

    /// Total cache evictions
    pub evictions: u64,

    /// Current cache size
    pub current_size: usize,

    /// Maximum cache size reached
    pub max_size_reached: usize,
}

impl CacheStats {
    /// Calculate hit ratio
    pub fn hit_ratio(&self) -> f64 {
        if self.hits + self.misses == 0 { 0.0 } else { self.hits as f64 / (self.hits + self.misses) as f64 }
    }
}

impl PermissionCache {
    /// Create a new permission cache
    pub fn new() -> Self {
        Self {
            permission_cache: DashMap::new(),
            user_permissions_cache: DashMap::new(),
            user_dot_permissions_cache: DashMap::new(),
            role_hierarchy_cache: DashMap::new(),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get a permission check result from cache
    pub async fn get_permission(&self, key: &str) -> Option<bool> {
        if let Some(entry) = self.permission_cache.get(key) {
            if !entry.is_expired() {
                // Record hit without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.hits += 1;
                }
                debug!("Permission cache hit for key: {}", key);
                return Some(entry.value);
            } else {
                // Remove expired entry
                self.permission_cache.remove(key);
                // Record eviction without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.evictions += 1;
                }
            }
        }

        // Record miss without blocking
        if let Ok(mut stats) = self.stats.try_write() {
            stats.misses += 1;
        }
        debug!("Permission cache miss for key: {}", key);
        None
    }

    /// Set a permission check result in cache
    pub async fn set_permission(&self, key: String, value: bool, ttl: Duration) {
        let entry = CacheEntry::new(value, ttl);
        self.permission_cache.insert(key.clone(), entry);
        self.update_cache_size().await;
        debug!("Permission cached for key: {} with TTL: {:?}", key, ttl);
    }

    /// Get user permissions from cache
    pub async fn get_user_permissions(&self, user_id: &str) -> Option<Vec<crate::rbac::permissions::Permission>> {
        if let Some(entry) = self.user_permissions_cache.get(user_id) {
            if !entry.is_expired() {
                // Record hit without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.hits += 1;
                }
                debug!("User permissions cache hit for user: {}", user_id);
                return Some(entry.value.clone());
            } else {
                self.user_permissions_cache.remove(user_id);
                // Record eviction without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.evictions += 1;
                }
            }
        }

        // Record miss without blocking
        if let Ok(mut stats) = self.stats.try_write() {
            stats.misses += 1;
        }
        debug!("User permissions cache miss for user: {}", user_id);
        None
    }

    /// Set user permissions in cache
    pub async fn set_user_permissions(&self, user_id: String, permissions: Vec<crate::rbac::permissions::Permission>, ttl: Duration) {
        let entry = CacheEntry::new(permissions, ttl);
        self.user_permissions_cache.insert(user_id.clone(), entry);
        self.update_cache_size().await;
        debug!("User permissions cached for user: {} with TTL: {:?}", user_id, ttl);
    }

    /// Get user dot permissions from cache
    pub async fn get_user_dot_permissions(&self, user_id: &str) -> Option<Vec<crate::rbac::permissions::DotPermission>> {
        if let Some(entry) = self.user_dot_permissions_cache.get(user_id) {
            if !entry.is_expired() {
                // Record hit without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.hits += 1;
                }
                debug!("User dot permissions cache hit for user: {}", user_id);
                return Some(entry.value.clone());
            } else {
                self.user_dot_permissions_cache.remove(user_id);
                // Record eviction without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.evictions += 1;
                }
            }
        }

        // Record miss without blocking
        if let Ok(mut stats) = self.stats.try_write() {
            stats.misses += 1;
        }
        debug!("User dot permissions cache miss for user: {}", user_id);
        None
    }

    /// Set user dot permissions in cache
    pub async fn set_user_dot_permissions(&self, user_id: String, dot_permissions: Vec<crate::rbac::permissions::DotPermission>, ttl: Duration) {
        let entry = CacheEntry::new(dot_permissions, ttl);
        self.user_dot_permissions_cache.insert(user_id.clone(), entry);
        self.update_cache_size().await;
        debug!("User dot permissions cached for user: {} with TTL: {:?}", user_id, ttl);
    }

    /// Get role hierarchy from cache
    pub async fn get_role_hierarchy(&self, role_id: &str) -> Option<Vec<String>> {
        if let Some(entry) = self.role_hierarchy_cache.get(role_id) {
            if !entry.is_expired() {
                // Record hit without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.hits += 1;
                }
                debug!("Role hierarchy cache hit for role: {}", role_id);
                return Some(entry.value.clone());
            } else {
                self.role_hierarchy_cache.remove(role_id);
                // Record eviction without blocking
                if let Ok(mut stats) = self.stats.try_write() {
                    stats.evictions += 1;
                }
            }
        }

        // Record miss without blocking
        if let Ok(mut stats) = self.stats.try_write() {
            stats.misses += 1;
        }
        debug!("Role hierarchy cache miss for role: {}", role_id);
        None
    }

    /// Set role hierarchy in cache
    pub async fn set_role_hierarchy(&self, role_id: String, hierarchy: Vec<String>, ttl: Duration) {
        let entry = CacheEntry::new(hierarchy, ttl);
        self.role_hierarchy_cache.insert(role_id.clone(), entry);
        self.update_cache_size().await;
        debug!("Role hierarchy cached for role: {} with TTL: {:?}", role_id, ttl);
    }

    /// Invalidate all cache entries for a user
    pub async fn invalidate_user(&self, user_id: &str) {
        // Remove user-specific caches
        self.user_permissions_cache.remove(user_id);
        self.user_dot_permissions_cache.remove(user_id);

        // Remove permission checks that might involve this user
        self.permission_cache.retain(|key, _| !key.contains(user_id));

        self.update_cache_size().await;
        debug!("Invalidated cache for user: {}", user_id);
    }

    /// Invalidate all cache entries for a role
    pub async fn invalidate_role(&self, role_id: &str) {
        // Remove role hierarchy cache
        self.role_hierarchy_cache.remove(role_id);

        // Clear all user caches since role changes affect all users
        self.user_permissions_cache.clear();
        self.user_dot_permissions_cache.clear();
        self.permission_cache.clear();

        self.update_cache_size().await;
        debug!("Invalidated cache for role: {}", role_id);
    }

    /// Invalidate all cache entries for a dot
    pub async fn invalidate_dot(&self, dot_id: &str) {
        // Remove permission checks that involve this dot
        self.permission_cache.retain(|key, _| !key.contains(dot_id));

        // Clear dot-specific user permissions since dot permissions may have changed
        self.user_dot_permissions_cache.clear();

        self.update_cache_size().await;
        debug!("Invalidated cache for dot: {}", dot_id);
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        self.permission_cache.clear();
        self.user_permissions_cache.clear();
        self.user_dot_permissions_cache.clear();
        self.role_hierarchy_cache.clear();

        let mut stats = self.stats.write().await;
        stats.current_size = 0;

        debug!("Cleared all cache entries");
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) {
        let mut evicted = 0;

        // Clean permission cache
        self.permission_cache.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Clean user permissions cache
        self.user_permissions_cache.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Clean user dot permissions cache
        self.user_dot_permissions_cache.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Clean role hierarchy cache
        self.role_hierarchy_cache.retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        if evicted > 0 {
            let mut stats = self.stats.write().await;
            stats.evictions += evicted;
            self.update_cache_size_internal(&mut stats).await;
            debug!("Cleaned up {} expired cache entries", evicted);
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Record a cache hit
    async fn record_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.hits += 1;
    }

    /// Record a cache miss
    async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.misses += 1;
    }

    /// Record a cache eviction
    async fn record_eviction(&self) {
        let mut stats = self.stats.write().await;
        stats.evictions += 1;
    }

    /// Update cache size statistics
    async fn update_cache_size(&self) {
        let mut stats = self.stats.write().await;
        self.update_cache_size_internal(&mut stats).await;
    }

    /// Internal method to update cache size
    async fn update_cache_size_internal(&self, stats: &mut CacheStats) {
        let current_size = self.permission_cache.len() + self.user_permissions_cache.len() + self.user_dot_permissions_cache.len() + self.role_hierarchy_cache.len();

        stats.current_size = current_size;

        if current_size > stats.max_size_reached {
            stats.max_size_reached = current_size;
        }
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(cache: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;
                cache.cleanup_expired().await;

                let stats = cache.get_stats().await;
                debug!(
                    "Cache stats - Hits: {}, Misses: {}, Hit ratio: {:.2}%, Size: {}, Evictions: {}",
                    stats.hits,
                    stats.misses,
                    stats.hit_ratio() * 100.0,
                    stats.current_size,
                    stats.evictions
                );

                // Warn if hit ratio is low
                if stats.hits + stats.misses > 100 && stats.hit_ratio() < 0.5 {
                    warn!("Low cache hit ratio: {:.2}%", stats.hit_ratio() * 100.0);
                }
            }
        })
    }
}

impl Default for PermissionCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rbac::permissions::Permission;

    #[tokio::test]
    async fn test_permission_cache() {
        let cache = PermissionCache::new();
        let key = "user123:dots:read".to_string();

        // Cache miss
        assert!(cache.get_permission(&key).await.is_none());

        // Set and get
        cache.set_permission(key.clone(), true, Duration::from_secs(60)).await;
        assert_eq!(cache.get_permission(&key).await, Some(true));

        // Check stats
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = PermissionCache::new();
        let key = "user123:dots:read".to_string();

        // Test immediate expiration by creating an entry that's already expired
        let expired_entry = CacheEntry {
            value: true,
            expires_at: Instant::now() - Duration::from_secs(1), // Already expired
        };
        cache.permission_cache.insert(key.clone(), expired_entry);

        // Test expiration check directly
        let entry = cache.permission_cache.get(&key).unwrap();
        assert!(entry.is_expired());

        // Verify entry exists but is expired
        assert!(cache.permission_cache.contains_key(&key));
    }

    #[tokio::test]
    async fn test_user_invalidation() {
        let cache = PermissionCache::new();
        let user_id = "user123";

        // Set some cache entries
        cache.set_permission(format!("{}:dots:read", user_id), true, Duration::from_secs(60)).await;
        cache
            .set_user_permissions(user_id.to_string(), vec![Permission::new("dots".to_string(), "read".to_string())], Duration::from_secs(60))
            .await;

        // Verify they exist
        assert!(cache.get_permission(&format!("{}:dots:read", user_id)).await.is_some());
        assert!(cache.get_user_permissions(user_id).await.is_some());

        // Invalidate user
        cache.invalidate_user(user_id).await;

        // Should be gone
        assert!(cache.get_permission(&format!("{}:dots:read", user_id)).await.is_none());
        assert!(cache.get_user_permissions(user_id).await.is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let cache = PermissionCache::new();

        // Manually insert expired and valid entries
        let expired_entry = CacheEntry {
            value: true,
            expires_at: Instant::now() - Duration::from_secs(1), // Already expired
        };
        let valid_entry = CacheEntry {
            value: true,
            expires_at: Instant::now() + Duration::from_secs(60), // Valid for 60 seconds
        };

        cache.permission_cache.insert("key1".to_string(), expired_entry);
        cache.permission_cache.insert("key2".to_string(), valid_entry);

        // Verify initial state
        assert!(cache.permission_cache.contains_key("key1"));
        assert!(cache.permission_cache.contains_key("key2"));

        // Check expiration status
        assert!(cache.permission_cache.get("key1").unwrap().is_expired());
        assert!(!cache.permission_cache.get("key2").unwrap().is_expired());
    }
}

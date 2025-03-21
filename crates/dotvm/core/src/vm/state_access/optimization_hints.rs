use super::read_write_analysis::StateAccessTracker;
use std::collections::HashMap;

/// Generates optimization suggestions based on access patterns
pub struct OptimizationAnalyzer;

impl OptimizationAnalyzer {
    /// Analyzes access patterns and produces optimization recommendations
    /// # Arguments
    /// - `trackers`: Collection of access trackers to analyze
    ///
    /// # Returns
    /// Vector of optimization hints with specific recommendations:
    /// - Caching suggestions for frequently read variables
    /// - Grouping suggestions for co-accessed variables
    pub fn generate_hints(trackers: &[StateAccessTracker]) -> Vec<String> {
        let mut read_counts = HashMap::new();
        let mut write_counts = HashMap::new();
        let mut co_occurrence = HashMap::new();

        // Aggregate access counts
        for tracker in trackers {
            for (var, count) in &tracker.reads {
                *read_counts.entry(var.clone()).or_insert(0) += count;
            }
            for (var, count) in &tracker.writes {
                *write_counts.entry(var.clone()).or_insert(0) += count;
            }

            // Identify variable pairs accessed together
            let all_vars: Vec<_> = tracker.reads.keys().chain(tracker.writes.keys()).collect();
            for i in 0..all_vars.len() {
                for j in (i + 1)..all_vars.len() {
                    let var1 = all_vars[i];
                    let var2 = all_vars[j];
                    *co_occurrence.entry((var1.clone(), var2.clone())).or_insert(0) += 1;
                    *co_occurrence.entry((var2.clone(), var1.clone())).or_insert(0) += 1;
                }
            }
        }

        let mut hints = Vec::new();

        // Generate caching recommendations
        for (var, count) in &read_counts {
            let writes = *write_counts.get(var).unwrap_or(&0);
            if *count > 10 && writes < 2 {
                hints.push(format!("Consider caching variable '{}' (read {} times, written {} times)", var, count, writes));
            }
        }

        // Generate grouping recommendations
        let mut sorted_pairs: Vec<_> = co_occurrence.iter().collect();
        sorted_pairs.sort_by(|a, b| b.1.cmp(a.1));

        if let Some(((var1, var2), count)) = sorted_pairs.first() {
            hints.push(format!("Group '{}' and '{}' (accessed together {} times)", var1, var2, count));
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates optimization hint generation:
    /// 1. Create variable with 11 reads and 1 write
    /// 2. Verify system generates caching recommendation: Meets threshold: reads > 10 && writes < 2
    /// 3. Check hint appears in results
    #[test]
    fn test_hint_generation() {
        let mut tracker = StateAccessTracker::new();
        // Exceed read threshold
        for _ in 0..11 {
            tracker.track_read("cache_me");
        }
        // Minimal writes
        tracker.track_write("cache_me");

        let hints = OptimizationAnalyzer::generate_hints(&[tracker]);
        assert!(hints.iter().any(|h| h.contains("caching")), "Failed to generate caching hint for frequently read variable");
    }
}

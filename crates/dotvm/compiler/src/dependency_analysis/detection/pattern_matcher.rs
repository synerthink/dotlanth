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

//! Pattern matching utilities for dependency detection

use std::collections::HashMap;

/// Types of patterns that can be matched
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    /// Exact string match
    Exact,
    /// Prefix match
    Prefix,
    /// Suffix match
    Suffix,
    /// Contains match
    Contains,
    /// Regular expression match
    Regex,
    /// Wildcard match (*, ?)
    Wildcard,
}

/// A pattern definition
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The pattern string
    pub pattern: String,
    /// Type of pattern matching
    pub pattern_type: PatternType,
    /// Case sensitive matching
    pub case_sensitive: bool,
    /// Priority for pattern matching (higher = more priority)
    pub priority: u32,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Pattern {
    /// Create a new exact match pattern
    pub fn exact(pattern: String) -> Self {
        Self {
            pattern,
            pattern_type: PatternType::Exact,
            case_sensitive: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new prefix match pattern
    pub fn prefix(pattern: String) -> Self {
        Self {
            pattern,
            pattern_type: PatternType::Prefix,
            case_sensitive: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new suffix match pattern
    pub fn suffix(pattern: String) -> Self {
        Self {
            pattern,
            pattern_type: PatternType::Suffix,
            case_sensitive: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new contains match pattern
    pub fn contains(pattern: String) -> Self {
        Self {
            pattern,
            pattern_type: PatternType::Contains,
            case_sensitive: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new regex match pattern
    pub fn regex(pattern: String) -> Self {
        Self {
            pattern,
            pattern_type: PatternType::Regex,
            case_sensitive: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a new wildcard match pattern
    pub fn wildcard(pattern: String) -> Self {
        Self {
            pattern,
            pattern_type: PatternType::Wildcard,
            case_sensitive: true,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Result of a pattern match
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The pattern that matched
    pub pattern: Pattern,
    /// The matched text
    pub matched_text: String,
    /// Start position of the match
    pub start_position: usize,
    /// End position of the match
    pub end_position: usize,
    /// Captured groups (for regex patterns)
    pub captured_groups: Vec<String>,
    /// Match score (higher = better match)
    pub score: f64,
}

/// Pattern matcher for finding patterns in text
#[derive(Debug)]
pub struct PatternMatcher {
    /// Registered patterns
    patterns: Vec<Pattern>,
    /// Compiled regex patterns (cached)
    regex_cache: HashMap<String, regex::Regex>,
    /// Maximum number of matches to return
    max_matches: usize,
    /// Whether to return overlapping matches
    allow_overlapping: bool,
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self {
            patterns: Vec::new(),
            regex_cache: HashMap::new(),
            max_matches: 1000,
            allow_overlapping: false,
        }
    }
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of matches
    pub fn with_max_matches(mut self, max: usize) -> Self {
        self.max_matches = max;
        self
    }

    /// Enable or disable overlapping matches
    pub fn with_overlapping(mut self, allow: bool) -> Self {
        self.allow_overlapping = allow;
        self
    }

    /// Add a pattern to the matcher
    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
        // Sort by priority (highest first)
        self.patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove all patterns
    pub fn clear_patterns(&mut self) {
        self.patterns.clear();
        self.regex_cache.clear();
    }

    /// Get all registered patterns
    pub fn get_patterns(&self) -> &[Pattern] {
        &self.patterns
    }

    /// Find all matches in the given text
    pub fn find_matches(&mut self, text: &str) -> Vec<MatchResult> {
        let mut matches = Vec::new();
        let mut used_positions = std::collections::HashSet::new();

        let patterns = self.patterns.clone(); // Clone to avoid borrow checker issues
        for pattern in &patterns {
            let pattern_matches = self.find_pattern_matches(text, pattern);

            for match_result in pattern_matches {
                // Check for overlapping matches if not allowed
                if !self.allow_overlapping {
                    let overlaps = (match_result.start_position..match_result.end_position).any(|pos| used_positions.contains(&pos));

                    if overlaps {
                        continue;
                    }
                }

                // Mark positions as used
                for pos in match_result.start_position..match_result.end_position {
                    used_positions.insert(pos);
                }

                matches.push(match_result);

                // Check max matches limit
                if matches.len() >= self.max_matches {
                    break;
                }
            }

            if matches.len() >= self.max_matches {
                break;
            }
        }

        // Sort by position
        matches.sort_by_key(|m| m.start_position);
        matches
    }

    /// Find matches for a specific pattern
    fn find_pattern_matches(&mut self, text: &str, pattern: &Pattern) -> Vec<MatchResult> {
        let search_text = if pattern.case_sensitive { text.to_string() } else { text.to_lowercase() };

        let search_pattern = if pattern.case_sensitive { pattern.pattern.clone() } else { pattern.pattern.to_lowercase() };

        match pattern.pattern_type {
            PatternType::Exact => self.find_exact_matches(&search_text, &search_pattern, pattern),
            PatternType::Prefix => self.find_prefix_matches(&search_text, &search_pattern, pattern),
            PatternType::Suffix => self.find_suffix_matches(&search_text, &search_pattern, pattern),
            PatternType::Contains => self.find_contains_matches(&search_text, &search_pattern, pattern),
            PatternType::Regex => self.find_regex_matches(text, pattern),
            PatternType::Wildcard => self.find_wildcard_matches(&search_text, &search_pattern, pattern),
        }
    }

    /// Find exact matches
    fn find_exact_matches(&self, text: &str, pattern: &str, original_pattern: &Pattern) -> Vec<MatchResult> {
        if text == pattern {
            vec![MatchResult {
                pattern: original_pattern.clone(),
                matched_text: text.to_string(),
                start_position: 0,
                end_position: text.len(),
                captured_groups: Vec::new(),
                score: 1.0,
            }]
        } else {
            Vec::new()
        }
    }

    /// Find prefix matches
    fn find_prefix_matches(&self, text: &str, pattern: &str, original_pattern: &Pattern) -> Vec<MatchResult> {
        if text.starts_with(pattern) {
            vec![MatchResult {
                pattern: original_pattern.clone(),
                matched_text: pattern.to_string(),
                start_position: 0,
                end_position: pattern.len(),
                captured_groups: Vec::new(),
                score: pattern.len() as f64 / text.len() as f64,
            }]
        } else {
            Vec::new()
        }
    }

    /// Find suffix matches
    fn find_suffix_matches(&self, text: &str, pattern: &str, original_pattern: &Pattern) -> Vec<MatchResult> {
        if text.ends_with(pattern) {
            let start_pos = text.len() - pattern.len();
            vec![MatchResult {
                pattern: original_pattern.clone(),
                matched_text: pattern.to_string(),
                start_position: start_pos,
                end_position: text.len(),
                captured_groups: Vec::new(),
                score: pattern.len() as f64 / text.len() as f64,
            }]
        } else {
            Vec::new()
        }
    }

    /// Find contains matches
    fn find_contains_matches(&self, text: &str, pattern: &str, original_pattern: &Pattern) -> Vec<MatchResult> {
        let mut matches = Vec::new();
        let mut start = 0;

        while let Some(pos) = text[start..].find(pattern) {
            let absolute_pos = start + pos;
            matches.push(MatchResult {
                pattern: original_pattern.clone(),
                matched_text: pattern.to_string(),
                start_position: absolute_pos,
                end_position: absolute_pos + pattern.len(),
                captured_groups: Vec::new(),
                score: pattern.len() as f64 / text.len() as f64,
            });

            start = absolute_pos + 1;
        }

        matches
    }

    /// Find regex matches
    fn find_regex_matches(&mut self, text: &str, pattern: &Pattern) -> Vec<MatchResult> {
        let regex = match self.get_or_compile_regex(&pattern.pattern) {
            Ok(regex) => regex,
            Err(_) => return Vec::new(),
        };

        let mut matches = Vec::new();
        for capture in regex.captures_iter(text) {
            if let Some(full_match) = capture.get(0) {
                let captured_groups: Vec<String> = capture.iter().skip(1).filter_map(|m| m.map(|m| m.as_str().to_string())).collect();

                matches.push(MatchResult {
                    pattern: pattern.clone(),
                    matched_text: full_match.as_str().to_string(),
                    start_position: full_match.start(),
                    end_position: full_match.end(),
                    captured_groups,
                    score: full_match.len() as f64 / text.len() as f64,
                });
            }
        }

        matches
    }

    /// Find wildcard matches
    fn find_wildcard_matches(&self, text: &str, pattern: &str, original_pattern: &Pattern) -> Vec<MatchResult> {
        // Convert wildcard pattern to regex
        let regex_pattern = pattern.replace("*", ".*").replace("?", ".");

        let regex = match regex::Regex::new(&format!("^{}$", regex_pattern)) {
            Ok(regex) => regex,
            Err(_) => return Vec::new(),
        };

        if regex.is_match(text) {
            vec![MatchResult {
                pattern: original_pattern.clone(),
                matched_text: text.to_string(),
                start_position: 0,
                end_position: text.len(),
                captured_groups: Vec::new(),
                score: 1.0,
            }]
        } else {
            Vec::new()
        }
    }

    /// Get or compile a regex pattern
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&regex::Regex, regex::Error> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = regex::Regex::new(pattern)?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::exact("test".to_string())
            .case_sensitive(false)
            .with_priority(10)
            .with_metadata("key".to_string(), "value".to_string());

        assert_eq!(pattern.pattern, "test");
        assert_eq!(pattern.pattern_type, PatternType::Exact);
        assert!(!pattern.case_sensitive);
        assert_eq!(pattern.priority, 10);
        assert_eq!(pattern.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_pattern_matcher_creation() {
        let matcher = PatternMatcher::new().with_max_matches(100).with_overlapping(true);

        assert_eq!(matcher.max_matches, 100);
        assert!(matcher.allow_overlapping);
    }

    #[test]
    fn test_exact_match() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::exact("hello".to_string()));

        let matches = matcher.find_matches("hello");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "hello");
        assert_eq!(matches[0].start_position, 0);
        assert_eq!(matches[0].end_position, 5);
    }

    #[test]
    fn test_prefix_match() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::prefix("dep:".to_string()));

        let matches = matcher.find_matches("dep:module1");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "dep:");
        assert_eq!(matches[0].start_position, 0);
        assert_eq!(matches[0].end_position, 4);
    }

    #[test]
    fn test_contains_match() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::contains("import".to_string()));

        let matches = matcher.find_matches("import math; import os");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].start_position, 0);
        assert_eq!(matches[1].start_position, 13);
    }

    #[test]
    fn test_regex_match() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::regex(r"import\s+(\w+)".to_string()));

        let matches = matcher.find_matches("import math");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].captured_groups.len(), 1);
        assert_eq!(matches[0].captured_groups[0], "math");
    }

    #[test]
    fn test_wildcard_match() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::wildcard("*.js".to_string()));

        let matches = matcher.find_matches("script.js");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "script.js");
    }

    #[test]
    fn test_case_insensitive() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::exact("HELLO".to_string()).case_sensitive(false));

        let matches = matcher.find_matches("hello");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut matcher = PatternMatcher::new();
        matcher.add_pattern(Pattern::contains("test".to_string()).with_priority(1));
        matcher.add_pattern(Pattern::exact("test".to_string()).with_priority(10));

        // Higher priority pattern should be checked first
        assert_eq!(matcher.patterns[0].priority, 10);
        assert_eq!(matcher.patterns[1].priority, 1);
    }

    #[test]
    fn test_overlapping_matches() {
        let mut matcher = PatternMatcher::new().with_overlapping(false);
        matcher.add_pattern(Pattern::contains("ab".to_string()));
        matcher.add_pattern(Pattern::contains("bc".to_string()));

        let matches = matcher.find_matches("abc");
        // Should only get one match due to no overlapping
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_max_matches_limit() {
        let mut matcher = PatternMatcher::new().with_max_matches(2);
        matcher.add_pattern(Pattern::contains("a".to_string()));

        let matches = matcher.find_matches("aaaa");
        assert_eq!(matches.len(), 2);
    }
}

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

//! Source position tracking for parsing

use std::fmt;

/// Represents a position in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl Position {
    /// Create a new position
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Create a position at the beginning of a file
    pub fn start() -> Self {
        Self::new(1, 1)
    }

    /// Create an invalid/unknown position
    pub fn unknown() -> Self {
        Self::new(0, 0)
    }

    /// Check if this is a valid position
    pub fn is_valid(&self) -> bool {
        self.line > 0 && self.column > 0
    }

    /// Advance to the next column
    pub fn next_column(&mut self) {
        self.column += 1;
    }

    /// Advance to the next line
    pub fn next_line(&mut self) {
        self.line += 1;
        self.column = 1;
    }

    /// Advance by a character (handles newlines)
    pub fn advance(&mut self, ch: char) {
        if ch == '\n' {
            self.next_line();
        } else {
            self.next_column();
        }
    }

    /// Advance by multiple characters
    pub fn advance_by(&mut self, text: &str) {
        for ch in text.chars() {
            self.advance(ch);
        }
    }

    /// Get the offset from another position (in characters)
    pub fn offset_from(&self, other: Position) -> Option<usize> {
        if self.line < other.line || (self.line == other.line && self.column < other.column) {
            return None;
        }

        if self.line == other.line {
            Some(self.column - other.column)
        } else {
            // For multi-line spans, we can't easily calculate character offset
            // without the source text, so return None
            None
        }
    }

    /// Create a span from this position to another
    pub fn span_to(&self, end: Position) -> Span {
        Span::new(*self, end)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::start()
    }
}

/// Represents a span of source code between two positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Span {
    /// Create a new span
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Create a span covering a single position
    pub fn single(position: Position) -> Self {
        let mut end = position;
        end.next_column();
        Self::new(position, end)
    }

    /// Create an unknown/invalid span
    pub fn unknown() -> Self {
        Self::new(Position::unknown(), Position::unknown())
    }

    /// Check if this span is valid
    pub fn is_valid(&self) -> bool {
        self.start.is_valid() && self.end.is_valid() && self.start <= self.end
    }

    /// Check if this span contains a position
    pub fn contains(&self, position: Position) -> bool {
        self.start <= position && position < self.end
    }

    /// Check if this span overlaps with another span
    pub fn overlaps(&self, other: Span) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Merge this span with another span
    pub fn merge(&self, other: Span) -> Span {
        let start = if self.start <= other.start { self.start } else { other.start };
        let end = if self.end >= other.end { self.end } else { other.end };
        Span::new(start, end)
    }

    /// Get the length of this span in lines
    pub fn line_count(&self) -> usize {
        if self.end.line >= self.start.line { self.end.line - self.start.line + 1 } else { 0 }
    }

    /// Check if this span is on a single line
    pub fn is_single_line(&self) -> bool {
        self.start.line == self.end.line
    }

    /// Get the column span for single-line spans
    pub fn column_span(&self) -> Option<usize> {
        if self.is_single_line() { Some(self.end.column - self.start.column) } else { None }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_single_line() {
            if let Some(span) = self.column_span() {
                if span <= 1 {
                    write!(f, "{}", self.start)
                } else {
                    write!(f, "{}:{}", self.start, self.end.column)
                }
            } else {
                write!(f, "{}-{}", self.start, self.end)
            }
        } else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

/// Utility for tracking position while parsing
#[derive(Debug, Clone)]
pub struct PositionTracker {
    /// Current position
    position: Position,
    /// Source text for validation
    source: String,
    /// Current byte offset in the source
    byte_offset: usize,
}

impl PositionTracker {
    /// Create a new position tracker
    pub fn new(source: String) -> Self {
        Self {
            position: Position::start(),
            source,
            byte_offset: 0,
        }
    }

    /// Get the current position
    pub fn position(&self) -> Position {
        self.position
    }

    /// Get the current byte offset
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Check if we're at the end of the source
    pub fn is_at_end(&self) -> bool {
        self.byte_offset >= self.source.len()
    }

    /// Peek at the current character without advancing
    pub fn peek_char(&self) -> Option<char> {
        self.source[self.byte_offset..].chars().next()
    }

    /// Peek at the next n characters
    pub fn peek_chars(&self, n: usize) -> String {
        self.source[self.byte_offset..].chars().take(n).collect()
    }

    /// Advance by one character and return it
    pub fn next_char(&mut self) -> Option<char> {
        if let Some(ch) = self.peek_char() {
            self.position.advance(ch);
            self.byte_offset += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    /// Advance by multiple characters
    pub fn advance_by(&mut self, count: usize) -> String {
        let mut result = String::new();
        for _ in 0..count {
            if let Some(ch) = self.next_char() {
                result.push(ch);
            } else {
                break;
            }
        }
        result
    }

    /// Skip whitespace and return the number of characters skipped
    pub fn skip_whitespace(&mut self) -> usize {
        let mut count = 0;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.next_char();
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Get the remaining source text
    pub fn remaining(&self) -> &str {
        &self.source[self.byte_offset..]
    }

    /// Get a slice of the source text
    pub fn slice(&self, start_offset: usize, end_offset: usize) -> Option<&str> {
        if end_offset <= self.source.len() && start_offset <= end_offset {
            Some(&self.source[start_offset..end_offset])
        } else {
            None
        }
    }

    /// Reset to the beginning
    pub fn reset(&mut self) {
        self.position = Position::start();
        self.byte_offset = 0;
    }

    /// Set position to a specific byte offset
    pub fn seek_to(&mut self, byte_offset: usize) -> Result<(), String> {
        if byte_offset > self.source.len() {
            return Err("Byte offset out of bounds".to_string());
        }

        // Recalculate position by scanning from the beginning
        self.position = Position::start();
        let mut current_offset = 0;

        for ch in self.source.chars() {
            if current_offset >= byte_offset {
                break;
            }
            self.position.advance(ch);
            current_offset += ch.len_utf8();
        }

        self.byte_offset = byte_offset;
        Ok(())
    }

    /// Create a span from a start position to the current position
    pub fn span_from(&self, start: Position) -> Span {
        start.span_to(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(10, 5);
        assert_eq!(pos.line, 10);
        assert_eq!(pos.column, 5);
        assert!(pos.is_valid());

        let start = Position::start();
        assert_eq!(start.line, 1);
        assert_eq!(start.column, 1);

        let unknown = Position::unknown();
        assert!(!unknown.is_valid());
    }

    #[test]
    fn test_position_advancement() {
        let mut pos = Position::start();

        pos.next_column();
        assert_eq!(pos, Position::new(1, 2));

        pos.next_line();
        assert_eq!(pos, Position::new(2, 1));

        pos.advance('a');
        assert_eq!(pos, Position::new(2, 2));

        pos.advance('\n');
        assert_eq!(pos, Position::new(3, 1));
    }

    #[test]
    fn test_position_advance_by() {
        let mut pos = Position::start();
        pos.advance_by("hello\nworld");
        assert_eq!(pos, Position::new(2, 6));
    }

    #[test]
    fn test_span_creation() {
        let start = Position::new(1, 1);
        let end = Position::new(1, 5);
        let span = Span::new(start, end);

        assert!(span.is_valid());
        assert!(span.is_single_line());
        assert_eq!(span.column_span(), Some(4));
        assert_eq!(span.line_count(), 1);
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(Position::new(1, 1), Position::new(1, 5));

        assert!(span.contains(Position::new(1, 1)));
        assert!(span.contains(Position::new(1, 3)));
        assert!(!span.contains(Position::new(1, 5))); // End is exclusive
        assert!(!span.contains(Position::new(2, 1)));
    }

    #[test]
    fn test_span_overlaps() {
        let span1 = Span::new(Position::new(1, 1), Position::new(1, 5));
        let span2 = Span::new(Position::new(1, 3), Position::new(1, 7));
        let span3 = Span::new(Position::new(1, 6), Position::new(1, 10));

        assert!(span1.overlaps(span2));
        assert!(!span1.overlaps(span3));
    }

    #[test]
    fn test_span_merge() {
        let span1 = Span::new(Position::new(1, 1), Position::new(1, 5));
        let span2 = Span::new(Position::new(1, 3), Position::new(1, 7));
        let merged = span1.merge(span2);

        assert_eq!(merged.start, Position::new(1, 1));
        assert_eq!(merged.end, Position::new(1, 7));
    }

    #[test]
    fn test_position_tracker() {
        let source = "hello\nworld\n";
        let mut tracker = PositionTracker::new(source.to_string());

        assert_eq!(tracker.position(), Position::start());
        assert_eq!(tracker.peek_char(), Some('h'));

        assert_eq!(tracker.next_char(), Some('h'));
        assert_eq!(tracker.position(), Position::new(1, 2));

        let chars = tracker.advance_by(4); // "ello"
        assert_eq!(chars, "ello");
        assert_eq!(tracker.position(), Position::new(1, 6));

        assert_eq!(tracker.next_char(), Some('\n'));
        assert_eq!(tracker.position(), Position::new(2, 1));
    }

    #[test]
    fn test_position_tracker_whitespace() {
        let source = "  \t\n  hello";
        let mut tracker = PositionTracker::new(source.to_string());

        let skipped = tracker.skip_whitespace();
        assert_eq!(skipped, 6); // 2 spaces + 1 tab + 1 newline + 2 spaces = 6
        assert_eq!(tracker.position(), Position::new(2, 3));
        assert_eq!(tracker.peek_char(), Some('h'));
    }

    #[test]
    fn test_position_tracker_seek() {
        let source = "hello\nworld";
        let mut tracker = PositionTracker::new(source.to_string());

        // Seek to position after "hello\n"
        assert!(tracker.seek_to(6).is_ok());
        assert_eq!(tracker.position(), Position::new(2, 1));
        assert_eq!(tracker.peek_char(), Some('w'));

        // Test invalid seek
        assert!(tracker.seek_to(1000).is_err());
    }

    #[test]
    fn test_position_display() {
        let pos = Position::new(10, 5);
        assert_eq!(format!("{}", pos), "10:5");

        let span = Span::new(Position::new(1, 1), Position::new(1, 5));
        assert_eq!(format!("{}", span), "1:1:5");

        let multiline_span = Span::new(Position::new(1, 1), Position::new(3, 5));
        assert_eq!(format!("{}", multiline_span), "1:1-3:5");
    }
}

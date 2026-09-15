//! Slide comments for `PowerPoint` presentations.
//!
//! Comments are stored in `ppt/comments/commentN.xml` parts and
//! linked to slides via relationships.

use crate::units::Emu;

/// A comment on a slide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The author of the comment.
    pub author: String,
    /// The author's initials.
    pub author_initials: String,
    /// The comment text.
    pub text: String,
    /// The date/time string (ISO 8601 format).
    pub date: String,
    /// The anchor position on the slide (x, y) in EMU.
    pub position: (Emu, Emu),
    /// Author index (0-based, for `authorId` attribute).
    pub author_id: u32,
}

impl Comment {
    /// Create a new comment.
    #[must_use]
    pub fn new(author: &str, text: &str, x: Emu, y: Emu) -> Self {
        // Generate initials from author name
        let initials: String = author
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect();

        Self {
            author: author.to_string(),
            author_initials: initials,
            text: text.to_string(),
            date: String::new(),
            position: (x, y),
            author_id: 0,
        }
    }

    /// Builder method: attach a date/time to this comment.
    #[must_use]
    pub fn with_date(mut self, date: &str) -> Self {
        self.date = date.to_string();
        self
    }

    /// Builder method: set the author ID for this comment.
    #[must_use]
    pub const fn with_author_id(mut self, id: u32) -> Self {
        self.author_id = id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_new() {
        let c = Comment::new("John Doe", "Nice slide!", Emu(100), Emu(200));
        assert_eq!(c.author, "John Doe");
        assert_eq!(c.author_initials, "JD");
        assert_eq!(c.text, "Nice slide!");
        assert_eq!(c.position, (Emu(100), Emu(200)));
    }

    #[test]
    fn test_comment_with_date() {
        let c =
            Comment::new("Alice", "Review this", Emu(0), Emu(0)).with_date("2024-01-15T10:30:00Z");
        assert_eq!(c.date, "2024-01-15T10:30:00Z");
    }

    #[test]
    fn test_comment_initials_single_name() {
        let c = Comment::new("Alice", "Test", Emu(0), Emu(0));
        assert_eq!(c.author_initials, "A");
    }
}

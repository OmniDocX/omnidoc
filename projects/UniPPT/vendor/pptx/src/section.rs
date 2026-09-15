//! Section management for `PowerPoint` presentations.
//!
//! Sections are stored in the presentation XML inside `<p:extLst>` as
//! `<p:sectionLst>` elements. Each section has a name and references
//! slides by their `p:sldId` relationship IDs.

/// A section in a presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The display name of the section.
    pub name: String,
    /// The slide indices (0-based) that belong to this section.
    pub slide_indices: Vec<usize>,
}

impl Section {
    /// Create a new section with the given name and starting slide index.
    #[must_use]
    pub fn new(name: &str, first_slide_index: usize) -> Self {
        Self {
            name: name.to_string(),
            slide_indices: vec![first_slide_index],
        }
    }

    /// Create a new section with the given name and multiple slide indices.
    #[must_use]
    pub fn with_slides(name: &str, slide_indices: Vec<usize>) -> Self {
        Self {
            name: name.to_string(),
            slide_indices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_new() {
        let s = Section::new("Introduction", 0);
        assert_eq!(s.name, "Introduction");
        assert_eq!(s.slide_indices, vec![0]);
    }

    #[test]
    fn test_section_with_slides() {
        let s = Section::with_slides("Chapter 1", vec![0, 1, 2]);
        assert_eq!(s.name, "Chapter 1");
        assert_eq!(s.slide_indices, vec![0, 1, 2]);
    }
}

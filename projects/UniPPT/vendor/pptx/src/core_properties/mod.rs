//! Core document properties (`docProps/core.xml`).
//!
//! These correspond to the Dublin Core metadata elements and Office-specific
//! properties in the OPC package.

mod xml;

/// Core document properties stored in `docProps/core.xml`.
///
/// # Examples
///
/// ```
/// use pptx::core_properties::CoreProperties;
///
/// let mut props = CoreProperties::new();
/// props.set_title("My Presentation");
/// props.set_author("Jane Doe");
/// assert_eq!(props.title(), "My Presentation");
/// assert_eq!(props.author(), "Jane Doe");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoreProperties {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) subject: String,
    pub(crate) keywords: String,
    pub(crate) comments: String,
    pub(crate) category: String,
    pub(crate) created: String,
    pub(crate) modified: String,
    pub(crate) last_modified_by: String,
    pub(crate) revision: String,
    pub(crate) content_status: String,
    pub(crate) language: String,
    pub(crate) version: String,
    pub(crate) identifier: Option<String>,
    pub(crate) last_printed: Option<String>,
}

impl CoreProperties {
    /// Create a new empty `CoreProperties`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // --- Getters ---

    /// Returns the document title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the document author.
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Returns the document subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the document keywords.
    #[must_use]
    pub fn keywords(&self) -> &str {
        &self.keywords
    }

    /// Returns the document comments.
    #[must_use]
    pub fn comments(&self) -> &str {
        &self.comments
    }

    /// Returns the document category.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Returns the creation date as a string.
    #[must_use]
    pub fn created(&self) -> &str {
        &self.created
    }

    /// Returns the last modified date as a string.
    #[must_use]
    pub fn modified(&self) -> &str {
        &self.modified
    }

    /// Returns the name of the last person who modified the document.
    #[must_use]
    pub fn last_modified_by(&self) -> &str {
        &self.last_modified_by
    }

    /// Returns the document revision number.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// Returns the document content status.
    #[must_use]
    pub fn content_status(&self) -> &str {
        &self.content_status
    }

    /// Returns the document language.
    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Returns the document version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the document identifier, if set.
    #[must_use]
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    /// Returns the last printed date, if set.
    #[must_use]
    pub fn last_printed(&self) -> Option<&str> {
        self.last_printed.as_deref()
    }

    // --- Setters ---

    /// Sets the document title.
    pub fn set_title(&mut self, value: impl Into<String>) {
        self.title = value.into();
    }

    /// Sets the document author.
    pub fn set_author(&mut self, value: impl Into<String>) {
        self.author = value.into();
    }

    /// Sets the document subject.
    pub fn set_subject(&mut self, value: impl Into<String>) {
        self.subject = value.into();
    }

    /// Sets the document keywords.
    pub fn set_keywords(&mut self, value: impl Into<String>) {
        self.keywords = value.into();
    }

    /// Sets the document comments.
    pub fn set_comments(&mut self, value: impl Into<String>) {
        self.comments = value.into();
    }

    /// Sets the document category.
    pub fn set_category(&mut self, value: impl Into<String>) {
        self.category = value.into();
    }

    /// Sets the creation date.
    pub fn set_created(&mut self, value: impl Into<String>) {
        self.created = value.into();
    }

    /// Sets the last modified date.
    pub fn set_modified(&mut self, value: impl Into<String>) {
        self.modified = value.into();
    }

    /// Sets the name of the last person who modified the document.
    pub fn set_last_modified_by(&mut self, value: impl Into<String>) {
        self.last_modified_by = value.into();
    }

    /// Sets the document revision number.
    pub fn set_revision(&mut self, value: impl Into<String>) {
        self.revision = value.into();
    }

    /// Sets the document content status.
    pub fn set_content_status(&mut self, value: impl Into<String>) {
        self.content_status = value.into();
    }

    /// Sets the document language.
    pub fn set_language(&mut self, value: impl Into<String>) {
        self.language = value.into();
    }

    /// Sets the document version.
    pub fn set_version(&mut self, value: impl Into<String>) {
        self.version = value.into();
    }

    /// Sets the document identifier.
    pub fn set_identifier(&mut self, value: impl Into<String>) {
        self.identifier = Some(value.into());
    }

    /// Sets the last printed date.
    pub fn set_last_printed(&mut self, value: impl Into<String>) {
        self.last_printed = Some(value.into());
    }
}

#[cfg(test)]
mod tests;

//! Validation and repair functionality for PPTX files.
//!
//! Provides `PptxValidator` for detecting structural issues in a presentation,
//! and `PptxRepairer` for automatically fixing common problems.

mod repairer;
mod validator;

#[cfg(test)]
mod tests;

pub use repairer::PptxRepairer;
pub use validator::PptxValidator;

/// Severity of a validation issue.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Category of a validation issue.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueCategory {
    MissingPart,
    InvalidXml,
    BrokenRelationship,
    MissingSlideRef,
    OrphanSlide,
    InvalidContentType,
    MissingNamespace,
}

/// A single validation issue found in a PPTX file.
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub category: IssueCategory,
    pub description: String,
    pub location: Option<String>,
}

impl ValidationIssue {
    pub(crate) fn new(
        severity: Severity,
        category: IssueCategory,
        description: impl Into<String>,
        location: Option<String>,
    ) -> Self {
        Self {
            severity,
            category,
            description: description.into(),
            location,
        }
    }
}

/// Result of a repair operation.
#[derive(Debug, Clone)]
pub struct RepairReport {
    pub issues_found: Vec<ValidationIssue>,
    pub issues_fixed: Vec<ValidationIssue>,
    pub is_valid: bool,
}

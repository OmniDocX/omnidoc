// ---------------------------------------------------------------------------
// Identifier newtypes
// ---------------------------------------------------------------------------

use crate::error::PptxError;

/// A relationship ID (e.g. "rId1", "rId42").
///
/// Must start with "rId" followed by one or more ASCII digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationshipId(String);

impl RelationshipId {
    /// Returns the inner string slice.
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for RelationshipId {
    type Error = PptxError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let digits = s
            .strip_prefix("rId")
            .ok_or_else(|| PptxError::InvalidValue {
                field: "RelationshipId",
                value: s.to_string(),
                expected: "string starting with \"rId\" followed by digits",
            })?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(PptxError::InvalidValue {
                field: "RelationshipId",
                value: s.to_string(),
                expected: "string starting with \"rId\" followed by digits",
            });
        }
        Ok(Self(s.to_string()))
    }
}

impl TryFrom<String> for RelationshipId {
    type Error = PptxError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        RelationshipId::try_from(s.as_str())
    }
}

impl std::fmt::Display for RelationshipId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for RelationshipId {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<RelationshipId> for String {
    #[inline]
    fn from(v: RelationshipId) -> Self {
        v.0
    }
}

/// A shape identifier within a slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ShapeId(pub u32);

/// A slide identifier within a presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SlideId(pub u32);

impl std::fmt::Display for ShapeId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for SlideId {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for ShapeId {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<ShapeId> for u32 {
    #[inline]
    fn from(v: ShapeId) -> Self {
        v.0
    }
}

impl From<u32> for SlideId {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<SlideId> for u32 {
    #[inline]
    fn from(v: SlideId) -> Self {
        v.0
    }
}

/// A duration or delay in milliseconds, used for animation timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DurationMs(pub u32);

/// A placeholder index within a slide layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlaceholderIndex(pub u32);

/// A connection-point index on a shape, used by connectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ConnectionPointIndex(pub u32);

impl std::fmt::Display for DurationMs {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for PlaceholderIndex {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for ConnectionPointIndex {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for DurationMs {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<DurationMs> for u32 {
    #[inline]
    fn from(v: DurationMs) -> Self {
        v.0
    }
}

impl From<u32> for PlaceholderIndex {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<PlaceholderIndex> for u32 {
    #[inline]
    fn from(v: PlaceholderIndex) -> Self {
        v.0
    }
}

impl From<u32> for ConnectionPointIndex {
    #[inline]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<ConnectionPointIndex> for u32 {
    #[inline]
    fn from(v: ConnectionPointIndex) -> Self {
        v.0
    }
}

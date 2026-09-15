mod xml;

use std::borrow::Cow;

use crate::error::{PackageError, PptxError, PptxResult};
use crate::opc::pack_uri::PackURI;
use crate::units::RelationshipId;

/// A single relationship from a source part (or the package) to a target.
#[derive(Debug, Clone)]
pub struct Relationship {
    /// Relationship ID (e.g. "rId1").
    pub r_id: RelationshipId,
    /// Relationship type URI.
    pub rel_type: Cow<'static, str>,
    /// Target reference: a relative URI for internal, or a URL for external.
    pub target_ref: String,
    /// Whether the target is external to the package.
    pub is_external: bool,
}

impl Relationship {
    pub fn new(
        r_id: RelationshipId,
        rel_type: impl Into<Cow<'static, str>>,
        target_ref: String,
        is_external: bool,
    ) -> Self {
        Self {
            r_id,
            rel_type: rel_type.into(),
            target_ref,
            is_external,
        }
    }

    /// Resolve the target reference to an absolute `PackURI` given the source's base URI.
    /// Only valid for internal relationships.
    ///
    /// # Errors
    ///
    /// Returns an error if the relationship is external or the URI cannot be resolved.
    pub fn target_partname(&self, base_uri: &str) -> PptxResult<PackURI> {
        if self.is_external {
            return Err(PptxError::Package(PackageError::InvalidPackUri(
                "cannot resolve external relationship to a partname".to_string(),
            )));
        }
        PackURI::from_rel_ref(base_uri, &self.target_ref)
    }
}

/// A collection of relationships for a single source (part or package).
///
/// Uses a `Vec` internally for cache-friendly iteration; typical collections
/// contain fewer than 20 entries so linear search is faster than hashing.
#[derive(Debug, Clone, Default)]
pub struct Relationships {
    pub(crate) rels: Vec<Relationship>,
    pub(crate) base_uri: String,
    /// Tracks the highest numeric rId ever assigned, for O(1) `next_r_id` generation.
    pub(crate) max_r_id: u32,
}

impl Relationships {
    pub fn new(base_uri: impl Into<String>) -> Self {
        Self {
            rels: Vec::new(),
            base_uri: base_uri.into(),
            max_r_id: 0,
        }
    }

    /// Get a relationship by its rId.
    #[must_use]
    pub fn get(&self, r_id: impl AsRef<str>) -> Option<&Relationship> {
        let r_id = r_id.as_ref();
        self.rels.iter().find(|r| r.r_id.as_str() == r_id)
    }

    /// Get the single relationship with the given reltype.
    /// Returns an error if zero or multiple relationships match.
    ///
    /// # Errors
    ///
    /// Returns an error if no relationship or multiple relationships match.
    pub fn by_reltype(&self, reltype: &str) -> PptxResult<&Relationship> {
        let mut iter = self.rels.iter().filter(|r| r.rel_type == reltype);
        let first = iter.next().ok_or_else(|| {
            PptxError::Package(PackageError::RelationshipNotFound(format!(
                "no relationship of type '{reltype}'"
            )))
        })?;
        if iter.next().is_some() {
            return Err(PptxError::Package(PackageError::RelationshipNotFound(
                format!("multiple relationships of type '{reltype}'"),
            )));
        }
        Ok(first)
    }

    /// Get all relationships with the given reltype.
    #[must_use]
    pub fn all_by_reltype(&self, reltype: &str) -> Vec<&Relationship> {
        self.rels.iter().filter(|r| r.rel_type == reltype).collect()
    }

    /// Add a relationship and return its rId string.
    pub fn add(&mut self, rel: Relationship) -> String {
        let num = extract_rid_num(rel.r_id.as_str());
        if num > self.max_r_id {
            self.max_r_id = num;
        }
        let r_id = rel.r_id.to_string();
        self.rels.push(rel);
        r_id
    }

    /// Add a new relationship, auto-generating an rId.
    pub fn add_relationship(
        &mut self,
        rel_type: impl Into<Cow<'static, str>>,
        target_ref: impl Into<String>,
        is_external: bool,
    ) -> String {
        self.max_r_id += 1;
        let r_id_str = format!("rId{}", self.max_r_id);
        // EXCEPTION(infallible): r_id_str is always a valid "rId<N>" string we just constructed.
        let r_id = RelationshipId::try_from(r_id_str.as_str())
            .unwrap_or_else(|_| unreachable!("auto-generated rId is always valid"));
        let rel = Relationship::new(r_id, rel_type, target_ref.into(), is_external);
        self.rels.push(rel);
        r_id_str
    }

    /// Find an existing relationship matching the given reltype and `target_ref`, or add a new one.
    pub fn or_add(&mut self, rel_type: &str, target_ref: &str, is_external: bool) -> String {
        if let Some(rel) = self.rels.iter().find(|r| {
            r.rel_type == rel_type && r.target_ref == target_ref && r.is_external == is_external
        }) {
            return rel.r_id.to_string();
        }
        self.add_relationship(rel_type.to_string(), target_ref, is_external)
    }

    /// Remove a relationship by rId.
    pub fn remove(&mut self, r_id: &str) -> Option<Relationship> {
        if let Some(pos) = self.rels.iter().position(|r| r.r_id.as_str() == r_id) {
            Some(self.rels.swap_remove(pos))
        } else {
            None
        }
    }

    /// Iterate over all relationships.
    pub fn iter(&self) -> impl Iterator<Item = &Relationship> {
        self.rels.iter()
    }

    /// Number of relationships in this collection.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rels.len()
    }

    /// Whether this collection is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rels.is_empty()
    }

    /// The base URI for resolving relative references.
    #[must_use]
    pub fn base_uri(&self) -> &str {
        &self.base_uri
    }
}

/// Extract the numeric portion from an rId like "rId3" -> 3.
pub(crate) fn extract_rid_num(r_id: &str) -> u32 {
    r_id.strip_prefix("rId")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

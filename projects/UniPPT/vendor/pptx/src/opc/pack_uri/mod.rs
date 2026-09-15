use std::fmt;

use crate::error::{PackageError, PptxError, PptxResult};

/// A pack URI (partname) representing an internal path within an OPC package.
///
/// Pack URIs always start with "/" and use forward slashes as separators.
/// Examples: "/ppt/slides/slide1.xml", "/ppt/presentation.xml"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackURI {
    uri: String,
}

impl PackURI {
    /// Create a new `PackURI` from a string.
    /// The URI must conform to OPC part name rules.
    ///
    /// # Errors
    ///
    /// Returns an error if the URI violates any OPC part name rule.
    pub fn new(uri: impl Into<String>) -> PptxResult<Self> {
        let uri = uri.into();
        validate_pack_uri(&uri)?;
        Ok(Self { uri })
    }

    /// Create a `PackURI` without validation. Caller must ensure the URI starts with "/".
    pub(crate) fn from_raw(uri: String) -> Self {
        debug_assert!(uri.starts_with('/'));
        Self { uri }
    }

    /// The package pseudo-partname "/".
    #[must_use]
    pub fn package() -> Self {
        Self {
            uri: "/".to_string(),
        }
    }

    /// The content types URI `/[Content_Types].xml`.
    #[must_use]
    pub fn content_types() -> Self {
        Self {
            uri: "/[Content_Types].xml".to_string(),
        }
    }

    /// Resolve a relative reference against a base URI to produce an absolute `PackURI`.
    ///
    /// For example, resolving "../slideLayouts/slideLayout1.xml" against "/ppt/slides"
    /// produces "/ppt/slideLayouts/slideLayout1.xml".
    ///
    /// # Errors
    ///
    /// Returns an error if the resolved URI is invalid.
    pub fn from_rel_ref(base_uri: &str, relative_ref: &str) -> PptxResult<Self> {
        // If the relative ref is already absolute, use it directly
        if relative_ref.starts_with('/') {
            return Self::new(relative_ref);
        }

        // Join base_uri and relative_ref, then normalize
        let joined = if base_uri.ends_with('/') {
            format!("{base_uri}{relative_ref}")
        } else {
            format!("{base_uri}/{relative_ref}")
        };

        let normalized = normalize_path(&joined);
        Self::new(normalized)
    }

    /// The base URI (directory portion) of this pack URI.
    ///
    /// For "/ppt/slides/slide1.xml" this returns "/ppt/slides".
    /// For the package pseudo-partname "/" this returns "/".
    #[inline]
    #[must_use]
    pub fn base_uri(&self) -> &str {
        if self.uri == "/" {
            return "/";
        }
        match self.uri.rfind('/') {
            Some(0) | None => "/",
            Some(idx) => &self.uri[..idx],
        }
    }

    /// The file extension without the leading dot.
    ///
    /// For "/ppt/slides/slide1.xml" this returns "xml".
    /// For "/ppt/printerSettings/printerSettings1.bin" this returns "bin".
    #[inline]
    #[must_use]
    pub fn ext(&self) -> &str {
        let filename = self.filename();
        filename.rfind('.').map_or("", |idx| &filename[idx + 1..])
    }

    /// The filename portion of this pack URI.
    ///
    /// For "/ppt/slides/slide1.xml" this returns "slide1.xml".
    /// For the package pseudo-partname "/" this returns "".
    #[inline]
    #[must_use]
    pub fn filename(&self) -> &str {
        if self.uri == "/" {
            return "";
        }
        self.uri
            .rfind('/')
            .map_or_else(|| &*self.uri, |idx| &self.uri[idx + 1..])
    }

    /// The pack URI without the leading slash, used as the ZIP member name.
    ///
    /// For "/ppt/slides/slide1.xml" this returns "ppt/slides/slide1.xml".
    #[inline]
    #[must_use]
    pub fn membername(&self) -> &str {
        if self.uri.len() > 1 {
            &self.uri[1..]
        } else {
            ""
        }
    }

    /// Compute a relative reference from `base_uri` to this pack URI.
    ///
    /// For `PackURI("/ppt/slideLayouts/slideLayout1.xml")` with `base_uri` "/ppt/slides"
    /// this returns "../slideLayouts/slideLayout1.xml".
    #[must_use]
    pub fn relative_ref(&self, base_uri: &str) -> String {
        if base_uri == "/" {
            return self.uri[1..].to_string();
        }
        compute_relative_path(base_uri, &self.uri)
    }

    /// The pack URI of the .rels file corresponding to this part.
    ///
    /// For "/ppt/slides/slide1.xml" this returns "/_rels/slide1.xml.rels" under the
    /// same directory: "/ppt/slides/_rels/slide1.xml.rels".
    /// For "/" (package) this returns "/_rels/.rels".
    #[must_use]
    pub fn rels_uri(&self) -> Self {
        let filename = self.filename();
        let rels_filename = format!("{filename}.rels");
        let base = self.base_uri();
        let rels_path = if base == "/" {
            format!("/_rels/{rels_filename}")
        } else {
            format!("{base}/_rels/{rels_filename}")
        };
        Self::from_raw(rels_path)
    }

    /// Returns the string representation of the pack URI.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.uri
    }

    /// Consumes the `PackURI` and returns the inner `String`.
    #[inline]
    #[must_use]
    pub fn into_string(self) -> String {
        self.uri
    }

    /// Returns the optional numeric index from a partname.
    ///
    /// For "/ppt/slides/slide21.xml" this returns Some(21).
    /// For "/ppt/presentation.xml" this returns None.
    #[must_use]
    pub fn idx(&self) -> Option<u32> {
        let filename = self.filename();
        let name_part = filename.rfind('.').map_or(filename, |idx| &filename[..idx]);
        // Find where the trailing digits start
        let digit_start = name_part
            .rfind(|c: char| !c.is_ascii_digit())
            .map_or(0, |i| i + 1);
        if digit_start >= name_part.len() {
            return None;
        }
        let digits = &name_part[digit_start..];
        if digits.is_empty() {
            return None;
        }
        digits.parse().ok()
    }
}

impl fmt::Display for PackURI {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.uri)
    }
}

impl AsRef<str> for PackURI {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.uri
    }
}

/// Validate a pack URI against OPC part name rules.
///
/// Rules enforced:
/// - Must start with `/`
/// - Must NOT end with `/` (unless it is exactly `/`, the package root)
/// - Must NOT contain `//` (empty segments)
/// - Must NOT contain `\` (backslash)
/// - Each segment must be non-empty and must not be `.` or `..`
fn validate_pack_uri(path: &str) -> PptxResult<()> {
    if !path.starts_with('/') {
        return Err(PptxError::Package(PackageError::InvalidPackUri(format!(
            "PackURI must begin with '/', got {path:?}"
        ))));
    }
    // The package root "/" is always valid; skip further checks for it.
    if path == "/" {
        return Ok(());
    }
    if path.ends_with('/') {
        return Err(PptxError::Package(PackageError::InvalidPackUri(format!(
            "PackURI must not end with '/', got {path:?}"
        ))));
    }
    if path.contains("//") {
        return Err(PptxError::Package(PackageError::InvalidPackUri(format!(
            "PackURI must not contain empty segments ('//'), got {path:?}"
        ))));
    }
    if path.contains('\\') {
        return Err(PptxError::Package(PackageError::InvalidPackUri(format!(
            "PackURI must not contain backslashes, got {path:?}"
        ))));
    }
    for segment in path.split('/').skip(1) {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(PptxError::Package(PackageError::InvalidPackUri(format!(
                "PackURI contains invalid segment {segment:?} in {path:?}"
            ))));
        }
    }
    Ok(())
}

/// Normalize a POSIX-style path, resolving ".." and "." segments.
pub(super) fn normalize_path(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            s => segments.push(s),
        }
    }
    format!("/{}", segments.join("/"))
}

/// Compute a relative path from `from` to `to`, both absolute POSIX paths.
pub(super) fn compute_relative_path(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();

    let common = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = from_parts.len() - common;
    let mut parts: Vec<&str> = vec![".."; ups];
    for part in &to_parts[common..] {
        parts.push(part);
    }
    parts.join("/")
}

#[cfg(test)]
mod tests;

use thiserror::Error;

/// Errors related to OPC package structure and part lookup.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("package not found: {0}")]
    PackageNotFound(String),

    #[error("part not found: {0}")]
    PartNotFound(String),

    #[error("relationship not found: {0}")]
    RelationshipNotFound(String),

    #[error("content type not found for: {0}")]
    ContentTypeNotFound(String),

    #[error("invalid pack URI: {0}")]
    InvalidPackUri(String),

    #[error("duplicate ZIP entry: {0}")]
    DuplicatePart(String),
}

/// Errors related to slide-level operations.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum SlideError {
    #[error("invalid slide index: {0}")]
    InvalidIndex(usize),
}

/// Errors that can occur when working with .pptx files.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum PptxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("XML attribute error: {0}")]
    XmlAttr(#[from] quick_xml::events::attributes::AttrError),

    #[error("{0}")]
    Package(#[from] PackageError),

    #[error("{0}")]
    Slide(#[from] SlideError),

    #[error("Invalid XML structure: {0}")]
    InvalidXml(String),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("UTF-8 string error: {0}")]
    Utf8Str(#[from] std::str::Utf8Error),

    #[error("invalid value for {field}: {value} (expected {expected})")]
    InvalidValue {
        field: &'static str,
        value: String,
        expected: &'static str,
    },

    #[error("resource limit exceeded: {message}")]
    ResourceLimit { message: String },
}

pub type PptxResult<T> = Result<T, PptxError>;

/// Extension trait to convert `Option<T>` into a `PptxResult<T>` with a `PartNotFound` error.
///
/// Reduces boilerplate for the common pattern of looking up a part by name
/// and returning a `PartNotFound` error when absent.
pub trait PartNotFoundExt<T> {
    /// Convert `None` into `Err(PptxError::Package(PackageError::PartNotFound(...)))` using the given part name.
    ///
    /// # Errors
    ///
    /// Returns `PptxError::Package(PackageError::PartNotFound(...))` when the option is `None`.
    fn or_part_not_found(self, partname: &str) -> PptxResult<T>;
}

impl<T> PartNotFoundExt<T> for Option<T> {
    fn or_part_not_found(self, partname: &str) -> PptxResult<T> {
        self.ok_or_else(|| PptxError::Package(PackageError::PartNotFound(partname.to_string())))
    }
}

impl PptxError {
    /// Convenience for creating `PartNotFound` from a string-like value.
    pub fn part_not_found(name: impl Into<String>) -> Self {
        Self::Package(PackageError::PartNotFound(name.into()))
    }

    /// Convenience for creating `RelationshipNotFound` from a string-like value.
    pub fn rel_not_found(name: impl Into<String>) -> Self {
        Self::Package(PackageError::RelationshipNotFound(name.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Display output tests ---

    #[test]
    fn display_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = PptxError::Io(io_err);
        let msg = format!("{err}");
        assert!(msg.contains("I/O error"), "got: {msg}");
        assert!(msg.contains("file missing"), "got: {msg}");
    }

    #[test]
    fn display_zip_error() {
        let err = PptxError::Zip(zip::result::ZipError::FileNotFound);
        let msg = format!("{err}");
        assert!(msg.contains("ZIP error"), "got: {msg}");
    }

    #[test]
    fn display_xml_error() {
        let xml_err = quick_xml::Error::Io(std::sync::Arc::new(std::io::Error::other("xml io")));
        let err = PptxError::Xml(xml_err);
        let msg = format!("{err}");
        assert!(msg.contains("XML error"), "got: {msg}");
    }

    #[test]
    fn display_invalid_xml() {
        let err = PptxError::InvalidXml("bad structure".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("Invalid XML structure"), "got: {msg}");
        assert!(msg.contains("bad structure"), "got: {msg}");
    }

    #[test]
    fn display_package_part_not_found() {
        let err = PptxError::Package(PackageError::PartNotFound("/ppt/slides/slide99.xml".into()));
        let msg = format!("{err}");
        assert!(msg.contains("part not found"), "got: {msg}");
        assert!(msg.contains("slide99"), "got: {msg}");
    }

    #[test]
    fn display_package_rel_not_found() {
        let err = PptxError::Package(PackageError::RelationshipNotFound("rId999".into()));
        let msg = format!("{err}");
        assert!(msg.contains("relationship not found"), "got: {msg}");
        assert!(msg.contains("rId999"), "got: {msg}");
    }

    #[test]
    fn display_slide_invalid_index() {
        let err = PptxError::Slide(SlideError::InvalidIndex(42));
        let msg = format!("{err}");
        assert!(msg.contains("invalid slide index"), "got: {msg}");
        assert!(msg.contains("42"), "got: {msg}");
    }

    #[test]
    fn display_invalid_value() {
        let err = PptxError::InvalidValue {
            field: "width",
            value: "-100".to_string(),
            expected: "positive integer",
        };
        let msg = format!("{err}");
        assert!(msg.contains("width"), "got: {msg}");
        assert!(msg.contains("-100"), "got: {msg}");
        assert!(msg.contains("positive integer"), "got: {msg}");
    }

    // --- From conversion tests ---

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: PptxError = io_err.into();
        assert!(matches!(err, PptxError::Io(_)));
    }

    #[test]
    fn from_zip_error() {
        let zip_err = zip::result::ZipError::FileNotFound;
        let err: PptxError = zip_err.into();
        assert!(matches!(err, PptxError::Zip(_)));
    }

    #[test]
    fn from_xml_error() {
        let xml_err = quick_xml::Error::Io(std::sync::Arc::new(std::io::Error::other("xml io")));
        let err: PptxError = xml_err.into();
        assert!(matches!(err, PptxError::Xml(_)));
    }

    #[test]
    fn from_package_error() {
        let pkg_err = PackageError::PackageNotFound("test".into());
        let err: PptxError = pkg_err.into();
        assert!(matches!(err, PptxError::Package(_)));
    }

    #[test]
    fn from_slide_error() {
        let slide_err = SlideError::InvalidIndex(0);
        let err: PptxError = slide_err.into();
        assert!(matches!(err, PptxError::Slide(_)));
    }

    // --- Convenience constructor tests ---

    #[test]
    fn part_not_found_constructor() {
        let err = PptxError::part_not_found("/ppt/slides/slide1.xml");
        match err {
            PptxError::Package(PackageError::PartNotFound(name)) => {
                assert_eq!(name, "/ppt/slides/slide1.xml");
            }
            other => panic!("expected PartNotFound, got: {other}"),
        }
    }

    #[test]
    fn part_not_found_constructor_with_string() {
        let err = PptxError::part_not_found(String::from("test.xml"));
        match err {
            PptxError::Package(PackageError::PartNotFound(name)) => {
                assert_eq!(name, "test.xml");
            }
            other => panic!("expected PartNotFound, got: {other}"),
        }
    }

    #[test]
    fn rel_not_found_constructor() {
        let err = PptxError::rel_not_found("rId5");
        match err {
            PptxError::Package(PackageError::RelationshipNotFound(name)) => {
                assert_eq!(name, "rId5");
            }
            other => panic!("expected RelationshipNotFound, got: {other}"),
        }
    }

    // --- PartNotFoundExt trait tests ---

    #[test]
    fn part_not_found_ext_none_returns_err() {
        let opt: Option<i32> = None;
        let result = opt.or_part_not_found("/ppt/test.xml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            PptxError::Package(PackageError::PartNotFound(name)) => {
                assert_eq!(name, "/ppt/test.xml");
            }
            other => panic!("expected PartNotFound, got: {other}"),
        }
    }

    #[test]
    fn part_not_found_ext_some_returns_ok() {
        let opt: Option<i32> = Some(42);
        let result = opt.or_part_not_found("irrelevant");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn part_not_found_ext_some_string_returns_ok() {
        let opt: Option<String> = Some("hello".into());
        let result = opt.or_part_not_found("irrelevant");
        assert_eq!(result.unwrap(), "hello");
    }
}

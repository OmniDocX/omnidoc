use crate::enums::shapes::{PlaceholderOrientation, PlaceholderSize, PpPlaceholderType};
use crate::units::PlaceholderIndex;

/// Placeholder properties from the `<p:ph>` element.
///
/// Placeholders are special shapes on a slide layout or slide that
/// are filled in by the user (e.g. title, body, slide number).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceholderFormat {
    /// Placeholder type. Corresponds to the `type` attribute of `<p:ph>`.
    pub ph_type: Option<PpPlaceholderType>,
    /// Placeholder index. Corresponds to the `idx` attribute of `<p:ph>`.
    pub idx: PlaceholderIndex,
    /// Placeholder orientation ("horz" or "vert").
    pub orient: Option<PlaceholderOrientation>,
    /// Placeholder size ("full", "half", or "quarter").
    pub sz: Option<PlaceholderSize>,
}

impl PlaceholderFormat {
    #[must_use]
    pub const fn is_title(&self) -> bool {
        matches!(
            self.ph_type,
            Some(PpPlaceholderType::Title | PpPlaceholderType::CenterTitle)
        )
    }

    #[must_use]
    pub const fn is_body(&self) -> bool {
        matches!(
            self.ph_type,
            Some(PpPlaceholderType::Body | PpPlaceholderType::Subtitle)
        )
    }

    #[must_use]
    pub const fn is_slide_number(&self) -> bool {
        matches!(self.ph_type, Some(PpPlaceholderType::SlideNumber))
    }

    #[must_use]
    pub const fn is_date(&self) -> bool {
        matches!(self.ph_type, Some(PpPlaceholderType::Date))
    }

    #[must_use]
    pub const fn is_footer(&self) -> bool {
        matches!(self.ph_type, Some(PpPlaceholderType::Footer))
    }

    /// Return the placeholder type enum.
    #[must_use]
    pub const fn placeholder_type(&self) -> Option<PpPlaceholderType> {
        self.ph_type
    }

    /// Serialize this placeholder format as an XML `<p:ph .../>` element string.
    #[must_use]
    pub fn to_xml_string(&self) -> String {
        let mut xml = String::from("<p:ph");
        if let Some(pt) = self.ph_type {
            xml.push_str(&format!(r#" type="{}""#, pt.to_xml_str()));
        }
        if self.idx.0 > 0 {
            xml.push_str(&format!(r#" idx="{}""#, self.idx));
        }
        if let Some(orient) = self.orient {
            xml.push_str(&format!(r#" orient="{}""#, orient.to_xml_str()));
        }
        if let Some(sz) = self.sz {
            xml.push_str(&format!(r#" sz="{}""#, sz.to_xml_str()));
        }
        xml.push_str("/>");
        xml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_type_conversion() {
        let ph = PlaceholderFormat {
            ph_type: Some(PpPlaceholderType::Title),
            idx: PlaceholderIndex(0),
            orient: None,
            sz: None,
        };
        assert_eq!(ph.placeholder_type(), Some(PpPlaceholderType::Title));
    }

    #[test]
    fn test_placeholder_type_center_title() {
        let ph = PlaceholderFormat {
            ph_type: Some(PpPlaceholderType::CenterTitle),
            idx: PlaceholderIndex(0),
            orient: None,
            sz: None,
        };
        assert_eq!(ph.placeholder_type(), Some(PpPlaceholderType::CenterTitle));
    }

    #[test]
    fn test_placeholder_type_none() {
        let ph = PlaceholderFormat {
            ph_type: None,
            idx: PlaceholderIndex(0),
            orient: None,
            sz: None,
        };
        assert_eq!(ph.placeholder_type(), None);
    }

    #[test]
    fn test_to_xml_string_basic() {
        let ph = PlaceholderFormat {
            ph_type: Some(PpPlaceholderType::Title),
            idx: PlaceholderIndex(0),
            orient: None,
            sz: None,
        };
        assert_eq!(ph.to_xml_string(), r#"<p:ph type="title"/>"#);
    }

    #[test]
    fn test_to_xml_string_with_idx() {
        let ph = PlaceholderFormat {
            ph_type: Some(PpPlaceholderType::Body),
            idx: PlaceholderIndex(1),
            orient: None,
            sz: None,
        };
        assert_eq!(ph.to_xml_string(), r#"<p:ph type="body" idx="1"/>"#);
    }

    #[test]
    fn test_to_xml_string_with_all_attrs() {
        let ph = PlaceholderFormat {
            ph_type: Some(PpPlaceholderType::Body),
            idx: PlaceholderIndex(2),
            orient: Some(PlaceholderOrientation::Vertical),
            sz: Some(PlaceholderSize::Half),
        };
        let xml = ph.to_xml_string();
        assert!(xml.contains(r#"type="body""#));
        assert!(xml.contains(r#"idx="2""#));
        assert!(xml.contains(r#"orient="vert""#));
        assert!(xml.contains(r#"sz="half""#));
    }

    #[test]
    fn test_to_xml_string_no_type() {
        let ph = PlaceholderFormat {
            ph_type: None,
            idx: PlaceholderIndex(0),
            orient: None,
            sz: None,
        };
        assert_eq!(ph.to_xml_string(), "<p:ph/>");
    }
}

//! Action settings and hyperlink types for shapes.

pub use crate::enums::action::PpActionType;
use crate::units::RelationshipId;
use crate::xml_util::xml_escape;

/// A hyperlink reference with address and optional tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink {
    /// The URL or address for the hyperlink.
    pub address: Option<String>,
    /// Tooltip text shown on hover.
    pub tooltip: Option<String>,
    /// The relationship ID for the hyperlink target.
    pub r_id: Option<RelationshipId>,
}

impl Hyperlink {
    /// Create a new hyperlink with the given URL.
    #[must_use]
    pub fn new(url: &str) -> Self {
        Self {
            address: Some(url.to_string()),
            tooltip: None,
            r_id: None,
        }
    }

    /// Create a new hyperlink with URL and tooltip.
    #[must_use]
    pub fn with_tooltip(url: &str, tooltip: &str) -> Self {
        Self {
            address: Some(url.to_string()),
            tooltip: Some(tooltip.to_string()),
            r_id: None,
        }
    }
}

/// Action settings for a shape click event.
///
/// Corresponds to the `<a:hlinkClick>` or `<a:hlinkHover>` elements
/// within a shape's `<p:cNvPr>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionSetting {
    /// The type of action to perform.
    pub action: PpActionType,
    /// Hyperlink details (URL, tooltip).
    pub hyperlink: Option<Hyperlink>,
    /// Target slide part name for slide navigation actions.
    pub target_slide: Option<String>,
    /// Relationship ID for named slide jump (used with `PpActionType::NamedSlide`).
    pub target_slide_r_id: Option<RelationshipId>,
}

impl ActionSetting {
    /// Create an action setting for a hyperlink URL.
    #[must_use]
    pub fn hyperlink(url: &str) -> Self {
        Self {
            action: PpActionType::Hyperlink,
            hyperlink: Some(Hyperlink::new(url)),
            target_slide: None,
            target_slide_r_id: None,
        }
    }

    /// Create an action setting for a hyperlink with tooltip.
    #[must_use]
    pub fn hyperlink_with_tooltip(url: &str, tooltip: &str) -> Self {
        Self {
            action: PpActionType::Hyperlink,
            hyperlink: Some(Hyperlink::with_tooltip(url, tooltip)),
            target_slide: None,
            target_slide_r_id: None,
        }
    }

    /// Create a next-slide action.
    #[must_use]
    pub const fn next_slide() -> Self {
        Self {
            action: PpActionType::NextSlide,
            hyperlink: None,
            target_slide: None,
            target_slide_r_id: None,
        }
    }

    /// Create a previous-slide action.
    #[must_use]
    pub const fn previous_slide() -> Self {
        Self {
            action: PpActionType::PreviousSlide,
            hyperlink: None,
            target_slide: None,
            target_slide_r_id: None,
        }
    }

    /// Create a named-slide action that jumps to a specific slide by relationship ID.
    #[must_use]
    pub fn named_slide(r_id: &RelationshipId) -> Self {
        Self {
            action: PpActionType::NamedSlide,
            hyperlink: None,
            target_slide: None,
            target_slide_r_id: Some(r_id.clone()),
        }
    }

    /// Write the `<a:hlinkClick>` XML element to the given writer.
    ///
    /// The `r_id` parameter is the relationship ID to use for external hyperlinks.
    /// If `None`, internal actions (next/previous/first/last slide) are generated.
    ///
    /// # Errors
    ///
    /// Returns `std::fmt::Error` if writing to the writer fails.
    pub fn write_xml<W: std::fmt::Write>(&self, w: &mut W, r_id: Option<&str>) -> std::fmt::Result {
        self.write_xml_element(w, "a:hlinkClick", r_id)
    }

    /// Write the `<a:hlinkHover>` XML element to the given writer.
    ///
    /// Same as `write_xml` but produces a hover action element.
    ///
    /// # Errors
    ///
    /// Returns `std::fmt::Error` if writing to the writer fails.
    pub fn write_hover_xml<W: std::fmt::Write>(
        &self,
        w: &mut W,
        r_id: Option<&str>,
    ) -> std::fmt::Result {
        self.write_xml_element(w, "a:hlinkHover", r_id)
    }

    /// Generate the `<a:hlinkClick>` XML element string.
    ///
    /// The `r_id` parameter is the relationship ID to use for external hyperlinks.
    /// If `None`, internal actions (next/previous/first/last slide) are generated.
    ///
    /// # Panics
    ///
    /// Panics if writing to a `String` fails (should never happen).
    #[must_use]
    pub fn to_xml_string(&self, r_id: Option<&str>) -> String {
        let mut s = String::new();
        // SAFETY: write to String is infallible
        self.write_xml(&mut s, r_id)
            .unwrap_or_else(|_| unreachable!("write to String should not fail"));
        s
    }

    /// Generate the `<a:hlinkHover>` XML element string.
    ///
    /// Same as `to_xml_string` but produces a hover action element.
    ///
    /// # Panics
    ///
    /// Panics if writing to a `String` fails (should never happen).
    #[must_use]
    pub fn to_hover_xml_string(&self, r_id: Option<&str>) -> String {
        let mut s = String::new();
        // SAFETY: write to String is infallible
        self.write_hover_xml(&mut s, r_id)
            .unwrap_or_else(|_| unreachable!("write to String should not fail"));
        s
    }

    /// Write XML for either hlinkClick or hlinkHover to the given writer.
    fn write_xml_element<W: std::fmt::Write>(
        &self,
        w: &mut W,
        element_name: &str,
        r_id: Option<&str>,
    ) -> std::fmt::Result {
        write!(w, "<{element_name}")?;

        match self.action {
            PpActionType::Hyperlink => {
                if let Some(rid) = r_id {
                    write!(w, r#" r:id="{}""#, xml_escape(rid))?;
                } else if let Some(ref hlink) = self.hyperlink {
                    if let Some(ref rid) = hlink.r_id {
                        write!(w, r#" r:id="{}""#, xml_escape(rid.as_str()))?;
                    }
                }
            }
            PpActionType::NamedSlide => {
                if let Some(ref rid) = self.target_slide_r_id {
                    write!(w, r#" r:id="{}""#, xml_escape(rid.as_str()))?;
                } else if let Some(rid) = r_id {
                    write!(w, r#" r:id="{}""#, xml_escape(rid))?;
                }
                w.write_str(r#" action="ppaction://hlinksldjump""#)?;
            }
            PpActionType::NextSlide => {
                w.write_str(r#" action="ppaction://hlinkshowjump?jump=nextslide""#)?;
            }
            PpActionType::PreviousSlide => {
                w.write_str(r#" action="ppaction://hlinkshowjump?jump=previousslide""#)?;
            }
            PpActionType::FirstSlide => {
                w.write_str(r#" action="ppaction://hlinkshowjump?jump=firstslide""#)?;
            }
            PpActionType::LastSlide => {
                w.write_str(r#" action="ppaction://hlinkshowjump?jump=lastslide""#)?;
            }
            PpActionType::EndShow => {
                w.write_str(r#" action="ppaction://hlinkshowjump?jump=endshow""#)?;
            }
            _ => {}
        }

        if let Some(ref hlink) = self.hyperlink {
            if let Some(ref tooltip) = hlink.tooltip {
                write!(w, r#" tooltip="{}""#, xml_escape(tooltip))?;
            }
        }

        w.write_str("/>")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperlink_action_xml() {
        let action = ActionSetting::hyperlink("https://example.com");
        let xml = action.to_xml_string(Some("rId2"));
        assert!(xml.contains(r#"r:id="rId2""#));
        assert!(xml.starts_with("<a:hlinkClick"));
        assert!(xml.ends_with("/>"));
    }

    #[test]
    fn test_hyperlink_with_tooltip_xml() {
        let action = ActionSetting::hyperlink_with_tooltip("https://example.com", "Click me");
        let xml = action.to_xml_string(Some("rId3"));
        assert!(xml.contains(r#"r:id="rId3""#));
        assert!(xml.contains(r#"tooltip="Click me""#));
    }

    #[test]
    fn test_next_slide_action_xml() {
        let action = ActionSetting::next_slide();
        let xml = action.to_xml_string(None);
        assert!(xml.contains("ppaction://hlinkshowjump?jump=nextslide"));
    }

    #[test]
    fn test_previous_slide_action_xml() {
        let action = ActionSetting::previous_slide();
        let xml = action.to_xml_string(None);
        assert!(xml.contains("ppaction://hlinkshowjump?jump=previousslide"));
    }

    #[test]
    fn test_tooltip_escaping() {
        let action = ActionSetting::hyperlink_with_tooltip("https://example.com", "A & B <test>");
        let xml = action.to_xml_string(Some("rId1"));
        assert!(xml.contains(r#"tooltip="A &amp; B &lt;test&gt;""#));
    }

    #[test]
    fn test_named_slide_action_xml() {
        let r_id = RelationshipId::try_from("rId5").unwrap();
        let action = ActionSetting::named_slide(&r_id);
        let xml = action.to_xml_string(None);
        assert!(xml.contains(r#"r:id="rId5""#));
        assert!(xml.contains(r#"action="ppaction://hlinksldjump""#));
    }

    #[test]
    fn test_hover_action_xml() {
        let action = ActionSetting::next_slide();
        let xml = action.to_hover_xml_string(None);
        assert!(xml.starts_with("<a:hlinkHover"));
        assert!(xml.contains("nextslide"));
    }

    #[test]
    fn test_hover_hyperlink_xml() {
        let action = ActionSetting::hyperlink("https://example.com");
        let xml = action.to_hover_xml_string(Some("rId3"));
        assert!(xml.starts_with("<a:hlinkHover"));
        assert!(xml.contains(r#"r:id="rId3""#));
    }
}

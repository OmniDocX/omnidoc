//! Bevel effect type for 3D shapes.

use crate::enums::dml::BevelType;

/// A bevel effect applied to the top or bottom face of a shape.
///
/// Corresponds to `<a:bevelT>` or `<a:bevelB>` within `<a:sp3d>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bevel {
    /// Bevel preset type (e.g. `BevelType::Circle`).
    pub bevel_type: BevelType,
    /// Bevel width in EMU.
    pub width: i64,
    /// Bevel height in EMU.
    pub height: i64,
}

impl Bevel {
    /// Create a bevel with the given type, width, and height.
    pub fn new(bevel_type: impl Into<BevelType>, width: i64, height: i64) -> Self {
        Self {
            bevel_type: bevel_type.into(),
            width,
            height,
        }
    }

    /// Create a circle bevel with the given dimensions.
    #[must_use]
    pub fn circle(width: i64, height: i64) -> Self {
        Self::new(BevelType::Circle, width, height)
    }

    /// Write XML for this bevel with the given element tag to the given writer.
    pub(crate) fn write_xml_with_tag<W: std::fmt::Write>(
        &self,
        w: &mut W,
        tag: &str,
    ) -> std::fmt::Result {
        write!(
            w,
            r#"<{} w="{}" h="{}" prst="{}"/>"#,
            tag,
            self.width,
            self.height,
            self.bevel_type.to_xml_str()
        )
    }

    /// Generate XML for this bevel with the given element tag (e.g. "a:bevelT" or "a:bevelB").
    #[cfg(test)]
    pub(super) fn to_xml_string_with_tag(&self, tag: &str) -> String {
        let mut s = String::new();
        self.write_xml_with_tag(&mut s, tag)
            .expect("write to String should not fail"); // EXCEPTION(test-only)
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bevel_new() {
        let bevel = Bevel::new(BevelType::Circle, 63_500, 25_400);
        assert_eq!(bevel.bevel_type, BevelType::Circle);
        assert_eq!(bevel.width, 63_500);
        assert_eq!(bevel.height, 25_400);
    }

    #[test]
    fn test_bevel_circle() {
        let bevel = Bevel::circle(63_500, 25_400);
        assert_eq!(bevel.bevel_type, BevelType::Circle);
    }

    #[test]
    fn test_bevel_top_xml() {
        let bevel = Bevel::new("circle", 63_500, 25_400);
        let xml = bevel.to_xml_string_with_tag("a:bevelT");
        assert_eq!(xml, r#"<a:bevelT w="63500" h="25400" prst="circle"/>"#);
    }

    #[test]
    fn test_bevel_bottom_xml() {
        let bevel = Bevel::new("relaxedInset", 50_800, 38_100);
        let xml = bevel.to_xml_string_with_tag("a:bevelB");
        assert_eq!(
            xml,
            r#"<a:bevelB w="50800" h="38100" prst="relaxedInset"/>"#
        );
    }

    #[test]
    fn test_bevel_various_types() {
        let types = [
            BevelType::Circle,
            BevelType::RelaxedInset,
            BevelType::Angle,
            BevelType::SoftRound,
            BevelType::Convex,
            BevelType::Slope,
            BevelType::Divot,
            BevelType::Riblet,
            BevelType::ArtDeco,
        ];
        for t in &types {
            let bevel = Bevel::new(t.clone(), 50_800, 25_400);
            let xml = bevel.to_xml_string_with_tag("a:bevelT");
            assert!(xml.contains(&format!(r#"prst="{}""#, t.to_xml_str())));
        }
    }
}

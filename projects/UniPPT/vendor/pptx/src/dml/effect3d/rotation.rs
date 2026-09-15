//! 3D rotation angles type.

use crate::xml_util::WriteXml;

/// 3D rotation angles.
///
/// Represents rotation around three axes, stored in 60,000ths of a degree
/// as required by OOXML (`<a:rot lat="..." lon="..." rev="..."/>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rotation3D {
    /// Latitude rotation in 60,000ths of a degree.
    pub lat: i64,
    /// Longitude rotation in 60,000ths of a degree.
    pub lon: i64,
    /// Revolution rotation in 60,000ths of a degree.
    pub rev: i64,
}

impl Rotation3D {
    /// Create a rotation from values already in 60,000ths of a degree.
    #[must_use]
    pub const fn new(lat: i64, lon: i64, rev: i64) -> Self {
        Self { lat, lon, rev }
    }

    /// Create a rotation from degrees (converts to 60,000ths internally).
    // EMU values fit in i64 range
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_degrees(lat: f64, lon: f64, rev: f64) -> Self {
        Self {
            lat: (lat * 60_000.0) as i64,
            lon: (lon * 60_000.0) as i64,
            rev: (rev * 60_000.0) as i64,
        }
    }
}

impl WriteXml for Rotation3D {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        write!(
            w,
            r#"<a:rot lat="{}" lon="{}" rev="{}"/>"#,
            self.lat, self.lon, self.rev
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation3d_new() {
        let rot = Rotation3D::new(0, 0, 0);
        assert_eq!(rot.lat, 0);
        assert_eq!(rot.lon, 0);
        assert_eq!(rot.rev, 0);
    }

    #[test]
    fn test_rotation3d_from_degrees() {
        let rot = Rotation3D::from_degrees(45.0, 90.0, 180.0);
        assert_eq!(rot.lat, 2_700_000);
        assert_eq!(rot.lon, 5_400_000);
        assert_eq!(rot.rev, 10_800_000);
    }

    #[test]
    fn test_rotation3d_xml() {
        let rot = Rotation3D::new(2_700_000, 5_400_000, 0);
        let xml = rot.to_xml_string();
        assert_eq!(xml, r#"<a:rot lat="2700000" lon="5400000" rev="0"/>"#);
    }

    #[test]
    fn test_rotation3d_zero_xml() {
        let rot = Rotation3D::new(0, 0, 0);
        assert_eq!(rot.to_xml_string(), r#"<a:rot lat="0" lon="0" rev="0"/>"#);
    }
}

mod identifiers;

pub use identifiers::{
    ConnectionPointIndex, DurationMs, PlaceholderIndex, RelationshipId, ShapeId, SlideId,
};

/// English Metric Units -- the base unit used internally by OOXML.
///
/// 1 inch = 914,400 EMU, 1 cm = 360,000 EMU, 1 pt = 12,700 EMU.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Emu(pub i64);

/// Length in inches.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Inches(pub f64);

/// Length in centimeters.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Cm(pub f64);

/// Length in points (1/72 inch).
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Pt(pub f64);

/// Length in millimeters.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Mm(pub f64);

/// Length in centipoints (1/100 of a point).
///
/// 1 centipoint = 1/100 point = 127 EMU.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Centipoints(pub f64);

/// Length in twips (1/20 of a point).
///
/// 1 twip = 1/20 point = 635 EMU.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Twips(pub f64);

const EMUS_PER_INCH: f64 = 914_400.0;
const EMUS_PER_CM: f64 = 360_000.0;
const EMUS_PER_PT: f64 = 12_700.0;
const EMUS_PER_MM: f64 = 36_000.0;
const EMUS_PER_CENTIPOINT: f64 = 127.0;
const EMUS_PER_TWIP: f64 = 635.0;

impl Emu {
    // Precision loss acceptable for display/measurement purposes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub const fn to_inches(self) -> f64 {
        self.0 as f64 / EMUS_PER_INCH
    }
    // Precision loss acceptable for display/measurement purposes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub const fn to_cm(self) -> f64 {
        self.0 as f64 / EMUS_PER_CM
    }
    // Precision loss acceptable for display/measurement purposes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub const fn to_pt(self) -> f64 {
        self.0 as f64 / EMUS_PER_PT
    }
    // Precision loss acceptable for display/measurement purposes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub const fn to_mm(self) -> f64 {
        self.0 as f64 / EMUS_PER_MM
    }
    // Precision loss acceptable for display/measurement purposes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub const fn to_centipoints(self) -> f64 {
        self.0 as f64 / EMUS_PER_CENTIPOINT
    }
    // Precision loss acceptable for display/measurement purposes
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub const fn to_twips(self) -> f64 {
        self.0 as f64 / EMUS_PER_TWIP
    }
}

impl From<Inches> for Emu {
    // EMU values fit in i64 range
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    fn from(val: Inches) -> Self {
        Self((val.0 * EMUS_PER_INCH) as i64)
    }
}

impl From<Cm> for Emu {
    // EMU values fit in i64 range
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    fn from(val: Cm) -> Self {
        Self((val.0 * EMUS_PER_CM) as i64)
    }
}

impl From<Pt> for Emu {
    // EMU values fit in i64 range
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    fn from(val: Pt) -> Self {
        Self((val.0 * EMUS_PER_PT) as i64)
    }
}

impl From<Mm> for Emu {
    // EMU values fit in i64 range
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    fn from(val: Mm) -> Self {
        Self((val.0 * EMUS_PER_MM) as i64)
    }
}

impl From<Emu> for Inches {
    #[inline]
    fn from(val: Emu) -> Self {
        Self(val.to_inches())
    }
}

impl From<Emu> for Cm {
    #[inline]
    fn from(val: Emu) -> Self {
        Self(val.to_cm())
    }
}

impl From<Emu> for Pt {
    #[inline]
    fn from(val: Emu) -> Self {
        Self(val.to_pt())
    }
}

impl From<Emu> for Mm {
    #[inline]
    fn from(val: Emu) -> Self {
        Self(val.to_mm())
    }
}

impl From<Centipoints> for Emu {
    // EMU values fit in i64 range
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    fn from(val: Centipoints) -> Self {
        Self((val.0 * EMUS_PER_CENTIPOINT) as i64)
    }
}

impl From<Emu> for Centipoints {
    #[inline]
    fn from(val: Emu) -> Self {
        Self(val.to_centipoints())
    }
}

impl From<Twips> for Emu {
    // EMU values fit in i64 range
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    fn from(val: Twips) -> Self {
        Self((val.0 * EMUS_PER_TWIP) as i64)
    }
}

impl From<Emu> for Twips {
    #[inline]
    fn from(val: Emu) -> Self {
        Self(val.to_twips())
    }
}

impl From<i64> for Emu {
    #[inline]
    fn from(val: i64) -> Self {
        Self(val)
    }
}

impl From<Emu> for i64 {
    #[inline]
    fn from(val: Emu) -> Self {
        val.0
    }
}

impl std::fmt::Display for Emu {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for Emu {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Emu {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests;

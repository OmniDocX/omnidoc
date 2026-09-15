//! Camera preset type enumeration for `DrawingML` 3D scenes.

/// Camera preset type for 3D scene (`<a:camera prst="...">`).
///
/// Based on the OOXML `ST_PresetCameraType` simple type.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraPreset {
    OrthographicFront,
    IsometricTopUp,
    IsometricTopDown,
    IsometricBottomUp,
    IsometricBottomDown,
    IsometricLeftUp,
    IsometricLeftDown,
    IsometricRightUp,
    IsometricRightDown,
    IsometricOffAxis1Left,
    IsometricOffAxis1Right,
    IsometricOffAxis1Top,
    IsometricOffAxis2Left,
    IsometricOffAxis2Right,
    IsometricOffAxis2Top,
    IsometricOffAxis3Left,
    IsometricOffAxis3Right,
    IsometricOffAxis3Bottom,
    IsometricOffAxis4Left,
    IsometricOffAxis4Right,
    IsometricOffAxis4Bottom,
    ObliqueTopLeft,
    ObliqueTop,
    ObliqueTopRight,
    ObliqueLeft,
    ObliqueRight,
    ObliqueBottomLeft,
    ObliqueBottom,
    ObliqueBottomRight,
    PerspectiveFront,
    PerspectiveLeft,
    PerspectiveRight,
    PerspectiveAbove,
    PerspectiveBelow,
    PerspectiveAboveLeftFacing,
    PerspectiveAboveRightFacing,
    PerspectiveContrastingLeftFacing,
    PerspectiveContrastingRightFacing,
    PerspectiveHeroicLeftFacing,
    PerspectiveHeroicRightFacing,
    PerspectiveHeroicExtremeLeftFacing,
    PerspectiveHeroicExtremeRightFacing,
    PerspectiveRelaxed,
    PerspectiveRelaxedModerately,
    /// Unknown / unrecognised camera preset preserved for round-tripping.
    Other(String),
}

impl CameraPreset {
    /// Return the XML attribute value for this camera preset.
    #[must_use]
    pub fn to_xml_str(&self) -> &str {
        match self {
            Self::OrthographicFront => "orthographicFront",
            Self::IsometricTopUp => "isometricTopUp",
            Self::IsometricTopDown => "isometricTopDown",
            Self::IsometricBottomUp => "isometricBottomUp",
            Self::IsometricBottomDown => "isometricBottomDown",
            Self::IsometricLeftUp => "isometricLeftUp",
            Self::IsometricLeftDown => "isometricLeftDown",
            Self::IsometricRightUp => "isometricRightUp",
            Self::IsometricRightDown => "isometricRightDown",
            Self::IsometricOffAxis1Left => "isometricOffAxis1Left",
            Self::IsometricOffAxis1Right => "isometricOffAxis1Right",
            Self::IsometricOffAxis1Top => "isometricOffAxis1Top",
            Self::IsometricOffAxis2Left => "isometricOffAxis2Left",
            Self::IsometricOffAxis2Right => "isometricOffAxis2Right",
            Self::IsometricOffAxis2Top => "isometricOffAxis2Top",
            Self::IsometricOffAxis3Left => "isometricOffAxis3Left",
            Self::IsometricOffAxis3Right => "isometricOffAxis3Right",
            Self::IsometricOffAxis3Bottom => "isometricOffAxis3Bottom",
            Self::IsometricOffAxis4Left => "isometricOffAxis4Left",
            Self::IsometricOffAxis4Right => "isometricOffAxis4Right",
            Self::IsometricOffAxis4Bottom => "isometricOffAxis4Bottom",
            Self::ObliqueTopLeft => "obliqueTopLeft",
            Self::ObliqueTop => "obliqueTop",
            Self::ObliqueTopRight => "obliqueTopRight",
            Self::ObliqueLeft => "obliqueLeft",
            Self::ObliqueRight => "obliqueRight",
            Self::ObliqueBottomLeft => "obliqueBottomLeft",
            Self::ObliqueBottom => "obliqueBottom",
            Self::ObliqueBottomRight => "obliqueBottomRight",
            Self::PerspectiveFront => "perspectiveFront",
            Self::PerspectiveLeft => "perspectiveLeft",
            Self::PerspectiveRight => "perspectiveRight",
            Self::PerspectiveAbove => "perspectiveAbove",
            Self::PerspectiveBelow => "perspectiveBelow",
            Self::PerspectiveAboveLeftFacing => "perspectiveAboveLeftFacing",
            Self::PerspectiveAboveRightFacing => "perspectiveAboveRightFacing",
            Self::PerspectiveContrastingLeftFacing => "perspectiveContrastingLeftFacing",
            Self::PerspectiveContrastingRightFacing => "perspectiveContrastingRightFacing",
            Self::PerspectiveHeroicLeftFacing => "perspectiveHeroicLeftFacing",
            Self::PerspectiveHeroicRightFacing => "perspectiveHeroicRightFacing",
            Self::PerspectiveHeroicExtremeLeftFacing => "perspectiveHeroicExtremeLeftFacing",
            Self::PerspectiveHeroicExtremeRightFacing => "perspectiveHeroicExtremeRightFacing",
            Self::PerspectiveRelaxed => "perspectiveRelaxed",
            Self::PerspectiveRelaxedModerately => "perspectiveRelaxedModerately",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse an XML camera preset attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Self {
        match s {
            "orthographicFront" => Self::OrthographicFront,
            "isometricTopUp" => Self::IsometricTopUp,
            "isometricTopDown" => Self::IsometricTopDown,
            "isometricBottomUp" => Self::IsometricBottomUp,
            "isometricBottomDown" => Self::IsometricBottomDown,
            "isometricLeftUp" => Self::IsometricLeftUp,
            "isometricLeftDown" => Self::IsometricLeftDown,
            "isometricRightUp" => Self::IsometricRightUp,
            "isometricRightDown" => Self::IsometricRightDown,
            "isometricOffAxis1Left" => Self::IsometricOffAxis1Left,
            "isometricOffAxis1Right" => Self::IsometricOffAxis1Right,
            "isometricOffAxis1Top" => Self::IsometricOffAxis1Top,
            "isometricOffAxis2Left" => Self::IsometricOffAxis2Left,
            "isometricOffAxis2Right" => Self::IsometricOffAxis2Right,
            "isometricOffAxis2Top" => Self::IsometricOffAxis2Top,
            "isometricOffAxis3Left" => Self::IsometricOffAxis3Left,
            "isometricOffAxis3Right" => Self::IsometricOffAxis3Right,
            "isometricOffAxis3Bottom" => Self::IsometricOffAxis3Bottom,
            "isometricOffAxis4Left" => Self::IsometricOffAxis4Left,
            "isometricOffAxis4Right" => Self::IsometricOffAxis4Right,
            "isometricOffAxis4Bottom" => Self::IsometricOffAxis4Bottom,
            "obliqueTopLeft" => Self::ObliqueTopLeft,
            "obliqueTop" => Self::ObliqueTop,
            "obliqueTopRight" => Self::ObliqueTopRight,
            "obliqueLeft" => Self::ObliqueLeft,
            "obliqueRight" => Self::ObliqueRight,
            "obliqueBottomLeft" => Self::ObliqueBottomLeft,
            "obliqueBottom" => Self::ObliqueBottom,
            "obliqueBottomRight" => Self::ObliqueBottomRight,
            "perspectiveFront" => Self::PerspectiveFront,
            "perspectiveLeft" => Self::PerspectiveLeft,
            "perspectiveRight" => Self::PerspectiveRight,
            "perspectiveAbove" => Self::PerspectiveAbove,
            "perspectiveBelow" => Self::PerspectiveBelow,
            "perspectiveAboveLeftFacing" => Self::PerspectiveAboveLeftFacing,
            "perspectiveAboveRightFacing" => Self::PerspectiveAboveRightFacing,
            "perspectiveContrastingLeftFacing" => Self::PerspectiveContrastingLeftFacing,
            "perspectiveContrastingRightFacing" => Self::PerspectiveContrastingRightFacing,
            "perspectiveHeroicLeftFacing" => Self::PerspectiveHeroicLeftFacing,
            "perspectiveHeroicRightFacing" => Self::PerspectiveHeroicRightFacing,
            "perspectiveHeroicExtremeLeftFacing" => Self::PerspectiveHeroicExtremeLeftFacing,
            "perspectiveHeroicExtremeRightFacing" => Self::PerspectiveHeroicExtremeRightFacing,
            "perspectiveRelaxed" => Self::PerspectiveRelaxed,
            "perspectiveRelaxedModerately" => Self::PerspectiveRelaxedModerately,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<&str> for CameraPreset {
    fn from(s: &str) -> Self {
        Self::from_xml_str(s)
    }
}

impl PartialEq<&str> for CameraPreset {
    fn eq(&self, other: &&str) -> bool {
        self.to_xml_str() == *other
    }
}

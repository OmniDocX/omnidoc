//! System color value enumeration for `DrawingML`.

/// Specifies a system color value (OS-defined color).
///
/// Corresponds to the `val` attribute on `<a:sysClr>`.
/// Unknown values are preserved via the `Other` variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemColorVal {
    ScrollBar,
    Background,
    ActiveCaption,
    InactiveCaption,
    Menu,
    Window,
    WindowFrame,
    MenuText,
    WindowText,
    CaptionText,
    ActiveBorder,
    InactiveBorder,
    AppWorkspace,
    Highlight,
    HighlightText,
    BtnFace,
    BtnShadow,
    GrayText,
    BtnText,
    InactiveCaptionText,
    BtnHighlight,
    ThreeDDkShadow,
    ThreeDLight,
    InfoText,
    InfoBk,
    HotLight,
    GradientActiveCaption,
    GradientInactiveCaption,
    MenuHighlight,
    MenuBar,
    /// Unknown / unrecognised system color value preserved for round-tripping.
    Other(String),
}

impl SystemColorVal {
    /// Return the XML attribute value for this system color.
    #[must_use]
    pub fn to_xml_str(&self) -> &str {
        match self {
            Self::ScrollBar => "scrollBar",
            Self::Background => "background",
            Self::ActiveCaption => "activeCaption",
            Self::InactiveCaption => "inactiveCaption",
            Self::Menu => "menu",
            Self::Window => "window",
            Self::WindowFrame => "windowFrame",
            Self::MenuText => "menuText",
            Self::WindowText => "windowText",
            Self::CaptionText => "captionText",
            Self::ActiveBorder => "activeBorder",
            Self::InactiveBorder => "inactiveBorder",
            Self::AppWorkspace => "appWorkspace",
            Self::Highlight => "highlight",
            Self::HighlightText => "highlightText",
            Self::BtnFace => "btnFace",
            Self::BtnShadow => "btnShadow",
            Self::GrayText => "grayText",
            Self::BtnText => "btnText",
            Self::InactiveCaptionText => "inactiveCaptionText",
            Self::BtnHighlight => "btnHighlight",
            Self::ThreeDDkShadow => "3dDkShadow",
            Self::ThreeDLight => "3dLight",
            Self::InfoText => "infoText",
            Self::InfoBk => "infoBk",
            Self::HotLight => "hotLight",
            Self::GradientActiveCaption => "gradientActiveCaption",
            Self::GradientInactiveCaption => "gradientInactiveCaption",
            Self::MenuHighlight => "menuHighlight",
            Self::MenuBar => "menuBar",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse an XML system color attribute value.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Self {
        match s {
            "scrollBar" => Self::ScrollBar,
            "background" => Self::Background,
            "activeCaption" => Self::ActiveCaption,
            "inactiveCaption" => Self::InactiveCaption,
            "menu" => Self::Menu,
            "window" => Self::Window,
            "windowFrame" => Self::WindowFrame,
            "menuText" => Self::MenuText,
            "windowText" => Self::WindowText,
            "captionText" => Self::CaptionText,
            "activeBorder" => Self::ActiveBorder,
            "inactiveBorder" => Self::InactiveBorder,
            "appWorkspace" => Self::AppWorkspace,
            "highlight" => Self::Highlight,
            "highlightText" => Self::HighlightText,
            "btnFace" => Self::BtnFace,
            "btnShadow" => Self::BtnShadow,
            "grayText" => Self::GrayText,
            "btnText" => Self::BtnText,
            "inactiveCaptionText" => Self::InactiveCaptionText,
            "btnHighlight" => Self::BtnHighlight,
            "3dDkShadow" => Self::ThreeDDkShadow,
            "3dLight" => Self::ThreeDLight,
            "infoText" => Self::InfoText,
            "infoBk" => Self::InfoBk,
            "hotLight" => Self::HotLight,
            "gradientActiveCaption" => Self::GradientActiveCaption,
            "gradientInactiveCaption" => Self::GradientInactiveCaption,
            "menuHighlight" => Self::MenuHighlight,
            "menuBar" => Self::MenuBar,
            other => Self::Other(other.to_string()),
        }
    }
}

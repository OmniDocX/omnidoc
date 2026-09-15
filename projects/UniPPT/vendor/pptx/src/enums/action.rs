//! Enumerations for click actions on shapes and text runs.

/// Specifies the type of action to perform when a shape or text run is clicked.
///
/// Alias: `PpAction`
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PpActionType {
    /// No action is performed.
    None,
    /// Navigate to a hyperlink (URL).
    Hyperlink,
    /// Navigate to a named slide.
    NamedSlide,
    /// Navigate to the first slide.
    FirstSlide,
    /// Navigate to the last slide.
    LastSlide,
    /// Navigate to the next slide.
    NextSlide,
    /// Navigate to the previous slide.
    PreviousSlide,
    /// End the slide show.
    EndShow,
    /// Run a macro.
    RunMacro,
    /// Run a program.
    RunProgram,
    /// Navigate to the last slide viewed.
    LastSlideViewed,
    /// Open an OLE verb.
    OleVerb,
}

/// Alias matching the python-pptx `PP_ACTION` name.
pub type PpAction = PpActionType;

impl PpActionType {
    /// Return the XML attribute value for this action type.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Hyperlink | Self::NamedSlide => "hlinkSldjump",
            Self::FirstSlide
            | Self::LastSlide
            | Self::NextSlide
            | Self::PreviousSlide
            | Self::EndShow
            | Self::LastSlideViewed => "hlinkShowJump",
            Self::RunMacro => "macro",
            Self::RunProgram => "program",
            Self::OleVerb => "ole",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_to_xml() {
        assert_eq!(PpActionType::None.to_xml_str(), "none");
        assert_eq!(PpActionType::Hyperlink.to_xml_str(), "hlinkSldjump");
        assert_eq!(PpActionType::FirstSlide.to_xml_str(), "hlinkShowJump");
        assert_eq!(PpActionType::EndShow.to_xml_str(), "hlinkShowJump");
        assert_eq!(PpActionType::RunMacro.to_xml_str(), "macro");
    }
}

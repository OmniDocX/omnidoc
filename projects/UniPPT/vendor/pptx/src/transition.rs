//! Slide transition types and settings.

use crate::xml_util::WriteXml;

/// The type of slide transition effect.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionType {
    /// No transition.
    None,
    /// Fade transition.
    Fade,
    /// Push transition.
    Push,
    /// Wipe transition.
    Wipe,
    /// Split transition.
    Split,
    /// Blinds transition.
    Blinds,
    /// Checker transition.
    Checker,
    /// Dissolve transition.
    Dissolve,
    /// Cover transition.
    Cover,
    /// Cut transition.
    Cut,
    /// Random transition.
    Random,
}

impl TransitionType {
    /// Return the XML element name for this transition type.
    #[must_use]
    pub const fn to_xml_element(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Fade => Some("p:fade"),
            Self::Push => Some("p:push"),
            Self::Wipe => Some("p:wipe"),
            Self::Split => Some("p:split"),
            Self::Blinds => Some("p:blinds"),
            Self::Checker => Some("p:checker"),
            Self::Dissolve => Some("p:dissolve"),
            Self::Cover => Some("p:cover"),
            Self::Cut => Some("p:cut"),
            Self::Random => Some("p:random"),
        }
    }
}

/// Settings for a slide transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideTransition {
    /// The type of transition effect.
    pub transition_type: TransitionType,
    /// Duration of the transition in milliseconds (maps to `spd` or `dur` attribute).
    pub duration: Option<u32>,
    /// Whether the slide advances on mouse click.
    pub advance_on_click: bool,
    /// Auto-advance after this many milliseconds (maps to `advTm` attribute).
    pub advance_after_time: Option<u32>,
}

impl SlideTransition {
    /// Create a new slide transition with the given type.
    #[must_use]
    pub const fn new(transition_type: TransitionType) -> Self {
        Self {
            transition_type,
            duration: None,
            advance_on_click: true,
            advance_after_time: None,
        }
    }

    /// Create a fade transition.
    #[must_use]
    pub const fn fade() -> Self {
        Self::new(TransitionType::Fade)
    }

    /// Create a push transition.
    #[must_use]
    pub const fn push() -> Self {
        Self::new(TransitionType::Push)
    }

    /// Create a wipe transition.
    #[must_use]
    pub const fn wipe() -> Self {
        Self::new(TransitionType::Wipe)
    }

    /// Set the transition duration in milliseconds.
    #[must_use]
    pub const fn with_duration(mut self, ms: u32) -> Self {
        self.duration = Some(ms);
        self
    }

    /// Set auto-advance time in milliseconds.
    #[must_use]
    pub const fn with_advance_after(mut self, ms: u32) -> Self {
        self.advance_after_time = Some(ms);
        self
    }

    /// Disable advance on click.
    #[must_use]
    pub const fn without_click_advance(mut self) -> Self {
        self.advance_on_click = false;
        self
    }
}

impl WriteXml for SlideTransition {
    fn write_xml<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str("<p:transition")?;

        if let Some(dur) = self.duration {
            if dur >= 2000 {
                w.write_str(r#" spd="slow""#)?;
            } else if dur >= 500 {
                w.write_str(r#" spd="med""#)?;
            } else {
                w.write_str(r#" spd="fast""#)?;
            }
        }

        if !self.advance_on_click {
            w.write_str(r#" advClick="0""#)?;
        }

        if let Some(adv_tm) = self.advance_after_time {
            write!(w, r#" advTm="{adv_tm}""#)?;
        }

        if let Some(elem) = self.transition_type.to_xml_element() {
            w.write_char('>')?;
            write!(w, "<{elem}/>")?;
            w.write_str("</p:transition>")
        } else {
            w.write_str("/>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_transition_xml() {
        let t = SlideTransition::fade();
        let xml = t.to_xml_string();
        assert!(xml.contains("<p:transition"));
        assert!(xml.contains("<p:fade/>"));
        assert!(xml.contains("</p:transition>"));
    }

    #[test]
    fn test_push_transition_xml() {
        let t = SlideTransition::push();
        let xml = t.to_xml_string();
        assert!(xml.contains("<p:push/>"));
    }

    #[test]
    fn test_wipe_transition_xml() {
        let t = SlideTransition::wipe();
        let xml = t.to_xml_string();
        assert!(xml.contains("<p:wipe/>"));
    }

    #[test]
    fn test_none_transition_xml() {
        let t = SlideTransition::new(TransitionType::None);
        let xml = t.to_xml_string();
        assert!(xml.contains("<p:transition"));
        assert!(xml.ends_with("/>"));
        assert!(!xml.contains("</p:transition>"));
    }

    #[test]
    fn test_transition_with_duration() {
        let t = SlideTransition::fade().with_duration(3000);
        let xml = t.to_xml_string();
        assert!(xml.contains(r#"spd="slow""#));
    }

    #[test]
    fn test_transition_fast_duration() {
        let t = SlideTransition::fade().with_duration(200);
        let xml = t.to_xml_string();
        assert!(xml.contains(r#"spd="fast""#));
    }

    #[test]
    fn test_transition_no_click_advance() {
        let t = SlideTransition::fade().without_click_advance();
        let xml = t.to_xml_string();
        assert!(xml.contains(r#"advClick="0""#));
    }

    #[test]
    fn test_transition_auto_advance() {
        let t = SlideTransition::fade().with_advance_after(5000);
        let xml = t.to_xml_string();
        assert!(xml.contains(r#"advTm="5000""#));
    }

    #[test]
    fn test_transition_all_options() {
        let t = SlideTransition::wipe()
            .with_duration(1000)
            .with_advance_after(3000)
            .without_click_advance();
        let xml = t.to_xml_string();
        assert!(xml.contains("<p:wipe/>"));
        assert!(xml.contains(r#"spd="med""#));
        assert!(xml.contains(r#"advClick="0""#));
        assert!(xml.contains(r#"advTm="3000""#));
    }

    #[test]
    fn test_transition_type_xml_elements() {
        assert_eq!(TransitionType::Fade.to_xml_element(), Some("p:fade"));
        assert_eq!(TransitionType::Push.to_xml_element(), Some("p:push"));
        assert_eq!(
            TransitionType::Dissolve.to_xml_element(),
            Some("p:dissolve")
        );
        assert_eq!(TransitionType::None.to_xml_element(), None);
    }
}

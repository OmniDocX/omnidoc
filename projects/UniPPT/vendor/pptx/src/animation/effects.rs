//! Animation effect type enums and trigger types.

// ---------------------------------------------------------------------------
// Entrance types
// ---------------------------------------------------------------------------

/// Preset entrance animation effects.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntranceType {
    Appear,
    Fade,
    FlyIn,
    Wipe,
    Split,
    Wheel,
    RandomBars,
    GrowAndTurn,
    Zoom,
    Bounce,
}

impl EntranceType {
    /// OOXML `presetID` for this entrance type.
    #[must_use]
    pub const fn preset_id(self) -> u32 {
        match self {
            Self::Appear => 1,
            Self::Fade => 10,
            Self::FlyIn => 2,
            Self::Wipe => 22,
            Self::Split => 16,
            Self::Wheel => 21,
            Self::RandomBars => 14,
            Self::GrowAndTurn => 15,
            Self::Zoom => 23,
            Self::Bounce => 24,
        }
    }
}

// ---------------------------------------------------------------------------
// Exit types
// ---------------------------------------------------------------------------

/// Preset exit animation effects.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitType {
    Disappear,
    Fade,
    FlyOut,
    Wipe,
    Split,
}

impl ExitType {
    /// OOXML `presetID` for this exit type.
    #[must_use]
    pub const fn preset_id(self) -> u32 {
        match self {
            Self::Disappear => 1,
            Self::Fade => 10,
            Self::FlyOut => 2,
            Self::Wipe => 22,
            Self::Split => 16,
        }
    }
}

// ---------------------------------------------------------------------------
// Emphasis types
// ---------------------------------------------------------------------------

/// Preset emphasis animation effects.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmphasisType {
    Bold,
    Grow,
    Spin,
    Transparency,
    Pulse,
    Teeter,
}

impl EmphasisType {
    /// OOXML `presetID` for this emphasis type.
    #[must_use]
    pub const fn preset_id(self) -> u32 {
        match self {
            Self::Bold => 1,
            Self::Grow => 6,
            Self::Spin => 8,
            Self::Transparency => 9,
            Self::Pulse => 10,
            Self::Teeter => 13,
        }
    }
}

// ---------------------------------------------------------------------------
// AnimationEffect
// ---------------------------------------------------------------------------

/// A single animation effect.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationEffect {
    Entrance(EntranceType),
    Exit(ExitType),
    Emphasis(EmphasisType),
    /// Custom motion path (the string is a VML-style path, e.g. `"M 0 0 L 1 1 E"`).
    MotionPath(String),
}

impl AnimationEffect {
    /// OOXML `presetClass` value.
    pub(super) const fn preset_class(&self) -> &'static str {
        match self {
            Self::Entrance(_) => "entr",
            Self::Exit(_) => "exit",
            Self::Emphasis(_) => "emph",
            Self::MotionPath(_) => "path",
        }
    }

    /// OOXML `presetID` value.
    pub(super) const fn preset_id(&self) -> u32 {
        match self {
            Self::Entrance(e) => e.preset_id(),
            Self::Exit(e) => e.preset_id(),
            Self::Emphasis(e) => e.preset_id(),
            Self::MotionPath(_) => 0, // custom path
        }
    }
}

// ---------------------------------------------------------------------------
// AnimationTrigger
// ---------------------------------------------------------------------------

/// How an animation is triggered relative to other animations.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationTrigger {
    /// Start on mouse click (default).
    OnClick,
    /// Start at the same time as the previous animation.
    WithPrevious,
    /// Start after the previous animation finishes.
    AfterPrevious,
}

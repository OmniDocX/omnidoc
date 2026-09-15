//! Slide animation types and settings.
//!
//! Provides basic support for `PowerPoint` slide animations (entrance, exit,
//! emphasis, and motion-path effects). Animations are serialised into the
//! `<p:timing>` element that sits at the end of every slide XML.

mod effects;
mod xml_gen;

pub use effects::{AnimationEffect, AnimationTrigger, EmphasisType, EntranceType, ExitType};

use crate::units::{DurationMs, ShapeId};

// ---------------------------------------------------------------------------
// SlideAnimation
// ---------------------------------------------------------------------------

/// A single animation applied to one shape on a slide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideAnimation {
    /// The `spid` of the target shape (OOXML shape identifier).
    pub target_shape_id: ShapeId,
    /// The animation effect to apply.
    pub effect: AnimationEffect,
    /// When the animation starts.
    pub trigger: AnimationTrigger,
    /// Duration in milliseconds (default 500).
    pub duration_ms: DurationMs,
    /// Delay before the animation starts, in milliseconds (default 0).
    pub delay_ms: DurationMs,
}

impl SlideAnimation {
    /// Create a new animation with sensible defaults.
    #[must_use]
    pub const fn new(target_shape_id: ShapeId, effect: AnimationEffect) -> Self {
        Self {
            target_shape_id,
            effect,
            trigger: AnimationTrigger::OnClick,
            duration_ms: DurationMs(500),
            delay_ms: DurationMs(0),
        }
    }

    /// Set the trigger type.
    #[must_use]
    pub const fn with_trigger(mut self, trigger: AnimationTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Set the duration in milliseconds.
    #[must_use]
    pub const fn with_duration(mut self, ms: DurationMs) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set the delay in milliseconds.
    #[must_use]
    pub const fn with_delay(mut self, ms: DurationMs) -> Self {
        self.delay_ms = ms;
        self
    }
}

// ---------------------------------------------------------------------------
// AnimationSequence
// ---------------------------------------------------------------------------

/// An ordered sequence of animations for a slide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationSequence {
    pub(crate) animations: Vec<SlideAnimation>,
}

impl AnimationSequence {
    /// Create an empty animation sequence.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            animations: Vec::new(),
        }
    }

    /// Append an animation to the sequence.
    pub fn add(&mut self, animation: SlideAnimation) {
        self.animations.push(animation);
    }

    /// Number of animations in the sequence.
    #[must_use]
    pub fn len(&self) -> usize {
        self.animations.len()
    }

    /// Returns `true` if the sequence contains no animations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }
}

/// Creates an empty animation sequence with no effects.
impl Default for AnimationSequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

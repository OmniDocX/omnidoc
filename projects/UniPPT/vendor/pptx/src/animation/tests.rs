use super::*;
use crate::units::{DurationMs, ShapeId};

#[test]
fn test_empty_sequence_returns_empty_string() {
    let seq = AnimationSequence::new();
    assert!(seq.is_empty());
    assert_eq!(seq.len(), 0);
    assert_eq!(seq.to_xml_string(), "");
}

#[test]
fn test_single_entrance_appear() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(4),
        AnimationEffect::Entrance(EntranceType::Appear),
    ));
    assert_eq!(seq.len(), 1);

    let xml = seq.to_xml_string();
    assert!(xml.starts_with("<p:timing>"));
    assert!(xml.ends_with("</p:timing>"));
    assert!(xml.contains("presetID=\"1\""));
    assert!(xml.contains("presetClass=\"entr\""));
    assert!(xml.contains("spid=\"4\""));
    assert!(xml.contains("nodeType=\"tmRoot\""));
    assert!(xml.contains("nodeType=\"mainSeq\""));
    assert!(xml.contains("nodeType=\"clickEffect\""));
    assert!(xml.contains("<p:strVal val=\"visible\"/>"));
}

#[test]
fn test_exit_effect() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(7),
        AnimationEffect::Exit(ExitType::Fade),
    ));

    let xml = seq.to_xml_string();
    assert!(xml.contains("presetClass=\"exit\""));
    assert!(xml.contains("presetID=\"10\""));
    assert!(xml.contains("spid=\"7\""));
    assert!(xml.contains("<p:strVal val=\"hidden\"/>"));
}

#[test]
fn test_emphasis_effect() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(3),
        AnimationEffect::Emphasis(EmphasisType::Spin),
    ));

    let xml = seq.to_xml_string();
    assert!(xml.contains("presetClass=\"emph\""));
    assert!(xml.contains("presetID=\"8\""));
    assert!(xml.contains("spid=\"3\""));
    assert!(xml.contains("<p:animEffect"));
}

#[test]
fn test_motion_path_effect() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(5),
        AnimationEffect::MotionPath("M 0 0 L 1 1 E".to_string()),
    ));

    let xml = seq.to_xml_string();
    assert!(xml.contains("presetClass=\"path\""));
    assert!(xml.contains("spid=\"5\""));
    assert!(xml.contains("M 0 0 L 1 1 E"));
    assert!(xml.contains("<p:anim"));
}

#[test]
fn test_with_trigger() {
    let anim = SlideAnimation::new(ShapeId(4), AnimationEffect::Entrance(EntranceType::Fade))
        .with_trigger(AnimationTrigger::WithPrevious);
    assert_eq!(anim.trigger, AnimationTrigger::WithPrevious);

    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(4),
        AnimationEffect::Entrance(EntranceType::Appear),
    ));
    seq.add(
        SlideAnimation::new(ShapeId(5), AnimationEffect::Entrance(EntranceType::Fade))
            .with_trigger(AnimationTrigger::WithPrevious),
    );

    let xml = seq.to_xml_string();
    assert!(xml.contains("nodeType=\"withEffect\""));
}

#[test]
fn test_after_previous_trigger() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(4),
        AnimationEffect::Entrance(EntranceType::Appear),
    ));
    seq.add(
        SlideAnimation::new(ShapeId(5), AnimationEffect::Entrance(EntranceType::Fade))
            .with_trigger(AnimationTrigger::AfterPrevious),
    );

    let xml = seq.to_xml_string();
    assert!(xml.contains("nodeType=\"afterEffect\""));
}

#[test]
fn test_custom_duration_and_delay() {
    let mut seq = AnimationSequence::new();
    seq.add(
        SlideAnimation::new(ShapeId(4), AnimationEffect::Entrance(EntranceType::Fade))
            .with_duration(DurationMs(1000))
            .with_delay(DurationMs(250)),
    );

    let xml = seq.to_xml_string();
    assert!(xml.contains("dur=\"1000\""));
    assert!(xml.contains("delay=\"250\""));
}

#[test]
fn test_multiple_click_groups() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(4),
        AnimationEffect::Entrance(EntranceType::Appear),
    ));
    seq.add(SlideAnimation::new(
        ShapeId(5),
        AnimationEffect::Entrance(EntranceType::Fade),
    ));

    let xml = seq.to_xml_string();
    // Each OnClick animation creates its own click group, so we should
    // see multiple `fill="hold"` wrappers.
    let click_effect_count = xml.matches("nodeType=\"clickEffect\"").count();
    assert_eq!(click_effect_count, 2);
}

#[test]
fn test_entrance_preset_ids() {
    assert_eq!(EntranceType::Appear.preset_id(), 1);
    assert_eq!(EntranceType::Fade.preset_id(), 10);
    assert_eq!(EntranceType::FlyIn.preset_id(), 2);
    assert_eq!(EntranceType::Wipe.preset_id(), 22);
    assert_eq!(EntranceType::Split.preset_id(), 16);
    assert_eq!(EntranceType::Wheel.preset_id(), 21);
    assert_eq!(EntranceType::RandomBars.preset_id(), 14);
    assert_eq!(EntranceType::GrowAndTurn.preset_id(), 15);
    assert_eq!(EntranceType::Zoom.preset_id(), 23);
    assert_eq!(EntranceType::Bounce.preset_id(), 24);
}

#[test]
fn test_exit_preset_ids() {
    assert_eq!(ExitType::Disappear.preset_id(), 1);
    assert_eq!(ExitType::Fade.preset_id(), 10);
    assert_eq!(ExitType::FlyOut.preset_id(), 2);
    assert_eq!(ExitType::Wipe.preset_id(), 22);
    assert_eq!(ExitType::Split.preset_id(), 16);
}

#[test]
fn test_emphasis_preset_ids() {
    assert_eq!(EmphasisType::Bold.preset_id(), 1);
    assert_eq!(EmphasisType::Grow.preset_id(), 6);
    assert_eq!(EmphasisType::Spin.preset_id(), 8);
    assert_eq!(EmphasisType::Transparency.preset_id(), 9);
    assert_eq!(EmphasisType::Pulse.preset_id(), 10);
    assert_eq!(EmphasisType::Teeter.preset_id(), 13);
}

#[test]
fn test_default_animation_sequence() {
    let seq = AnimationSequence::default();
    assert!(seq.is_empty());
}

#[test]
fn test_slide_animation_defaults() {
    let anim = SlideAnimation::new(ShapeId(4), AnimationEffect::Entrance(EntranceType::Appear));
    assert_eq!(anim.target_shape_id, ShapeId(4));
    assert_eq!(anim.trigger, AnimationTrigger::OnClick);
    assert_eq!(anim.duration_ms, DurationMs(500));
    assert_eq!(anim.delay_ms, DurationMs(0));
}

#[test]
fn test_prev_next_cond_lists() {
    let mut seq = AnimationSequence::new();
    seq.add(SlideAnimation::new(
        ShapeId(4),
        AnimationEffect::Entrance(EntranceType::Appear),
    ));

    let xml = seq.to_xml_string();
    assert!(xml.contains("<p:prevCondLst>"));
    assert!(xml.contains("<p:nextCondLst>"));
    assert!(xml.contains("<p:sldTgt/>"));
}

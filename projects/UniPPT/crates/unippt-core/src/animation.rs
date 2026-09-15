use std::borrow::Cow;
use std::collections::HashSet;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::model::{
    AnimationClass, AnimationEffect, AnimationKind, AnimationPropertyAnimation,
    AnimationPropertyKeyframe, AnimationTrigger, SceneObject, SlideTransition,
};

#[derive(Debug)]
struct TimingFrame {
    pending: Option<PendingEffect>,
    source_timing_id: Option<u32>,
    node_type: Option<String>,
    start_delay_ms: Option<u64>,
    start_delay_indefinite: bool,
    starts_on_begin: bool,
    starts_on_click: bool,
}

#[derive(Debug)]
struct PendingEffect {
    source_timing_id: Option<u32>,
    target_shape_id: Option<u32>,
    effect: AnimationKind,
    effect_priority: u8,
    class: AnimationClass,
    trigger: AnimationTrigger,
    duration_ms: Option<u64>,
    delay_ms: Option<u64>,
    acceleration: Option<u32>,
    deceleration: Option<u32>,
    speed: Option<i32>,
    time_filter: Option<String>,
    repeat_count: Option<String>,
    repeat_duration_ms: Option<u64>,
    auto_reverse: bool,
    preset_id: Option<u32>,
    preset_subtype: Option<u32>,
    direction: Option<String>,
    motion_path: Option<String>,
    fade_filter: Option<String>,
    property_animations: Vec<AnimationPropertyAnimation>,
    media_action: Option<String>,
    /// Absolute offset within the current main-sequence activation group.
    /// Some producers (including the 9.pptx corpus) store small idle gaps on
    /// wrapper cTn nodes instead of the effect node itself.
    start_hint_ms: Option<u64>,
    /// A click can be encoded on an ancestor activation group instead of as a
    /// `clickEffect` node.  The first effect in that group becomes OnClick;
    /// the remaining effects retain their with/after relationship.
    click_group_id: Option<u32>,
}

#[derive(Debug)]
struct PendingPropertyAnimation {
    behavior: AnimationPropertyAnimation,
    current_keyframe: Option<AnimationPropertyKeyframe>,
}

pub(crate) struct ParsedSlideTiming {
    pub animations: Vec<AnimationEffect>,
    pub transition: Option<SlideTransition>,
    pub source_timing_xml: Option<String>,
    pub source_transition_xml: Option<String>,
}

pub(crate) fn parse_slide_timing(slide_xml: &[u8], objects: &[SceneObject]) -> ParsedSlideTiming {
    let source_timing_xml = extract_xml_element(slide_xml, b"p:timing")
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
    let source_transition_xml = extract_xml_element(slide_xml, b"p:transition")
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
    let mut animations = source_timing_xml
        .as_deref()
        .map(parse_timing_fragment)
        .unwrap_or_default();
    for (order, animation) in animations.iter_mut().enumerate() {
        animation.order = order as u32;
        animation.target_object_id = animation
            .target_shape_id
            .and_then(|shape_id| find_object_id(objects, shape_id));
    }
    let transition = source_transition_xml
        .as_deref()
        .and_then(parse_transition_fragment)
        .or_else(|| {
            extract_fallback_transition_xml(slide_xml)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(parse_transition_fragment)
        });
    ParsedSlideTiming {
        animations,
        transition,
        source_timing_xml,
        source_transition_xml,
    }
}

fn parse_timing_fragment(xml: &str) -> Vec<AnimationEffect> {
    let mut reader = Reader::from_reader(xml.as_bytes());
    reader.config_mut().trim_text(true);
    let mut stack: Vec<TimingFrame> = Vec::new();
    let mut element_stack: Vec<Vec<u8>> = Vec::new();
    let mut effects = Vec::new();
    let mut property_animation: Option<PendingPropertyAnimation> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"cTn" {
                    stack.push(TimingFrame {
                        pending: pending_from_ctn(&element),
                        source_timing_id: attr_u32(&element, b"id"),
                        node_type: attr(&element, b"nodeType"),
                        start_delay_ms: None,
                        start_delay_indefinite: false,
                        starts_on_begin: false,
                        starts_on_click: false,
                    });
                    update_ancestor_duration(&mut stack, &element);
                    update_property_animation_ctn(&mut property_animation, &element);
                } else {
                    update_property_animation_start(&mut property_animation, &local, &element);
                    update_pending(&mut stack, &element_stack, &local, &element);
                }
                element_stack.push(local);
            }
            Ok(Event::Empty(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"cTn" {
                    update_ancestor_duration(&mut stack, &element);
                    update_property_animation_ctn(&mut property_animation, &element);
                    let mut frame = TimingFrame {
                        pending: pending_from_ctn(&element),
                        source_timing_id: attr_u32(&element, b"id"),
                        node_type: attr(&element, b"nodeType"),
                        start_delay_ms: None,
                        start_delay_indefinite: false,
                        starts_on_begin: false,
                        starts_on_click: false,
                    };
                    if let Some(mut pending) = frame.pending.take() {
                        apply_activation_context(&mut pending, &frame, &stack);
                        effects.push(pending);
                    }
                } else {
                    update_property_animation_start(&mut property_animation, &local, &element);
                    update_pending(&mut stack, &element_stack, &local, &element);
                }
            }
            Ok(Event::Text(text)) => {
                if element_stack.last().map(Vec::as_slice) == Some(&b"attrName"[..]) {
                    if let (Some(property), Ok(value)) =
                        (property_animation.as_mut(), text.decode())
                    {
                        let value = value.trim();
                        if !value.is_empty() {
                            property.behavior.attributes.push(value.to_string());
                        }
                    }
                }
            }
            Ok(Event::End(element)) if local_name(element.name().as_ref()) == b"cTn" => {
                if let Some(mut frame) = stack.pop() {
                    if let Some(mut pending) = frame.pending.take() {
                        apply_activation_context(&mut pending, &frame, &stack);
                        effects.push(pending);
                    }
                }
                element_stack.pop();
            }
            Ok(Event::End(element)) => {
                match local_name(element.name().as_ref()) {
                    b"tav" => finish_property_keyframe(&mut property_animation),
                    b"anim" => {
                        if let (Some(property), Some(pending)) =
                            (property_animation.take(), nearest_pending(&mut stack))
                        {
                            if !property.behavior.attributes.is_empty() {
                                pending.property_animations.push(property.behavior);
                            }
                        }
                    }
                    _ => {}
                }
                element_stack.pop();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    materialize_effects(effects)
}

fn pending_from_ctn(element: &BytesStart<'_>) -> Option<PendingEffect> {
    let node_type = attr(element, b"nodeType");
    let preset_class = attr(element, b"presetClass");
    let preset_id = attr_u32(element, b"presetID");
    let is_effect = preset_class.is_some()
        || matches!(
            node_type.as_deref(),
            Some("clickEffect" | "withEffect" | "afterEffect")
        );
    if !is_effect {
        return None;
    }
    let class = match preset_class.as_deref() {
        Some("entr") => AnimationClass::Entrance,
        Some("exit") => AnimationClass::Exit,
        Some("emph") => AnimationClass::Emphasis,
        Some("path") => AnimationClass::MotionPath,
        Some("mediacall") => AnimationClass::Media,
        _ => AnimationClass::Custom,
    };
    let trigger = match node_type.as_deref() {
        Some("withEffect") => AnimationTrigger::WithPrevious,
        Some("afterEffect") => AnimationTrigger::AfterPrevious,
        _ => AnimationTrigger::OnClick,
    };
    Some(PendingEffect {
        source_timing_id: attr_u32(element, b"id"),
        target_shape_id: None,
        effect: preset_default_effect(preset_id, class),
        effect_priority: 0,
        class,
        trigger,
        duration_ms: parse_time(attr(element, b"dur").as_deref()),
        delay_ms: None,
        acceleration: attr_u32(element, b"accel").map(|value| value.min(100_000)),
        deceleration: attr_u32(element, b"decel").map(|value| value.min(100_000)),
        speed: attr_i32(element, b"spd"),
        time_filter: attr(element, b"tmFilter"),
        repeat_count: attr(element, b"repeatCount"),
        repeat_duration_ms: parse_time(attr(element, b"repeatDur").as_deref()),
        auto_reverse: matches!(attr(element, b"autoRev").as_deref(), Some("1" | "true")),
        preset_id,
        preset_subtype: attr_u32(element, b"presetSubtype"),
        direction: None,
        motion_path: None,
        fade_filter: None,
        property_animations: vec![],
        media_action: None,
        start_hint_ms: None,
        click_group_id: None,
    })
}

fn update_property_animation_start(
    pending: &mut Option<PendingPropertyAnimation>,
    local: &[u8],
    element: &BytesStart<'_>,
) {
    match local {
        b"anim" => {
            *pending = Some(PendingPropertyAnimation {
                behavior: AnimationPropertyAnimation {
                    attributes: vec![],
                    calculation_mode: attr(element, b"calcmode"),
                    value_type: attr(element, b"valueType"),
                    additive: None,
                    bounce_end: attr_u32(element, b"bounceEnd"),
                    from: attr(element, b"from"),
                    to: attr(element, b"to"),
                    by: attr(element, b"by"),
                    duration_ms: None,
                    fill: None,
                    keyframes: vec![],
                },
                current_keyframe: None,
            });
        }
        b"cBhvr" => {
            if let Some(property) = pending.as_mut() {
                property.behavior.additive = attr(element, b"additive");
            }
        }
        b"cTn" => update_property_animation_ctn(pending, element),
        b"tav" => {
            if let Some(property) = pending.as_mut() {
                property.current_keyframe = Some(AnimationPropertyKeyframe {
                    time: attr_u32(element, b"tm").unwrap_or(0).min(100_000),
                    value: String::new(),
                    formula: attr(element, b"fmla"),
                });
            }
        }
        b"fltVal" | b"intVal" | b"strVal" | b"boolVal" => {
            if let Some(keyframe) = pending
                .as_mut()
                .and_then(|property| property.current_keyframe.as_mut())
            {
                if let Some(value) = attr(element, b"val") {
                    keyframe.value = value;
                }
            }
        }
        _ => {}
    }
}

fn update_property_animation_ctn(
    pending: &mut Option<PendingPropertyAnimation>,
    element: &BytesStart<'_>,
) {
    if let Some(property) = pending.as_mut() {
        property.behavior.duration_ms = parse_time(attr(element, b"dur").as_deref());
        property.behavior.fill = attr(element, b"fill");
    }
}

fn finish_property_keyframe(pending: &mut Option<PendingPropertyAnimation>) {
    let Some(property) = pending.as_mut() else {
        return;
    };
    if let Some(keyframe) = property.current_keyframe.take() {
        if !keyframe.value.is_empty() || keyframe.formula.is_some() {
            property.behavior.keyframes.push(keyframe);
        }
    }
}

fn update_ancestor_duration(stack: &mut [TimingFrame], element: &BytesStart<'_>) {
    let Some(duration) = parse_time(attr(element, b"dur").as_deref()) else {
        return;
    };
    if let Some(pending) = nearest_pending(stack) {
        pending.duration_ms = Some(pending.duration_ms.unwrap_or(0).max(duration));
    }
}

fn update_pending(
    stack: &mut [TimingFrame],
    element_stack: &[Vec<u8>],
    local: &[u8],
    element: &BytesStart<'_>,
) {
    if local == b"cond" && element_stack.last().map(Vec::as_slice) == Some(&b"stCondLst"[..]) {
        if let Some(frame) = stack.last_mut() {
            match attr(element, b"delay").as_deref() {
                Some("indefinite") => frame.start_delay_indefinite = true,
                delay => {
                    if frame.start_delay_ms.is_none() {
                        frame.start_delay_ms = parse_time(delay);
                    }
                }
            }
            match attr(element, b"evt").as_deref() {
                Some("onBegin") => frame.starts_on_begin = true,
                Some("onClick") => frame.starts_on_click = true,
                _ => {}
            }
        }
        return;
    }
    let Some(pending) = nearest_pending(stack) else {
        return;
    };
    match local {
        b"spTgt" => {
            pending.target_shape_id = pending
                .target_shape_id
                .or_else(|| attr_u32(element, b"spid"));
        }
        b"set" => set_effect(pending, AnimationKind::Appear, 1),
        b"animEffect" => {
            let filter = attr(element, b"filter").unwrap_or_default();
            let kind = effect_from_filter(&filter);
            set_effect(pending, kind, 3);
            pending.direction = direction_from_filter(&filter).or_else(|| attr(element, b"dir"));
            if filter_lists_fade(&filter) {
                pending.fade_filter = Some(
                    fade_transition(attr(element, b"transition").as_deref(), pending.class).into(),
                );
            }
        }
        b"animMotion" => {
            set_effect(pending, AnimationKind::MotionPath, 4);
            pending.class = AnimationClass::MotionPath;
            pending.motion_path = attr(element, b"path");
        }
        b"animRot" => set_effect(pending, AnimationKind::Spin, 4),
        b"animScale" => set_effect(pending, AnimationKind::GrowShrink, 4),
        b"cmd" => {
            set_effect(pending, AnimationKind::Media, 5);
            pending.class = AnimationClass::Media;
            if let Some(command) = attr(element, b"cmd") {
                pending.media_action = Some(normalize_media_action(&command).into());
            } else {
                pending.media_action.get_or_insert_with(|| "play".into());
            }
        }
        b"audio" | b"video" => {
            set_effect(pending, AnimationKind::Media, 5);
            pending.class = AnimationClass::Media;
            pending.media_action.get_or_insert_with(|| "play".into());
        }
        _ => {}
    }
}

fn apply_activation_context(
    pending: &mut PendingEffect,
    frame: &TimingFrame,
    ancestors: &[TimingFrame],
) {
    pending.delay_ms = frame.start_delay_ms;
    if frame.starts_on_click {
        pending.trigger = AnimationTrigger::OnClick;
    }

    let has_main_sequence = ancestors
        .iter()
        .any(|ancestor| ancestor.node_type.as_deref() == Some("mainSeq"));
    let click_group_index = ancestors
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, ancestor)| {
            (ancestor.starts_on_click
                || (has_main_sequence
                    && ancestor.start_delay_indefinite
                    && !ancestor.starts_on_begin
                    && ancestor.node_type.as_deref() != Some("mainSeq")))
            .then_some(index)
        });
    pending.click_group_id = click_group_index.and_then(|index| ancestors[index].source_timing_id);

    let first_relevant = click_group_index.unwrap_or(0);
    let mut saw_numeric_start = frame.start_delay_ms.is_some();
    let mut start_hint_ms = frame.start_delay_ms.unwrap_or(0);
    for ancestor in &ancestors[first_relevant..] {
        if ancestor.pending.is_none() {
            if let Some(delay) = ancestor.start_delay_ms {
                saw_numeric_start = true;
                start_hint_ms = start_hint_ms.saturating_add(delay);
            }
        }
    }
    pending.start_hint_ms = saw_numeric_start.then_some(start_hint_ms);
}

fn nearest_pending(stack: &mut [TimingFrame]) -> Option<&mut PendingEffect> {
    stack
        .iter_mut()
        .rev()
        .find_map(|frame| frame.pending.as_mut())
}

fn set_effect(pending: &mut PendingEffect, effect: AnimationKind, priority: u8) {
    if priority >= pending.effect_priority {
        pending.effect = effect;
        pending.effect_priority = priority;
    }
}

/// True when the native `p:animEffect` filter list carries the `fade` token
/// (for example `fade` or `fade;wipe(left)`), which PowerPoint plays as an
/// opacity ramp alongside the primary behavior.
fn filter_lists_fade(filter: &str) -> bool {
    filter.split(';').any(|segment| {
        segment
            .split('(')
            .next()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case("fade"))
    })
}

fn fade_transition(transition: Option<&str>, class: AnimationClass) -> &'static str {
    match transition {
        Some("in") => "in",
        Some("out") => "out",
        _ => {
            if class == AnimationClass::Exit {
                "out"
            } else {
                "in"
            }
        }
    }
}

fn materialize_effects(pending_effects: Vec<PendingEffect>) -> Vec<AnimationEffect> {
    let mut output = Vec::with_capacity(pending_effects.len());
    let mut seen_click_groups = HashSet::new();
    let mut group_start_ms = 0u64;
    let mut group_end_ms = 0u64;

    for mut pending in pending_effects {
        if pending
            .click_group_id
            .is_some_and(|group_id| seen_click_groups.insert(group_id))
        {
            pending.trigger = AnimationTrigger::OnClick;
        }
        let first_in_batch = output.is_empty() || pending.trigger == AnimationTrigger::OnClick;
        let source_delay = pending.delay_ms.unwrap_or(0);
        let delay_ms = if first_in_batch {
            pending.start_hint_ms.unwrap_or(source_delay)
        } else {
            match pending.trigger {
                AnimationTrigger::WithPrevious => pending
                    .start_hint_ms
                    .map(|start| start.saturating_sub(group_start_ms))
                    .unwrap_or(source_delay),
                AnimationTrigger::AfterPrevious => pending
                    .start_hint_ms
                    .map(|start| start.saturating_sub(group_end_ms))
                    .unwrap_or(source_delay),
                AnimationTrigger::OnClick => unreachable!(),
            }
        };
        let start_ms = if first_in_batch {
            delay_ms
        } else {
            match pending.trigger {
                AnimationTrigger::WithPrevious => group_start_ms.saturating_add(delay_ms),
                AnimationTrigger::AfterPrevious => group_end_ms.saturating_add(delay_ms),
                AnimationTrigger::OnClick => unreachable!(),
            }
        };
        if first_in_batch || pending.trigger == AnimationTrigger::AfterPrevious {
            group_start_ms = start_ms;
            group_end_ms = start_ms;
        }
        let duration_ms = pending.duration_ms.unwrap_or(500).max(1);
        group_end_ms = group_end_ms.max(start_ms.saturating_add(duration_ms));

        output.push(AnimationEffect {
            id: format!(
                "anim-{}",
                pending
                    .source_timing_id
                    .map_or_else(|| output.len().to_string(), |id| id.to_string())
            ),
            source_timing_id: pending.source_timing_id,
            target_object_id: None,
            target_shape_id: pending.target_shape_id,
            effect: pending.effect,
            class: pending.class,
            trigger: pending.trigger,
            duration_ms,
            delay_ms,
            acceleration: pending.acceleration,
            deceleration: pending.deceleration,
            speed: pending.speed,
            time_filter: pending.time_filter,
            repeat_count: pending.repeat_count,
            repeat_duration_ms: pending.repeat_duration_ms,
            auto_reverse: pending.auto_reverse,
            order: output.len() as u32,
            preset_id: pending.preset_id,
            preset_subtype: pending.preset_subtype,
            direction: pending.direction,
            motion_path: pending.motion_path,
            fade_filter: pending.fade_filter,
            property_animations: pending.property_animations,
            media_action: pending.media_action,
        });
    }
    output
}

fn normalize_media_action(command: &str) -> &'static str {
    let command = command.to_ascii_lowercase();
    if command.contains("pause") {
        "pause"
    } else if command.contains("stop") {
        "stop"
    } else {
        "play"
    }
}

fn preset_default_effect(preset_id: Option<u32>, class: AnimationClass) -> AnimationKind {
    match (class, preset_id) {
        (AnimationClass::MotionPath, _) => AnimationKind::MotionPath,
        (AnimationClass::Media, _) => AnimationKind::Media,
        (AnimationClass::Entrance, Some(6)) => AnimationKind::Circle,
        (AnimationClass::Entrance, Some(9)) => AnimationKind::Dissolve,
        (AnimationClass::Entrance, Some(14)) => AnimationKind::RandomBars,
        (AnimationClass::Entrance, Some(16)) => AnimationKind::Split,
        (AnimationClass::Entrance, Some(21)) => AnimationKind::Wheel,
        (_, Some(1)) => AnimationKind::Appear,
        (_, Some(2)) => AnimationKind::FlyIn,
        (_, Some(10)) => AnimationKind::Fade,
        (_, Some(22)) => AnimationKind::Wipe,
        (_, Some(23)) => AnimationKind::Zoom,
        _ => AnimationKind::Custom,
    }
}

fn effect_from_filter(filter: &str) -> AnimationKind {
    let filter = filter.to_ascii_lowercase();
    if filter.contains("randombar") {
        AnimationKind::RandomBars
    } else if filter.contains("dissolve") {
        AnimationKind::Dissolve
    } else if filter.contains("wheel") {
        AnimationKind::Wheel
    } else if filter.contains("circle") {
        AnimationKind::Circle
    } else if filter.contains("barn") {
        AnimationKind::Split
    } else if filter.contains("fade") {
        AnimationKind::Fade
    } else if filter.contains("wipe") {
        AnimationKind::Wipe
    } else if filter.contains("fly") || filter.contains("slide") || filter.contains("strips") {
        AnimationKind::FlyIn
    } else if filter.contains("zoom") {
        AnimationKind::Zoom
    } else {
        AnimationKind::Custom
    }
}

fn direction_from_filter(filter: &str) -> Option<String> {
    let start = filter.find('(')? + 1;
    let end = filter[start..].find(')')? + start;
    Some(filter[start..end].to_string())
}

fn parse_transition_fragment(xml: &str) -> Option<SlideTransition> {
    let mut reader = Reader::from_reader(xml.as_bytes());
    reader.config_mut().trim_text(true);
    let mut duration_ms = 700;
    let mut advance_on_click = true;
    let mut advance_after_ms = None;
    let mut kind = None;
    let mut direction = None;
    let mut transition_depth = None;
    let mut unsupported_effect = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"transition" {
                    transition_depth = Some(0usize);
                    duration_ms = parse_time(attr(&element, b"dur").as_deref()).unwrap_or_else(
                        || match attr(&element, b"spd").as_deref() {
                            Some("fast") => 500,
                            Some("slow") => 2_000,
                            _ => 700,
                        },
                    );
                    advance_on_click = attr(&element, b"advClick").as_deref() != Some("0");
                    advance_after_ms = parse_time(attr(&element, b"advTm").as_deref());
                } else if let Some(depth) = transition_depth.as_mut() {
                    if *depth == 0 {
                        apply_transition_child(
                            &local,
                            &element,
                            &mut kind,
                            &mut direction,
                            &mut unsupported_effect,
                        );
                    }
                    *depth += 1;
                }
            }
            Ok(Event::Empty(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"transition" {
                    duration_ms = parse_time(attr(&element, b"dur").as_deref()).unwrap_or_else(
                        || match attr(&element, b"spd").as_deref() {
                            Some("fast") => 500,
                            Some("slow") => 2_000,
                            _ => 700,
                        },
                    );
                    advance_on_click = attr(&element, b"advClick").as_deref() != Some("0");
                    advance_after_ms = parse_time(attr(&element, b"advTm").as_deref());
                } else if transition_depth == Some(0) {
                    apply_transition_child(
                        &local,
                        &element,
                        &mut kind,
                        &mut direction,
                        &mut unsupported_effect,
                    );
                }
            }
            Ok(Event::End(element)) => {
                if local_name(element.name().as_ref()) == b"transition" {
                    transition_depth = None;
                } else if let Some(depth) = transition_depth.as_mut() {
                    *depth = depth.saturating_sub(1);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return None,
            _ => {}
        }
    }
    if unsupported_effect && kind.is_none() {
        return None;
    }
    Some(SlideTransition {
        kind: kind.unwrap_or_else(|| "none".into()),
        duration_ms,
        advance_on_click,
        advance_after_ms,
        direction,
    })
}

fn apply_transition_child(
    local: &[u8],
    element: &BytesStart<'_>,
    kind: &mut Option<String>,
    direction: &mut Option<String>,
    unsupported_effect: &mut bool,
) {
    if local == b"prstTrans" {
        if kind.is_none() {
            *kind = attr(element, b"prst");
            *direction = attr(element, b"dir").or_else(|| attr(element, b"orient"));
        }
    } else if is_supported_transition(local) {
        if kind.is_none() {
            *kind = Some(String::from_utf8_lossy(local).into_owned());
            *direction = attr(element, b"dir").or_else(|| attr(element, b"orient"));
        }
    } else if !matches!(local, b"sndAc" | b"extLst") {
        // PowerPoint 2010/2013 extensions such as p14:ripple and
        // p15:prstTrans are losslessly retained in source_transition_xml. If
        // the browser scene cannot model the Choice branch yet, signal the
        // caller to use the matching mc:Fallback transition for preview.
        *unsupported_effect = true;
    }
}

fn is_supported_transition(local: &[u8]) -> bool {
    matches!(
        local,
        b"blinds"
            | b"checker"
            | b"circle"
            | b"comb"
            | b"cover"
            | b"cut"
            | b"diamond"
            | b"dissolve"
            | b"fade"
            | b"newsflash"
            | b"plus"
            | b"pull"
            | b"push"
            | b"random"
            | b"randomBar"
            | b"split"
            | b"strips"
            | b"wedge"
            | b"wheel"
            | b"wipe"
            | b"zoom"
            | b"morph"
            // Office 2010 transition extensions (p14).  These names are kept
            // verbatim in the scene model so the browser can reproduce the
            // native effect instead of silently selecting mc:Fallback/fade.
            | b"conveyor"
            | b"doors"
            | b"ferris"
            | b"flash"
            | b"gallery"
            | b"glitter"
            | b"honeycomb"
            | b"pan"
            | b"prism"
            | b"reveal"
            | b"ripple"
            | b"switch"
            | b"vortex"
            | b"warp"
    )
}

fn extract_fallback_transition_xml(xml: &[u8]) -> Option<&[u8]> {
    let primary_start = find_bytes(xml, b"<p:transition")?;
    let alternate_end =
        primary_start + find_bytes(&xml[primary_start..], b"</mc:AlternateContent>")?;
    let fallback_start =
        primary_start + find_bytes(&xml[primary_start..alternate_end], b"<mc:Fallback")?;
    let fallback_open_end = fallback_start + find_bytes(&xml[fallback_start..], b">")? + 1;
    let fallback_close =
        fallback_open_end + find_bytes(&xml[fallback_open_end..alternate_end], b"</mc:Fallback>")?;
    extract_xml_element(&xml[fallback_open_end..fallback_close], b"p:transition")
}

fn parse_time(value: Option<&str>) -> Option<u64> {
    value?.parse::<u64>().ok()
}

fn attr(element: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    for attribute in element.attributes().with_checks(false).flatten() {
        if local_name(attribute.key.as_ref()) == key {
            return match attribute.value {
                Cow::Borrowed(bytes) => String::from_utf8(bytes.to_vec()).ok(),
                Cow::Owned(bytes) => String::from_utf8(bytes).ok(),
            };
        }
    }
    None
}

fn attr_u32(element: &BytesStart<'_>, key: &[u8]) -> Option<u32> {
    attr(element, key)?.parse().ok()
}

fn attr_i32(element: &BytesStart<'_>, key: &[u8]) -> Option<i32> {
    attr(element, key)?.parse().ok()
}

fn local_name(name: &[u8]) -> &[u8] {
    name.iter()
        .position(|byte| *byte == b':')
        .map_or(name, |index| &name[index + 1..])
}

fn find_object_id(objects: &[SceneObject], shape_id: u32) -> Option<String> {
    for object in objects {
        if object.source_shape_id == Some(shape_id) {
            return Some(object.id.clone());
        }
        if let Some(id) = find_object_id(&object.children, shape_id) {
            return Some(id);
        }
    }
    None
}

pub(crate) fn extract_xml_element<'a>(xml: &'a [u8], qualified_name: &[u8]) -> Option<&'a [u8]> {
    let mut opening = Vec::with_capacity(qualified_name.len() + 1);
    opening.push(b'<');
    opening.extend_from_slice(qualified_name);
    let start = find_bytes(xml, &opening)?;
    let opening_end = start + find_bytes(&xml[start..], b">")? + 1;
    if xml[start..opening_end].ends_with(b"/>") {
        return Some(&xml[start..opening_end]);
    }
    let mut closing = Vec::with_capacity(qualified_name.len() + 3);
    closing.extend_from_slice(b"</");
    closing.extend_from_slice(qualified_name);
    closing.push(b'>');
    let close_start = opening_end + find_bytes(&xml[opening_end..], &closing)?;
    Some(&xml[start..close_start + closing.len()])
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_the_native_fade_filter_into_the_model() {
        let entrance = r##"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="1" presetID="42" presetClass="entr" nodeType="clickEffect"><p:childTnLst><p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="2" dur="500"/><p:tgtEl><p:spTgt spid="7"/></p:tgtEl></p:cBhvr></p:animEffect><p:anim calcmode="lin" valueType="num"><p:cBhvr><p:cTn id="3" dur="500"/><p:tgtEl><p:spTgt spid="7"/></p:tgtEl><p:attrNameLst><p:attrName>ppt_y</p:attrName></p:attrNameLst></p:cBhvr><p:tavLst><p:tav tm="0"><p:val><p:strVal val="#ppt_y+.1"/></p:val></p:tav><p:tav tm="100000"><p:val><p:strVal val="#ppt_y"/></p:val></p:tav></p:tavLst></p:anim></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"##;
        let effects = parse_timing_fragment(entrance);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].fade_filter.as_deref(), Some("in"));
        assert_eq!(effects[0].property_animations.len(), 1);

        let exit = r#"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="1" presetClass="exit" nodeType="clickEffect"><p:childTnLst><p:animEffect filter="fade;wipe(left)"><p:cBhvr><p:cTn id="2" dur="500"/><p:tgtEl><p:spTgt spid="7"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#;
        let effects = parse_timing_fragment(exit);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].fade_filter.as_deref(), Some("out"));

        let wipe_only = r#"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="1" presetClass="entr" nodeType="clickEffect"><p:childTnLst><p:animEffect transition="in" filter="wipe(left)"><p:cBhvr><p:cTn id="2" dur="500"/><p:tgtEl><p:spTgt spid="7"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#;
        assert_eq!(parse_timing_fragment(wipe_only)[0].fade_filter, None);
    }

    #[test]
    fn parses_common_effects_and_transition() {
        let xml = br#"<p:sld xmlns:p="p"><p:cSld/><p:transition spd="fast" advClick="0" advTm="2500"><p:push dir="l"/></p:transition><p:timing><p:tnLst><p:par><p:cTn id="7" presetID="22" presetClass="entr" nodeType="clickEffect" accel="40000" decel="40000" repeatCount="2000" repeatDur="2600" autoRev="1"><p:stCondLst><p:cond delay="120"/></p:stCondLst><p:childTnLst><p:animEffect filter="wipe(left)"><p:cBhvr><p:cTn id="8" dur="650"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#;
        let parsed = parse_slide_timing(xml, &[]);
        assert_eq!(parsed.animations.len(), 1);
        let effect = &parsed.animations[0];
        assert_eq!(effect.source_timing_id, Some(7));
        assert_eq!(effect.target_shape_id, Some(42));
        assert_eq!(effect.effect, AnimationKind::Wipe);
        assert_eq!(effect.trigger, AnimationTrigger::OnClick);
        assert_eq!(effect.duration_ms, 650);
        assert_eq!(effect.delay_ms, 120);
        assert_eq!(effect.acceleration, Some(40_000));
        assert_eq!(effect.deceleration, Some(40_000));
        assert_eq!(effect.repeat_count.as_deref(), Some("2000"));
        assert_eq!(effect.repeat_duration_ms, Some(2_600));
        assert!(effect.auto_reverse);
        assert_eq!(effect.direction.as_deref(), Some("left"));
        let transition = parsed.transition.unwrap();
        assert_eq!(transition.kind, "push");
        assert_eq!(transition.duration_ms, 500);
        assert!(!transition.advance_on_click);
        assert_eq!(transition.advance_after_ms, Some(2_500));
    }

    #[test]
    fn parses_generic_numeric_property_behaviors_without_flattening_expressions() {
        let xml = r##"<p:timing xmlns:p="p" xmlns:p14="p14"><p:tnLst><p:par><p:cTn id="5" presetID="2" presetClass="entr" nodeType="withEffect"><p:childTnLst><p:anim calcmode="lin" valueType="num" from="#ppt_x-.2" to="#ppt_x" by="0.2" p14:bounceEnd="67000"><p:cBhvr additive="base"><p:cTn id="6" dur="2000" fill="hold"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl><p:attrNameLst><p:attrName>ppt_x</p:attrName></p:attrNameLst></p:cBhvr><p:tavLst><p:tav tm="0" fmla="#ppt_w*sin(2.5*pi*$)"><p:val><p:strVal val="0-#ppt_w/2"/></p:val></p:tav><p:tav tm="100000"><p:val><p:strVal val="#ppt_x"/></p:val></p:tav></p:tavLst></p:anim><p:anim calcmode="lin" valueType="num"><p:cBhvr><p:cTn id="7" dur="2000"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl><p:attrNameLst><p:attrName>style.rotation</p:attrName></p:attrNameLst></p:cBhvr><p:tavLst><p:tav tm="0"><p:val><p:fltVal val="360"/></p:val></p:tav><p:tav tm="100000"><p:val><p:fltVal val="0"/></p:val></p:tav></p:tavLst></p:anim></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"##;
        let effects = parse_timing_fragment(xml);
        assert_eq!(effects.len(), 1);
        let effect = &effects[0];
        assert_eq!(effect.target_shape_id, Some(42));
        assert_eq!(effect.property_animations.len(), 2);

        let x = &effect.property_animations[0];
        assert_eq!(x.attributes, ["ppt_x"]);
        assert_eq!(x.calculation_mode.as_deref(), Some("lin"));
        assert_eq!(x.value_type.as_deref(), Some("num"));
        assert_eq!(x.additive.as_deref(), Some("base"));
        assert_eq!(x.bounce_end, Some(67_000));
        assert_eq!(x.from.as_deref(), Some("#ppt_x-.2"));
        assert_eq!(x.to.as_deref(), Some("#ppt_x"));
        assert_eq!(x.by.as_deref(), Some("0.2"));
        assert_eq!(x.duration_ms, Some(2_000));
        assert_eq!(x.fill.as_deref(), Some("hold"));
        assert_eq!(x.keyframes.len(), 2);
        assert_eq!(x.keyframes[0].time, 0);
        assert_eq!(x.keyframes[0].value, "0-#ppt_w/2");
        assert_eq!(
            x.keyframes[0].formula.as_deref(),
            Some("#ppt_w*sin(2.5*pi*$)")
        );
        assert_eq!(x.keyframes[1].time, 100_000);
        assert_eq!(x.keyframes[1].value, "#ppt_x");

        let rotation = &effect.property_animations[1];
        assert_eq!(rotation.attributes, ["style.rotation"]);
        assert_eq!(rotation.keyframes[0].value, "360");
        assert_eq!(rotation.keyframes[1].value, "0");
    }

    #[test]
    fn maps_native_powerpoint_object_transition_filters_without_fade_fallbacks() {
        let cases = [
            (
                14,
                10,
                "randombar(horizontal)",
                AnimationKind::RandomBars,
                Some("horizontal"),
            ),
            (9, 0, "dissolve", AnimationKind::Dissolve, None),
            (21, 1, "wheel(1)", AnimationKind::Wheel, Some("1")),
            (6, 16, "circle(in)", AnimationKind::Circle, Some("in")),
            (
                16,
                42,
                "barn(outHorizontal)",
                AnimationKind::Split,
                Some("outHorizontal"),
            ),
        ];

        for (preset_id, preset_subtype, filter, expected_kind, expected_direction) in cases {
            let xml = format!(
                r#"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="1" presetID="{preset_id}" presetClass="entr" presetSubtype="{preset_subtype}" nodeType="afterEffect"><p:childTnLst><p:set><p:cBhvr><p:cTn id="2" dur="1"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl></p:cBhvr></p:set><p:animEffect transition="in" filter="{filter}"><p:cBhvr><p:cTn id="3" dur="500"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#
            );
            let parsed = parse_timing_fragment(&xml);
            assert_eq!(parsed.len(), 1, "preset {preset_id}/{preset_subtype}");
            let effect = &parsed[0];
            assert_eq!(effect.effect, expected_kind, "filter {filter}");
            assert_eq!(
                effect.direction.as_deref(),
                expected_direction,
                "filter {filter}"
            );
            assert_eq!(effect.preset_id, Some(preset_id));
            assert_eq!(effect.preset_subtype, Some(preset_subtype));
            assert_eq!(effect.target_shape_id, Some(42));
        }
    }

    #[test]
    fn parses_office_preset_transition_without_falling_back_to_fade() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:p14="p14" xmlns:p15="p15" xmlns:mc="mc"><p:cSld/><mc:AlternateContent><mc:Choice Requires="p15"><p:transition spd="slow" p14:dur="1250"><p15:prstTrans prst="pageCurlDouble"/></p:transition></mc:Choice><mc:Fallback><p:transition spd="slow"><p:fade/></p:transition></mc:Fallback></mc:AlternateContent></p:sld>"#;
        let parsed = parse_slide_timing(xml, &[]);
        let transition = parsed.transition.expect("native transition");
        assert_eq!(transition.kind, "pageCurlDouble");
        assert_eq!(transition.duration_ms, 1_250);
        let source = parsed
            .source_transition_xml
            .expect("native Choice transition remains loss-aware");
        assert!(source.contains("p15:prstTrans"));
        assert!(source.contains("pageCurlDouble"));
        assert!(!source.contains("p:fade"));
    }

    #[test]
    fn parses_office_ripple_transition_without_falling_back_to_fade() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:p14="p14" xmlns:mc="mc"><p:cSld/><mc:AlternateContent><mc:Choice Requires="p14"><p:transition spd="slow" p14:dur="1400"><p14:ripple/></p:transition></mc:Choice><mc:Fallback><p:transition spd="slow"><p:fade/></p:transition></mc:Fallback></mc:AlternateContent></p:sld>"#;
        let parsed = parse_slide_timing(xml, &[]);
        let transition = parsed.transition.expect("native ripple transition");
        assert_eq!(transition.kind, "ripple");
        assert_eq!(transition.duration_ms, 1_400);
    }

    #[test]
    fn keeps_fallback_for_an_unknown_choice_transition() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:p14="p14" xmlns:mc="mc"><p:cSld/><mc:AlternateContent><mc:Choice Requires="p14"><p:transition p14:dur="900"><p14:futureEffect/></p:transition></mc:Choice><mc:Fallback><p:transition spd="fast"><p:fade/></p:transition></mc:Fallback></mc:AlternateContent></p:sld>"#;
        let transition = parse_slide_timing(xml, &[])
            .transition
            .expect("fallback transition");
        assert_eq!(transition.kind, "fade");
        assert_eq!(transition.duration_ms, 500);
    }

    #[test]
    fn parses_native_media_commands_without_flattening_the_timing_fragment() {
        let xml = br#"<p:sld xmlns:p="p"><p:cSld/><p:timing><p:tnLst><p:par><p:cTn id="5" presetID="1" presetClass="mediacall" nodeType="withEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:cmd type="call" cmd="pause"><p:cBhvr><p:cTn id="6" dur="1"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl></p:cBhvr></p:cmd></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#;
        let parsed = parse_slide_timing(xml, &[]);
        assert_eq!(parsed.animations.len(), 1);
        assert_eq!(parsed.animations[0].effect, AnimationKind::Media);
        assert_eq!(parsed.animations[0].class, AnimationClass::Media);
        assert_eq!(parsed.animations[0].media_action.as_deref(), Some("pause"));
        assert_eq!(parsed.animations[0].target_shape_id, Some(42));
        assert_eq!(
            parsed.source_timing_xml.as_deref(),
            std::str::from_utf8(extract_xml_element(xml, b"p:timing").unwrap()).ok()
        );
    }

    #[test]
    fn preserves_wrapper_time_slots_without_inventing_clicks_for_on_begin_sequences() {
        // Reduced from 9.pptx.  Its producer stores absolute time slots on the
        // wrapper cTn nodes while every effect is with/after previous.  The
        // onBegin condition makes this one automatic batch, not a click batch.
        let xml = r#"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="1" nodeType="tmRoot"><p:childTnLst><p:seq><p:cTn id="2" nodeType="mainSeq"><p:childTnLst><p:par><p:cTn id="3"><p:stCondLst><p:cond delay="indefinite"/><p:cond evt="onBegin" delay="0"><p:tn val="2"/></p:cond></p:stCondLst><p:childTnLst><p:par><p:cTn id="4"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:par><p:cTn id="5" presetID="10" presetClass="entr" nodeType="afterEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:animEffect filter="fade"><p:cBhvr><p:cTn id="6" dur="500"/><p:tgtEl><p:spTgt spid="10"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par><p:par><p:cTn id="7"><p:stCondLst><p:cond delay="600"/></p:stCondLst><p:childTnLst><p:par><p:cTn id="8" presetID="10" presetClass="entr" nodeType="afterEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:animEffect filter="fade"><p:cBhvr><p:cTn id="9" dur="500"/><p:tgtEl><p:spTgt spid="11"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#;
        let effects = parse_timing_fragment(xml);
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].trigger, AnimationTrigger::AfterPrevious);
        assert_eq!(effects[0].delay_ms, 0);
        assert_eq!(effects[1].trigger, AnimationTrigger::AfterPrevious);
        assert_eq!(effects[1].delay_ms, 100);
    }

    #[test]
    fn promotes_only_the_first_effect_of_an_ancestor_click_group() {
        // Some OOXML producers put the activation on the wrapper cTn and use
        // withEffect/afterEffect for its descendants.  Preserve that as one
        // PowerPoint click batch rather than zero clicks or one click per node.
        let xml = r#"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="1" nodeType="tmRoot"><p:childTnLst><p:seq><p:cTn id="2" nodeType="mainSeq"><p:childTnLst><p:par><p:cTn id="3"><p:stCondLst><p:cond delay="indefinite"/></p:stCondLst><p:childTnLst><p:par><p:cTn id="4" presetID="10" presetClass="entr" nodeType="withEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:animEffect filter="fade"><p:cBhvr><p:cTn id="5" dur="500"/><p:tgtEl><p:spTgt spid="10"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par><p:par><p:cTn id="6" presetID="10" presetClass="entr" nodeType="afterEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:animEffect filter="fade"><p:cBhvr><p:cTn id="7" dur="500"/><p:tgtEl><p:spTgt spid="11"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#;
        let effects = parse_timing_fragment(xml);
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0].trigger, AnimationTrigger::OnClick);
        assert_eq!(effects[1].trigger, AnimationTrigger::AfterPrevious);
    }

    #[test]
    fn respects_an_on_click_condition_on_the_effect_node() {
        let xml = r#"<p:timing xmlns:p="p"><p:tnLst><p:par><p:cTn id="7" presetID="10" presetClass="entr" nodeType="withEffect"><p:stCondLst><p:cond evt="onClick" delay="0"/></p:stCondLst><p:childTnLst><p:animEffect filter="fade"><p:cBhvr><p:cTn id="8" dur="500"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"#;
        let effects = parse_timing_fragment(xml);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].trigger, AnimationTrigger::OnClick);
    }
}

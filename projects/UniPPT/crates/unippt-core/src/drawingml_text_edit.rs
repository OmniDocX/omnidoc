//! Loss-aware DrawingML text editing.
//!
//! This module deliberately patches ranges in the imported XML instead of
//! serialising `p:txBody` again.  The approach mirrors UniCell's native shape
//! editor: editor-only `source_index` values identify the original paragraph
//! and run fragments, so unknown attributes, extension lists, field metadata
//! and vendor markup survive edits byte-for-byte.

use std::collections::HashSet;
use std::ops::Range;

use crate::font_policy::primary_family;
use crate::model::{FontSlots, HyperlinkAction, RichTextParagraph, RichTextRun, TextFrameStyle};

#[derive(Debug, Clone)]
struct XmlNode {
    qname: String,
    local_name: String,
    range: Range<usize>,
    open_end: usize,
    close_start: usize,
    parent: Option<usize>,
}

impl XmlNode {
    fn content_range(&self) -> Range<usize> {
        self.open_end..self.close_start
    }
}

/// Patch structured text while retaining every untouched source fragment.
pub(crate) fn patch_structured_text(
    xml: &str,
    original: &[RichTextParagraph],
    edited: &[RichTextParagraph],
) -> Result<String, String> {
    let nodes = parse_nodes(xml)?;
    let body_index = nodes
        .iter()
        .position(|node| node.local_name == "txBody")
        .ok_or_else(|| "shape has no p:txBody".to_string())?;
    let existing = direct_children_named(&nodes, body_index, "p");
    let assignments = source_assignments(
        edited.iter().map(|paragraph| paragraph.source_index),
        existing.len(),
        "paragraph",
    )?;

    let replacements = edited
        .iter()
        .zip(assignments.iter().copied())
        .enumerate()
        .map(|(edited_index, (paragraph, source_index))| {
            if let Some(source_index) = source_index {
                let source_node = &nodes[existing[source_index]];
                let source_model = original
                    .get(source_index)
                    .or_else(|| original.get(edited_index));
                patch_paragraph(xml, source_node, source_model, paragraph)
            } else {
                Ok(make_paragraph(paragraph))
            }
        })
        .collect::<Result<Vec<_>, String>>()?;

    let body = &nodes[body_index];
    let mut fragment = xml[body.range.clone()].to_string();
    let body_start = body.range.start;
    if existing.len() == replacements.len()
        && assignments
            .iter()
            .enumerate()
            .all(|(index, source)| *source == Some(index))
    {
        let patches = existing
            .iter()
            .zip(replacements)
            .map(|(node, replacement)| {
                let range = &nodes[*node].range;
                (
                    range.start - body_start..range.end - body_start,
                    replacement,
                )
            })
            .collect();
        apply_patches(&mut fragment, patches);
    } else if let Some(first) = existing.first().copied() {
        let replacement = replacements.concat();
        let mut patches = Vec::with_capacity(existing.len());
        for (index, node) in existing.iter().copied().enumerate() {
            let range = &nodes[node].range;
            patches.push((
                range.start - body_start..range.end - body_start,
                if node == first && index == 0 {
                    replacement.clone()
                } else {
                    String::new()
                },
            ));
        }
        apply_patches(&mut fragment, patches);
    } else if !replacements.is_empty() {
        let insertion = body.close_start - body_start;
        fragment.insert_str(insertion, &replacements.concat());
    }

    Ok(replace_range(xml, body.range.clone(), &fragment))
}

/// Patch only the body properties represented by the scene model.  All other
/// bodyPr attributes and children (autofit, columns, 3-D, extensions, ...) are
/// retained exactly.
pub(crate) fn patch_text_frame(
    xml: &str,
    original: &TextFrameStyle,
    edited: &TextFrameStyle,
    emu_per_px: f64,
) -> Result<String, String> {
    if original == edited {
        return Ok(xml.to_string());
    }
    let nodes = parse_nodes(xml)?;
    let body_index = nodes
        .iter()
        .position(|node| node.local_name == "txBody")
        .ok_or_else(|| "shape has no p:txBody".to_string())?;
    let Some(body_pr_index) = direct_children_named(&nodes, body_index, "bodyPr")
        .into_iter()
        .next()
    else {
        let body = &nodes[body_index];
        let mut fragment = xml[body.range.clone()].to_string();
        let insertion = body.open_end - body.range.start;
        fragment.insert_str(insertion, &make_body_pr(edited, emu_per_px));
        return Ok(replace_range(xml, body.range.clone(), &fragment));
    };

    let body_pr = &nodes[body_pr_index];
    let mut fragment = xml[body_pr.range.clone()].to_string();
    for (changed, name, value) in [
        (
            original.margin_left != edited.margin_left,
            "lIns",
            margin_emu(edited.margin_left, emu_per_px),
        ),
        (
            original.margin_right != edited.margin_right,
            "rIns",
            margin_emu(edited.margin_right, emu_per_px),
        ),
        (
            original.margin_top != edited.margin_top,
            "tIns",
            margin_emu(edited.margin_top, emu_per_px),
        ),
        (
            original.margin_bottom != edited.margin_bottom,
            "bIns",
            margin_emu(edited.margin_bottom, emu_per_px),
        ),
    ] {
        if changed {
            fragment = set_start_attribute(&fragment, name, Some(&value));
        }
    }
    if original.vertical_align != edited.vertical_align {
        let anchor = match edited.vertical_align.as_str() {
            "top" => "t",
            "bottom" => "b",
            _ => "ctr",
        };
        fragment = set_start_attribute(&fragment, "anchor", Some(anchor));
    }
    if original.vertical_type != edited.vertical_type {
        let vertical_type = edited.vertical_type.trim();
        fragment = set_start_attribute(
            &fragment,
            "vert",
            (!vertical_type.is_empty() && vertical_type != "horz").then_some(vertical_type),
        );
    }
    if original.word_wrap != edited.word_wrap {
        fragment = set_start_attribute(
            &fragment,
            "wrap",
            Some(if edited.word_wrap { "square" } else { "none" }),
        );
    }
    Ok(replace_range(xml, body_pr.range.clone(), &fragment))
}

pub(crate) fn structured_text_body(
    paragraphs: &[RichTextParagraph],
    frame: &TextFrameStyle,
    emu_per_px: f64,
) -> String {
    let paragraphs = paragraphs.iter().map(make_paragraph).collect::<String>();
    format!(
        "<p:txBody>{}<a:lstStyle/>{paragraphs}</p:txBody>",
        make_body_pr(frame, emu_per_px)
    )
}

/// Replace or insert one direct child of `p:spPr` without serialising the
/// surrounding shape properties. Unknown effects, extension lists, theme
/// references and vendor markup remain byte-identical.
pub(crate) fn patch_shape_property_child(
    xml: &str,
    candidate_local_names: &[&str],
    replacement: &str,
    insert_before_local_names: &[&str],
) -> Result<String, String> {
    let nodes = parse_nodes(xml)?;
    let properties = nodes
        .iter()
        .position(|node| node.local_name == "spPr")
        .ok_or_else(|| "shape has no p:spPr".to_string())?;
    if let Some(existing) = nodes.iter().enumerate().find_map(|(index, node)| {
        (node.parent == Some(properties)
            && candidate_local_names.contains(&node.local_name.as_str()))
        .then_some(index)
    }) {
        return Ok(replace_range(
            xml,
            nodes[existing].range.clone(),
            replacement,
        ));
    }
    if replacement.is_empty() {
        return Ok(xml.to_string());
    }

    let properties_node = &nodes[properties];
    if properties_node.open_end == properties_node.range.end {
        return Ok(append_child(xml, properties_node, replacement));
    }
    let insertion = nodes
        .iter()
        .find(|node| {
            node.parent == Some(properties)
                && insert_before_local_names.contains(&node.local_name.as_str())
        })
        .map_or(properties_node.close_start, |node| node.range.start);
    let mut output = xml.to_string();
    output.insert_str(insertion, replacement);
    Ok(output)
}

/// Patch only the native inner/outer shadow in `a:effectLst`, retaining glow,
/// reflection, soft-edge and all unknown effect children.
pub(crate) fn patch_shape_shadow(
    xml: &str,
    replacement_effect: Option<&str>,
) -> Result<String, String> {
    let nodes = parse_nodes(xml)?;
    let properties = nodes
        .iter()
        .position(|node| node.local_name == "spPr")
        .ok_or_else(|| "shape has no p:spPr".to_string())?;
    let effect_list = nodes.iter().enumerate().find_map(|(index, node)| {
        (node.parent == Some(properties) && node.local_name == "effectLst").then_some(index)
    });

    if let Some(effect_list) = effect_list {
        let shadows = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.parent == Some(effect_list)
                    && matches!(node.local_name.as_str(), "outerShdw" | "innerShdw")
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if shadows.is_empty() {
            if let Some(replacement) = replacement_effect {
                let effect_node = &nodes[effect_list];
                if effect_node.open_end == effect_node.range.end {
                    return Ok(append_child(xml, effect_node, replacement));
                }
                let mut output = xml.to_string();
                output.insert_str(effect_node.close_start, replacement);
                return Ok(output);
            }
            return Ok(xml.to_string());
        }
        let mut output = xml.to_string();
        let patches = shadows
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (
                    nodes[*node].range.clone(),
                    if index == 0 {
                        replacement_effect.unwrap_or_default().to_string()
                    } else {
                        String::new()
                    },
                )
            })
            .collect();
        apply_patches(&mut output, patches);
        return Ok(output);
    }

    let Some(replacement) = replacement_effect else {
        return Ok(xml.to_string());
    };
    patch_shape_property_child(
        xml,
        &["effectLst"],
        &format!("<a:effectLst>{replacement}</a:effectLst>"),
        &["extLst"],
    )
}

fn patch_paragraph(
    xml: &str,
    paragraph: &XmlNode,
    original: Option<&RichTextParagraph>,
    edited: &RichTextParagraph,
) -> Result<String, String> {
    let source = &xml[paragraph.range.clone()];
    let nodes = parse_nodes(source)?;
    let root_index = nodes
        .iter()
        .position(|node| node.local_name == "p")
        .ok_or_else(|| "malformed DrawingML paragraph".to_string())?;
    let original_runs = original
        .map(|paragraph| paragraph.runs.as_slice())
        .unwrap_or(&[]);
    let mut existing = direct_run_children(&nodes, root_index);
    if existing.len() != original_runs.len() {
        // The vendor parser currently exposes regular runs and line breaks but
        // can leave date/slide-number fields opaque.  Do not let such a field
        // shift every following source_index; retain it in place as an
        // untouched sibling. If a future importer exposes fields, the counts
        // match and they become directly editable through this same path.
        existing.retain(|index| nodes[*index].local_name != "fld");
    }
    let assignments = source_assignments(
        edited.runs.iter().map(|run| run.source_index),
        existing.len(),
        "text run",
    )?;
    let replacements = edited
        .runs
        .iter()
        .zip(assignments.iter().copied())
        .enumerate()
        .map(|(edited_index, (run, source_index))| {
            if let Some(source_index) = source_index {
                let source_node = &nodes[existing[source_index]];
                let source_model = original_runs
                    .get(source_index)
                    .or_else(|| original_runs.get(edited_index));
                patch_run(source, source_node, source_model, run)
            } else {
                Ok(make_run(run))
            }
        })
        .collect::<Result<Vec<_>, String>>()?;

    let root = &nodes[root_index];
    let mut fragment = source.to_string();
    if existing.len() == replacements.len()
        && assignments
            .iter()
            .enumerate()
            .all(|(index, source)| *source == Some(index))
    {
        let patches = existing
            .iter()
            .zip(replacements)
            .map(|(node, replacement)| (nodes[*node].range.clone(), replacement))
            .collect();
        apply_patches(&mut fragment, patches);
    } else if !existing.is_empty() {
        let replacement = replacements.concat();
        let patches = existing
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (
                    nodes[*node].range.clone(),
                    if index == 0 {
                        replacement.clone()
                    } else {
                        String::new()
                    },
                )
            })
            .collect();
        apply_patches(&mut fragment, patches);
    } else if !replacements.is_empty() {
        let insertion = direct_children_named(&nodes, root_index, "endParaRPr")
            .first()
            .map_or(root.close_start, |node| nodes[*node].range.start);
        if root.open_end == root.range.end {
            fragment = append_child(&fragment, root, &replacements.concat());
        } else {
            fragment.insert_str(insertion, &replacements.concat());
        }
    }

    if let Some(original) = original {
        fragment = patch_paragraph_properties(&fragment, original, edited)?;
    }
    Ok(fragment)
}

fn patch_run(
    xml: &str,
    run: &XmlNode,
    original: Option<&RichTextRun>,
    edited: &RichTextRun,
) -> Result<String, String> {
    let mut fragment = xml[run.range.clone()].to_string();
    let nodes = parse_nodes(&fragment)?;
    let root = nodes
        .iter()
        .position(|node| matches!(node.local_name.as_str(), "r" | "fld" | "br"))
        .ok_or_else(|| "malformed DrawingML text run".to_string())?;
    let text_node = direct_children_named(&nodes, root, "t").into_iter().next();
    let properties_node = direct_children_named(&nodes, root, "rPr")
        .into_iter()
        .next();

    if original.is_none_or(|before| before.text != edited.text) {
        if let Some(text_node) = text_node {
            let node = &nodes[text_node];
            let replacement = patch_text_element(&fragment[node.range.clone()], &edited.text);
            fragment = replace_range(&fragment, node.range.clone(), &replacement);
        } else {
            let nodes = parse_nodes(&fragment)?;
            let root = &nodes[root];
            fragment = append_child(&fragment, root, &make_text_element(&edited.text));
        }
    }

    let style_changed = original.is_none_or(|before| run_style_changed(before, edited));
    if style_changed {
        // Reparse because a preceding text replacement can shift the ranges.
        let nodes = parse_nodes(&fragment)?;
        let root = nodes
            .iter()
            .position(|node| matches!(node.local_name.as_str(), "r" | "fld" | "br"))
            .ok_or_else(|| "malformed DrawingML text run".to_string())?;
        let properties = direct_children_named(&nodes, root, "rPr")
            .into_iter()
            .next();
        if let Some(properties) = properties {
            let node = &nodes[properties];
            let replacement =
                patch_run_properties(&fragment[node.range.clone()], original, edited)?;
            fragment = replace_range(&fragment, node.range.clone(), &replacement);
        } else {
            let properties = make_run_properties(edited);
            let nodes = parse_nodes(&fragment)?;
            let root = &nodes[root];
            if root.open_end == root.range.end {
                fragment = append_child(&fragment, root, &properties);
            } else {
                fragment.insert_str(root.open_end, &properties);
            }
        }
    }
    let _ = properties_node; // Kept explicit to document the preserved source slot.
    Ok(fragment)
}

fn patch_run_properties(
    properties: &str,
    original: Option<&RichTextRun>,
    edited: &RichTextRun,
) -> Result<String, String> {
    let mut fragment = properties.to_string();
    if original.is_none_or(|before| before.font_size != edited.font_size) {
        fragment = set_start_attribute(
            &fragment,
            "sz",
            Some(&((edited.font_size.max(0.0) * 75.0).round() as i64).to_string()),
        );
    }
    if original.is_none_or(|before| before.bold != edited.bold) {
        fragment = set_start_attribute(&fragment, "b", Some(if edited.bold { "1" } else { "0" }));
    }
    if original.is_none_or(|before| before.italic != edited.italic) {
        fragment = set_start_attribute(&fragment, "i", Some(if edited.italic { "1" } else { "0" }));
    }
    if original.is_none_or(|before| {
        before.underline != edited.underline || before.underline_style != edited.underline_style
    }) {
        let underline = edited
            .underline_style
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(if edited.underline { "sng" } else { "none" });
        fragment = set_start_attribute(&fragment, "u", Some(underline));
    }
    if original.is_none_or(|before| before.strikethrough != edited.strikethrough) {
        fragment = set_start_attribute(
            &fragment,
            "strike",
            Some(if edited.strikethrough {
                "sngStrike"
            } else {
                "noStrike"
            }),
        );
    }
    if original.is_none_or(|before| before.baseline != edited.baseline || before.baseline_offset != edited.baseline_offset) {
        let baseline = run_baseline(edited).to_string();
        fragment = set_start_attribute(&fragment, "baseline", Some(&baseline));
    }
    if let Some(before) = original {
        if before.native_fonts != edited.native_fonts {
            fragment =
                patch_font_slots(&fragment, Some(&before.native_fonts), &edited.native_fonts)?;
        } else if before.native_font_family != edited.native_font_family {
            // Backward-compatible explicit "one font for everything" edit.
            // CSS stack changes alone never enter this path.
            let mut slots = FontSlots::unified(native_or_primary_font(edited));
            slots.language_id = edited
                .native_fonts
                .language_id
                .clone()
                .or_else(|| before.native_fonts.language_id.clone());
            fragment = patch_font_slots(&fragment, Some(&before.native_fonts), &slots)?;
        }
    } else {
        let slots = run_font_slots_for_new_xml(edited);
        fragment = patch_font_slots(&fragment, None, &slots)?;
    }
    if original.is_none_or(|before| before.color != edited.color || before.alpha != edited.alpha) {
        fragment = patch_run_color(&fragment, &edited.color, edited.alpha.clamp(0.0, 1.0))?;
    }
    if original.is_none_or(|before| before.hyperlinks.click != edited.hyperlinks.click) {
        fragment = patch_run_hyperlink(&fragment, "hlinkClick", edited.hyperlinks.click.as_ref())?;
    }
    if original.is_none_or(|before| before.hyperlinks.hover != edited.hyperlinks.hover) {
        fragment = patch_run_hyperlink(
            &fragment,
            "hlinkMouseOver",
            edited.hyperlinks.hover.as_ref(),
        )?;
    }
    Ok(fragment)
}

fn patch_run_hyperlink(
    properties: &str,
    local_name: &str,
    action: Option<&HyperlinkAction>,
) -> Result<String, String> {
    let nodes = parse_nodes(properties)?;
    let root = nodes
        .iter()
        .position(|node| node.local_name == "rPr")
        .ok_or_else(|| "malformed a:rPr".to_string())?;
    let existing = direct_children_named(&nodes, root, local_name)
        .into_iter()
        .next();
    if let Some(existing) = existing {
        let node = &nodes[existing];
        let Some(action) = action else {
            return Ok(replace_range(properties, node.range.clone(), ""));
        };
        let mut native = properties[node.range.clone()].to_string();
        native = set_start_attribute(&native, "r:id", action.relationship_id.as_deref());
        native = set_start_attribute(&native, "action", action.action.as_deref());
        native = set_start_attribute(&native, "tooltip", action.tooltip.as_deref());
        return Ok(replace_range(properties, node.range.clone(), &native));
    }
    let Some(action) = action else {
        return Ok(properties.to_string());
    };
    Ok(append_child(
        properties,
        &nodes[root],
        &run_hyperlink_xml(local_name, action),
    ))
}

fn run_hyperlink_xml(local_name: &str, action: &HyperlinkAction) -> String {
    let mut output = format!("<a:{local_name}");
    if let Some(relationship_id) = action.relationship_id.as_deref() {
        output.push_str(&format!(" r:id=\"{}\"", xml_escape(relationship_id)));
    }
    if let Some(value) = action.action.as_deref() {
        output.push_str(&format!(" action=\"{}\"", xml_escape(value)));
    }
    if let Some(value) = action.tooltip.as_deref() {
        output.push_str(&format!(" tooltip=\"{}\"", xml_escape(value)));
    }
    output.push_str("/>");
    output
}

fn patch_font_slots(
    properties: &str,
    original: Option<&FontSlots>,
    edited: &FontSlots,
) -> Result<String, String> {
    let mut fragment = properties.to_string();
    if original.is_none_or(|before| before.language_id != edited.language_id) {
        fragment = set_start_attribute(&fragment, "lang", edited.language_id.as_deref());
    }
    for (tag, before, after) in [
        (
            "latin",
            original.and_then(|slots| slots.latin.as_deref()),
            edited.latin.as_deref(),
        ),
        (
            "ea",
            original.and_then(|slots| slots.east_asia.as_deref()),
            edited.east_asia.as_deref(),
        ),
        (
            "cs",
            original.and_then(|slots| slots.complex_script.as_deref()),
            edited.complex_script.as_deref(),
        ),
        (
            "sym",
            original.and_then(|slots| slots.symbol.as_deref()),
            edited.symbol.as_deref(),
        ),
    ] {
        if original.is_some() && before == after {
            continue;
        }
        fragment = patch_font_slot(&fragment, tag, after)?;
    }
    Ok(fragment)
}

fn patch_font_slot(properties: &str, tag: &str, typeface: Option<&str>) -> Result<String, String> {
    let nodes = parse_nodes(properties)?;
    let root = nodes
        .iter()
        .position(|node| node.local_name == "rPr")
        .ok_or_else(|| "malformed a:rPr".to_string())?;
    let existing = direct_children_named(&nodes, root, tag).into_iter().next();
    match (existing, typeface) {
        (Some(existing), Some(typeface)) => {
            let node = &nodes[existing];
            let replacement =
                set_start_attribute(&properties[node.range.clone()], "typeface", Some(typeface));
            Ok(replace_range(properties, node.range.clone(), &replacement))
        }
        (Some(existing), None) => Ok(replace_range(properties, nodes[existing].range.clone(), "")),
        (None, Some(typeface)) => Ok(append_child(
            properties,
            &nodes[root],
            &format!("<a:{tag} typeface=\"{}\"/>", xml_escape(typeface)),
        )),
        (None, None) => Ok(properties.to_string()),
    }
}

fn patch_run_color(properties: &str, color: &str, alpha: f64) -> Result<String, String> {
    let color = css_hex(color).unwrap_or("000000");
    let alpha = (alpha * 100_000.0).round() as i64;
    let nodes = parse_nodes(properties)?;
    let root = nodes
        .iter()
        .position(|node| node.local_name == "rPr")
        .ok_or_else(|| "malformed a:rPr".to_string())?;
    let solid = direct_children_named(&nodes, root, "solidFill")
        .into_iter()
        .next();
    if let Some(solid) = solid {
        let color_node = nodes.iter().enumerate().find(|(_, node)| {
            node.parent == Some(solid)
                && matches!(
                    node.local_name.as_str(),
                    "srgbClr" | "schemeClr" | "sysClr" | "prstClr" | "scrgbClr" | "hslClr"
                )
        });
        if let Some((_, node)) = color_node {
            let source = &properties[node.range.clone()];
            let source_nodes = parse_nodes(source)?;
            let source_root = &source_nodes[0];
            let alpha_node = direct_children_named(&source_nodes, 0, "alpha")
                .into_iter()
                .next();
            let mut color_fragment = if let Some(alpha_node) = alpha_node {
                let alpha_node = &source_nodes[alpha_node];
                let replacement = set_start_attribute(
                    &source[alpha_node.range.clone()],
                    "val",
                    Some(&alpha.to_string()),
                );
                replace_range(source, alpha_node.range.clone(), &replacement)
            } else {
                append_child(source, source_root, &format!("<a:alpha val=\"{alpha}\"/>"))
            };
            // A direct colour edit must use an RGB choice, but the transform
            // children (including vendor extensions) are retained.
            if source_root.local_name == "srgbClr" {
                color_fragment = set_start_attribute(&color_fragment, "val", Some(color));
            } else {
                let updated_nodes = parse_nodes(&color_fragment)?;
                let updated_root = &updated_nodes[0];
                color_fragment = format!(
                    "<a:srgbClr val=\"{color}\">{}</a:srgbClr>",
                    &color_fragment[updated_root.content_range()]
                );
            }
            Ok(replace_range(
                properties,
                node.range.clone(),
                &color_fragment,
            ))
        } else {
            let replacement =
                format!("<a:srgbClr val=\"{color}\"><a:alpha val=\"{alpha}\"/></a:srgbClr>");
            Ok(replace_range(
                properties,
                nodes[solid].range.clone(),
                &append_child(
                    &properties[nodes[solid].range.clone()],
                    &XmlNode {
                        range: 0..nodes[solid].range.len(),
                        open_end: nodes[solid].open_end - nodes[solid].range.start,
                        close_start: nodes[solid].close_start - nodes[solid].range.start,
                        qname: nodes[solid].qname.clone(),
                        local_name: nodes[solid].local_name.clone(),
                        parent: None,
                    },
                    &replacement,
                ),
            ))
        }
    } else {
        let replacement = format!(
            "<a:solidFill><a:srgbClr val=\"{color}\"><a:alpha val=\"{alpha}\"/></a:srgbClr></a:solidFill>"
        );
        Ok(append_child(properties, &nodes[root], &replacement))
    }
}

fn patch_paragraph_properties(
    paragraph: &str,
    original: &RichTextParagraph,
    edited: &RichTextParagraph,
) -> Result<String, String> {
    let style_changed = original.align != edited.align
        || original.level != edited.level
        || original.bullet != edited.bullet
        || original.line_spacing != edited.line_spacing
        || original.space_before != edited.space_before
        || original.space_after != edited.space_after;
    if !style_changed {
        return Ok(paragraph.to_string());
    }
    let nodes = parse_nodes(paragraph)?;
    let root = nodes
        .iter()
        .position(|node| node.local_name == "p")
        .ok_or_else(|| "malformed a:p".to_string())?;
    let p_pr = direct_children_named(&nodes, root, "pPr")
        .into_iter()
        .next();
    let mut fragment = if let Some(p_pr) = p_pr {
        let range = nodes[p_pr].range.clone();
        let patched =
            patch_existing_paragraph_properties(&paragraph[range.clone()], original, edited)?;
        replace_range(paragraph, range, &patched)
    } else {
        if nodes[root].open_end == nodes[root].range.end {
            append_child(paragraph, &nodes[root], &make_paragraph_properties(edited))
        } else {
            let mut fragment = paragraph.to_string();
            fragment.insert_str(nodes[root].open_end, &make_paragraph_properties(edited));
            fragment
        }
    };
    // Ensure we never accidentally produce an empty, non-self-closing pPr.
    if fragment.is_empty() {
        fragment = paragraph.to_string();
    }
    Ok(fragment)
}

fn patch_existing_paragraph_properties(
    properties: &str,
    original: &RichTextParagraph,
    edited: &RichTextParagraph,
) -> Result<String, String> {
    let mut fragment = properties.to_string();
    if original.align != edited.align {
        fragment = set_start_attribute(&fragment, "algn", Some(paragraph_alignment(&edited.align)));
    }
    if original.level != edited.level {
        fragment = set_start_attribute(&fragment, "lvl", Some(&edited.level.to_string()));
    }
    if original.line_spacing != edited.line_spacing {
        fragment = replace_property_child(
            &fragment,
            "lnSpc",
            edited.line_spacing.map(|value| {
                format!(
                    "<a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>",
                    (value * 100_000.0).round() as i64
                )
            }),
        )?;
    }
    if original.space_before != edited.space_before {
        fragment = replace_property_child(
            &fragment,
            "spcBef",
            edited.space_before.map(|value| {
                format!(
                    "<a:spcBef><a:spcPts val=\"{}\"/></a:spcBef>",
                    (value * 75.0).round() as i64
                )
            }),
        )?;
    }
    if original.space_after != edited.space_after {
        fragment = replace_property_child(
            &fragment,
            "spcAft",
            edited.space_after.map(|value| {
                format!(
                    "<a:spcAft><a:spcPts val=\"{}\"/></a:spcAft>",
                    (value * 75.0).round() as i64
                )
            }),
        )?;
    }
    if original.bullet != edited.bullet {
        fragment = patch_bullet(&fragment, edited.bullet.as_deref())?;
    }
    Ok(fragment)
}

fn replace_property_child(
    properties: &str,
    child_name: &str,
    replacement: Option<String>,
) -> Result<String, String> {
    let nodes = parse_nodes(properties)?;
    let root = nodes
        .iter()
        .position(|node| node.local_name == "pPr")
        .ok_or_else(|| "malformed a:pPr".to_string())?;
    if let Some(child) = direct_children_named(&nodes, root, child_name)
        .into_iter()
        .next()
    {
        Ok(replace_range(
            properties,
            nodes[child].range.clone(),
            replacement.as_deref().unwrap_or(""),
        ))
    } else if let Some(replacement) = replacement {
        Ok(append_child(properties, &nodes[root], &replacement))
    } else {
        Ok(properties.to_string())
    }
}

fn patch_bullet(properties: &str, bullet: Option<&str>) -> Result<String, String> {
    let nodes = parse_nodes(properties)?;
    let root = nodes
        .iter()
        .position(|node| node.local_name == "pPr")
        .ok_or_else(|| "malformed a:pPr".to_string())?;
    let existing = nodes
        .iter()
        .enumerate()
        .find(|(_, node)| {
            node.parent == Some(root)
                && matches!(
                    node.local_name.as_str(),
                    "buNone" | "buChar" | "buAutoNum" | "buBlip"
                )
        })
        .map(|(index, _)| index);
    let replacement = match bullet {
        None => "<a:buNone/>".to_string(),
        Some("1.") => "<a:buAutoNum type=\"arabicPeriod\"/>".to_string(),
        Some(value) => format!("<a:buChar char=\"{}\"/>", xml_escape(value)),
    };
    if let Some(existing) = existing {
        Ok(replace_range(
            properties,
            nodes[existing].range.clone(),
            &replacement,
        ))
    } else {
        Ok(append_child(properties, &nodes[root], &replacement))
    }
}

fn make_paragraph(paragraph: &RichTextParagraph) -> String {
    let runs = paragraph.runs.iter().map(make_run).collect::<String>();
    format!(
        "<a:p>{}{runs}<a:endParaRPr/></a:p>",
        make_paragraph_properties(paragraph)
    )
}

fn make_paragraph_properties(paragraph: &RichTextParagraph) -> String {
    let mut children = String::new();
    if let Some(value) = paragraph.line_spacing {
        children.push_str(&format!(
            "<a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>",
            (value * 100_000.0).round() as i64
        ));
    }
    if let Some(value) = paragraph.space_before {
        children.push_str(&format!(
            "<a:spcBef><a:spcPts val=\"{}\"/></a:spcBef>",
            (value * 75.0).round() as i64
        ));
    }
    if let Some(value) = paragraph.space_after {
        children.push_str(&format!(
            "<a:spcAft><a:spcPts val=\"{}\"/></a:spcAft>",
            (value * 75.0).round() as i64
        ));
    }
    if let Some(bullet) = paragraph.bullet.as_deref() {
        if bullet == "1." {
            children.push_str("<a:buAutoNum type=\"arabicPeriod\"/>");
        } else {
            children.push_str(&format!("<a:buChar char=\"{}\"/>", xml_escape(bullet)));
        }
    }
    format!(
        "<a:pPr algn=\"{}\" lvl=\"{}\">{children}</a:pPr>",
        paragraph_alignment(&paragraph.align),
        paragraph.level
    )
}

fn make_run(run: &RichTextRun) -> String {
    format!(
        "<a:r>{}{}</a:r>",
        make_run_properties(run),
        make_text_element(&run.text)
    )
}

fn make_run_properties(run: &RichTextRun) -> String {
    let underline = run
        .underline_style
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(if run.underline { "sng" } else { "none" });
    let strike = if run.strikethrough {
        "sngStrike"
    } else {
        "noStrike"
    };
    let baseline = run_baseline(run);
    let color = css_hex(&run.color).unwrap_or("000000");
    let alpha = (run.alpha.clamp(0.0, 1.0) * 100_000.0).round() as i64;
    let native_fonts = run_font_slots_for_new_xml(run);
    let language = native_fonts
        .language_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!(" lang=\"{}\"", xml_escape(value)))
        .unwrap_or_default();
    let font_nodes = font_slots_xml(&native_fonts);
    let hyperlinks = [
        ("hlinkClick", run.hyperlinks.click.as_ref()),
        ("hlinkMouseOver", run.hyperlinks.hover.as_ref()),
    ]
    .into_iter()
    .filter_map(|(name, action)| action.map(|action| run_hyperlink_xml(name, action)))
    .collect::<String>();
    format!(
        "<a:rPr{language} sz=\"{}\" b=\"{}\" i=\"{}\" u=\"{}\" strike=\"{}\" baseline=\"{}\"><a:solidFill><a:srgbClr val=\"{}\"><a:alpha val=\"{}\"/></a:srgbClr></a:solidFill>{font_nodes}{hyperlinks}</a:rPr>",
        (run.font_size.max(0.0) * 75.0).round() as i64,
        i32::from(run.bold),
        i32::from(run.italic),
        xml_escape(underline),
        strike,
        baseline,
        color,
        alpha,
    )
}

fn run_font_slots_for_new_xml(run: &RichTextRun) -> FontSlots {
    if !run.native_fonts.is_empty() {
        return run.native_fonts.clone();
    }
    FontSlots::unified(native_or_primary_font(run))
}

fn font_slots_xml(slots: &FontSlots) -> String {
    [
        ("latin", slots.latin.as_deref()),
        ("ea", slots.east_asia.as_deref()),
        ("cs", slots.complex_script.as_deref()),
        ("sym", slots.symbol.as_deref()),
    ]
    .into_iter()
    .filter_map(|(tag, value)| {
        value
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("<a:{tag} typeface=\"{}\"/>", xml_escape(value)))
    })
    .collect()
}

fn make_text_element(text: &str) -> String {
    let preserve = if needs_preserve_space(text) {
        " xml:space=\"preserve\""
    } else {
        ""
    };
    format!("<a:t{preserve}>{}</a:t>", xml_escape(text))
}

fn patch_text_element(element: &str, text: &str) -> String {
    let Ok(nodes) = parse_nodes(element) else {
        return make_text_element(text);
    };
    let Some(node) = nodes.iter().find(|node| node.local_name == "t") else {
        return make_text_element(text);
    };
    let mut fragment = element.to_string();
    fragment.replace_range(node.content_range(), &xml_escape(text));
    if needs_preserve_space(text) {
        set_start_attribute(&fragment, "xml:space", Some("preserve"))
    } else {
        fragment
    }
}

pub(crate) fn make_body_pr(frame: &TextFrameStyle, emu_per_px: f64) -> String {
    let anchor = match frame.vertical_align.as_str() {
        "top" => "t",
        "bottom" => "b",
        _ => "ctr",
    };
    let autofit = match frame.auto_size.as_str() {
        // `shrinkText` was emitted by early HTML compiler builds.  Accept it
        // on export so older UDOC documents gain the intended PowerPoint
        // autofit instead of silently losing the policy.
        "textToFitShape" | "normAutofit" | "shrinkText" => "<a:normAutofit/>",
        "shapeToFitText" | "spAutoFit" => "<a:spAutoFit/>",
        _ => "",
    };
    let vertical_type = frame.vertical_type.trim();
    let vertical_type = if vertical_type.is_empty() || vertical_type == "horz" {
        String::new()
    } else {
        format!(" vert=\"{}\"", xml_escape(vertical_type))
    };
    format!(
        "<a:bodyPr wrap=\"{}\" anchor=\"{anchor}\"{vertical_type} lIns=\"{}\" rIns=\"{}\" tIns=\"{}\" bIns=\"{}\">{autofit}</a:bodyPr>",
        if frame.word_wrap { "square" } else { "none" },
        margin_emu(frame.margin_left, emu_per_px),
        margin_emu(frame.margin_right, emu_per_px),
        margin_emu(frame.margin_top, emu_per_px),
        margin_emu(frame.margin_bottom, emu_per_px),
    )
}

fn margin_emu(value: f64, emu_per_px: f64) -> String {
    (value.max(0.0) * emu_per_px).round().to_string()
}

fn run_baseline(run: &RichTextRun) -> i64 {
    if let Some(offset) = run.baseline_offset.filter(|v| v.is_finite()) {
        return (offset.clamp(-100.0, 100.0) * 1000.0).round() as i64;
    }
    match run.baseline.as_str() { "super" => 30000, "sub" => -25000, _ => 0 }
}

fn run_style_changed(before: &RichTextRun, after: &RichTextRun) -> bool {
    before.native_font_family != after.native_font_family
        || before.native_fonts != after.native_fonts
        || before.font_size != after.font_size
        || before.color != after.color
        || before.bold != after.bold
        || before.italic != after.italic
        || before.underline != after.underline
        || before.underline_style != after.underline_style
        || before.strikethrough != after.strikethrough
        || before.baseline != after.baseline
        || before.baseline_offset != after.baseline_offset
        || before.alpha != after.alpha
        || before.hyperlinks != after.hyperlinks
}

fn native_or_primary_font(run: &RichTextRun) -> String {
    run.native_font_family
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.starts_with('+'))
        .map_or_else(|| primary_family(&run.font_family), str::to_string)
}

fn paragraph_alignment(value: &str) -> &str {
    match value {
        "center" => "ctr",
        "right" => "r",
        "justify" => "just",
        "justifyLow" => "justLow",
        "distributed" => "dist",
        "thaiDistributed" => "thaiDist",
        _ => "l",
    }
}

fn source_assignments(
    source_indices: impl Iterator<Item = Option<usize>>,
    existing_len: usize,
    label: &str,
) -> Result<Vec<Option<usize>>, String> {
    let source_indices: Vec<Option<usize>> = source_indices.collect();
    let has_identity = source_indices.iter().any(Option::is_some);
    let mut used = HashSet::new();
    source_indices
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            let source = if has_identity {
                source
            } else {
                (index < existing_len).then_some(index)
            };
            if let Some(source) = source {
                if source >= existing_len {
                    return Err(format!("{label}.source_index {source} is out of range"));
                }
                if !used.insert(source) {
                    return Err(format!("duplicate {label}.source_index {source}"));
                }
            }
            Ok(source)
        })
        .collect()
}

fn direct_children_named(nodes: &[XmlNode], parent: usize, local_name: &str) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent == Some(parent) && node.local_name == local_name)
        .map(|(index, _)| index)
        .collect()
}

fn direct_run_children(nodes: &[XmlNode], parent: usize) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.parent == Some(parent) && matches!(node.local_name.as_str(), "r" | "fld" | "br")
        })
        .map(|(index, _)| index)
        .collect()
}

fn parse_nodes(xml: &str) -> Result<Vec<XmlNode>, String> {
    let bytes = xml.as_bytes();
    let mut nodes = Vec::<XmlNode>::new();
    let mut stack = Vec::<usize>::new();
    let mut position = 0usize;
    while let Some(relative) = xml[position..].find('<') {
        let start = position + relative;
        if xml[start..].starts_with("<!--") {
            position = xml[start + 4..]
                .find("-->")
                .map(|end| start + 4 + end + 3)
                .ok_or_else(|| "unterminated XML comment".to_string())?;
            continue;
        }
        if xml[start..].starts_with("<![CDATA[") {
            position = xml[start + 9..]
                .find("]]>")
                .map(|end| start + 9 + end + 3)
                .ok_or_else(|| "unterminated CDATA section".to_string())?;
            continue;
        }
        if xml[start..].starts_with("<?") {
            position = xml[start + 2..]
                .find("?>")
                .map(|end| start + 2 + end + 2)
                .ok_or_else(|| "unterminated XML processing instruction".to_string())?;
            continue;
        }
        if xml[start..].starts_with("<!") {
            position = find_tag_end(bytes, start)? + 1;
            continue;
        }
        let tag_end = find_tag_end(bytes, start)?;
        let mut cursor = start + 1;
        let closing = bytes.get(cursor) == Some(&b'/');
        if closing {
            cursor += 1;
        }
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        let name_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(*byte, b'/' | b'>'))
        {
            cursor += 1;
        }
        if cursor == name_start {
            return Err("malformed XML tag".to_string());
        }
        let qname = &xml[name_start..cursor];
        if closing {
            let index = stack
                .pop()
                .ok_or_else(|| format!("unexpected closing tag {qname}"))?;
            if nodes[index].qname != qname {
                return Err(format!(
                    "mismatched closing tag {qname}, expected {}",
                    nodes[index].qname
                ));
            }
            nodes[index].close_start = start;
            nodes[index].range.end = tag_end + 1;
        } else {
            let self_closing = xml[start..=tag_end].trim_end().ends_with("/>");
            let index = nodes.len();
            nodes.push(XmlNode {
                qname: qname.to_string(),
                local_name: qname.rsplit(':').next().unwrap_or(qname).to_string(),
                range: start..tag_end + 1,
                open_end: tag_end + 1,
                close_start: if self_closing {
                    tag_end.saturating_sub(1)
                } else {
                    0
                },
                parent: stack.last().copied(),
            });
            if !self_closing {
                stack.push(index);
            }
        }
        position = tag_end + 1;
    }
    if !stack.is_empty() {
        return Err("unterminated XML element".to_string());
    }
    Ok(nodes)
}

fn find_tag_end(bytes: &[u8], start: usize) -> Result<usize, String> {
    let mut quote = None;
    for (index, byte) in bytes.iter().enumerate().skip(start + 1) {
        match (*byte, quote) {
            (b'\'' | b'"', None) => quote = Some(*byte),
            (value, Some(current)) if value == current => quote = None,
            (b'>', None) => return Ok(index),
            _ => {}
        }
    }
    Err("unterminated XML start tag".to_string())
}

fn set_start_attribute(xml: &str, attribute: &str, value: Option<&str>) -> String {
    let Ok(end) = find_tag_end(xml.as_bytes(), 0) else {
        return xml.to_string();
    };
    let tag = &xml[..=end];
    let Some(range) = attribute_value_range(tag, attribute) else {
        if let Some(value) = value {
            let insert = tag
                .rfind("/>")
                .unwrap_or_else(|| tag.rfind('>').unwrap_or(tag.len()));
            return format!(
                "{} {attribute}=\"{}\"{}{}",
                &tag[..insert],
                xml_escape(value),
                &tag[insert..],
                &xml[end + 1..]
            );
        }
        return xml.to_string();
    };
    if let Some(value) = value {
        format!(
            "{}{}{}",
            &xml[..range.start],
            xml_escape(value),
            &xml[range.end..]
        )
    } else {
        let mut remove_start = range.start;
        while remove_start > 0 && xml.as_bytes()[remove_start - 1].is_ascii_whitespace() {
            remove_start -= 1;
        }
        let mut remove_end = range.end;
        if xml.as_bytes().get(remove_end) == Some(&b'\'')
            || xml.as_bytes().get(remove_end) == Some(&b'"')
        {
            remove_end += 1;
        }
        // Include `name=` and its opening quote.
        let name_start = xml[..range.start].rfind(attribute).unwrap_or(remove_start);
        remove_start = name_start;
        while remove_start > 0 && xml.as_bytes()[remove_start - 1].is_ascii_whitespace() {
            remove_start -= 1;
        }
        format!("{}{}", &xml[..remove_start], &xml[remove_end..])
    }
}

fn attribute_value_range(tag: &str, attribute: &str) -> Option<Range<usize>> {
    let bytes = tag.as_bytes();
    let mut position = 1usize;
    while position < bytes.len() {
        while bytes
            .get(position)
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'/')
        {
            position += 1;
        }
        if bytes.get(position).is_none_or(|byte| *byte == b'>') {
            break;
        }
        let name_start = position;
        while bytes
            .get(position)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(*byte, b'=' | b'/' | b'>'))
        {
            position += 1;
        }
        let name = &tag[name_start..position];
        if name.is_empty() {
            break;
        }
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        if bytes.get(position) != Some(&b'=') {
            continue;
        }
        position += 1;
        while bytes.get(position).is_some_and(u8::is_ascii_whitespace) {
            position += 1;
        }
        let quote = *bytes.get(position)?;
        if !matches!(quote, b'\'' | b'"') {
            continue;
        }
        position += 1;
        let value_start = position;
        while bytes.get(position).is_some_and(|byte| *byte != quote) {
            position += 1;
        }
        let value_end = position;
        position += 1;
        if name == attribute {
            return Some(value_start..value_end);
        }
    }
    None
}

fn apply_patches(fragment: &mut String, mut patches: Vec<(Range<usize>, String)>) {
    patches.sort_by(|left, right| {
        right
            .0
            .start
            .cmp(&left.0.start)
            .then_with(|| right.0.end.cmp(&left.0.end))
    });
    for (range, replacement) in patches {
        fragment.replace_range(range, &replacement);
    }
}

fn replace_range(xml: &str, range: Range<usize>, replacement: &str) -> String {
    format!(
        "{}{}{}",
        &xml[..range.start],
        replacement,
        &xml[range.end..]
    )
}

fn append_child(xml: &str, node: &XmlNode, child: &str) -> String {
    if node.open_end == node.range.end {
        // Expand `<a:rPr/>` before adding children. Inserting at the slash
        // would place markup inside the start tag and produce invalid XML.
        let slash = node.range.end.saturating_sub(2);
        format!(
            "{}>{}</{}>{}",
            &xml[..slash],
            child,
            node.qname,
            &xml[node.range.end..]
        )
    } else {
        let mut output = xml.to_string();
        output.insert_str(node.close_start, child);
        output
    }
}

fn needs_preserve_space(text: &str) -> bool {
    text.starts_with(char::is_whitespace) || text.ends_with(char::is_whitespace)
}

fn css_hex(value: &str) -> Option<&str> {
    let value = value.strip_prefix('#')?;
    (value.len() >= 6 && value[..6].bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some(&value[..6])
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source_index: Option<usize>, text: &str, color: &str) -> RichTextRun {
        RichTextRun {
            source_index,
            text: text.into(),
            font_family: "Aptos".into(),
            native_font_family: Some("Aptos".into()),
            native_fonts: FontSlots::default(),
            font_size: 24.0,
            color: color.into(),
            gradient: None,
            alpha: 1.0,
            bold: false,
            italic: false,
            underline: false,
            underline_style: None,
            strikethrough: false,
            baseline: "normal".into(),
            baseline_offset: None,
            hyperlinks: crate::model::ObjectHyperlinks::default(),
        }
    }

    fn paragraph(source_index: Option<usize>, runs: Vec<RichTextRun>) -> RichTextParagraph {
        RichTextParagraph {
            source_index,
            runs,
            align: "left".into(),
            level: 0,
            bullet: None,
            line_spacing: None,
            space_before: None,
            space_after: None,
        }
    }

    #[test]
    fn patches_one_run_and_preserves_unknown_markup() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a" xmlns:u="vendor"><p:txBody><a:bodyPr foo="keep"><u:bodyExt/></a:bodyPr><a:lstStyle/><a:p data="opaque"><a:pPr algn="l"><u:pExt id="7"/></a:pPr><a:r data="keep"><a:rPr lang="zh-CN" sz="1800"><a:solidFill><a:srgbClr val="112233"><u:transform/></a:srgbClr></a:solidFill><u:rExt exact="yes"/></a:rPr><a:t>before</a:t><u:afterText/></a:r><a:fld id="field-1"><a:rPr lang="en-US"/><a:t>field</a:t><u:fldExt/></a:fld><a:endParaRPr><u:endExt/></a:endParaRPr></a:p></p:txBody></p:sp>"#;
        let original = vec![paragraph(
            Some(0),
            vec![
                run(Some(0), "before", "#112233"),
                run(Some(1), "field", "#000000"),
            ],
        )];
        let mut edited = original.clone();
        edited[0].runs[0].text = " after ".into();
        edited[0].runs[0].bold = true;

        let output = patch_structured_text(xml, &original, &edited).unwrap();

        assert!(output.contains("<a:t xml:space=\"preserve\"> after </a:t>"));
        assert!(output.contains("b=\"1\""));
        assert!(output.contains("<u:rExt exact=\"yes\"/>"));
        assert!(output.contains("<u:transform/>"));
        assert!(output.contains(
            "<a:fld id=\"field-1\"><a:rPr lang=\"en-US\"/><a:t>field</a:t><u:fldExt/></a:fld>"
        ));
        assert!(output.contains("<u:pExt id=\"7\"/>"));
        assert!(output.contains("<u:endExt/>"));
        assert!(output.contains("<u:bodyExt/>"));
    }

    #[test]
    fn css_font_stack_writes_only_primary_drawingml_typeface() {
        let mut value = run(None, "mixed", "#000000");
        value.font_family = "Calibri, 'Microsoft YaHei', sans-serif".into();
        value.native_font_family = None;
        let xml = make_run_properties(&value);
        assert!(xml.contains("<a:latin typeface=\"Calibri\"/>"));
        assert!(xml.contains("<a:ea typeface=\"Calibri\"/>"));
        assert!(!xml.contains("Microsoft YaHei"));
        assert!(!xml.contains("sans-serif"));
    }

    #[test]
    fn text_only_edit_preserves_all_native_font_slots_byte_for_byte() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a"><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN"><a:latin typeface="Calibri"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/><a:sym typeface="Wingdings"/></a:rPr><a:t>ABC中文</a:t></a:r></a:p></p:txBody></p:sp>"#;
        let mut source = run(Some(0), "ABC中文", "#000000");
        source.native_font_family = Some("Microsoft YaHei".into());
        source.native_fonts = FontSlots {
            latin: Some("Calibri".into()),
            east_asia: Some("Microsoft YaHei".into()),
            complex_script: Some("Arial".into()),
            symbol: Some("Wingdings".into()),
            language_id: Some("zh-CN".into()),
        };
        let original = vec![paragraph(Some(0), vec![source])];
        let mut edited = original.clone();
        edited[0].runs[0].text = "DEF中文".into();

        let output = patch_structured_text(xml, &original, &edited).unwrap();
        assert!(output.contains(r#"lang="zh-CN""#));
        assert!(output.contains(r#"<a:latin typeface="Calibri"/>"#));
        assert!(output.contains(r#"<a:ea typeface="Microsoft YaHei"/>"#));
        assert!(output.contains(r#"<a:cs typeface="Arial"/>"#));
        assert!(output.contains(r#"<a:sym typeface="Wingdings"/>"#));
        assert!(output.contains("<a:t>DEF中文</a:t>"));
    }

    #[test]
    fn changing_one_native_slot_does_not_touch_the_other_scripts() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a"><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="zh-CN"><a:latin typeface="Calibri"/><a:ea typeface="Microsoft YaHei"/><a:cs typeface="Arial"/><a:sym typeface="Wingdings"/></a:rPr><a:t>ABC中文</a:t></a:r></a:p></p:txBody></p:sp>"#;
        let slots = FontSlots {
            latin: Some("Calibri".into()),
            east_asia: Some("Microsoft YaHei".into()),
            complex_script: Some("Arial".into()),
            symbol: Some("Wingdings".into()),
            language_id: Some("zh-CN".into()),
        };
        let mut source = run(Some(0), "ABC中文", "#000000");
        source.native_fonts = slots;
        let original = vec![paragraph(Some(0), vec![source])];
        let mut edited = original.clone();
        edited[0].runs[0].native_fonts.east_asia = Some("等线".into());

        let output = patch_structured_text(xml, &original, &edited).unwrap();
        assert!(output.contains(r#"<a:latin typeface="Calibri"/>"#));
        assert!(output.contains(r#"<a:ea typeface="等线"/>"#));
        assert!(output.contains(r#"<a:cs typeface="Arial"/>"#));
        assert!(output.contains(r#"<a:sym typeface="Wingdings"/>"#));
    }

    #[test]
    fn new_mixed_script_run_writes_exact_slots_without_css_stack_leakage() {
        let mut value = run(None, "ABC中文", "#000000");
        value.font_family = "Calibri, 'Microsoft YaHei', sans-serif".into();
        value.native_font_family = Some("Microsoft YaHei".into());
        value.native_fonts = FontSlots {
            latin: Some("Calibri".into()),
            east_asia: Some("Microsoft YaHei".into()),
            complex_script: Some("Arial".into()),
            symbol: Some("Wingdings".into()),
            language_id: Some("zh-CN".into()),
        };
        let xml = make_run_properties(&value);
        assert!(xml.contains(r#"lang="zh-CN""#));
        assert!(xml.contains(r#"<a:latin typeface="Calibri"/>"#));
        assert!(xml.contains(r#"<a:ea typeface="Microsoft YaHei"/>"#));
        assert!(xml.contains(r#"<a:cs typeface="Arial"/>"#));
        assert!(xml.contains(r#"<a:sym typeface="Wingdings"/>"#));
        assert!(!xml.contains("Calibri, 'Microsoft YaHei'"));
        assert!(!xml.contains("sans-serif"));
    }

    #[test]
    fn source_indices_reorder_runs_without_losing_fields() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a"><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r marker="first"><a:t>A</a:t></a:r><a:fld id="second"><a:t>B</a:t></a:fld><a:endParaRPr/></a:p></p:txBody></p:sp>"#;
        let original = vec![paragraph(
            Some(0),
            vec![run(Some(0), "A", "#000000"), run(Some(1), "B", "#000000")],
        )];
        let edited = vec![paragraph(
            Some(0),
            vec![run(Some(1), "B", "#000000"), run(Some(0), "A", "#000000")],
        )];

        let output = patch_structured_text(xml, &original, &edited).unwrap();
        assert!(output.find("id=\"second\"").unwrap() < output.find("marker=\"first\"").unwrap());
    }

    #[test]
    fn opaque_field_does_not_shift_imported_run_indices() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a" xmlns:u="vendor"><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:fld id="opaque"><a:t>7</a:t><u:fieldExt/></a:fld><a:r marker="editable"><a:t>A</a:t></a:r><a:endParaRPr/></a:p></p:txBody></p:sp>"#;
        // This mirrors the current vendor parser: the field is retained in
        // source XML but only the regular run appears in the scene model.
        let original = vec![paragraph(Some(0), vec![run(Some(0), "A", "#000000")])];
        let mut edited = original.clone();
        edited[0].runs[0].text = "changed".into();

        let output = patch_structured_text(xml, &original, &edited).unwrap();
        assert!(output.contains("<a:fld id=\"opaque\"><a:t>7</a:t><u:fieldExt/></a:fld>"));
        assert!(output.contains("marker=\"editable\""));
        assert!(output.contains(">changed</a:t>"));
    }

    #[test]
    fn patches_body_properties_without_replacing_children() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a" xmlns:u="vendor"><p:txBody><a:bodyPr wrap="square" lIns="0" custom="keep"><a:normAutofit/><u:ext/></a:bodyPr><a:lstStyle/><a:p/></p:txBody></p:sp>"#;
        let original = TextFrameStyle {
            margin_left: 0.0,
            margin_right: 0.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
            vertical_align: "center".into(),
            vertical_type: "horz".into(),
            word_wrap: true,
            auto_size: "textToFitShape".into(),
        };
        let mut edited = original.clone();
        edited.margin_left = 8.0;
        edited.vertical_align = "bottom".into();
        edited.vertical_type = "eaVert".into();

        let output = patch_text_frame(xml, &original, &edited, 9_525.0).unwrap();
        assert!(output.contains("lIns=\"76200\""));
        assert!(output.contains("anchor=\"b\""));
        assert!(output.contains("vert=\"eaVert\""));
        assert!(output.contains("custom=\"keep\""));
        assert!(output.contains("<a:normAutofit/><u:ext/>"));
    }

    #[test]
    fn legacy_shrink_text_alias_writes_native_powerpoint_autofit() {
        let frame = TextFrameStyle {
            margin_left: 0.0,
            margin_right: 0.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
            vertical_align: "top".into(),
            vertical_type: "horz".into(),
            word_wrap: false,
            auto_size: "shrinkText".into(),
        };

        let output = make_body_pr(&frame, 9_525.0);
        assert!(output.contains("wrap=\"none\""));
        assert!(output.contains("<a:normAutofit/>"));
    }

    #[test]
    fn patches_shape_children_without_flattening_effects_or_extensions() {
        let xml = r#"<p:sp xmlns:p="p" xmlns:a="a" xmlns:u="vendor"><p:spPr data="keep"><a:xfrm/><a:prstGeom prst="rect"><a:avLst/><u:geom/></a:prstGeom><a:solidFill><a:schemeClr val="accent1"><u:transform/></a:schemeClr></a:solidFill><a:ln><a:solidFill><a:srgbClr val="111111"/></a:solidFill><u:line/></a:ln><a:effectLst><a:glow rad="123"><a:schemeClr val="accent2"/></a:glow><a:outerShdw blurRad="1"><a:srgbClr val="000000"/></a:outerShdw><u:effect/></a:effectLst><a:extLst><u:shapeExt/></a:extLst></p:spPr></p:sp>"#;
        let output = patch_shape_property_child(
            xml,
            &["solidFill", "gradFill", "noFill"],
            "<a:solidFill><a:srgbClr val=\"ABCDEF\"/></a:solidFill>",
            &["ln", "effectLst", "extLst"],
        )
        .unwrap();
        let output = patch_shape_shadow(
            &output,
            Some("<a:innerShdw blurRad=\"9\"><a:srgbClr val=\"010203\"/></a:innerShdw>"),
        )
        .unwrap();

        assert!(output.contains("data=\"keep\""));
        assert!(output.contains("<u:geom/>"));
        assert!(output.contains("<u:line/>"));
        assert!(output.contains("<a:glow rad=\"123\""));
        assert!(output.contains("<u:effect/>"));
        assert!(output.contains("<u:shapeExt/>"));
        assert!(output.contains("<a:innerShdw blurRad=\"9\""));
        assert!(!output.contains("<a:outerShdw"));
        assert!(!output.contains("<u:transform/>"));
    }

    #[test]
    fn precise_script_offset_writes_and_patches_without_losing_other_properties() {
        let mut before = run(None, "0", "#112233");
        before.baseline = "sub".into();
        let mut edited = before.clone();
        edited.baseline_offset = Some(-47.5);
        assert_eq!(run_baseline(&before), -25000);
        assert_eq!(run_baseline(&edited), -47500);
        assert!(run_style_changed(&before, &edited));
        let source = r#"<a:rPr sz="1200" baseline="-25000" data-keep="yes"><a:latin typeface="Times New Roman"/></a:rPr>"#;
        let output = patch_run_properties(source, Some(&before), &edited).unwrap();
        assert!(output.contains("baseline=\"-47500\""));
        assert!(output.contains("data-keep=\"yes\""));
        assert!(output.contains("Times New Roman"));
        edited.baseline_offset = None;
        assert_eq!(run_baseline(&edited), -25000);
    }
}

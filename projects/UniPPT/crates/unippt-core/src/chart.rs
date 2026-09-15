//! Loss-aware, read-only projection of native PowerPoint chart parts.
//!
//! A chart remains authoritative in its original OPC part.  This module only
//! resolves the slide `p:graphicFrame/c:chart` relationship and exposes the
//! cached title/categories/series needed by the browser renderer.  Unknown
//! chart XML and vendor extensions are never rewritten by the parser.

use std::collections::{BTreeMap, HashMap};

use pptx::opc::pack_uri::PackURI;
use pptx::theme::ThemeColorScheme;
use pptx::Presentation;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartData {
    /// Relationship on the owning slide (`rIdN`).
    #[serde(default)]
    pub relationship_id: Option<String>,
    /// Absolute OPC chart part name, retained as loss-aware identity.
    #[serde(default)]
    pub source_part_name: Option<String>,
    pub chart_type: ChartKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub legend: ChartLegend,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub series: Vec<ChartSeries>,
    /// `column` or `bar` for DrawingML `barDir`.
    #[serde(default)]
    pub bar_direction: Option<String>,
    #[serde(default)]
    pub grouping: Option<String>,
    /// Doughnut hole size as a normalized 0..1 value.
    #[serde(default)]
    pub hole_size: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChartKind {
    Line,
    Bar,
    Pie,
    Doughnut,
    Area,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChartLegend {
    pub visible: bool,
    pub position: String,
    pub overlay: bool,
}

impl Default for ChartLegend {
    fn default() -> Self {
        Self {
            visible: false,
            position: "right".into(),
            overlay: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeries {
    #[serde(default)]
    pub source_index: Option<u32>,
    pub name: String,
    #[serde(default)]
    pub values: Vec<Option<f64>>,
    pub color: String,
    /// Optional per-slice/per-point colours, primarily for pie charts.
    #[serde(default)]
    pub point_colors: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChartFrameReference {
    pub shape_id: u32,
    pub relationship_id: String,
}

/// Resolve every chart on a slide without mutating the OPC graph.
pub(crate) fn resolve_slide_charts(
    presentation: &Presentation,
    slide_part_name: &PackURI,
    slide_xml: &[u8],
    theme: &ThemeColorScheme,
) -> HashMap<u32, ChartData> {
    let Some(slide_part) = presentation.package().part(slide_part_name) else {
        return HashMap::new();
    };
    parse_chart_frame_references(slide_xml)
        .into_iter()
        .map(|reference| {
            let mut chart = slide_part
                .related_partname(&reference.relationship_id)
                .ok()
                .and_then(|part_name| {
                    let part = presentation.package().part(&part_name)?;
                    let mut chart = parse_chart_xml(&part.blob, theme);
                    chart.source_part_name = Some(part_name.to_string());
                    Some(chart)
                })
                .unwrap_or_else(|| ChartData {
                    relationship_id: None,
                    source_part_name: None,
                    chart_type: ChartKind::Unknown,
                    title: String::new(),
                    legend: ChartLegend::default(),
                    categories: Vec::new(),
                    series: Vec::new(),
                    bar_direction: None,
                    grouping: None,
                    hole_size: None,
                });
            chart.relationship_id = Some(reference.relationship_id);
            (reference.shape_id, chart)
        })
        .collect()
}

pub(crate) fn parse_chart_frame_references(slide_xml: &[u8]) -> Vec<ChartFrameReference> {
    let mut reader = Reader::from_reader(slide_xml);
    reader.config_mut().trim_text(true);
    let mut depth = 0usize;
    let mut frame_depth = None;
    let mut shape_id = None;
    let mut relationship_id = None;
    let mut output = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = element.name();
                let local = local_name(name.as_ref());
                if local == b"graphicFrame" && frame_depth.is_none() {
                    frame_depth = Some(depth);
                    shape_id = None;
                    relationship_id = None;
                } else if frame_depth.is_some() && local == b"cNvPr" && shape_id.is_none() {
                    shape_id = attr(&element, b"id").and_then(|value| value.parse().ok());
                } else if frame_depth.is_some() && local == b"chart" {
                    relationship_id = attr(&element, b"id");
                }
                depth += 1;
            }
            Ok(Event::Empty(element)) => {
                let name = element.name();
                let local = local_name(name.as_ref());
                if frame_depth.is_some() && local == b"cNvPr" && shape_id.is_none() {
                    shape_id = attr(&element, b"id").and_then(|value| value.parse().ok());
                } else if frame_depth.is_some() && local == b"chart" {
                    relationship_id = attr(&element, b"id");
                }
            }
            Ok(Event::End(element)) => {
                depth = depth.saturating_sub(1);
                if local_name(element.name().as_ref()) == b"graphicFrame"
                    && frame_depth == Some(depth)
                {
                    if let (Some(shape_id), Some(relationship_id)) =
                        (shape_id.take(), relationship_id.take())
                    {
                        output.push(ChartFrameReference {
                            shape_id,
                            relationship_id,
                        });
                    }
                    frame_depth = None;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    output
}

pub(crate) fn parse_chart_xml(xml: &[u8], theme: &ThemeColorScheme) -> ChartData {
    let title = capture_direct_children(xml, b"chart", b"title")
        .into_iter()
        .next()
        .map_or_else(String::new, |fragment| text_content(&fragment));
    let legend = capture_direct_children(xml, b"chart", b"legend")
        .into_iter()
        .next()
        .map_or_else(ChartLegend::default, |fragment| parse_legend(&fragment));
    let Some((plot_local, plot_xml)) = capture_first_plot(xml) else {
        return ChartData {
            relationship_id: None,
            source_part_name: None,
            chart_type: ChartKind::Unknown,
            title,
            legend,
            categories: Vec::new(),
            series: Vec::new(),
            bar_direction: None,
            grouping: None,
            hole_size: None,
        };
    };
    let chart_type = chart_kind(&plot_local);
    let bar_direction =
        child_attribute(&plot_xml, plot_local.as_bytes(), b"barDir", b"val").map(|value| {
            if value == "bar" {
                "bar".into()
            } else {
                "column".into()
            }
        });
    let grouping = child_attribute(&plot_xml, plot_local.as_bytes(), b"grouping", b"val");
    let hole_size = child_attribute(&plot_xml, plot_local.as_bytes(), b"holeSize", b"val")
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| (value / 100.0).clamp(0.0, 0.95));
    let series_fragments = capture_direct_children(&plot_xml, plot_local.as_bytes(), b"ser");
    let mut series = series_fragments
        .iter()
        .enumerate()
        .map(|(index, fragment)| parse_series(fragment, chart_type, index, theme))
        .collect::<Vec<_>>();
    for (index, item) in series.iter_mut().enumerate() {
        if item.color.is_empty() {
            item.color = palette_color(index, theme);
        }
        if item.point_colors.len() < item.values.len() {
            item.point_colors.resize(item.values.len(), None);
        }
    }
    let categories = series_fragments
        .iter()
        .find_map(|fragment| {
            let category = capture_direct_children(fragment, b"ser", b"cat")
                .into_iter()
                .next()?;
            let values = cache_strings(&category);
            (!values.is_empty()).then_some(values)
        })
        .unwrap_or_else(|| {
            let count = series
                .iter()
                .map(|item| item.values.len())
                .max()
                .unwrap_or(0);
            (1..=count).map(|index| index.to_string()).collect()
        });
    ChartData {
        relationship_id: None,
        source_part_name: None,
        chart_type,
        title,
        legend,
        categories,
        series,
        bar_direction,
        grouping,
        hole_size,
    }
}

fn parse_series(
    xml: &[u8],
    chart_type: ChartKind,
    fallback_index: usize,
    theme: &ThemeColorScheme,
) -> ChartSeries {
    let source_index =
        child_attribute(xml, b"ser", b"idx", b"val").and_then(|value| value.parse().ok());
    let name = capture_direct_children(xml, b"ser", b"tx")
        .into_iter()
        .next()
        .map_or_else(
            || format!("Series {}", fallback_index + 1),
            |value| {
                let value = text_content(&value);
                if value.is_empty() {
                    format!("Series {}", fallback_index + 1)
                } else {
                    value
                }
            },
        );
    let value_local = if matches!(chart_type, ChartKind::Unknown) {
        b"yVal".as_slice()
    } else {
        b"val".as_slice()
    };
    let values = capture_direct_children(xml, b"ser", value_local)
        .into_iter()
        .next()
        .map_or_else(Vec::new, |value| cache_numbers(&value));
    let sp_pr = capture_direct_children(xml, b"ser", b"spPr")
        .into_iter()
        .next();
    let color = sp_pr.as_deref().map_or_else(String::new, |shape| {
        if matches!(chart_type, ChartKind::Line) {
            capture_direct_children(shape, b"spPr", b"ln")
                .into_iter()
                .next()
                .as_deref()
                .and_then(|line| first_color(line, theme))
                .or_else(|| first_color(shape, theme))
                .unwrap_or_default()
        } else {
            first_color(shape, theme).unwrap_or_default()
        }
    });
    let mut point_colors = Vec::<Option<String>>::new();
    for point in capture_direct_children(xml, b"ser", b"dPt") {
        let Some(index) = child_attribute(&point, b"dPt", b"idx", b"val")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            continue;
        };
        if point_colors.len() <= index {
            point_colors.resize(index + 1, None);
        }
        point_colors[index] = capture_direct_children(&point, b"dPt", b"spPr")
            .into_iter()
            .next()
            .as_deref()
            .and_then(|shape| first_color(shape, theme));
    }
    ChartSeries {
        source_index,
        name,
        values,
        color,
        point_colors,
    }
}

fn parse_legend(xml: &[u8]) -> ChartLegend {
    let deleted =
        child_attribute(xml, b"legend", b"delete", b"val").is_some_and(|value| truthy(&value));
    let position = child_attribute(xml, b"legend", b"legendPos", b"val")
        .map_or_else(|| "right".into(), |value| legend_position(&value));
    let overlay =
        child_attribute(xml, b"legend", b"overlay", b"val").is_some_and(|value| truthy(&value));
    ChartLegend {
        visible: !deleted,
        position,
        overlay,
    }
}

fn legend_position(value: &str) -> String {
    match value {
        "l" => "left",
        "t" => "top",
        "b" => "bottom",
        "tr" => "topRight",
        _ => "right",
    }
    .into()
}

fn chart_kind(local: &str) -> ChartKind {
    match local {
        "lineChart" => ChartKind::Line,
        "barChart" => ChartKind::Bar,
        "pieChart" | "pie3DChart" => ChartKind::Pie,
        "doughnutChart" => ChartKind::Doughnut,
        "areaChart" => ChartKind::Area,
        _ => ChartKind::Unknown,
    }
}

fn capture_first_plot(xml: &[u8]) -> Option<(String, Vec<u8>)> {
    let supported = [
        b"lineChart".as_slice(),
        b"barChart".as_slice(),
        b"pieChart".as_slice(),
        b"pie3DChart".as_slice(),
        b"doughnutChart".as_slice(),
        b"areaChart".as_slice(),
    ];
    capture_first_direct_child_matching(xml, b"plotArea", |local| supported.contains(&local))
}

fn child_attribute(xml: &[u8], parent: &[u8], child: &[u8], attribute: &[u8]) -> Option<String> {
    let fragment = capture_direct_children(xml, parent, child)
        .into_iter()
        .next()?;
    let mut reader = Reader::from_reader(fragment.as_slice());
    loop {
        match reader.read_event() {
            Ok(Event::Start(element) | Event::Empty(element))
                if local_name(element.name().as_ref()) == child =>
            {
                return attr(&element, attribute);
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn cache_strings(xml: &[u8]) -> Vec<String> {
    let (count, points) = cache_points(xml);
    let length = count.max(points.keys().next_back().map_or(0, |index| index + 1));
    let mut output = vec![String::new(); length];
    for (index, value) in points {
        if let Some(slot) = output.get_mut(index) {
            *slot = value;
        }
    }
    output
}

fn cache_numbers(xml: &[u8]) -> Vec<Option<f64>> {
    let (count, points) = cache_points(xml);
    let length = count.max(points.keys().next_back().map_or(0, |index| index + 1));
    let mut output = vec![None; length];
    for (index, value) in points {
        if let Some(slot) = output.get_mut(index) {
            *slot = value.parse().ok();
        }
    }
    output
}

fn cache_points(xml: &[u8]) -> (usize, BTreeMap<usize, String>) {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut count = 0usize;
    let mut point = None;
    let mut in_value = false;
    let mut value = String::new();
    let mut points = BTreeMap::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match local_name(element.name().as_ref()) {
                b"pt" => point = attr(&element, b"idx").and_then(|value| value.parse().ok()),
                b"v" if point.is_some() => {
                    in_value = true;
                    value.clear();
                }
                _ => {}
            },
            Ok(Event::Empty(element)) => {
                if local_name(element.name().as_ref()) == b"ptCount" {
                    count = attr(&element, b"val")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(count);
                }
            }
            Ok(Event::Text(text)) if in_value => {
                if let Ok(decoded) = text.decode() {
                    value.push_str(&decoded);
                }
            }
            Ok(Event::End(element)) => match local_name(element.name().as_ref()) {
                b"v" if in_value => {
                    in_value = false;
                    if let Some(index) = point {
                        points.insert(index, value.clone());
                    }
                }
                b"pt" => point = None,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    (count, points)
}

fn text_content(xml: &[u8]) -> String {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut in_text = false;
    let mut rich = String::new();
    let mut fallback = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match local_name(element.name().as_ref()) {
                b"t" => in_text = true,
                b"v" if rich.is_empty() => in_text = true,
                _ => {}
            },
            Ok(Event::Text(text)) if in_text => {
                if let Ok(value) = text.decode() {
                    if rich.is_empty() {
                        fallback.push_str(&value);
                    } else {
                        rich.push_str(&value);
                    }
                }
            }
            Ok(Event::End(element)) => match local_name(element.name().as_ref()) {
                b"t" => {
                    if rich.is_empty() && !fallback.is_empty() {
                        rich.push_str(&fallback);
                        fallback.clear();
                    }
                    in_text = false;
                }
                b"v" => in_text = false,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    if rich.is_empty() {
        fallback
    } else {
        rich
    }
}

fn first_color(xml: &[u8], theme: &ThemeColorScheme) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    let mut solid_fill_depth = None;
    let mut depth = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let name = element.name();
                let local = local_name(name.as_ref());
                if local == b"solidFill" && solid_fill_depth.is_none() {
                    solid_fill_depth = Some(depth);
                } else if solid_fill_depth.is_some() {
                    if let Some(color) = chart_color(&element, theme) {
                        return Some(color);
                    }
                }
                depth += 1;
            }
            Ok(Event::Empty(element)) if solid_fill_depth.is_some() => {
                if let Some(color) = chart_color(&element, theme) {
                    return Some(color);
                }
            }
            Ok(Event::End(element)) => {
                depth = depth.saturating_sub(1);
                if local_name(element.name().as_ref()) == b"solidFill"
                    && solid_fill_depth == Some(depth)
                {
                    solid_fill_depth = None;
                }
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn chart_color(element: &BytesStart<'_>, theme: &ThemeColorScheme) -> Option<String> {
    match local_name(element.name().as_ref()) {
        b"srgbClr" => attr(element, b"val").map(|value| format!("#{value}")),
        b"sysClr" => attr(element, b"lastClr").map(|value| format!("#{value}")),
        b"schemeClr" => attr(element, b"val")
            .and_then(|name| theme.by_name(&name))
            .map(|value| value.to_string()),
        b"prstClr" => attr(element, b"val").and_then(|value| preset_color(&value)),
        _ => None,
    }
}

fn preset_color(value: &str) -> Option<String> {
    Some(
        match value {
            "black" => "#000000",
            "white" => "#FFFFFF",
            "red" => "#FF0000",
            "green" => "#008000",
            "blue" => "#0000FF",
            "yellow" => "#FFFF00",
            "gray" | "grey" => "#808080",
            _ => return None,
        }
        .into(),
    )
}

fn palette_color(index: usize, theme: &ThemeColorScheme) -> String {
    const FALLBACK: [&str; 6] = [
        "#4472C4", "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47",
    ];
    let name = format!("accent{}", index % 6 + 1);
    theme
        .by_name(&name)
        .map(|value| value.to_string())
        .unwrap_or_else(|| FALLBACK[index % FALLBACK.len()].into())
}

fn capture_direct_children(xml: &[u8], parent: &[u8], child: &[u8]) -> Vec<Vec<u8>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut stack = Vec::<Vec<u8>>::new();
    let mut capture: Option<(Writer<Vec<u8>>, usize)> = None;
    let mut output = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                if let Some((writer, depth)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Start(element.to_owned()));
                    *depth += 1;
                } else {
                    let local = local_name(element.name().as_ref()).to_vec();
                    if local == child && stack.last().is_some_and(|value| value == parent) {
                        let mut writer = Writer::new(Vec::new());
                        let _ = writer.write_event(Event::Start(element.to_owned()));
                        capture = Some((writer, 1));
                    } else {
                        stack.push(local);
                    }
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Empty(element.to_owned()));
                } else if local_name(element.name().as_ref()) == child
                    && stack.last().is_some_and(|value| value == parent)
                {
                    let mut writer = Writer::new(Vec::new());
                    let _ = writer.write_event(Event::Empty(element.to_owned()));
                    output.push(writer.into_inner());
                }
            }
            Ok(Event::Text(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::Text(text.to_owned()));
                }
            }
            Ok(Event::CData(text)) => {
                if let Some((writer, _)) = capture.as_mut() {
                    let _ = writer.write_event(Event::CData(text.to_owned()));
                }
            }
            Ok(Event::End(element)) => {
                if let Some((writer, depth)) = capture.as_mut() {
                    let _ = writer.write_event(Event::End(element.to_owned()));
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        let (writer, _) = capture.take().expect("capture exists at depth zero");
                        output.push(writer.into_inner());
                    }
                } else {
                    stack.pop();
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    output
}

fn capture_first_direct_child_matching(
    xml: &[u8],
    parent: &[u8],
    matches: impl Fn(&[u8]) -> bool,
) -> Option<(String, Vec<u8>)> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut stack = Vec::<Vec<u8>>::new();
    let mut capture: Option<(String, Writer<Vec<u8>>, usize)> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                if let Some((_, writer, depth)) = capture.as_mut() {
                    writer.write_event(Event::Start(element.to_owned())).ok()?;
                    *depth += 1;
                } else {
                    let local = local_name(element.name().as_ref()).to_vec();
                    if stack.last().is_some_and(|value| value == parent) && matches(&local) {
                        let mut writer = Writer::new(Vec::new());
                        writer.write_event(Event::Start(element.to_owned())).ok()?;
                        capture = Some((String::from_utf8_lossy(&local).into_owned(), writer, 1));
                    } else {
                        stack.push(local);
                    }
                }
            }
            Ok(Event::Empty(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if capture.is_none()
                    && stack.last().is_some_and(|value| value == parent)
                    && matches(&local)
                {
                    let mut writer = Writer::new(Vec::new());
                    writer.write_event(Event::Empty(element.to_owned())).ok()?;
                    return Some((
                        String::from_utf8_lossy(&local).into_owned(),
                        writer.into_inner(),
                    ));
                }
                if let Some((_, writer, _)) = capture.as_mut() {
                    writer.write_event(Event::Empty(element.to_owned())).ok()?;
                }
            }
            Ok(Event::Text(text)) => {
                if let Some((_, writer, _)) = capture.as_mut() {
                    writer.write_event(Event::Text(text.to_owned())).ok()?;
                }
            }
            Ok(Event::CData(text)) => {
                if let Some((_, writer, _)) = capture.as_mut() {
                    writer.write_event(Event::CData(text.to_owned())).ok()?;
                }
            }
            Ok(Event::End(element)) => {
                if let Some((_, writer, depth)) = capture.as_mut() {
                    writer.write_event(Event::End(element.to_owned())).ok()?;
                    *depth = depth.saturating_sub(1);
                    if *depth == 0 {
                        let (local, writer, _) = capture.take()?;
                        return Some((local, writer.into_inner()));
                    }
                } else {
                    stack.pop();
                }
            }
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

fn attr(element: &BytesStart<'_>, wanted: &[u8]) -> Option<String> {
    element
        .attributes()
        .with_checks(false)
        .flatten()
        .find_map(|attribute| {
            (local_name(attribute.key.as_ref()) == wanted)
                .then(|| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
        })
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn truthy(value: &str) -> bool {
    matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "on")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_graphic_frame_shape_id_to_chart_relationship() {
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:c="c" xmlns:r="r"><p:cSld><p:spTree>
          <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="8" name="Chart 7"/></p:nvGraphicFramePr><p:xfrm><a:off x="1" y="2"/><a:ext cx="3" cy="4"/></p:xfrm><a:graphic><a:graphicData uri="chart"><c:chart r:id="rId5"/></a:graphicData></a:graphic></p:graphicFrame>
          <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="9" name="Table 8"/></p:nvGraphicFramePr><a:graphic><a:graphicData uri="table"><a:tbl/></a:graphicData></a:graphic></p:graphicFrame>
        </p:spTree></p:cSld></p:sld>"#;
        assert_eq!(
            parse_chart_frame_references(xml),
            vec![ChartFrameReference {
                shape_id: 8,
                relationship_id: "rId5".into()
            }]
        );
    }

    #[test]
    fn parses_line_chart_title_legend_categories_series_and_colors() {
        let xml = br#"<c:chartSpace xmlns:c="c" xmlns:a="a"><c:chart><c:title><c:tx><c:rich><a:p><a:r><a:t>Revenue</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:lineChart><c:grouping val="standard"/>
          <c:ser><c:idx val="2"/><c:tx><c:v>North</c:v></c:tx><c:spPr><a:ln><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:ln></c:spPr><c:cat><c:strRef><c:strCache><c:ptCount val="2"/><c:pt idx="0"><c:v>Jan</c:v></c:pt><c:pt idx="1"><c:v>Feb</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>10.5</c:v></c:pt><c:pt idx="1"><c:v>12</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>
          <c:ser><c:idx val="3"/><c:tx><c:v>South</c:v></c:tx><c:spPr><a:ln><a:solidFill><a:schemeClr val="accent2"/></a:solidFill></a:ln></c:spPr><c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>Jan</c:v></c:pt><c:pt idx="1"><c:v>Feb</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>8</c:v></c:pt><c:pt idx="1"><c:v>14</c:v></c:pt></c:numLit></c:val></c:ser>
        </c:lineChart></c:plotArea><c:legend><c:legendPos val="b"/><c:overlay val="0"/></c:legend></c:chart></c:chartSpace>"#;
        let theme = ThemeColorScheme::default();
        let chart = parse_chart_xml(xml, &theme);
        assert_eq!(chart.chart_type, ChartKind::Line);
        assert_eq!(chart.title, "Revenue");
        assert_eq!(chart.legend.position, "bottom");
        assert!(chart.legend.visible);
        assert_eq!(chart.categories, vec!["Jan", "Feb"]);
        assert_eq!(chart.series.len(), 2);
        assert_eq!(chart.series[0].source_index, Some(2));
        assert_eq!(chart.series[0].name, "North");
        assert_eq!(chart.series[0].values, vec![Some(10.5), Some(12.0)]);
        assert_eq!(chart.series[0].color, "#112233");
        assert_eq!(chart.series[1].color, theme.accent2.to_string());
    }

    #[test]
    fn recognizes_required_plot_types_and_doughnut_hole() {
        let theme = ThemeColorScheme::default();
        for (local, expected) in [
            ("barChart", ChartKind::Bar),
            ("pieChart", ChartKind::Pie),
            ("doughnutChart", ChartKind::Doughnut),
            ("areaChart", ChartKind::Area),
        ] {
            let detail = if local == "barChart" {
                "<c:barDir val=\"bar\"/>"
            } else if local == "doughnutChart" {
                "<c:holeSize val=\"65\"/>"
            } else {
                ""
            };
            let xml = format!(
                "<c:chartSpace xmlns:c=\"c\"><c:chart><c:plotArea><c:{local}>{detail}<c:ser><c:tx><c:v>S</c:v></c:tx><c:cat><c:strLit><c:pt idx=\"0\"><c:v>A</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:pt idx=\"0\"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser></c:{local}></c:plotArea></c:chart></c:chartSpace>"
            );
            let chart = parse_chart_xml(xml.as_bytes(), &theme);
            assert_eq!(chart.chart_type, expected);
            assert_eq!(chart.series[0].values, vec![Some(1.0)]);
            if expected == ChartKind::Bar {
                assert_eq!(chart.bar_direction.as_deref(), Some("bar"));
            }
            if expected == ChartKind::Doughnut {
                assert_eq!(chart.hole_size, Some(0.65));
            }
        }
    }
}

use std::collections::{HashMap, HashSet};

use quick_xml::events::attributes::Attributes;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{PptxError, PptxResult};
use crate::xml_util::local_name;

use super::SmartArtNode;

/// Extract a `parOf` connection (`srcId`, `destId`) from XML attributes.
///
/// Returns `Some((src, dest))` when the element has `type="parOf"` and
/// both `srcId` and `destId` are present and non-empty.
///
/// # Errors
///
/// Returns an error if any attribute is malformed.
fn parse_cxn_attrs(attrs: Attributes<'_>) -> PptxResult<Option<(String, String)>> {
    let mut ctype = String::new();
    let mut src = String::new();
    let mut dest = String::new();
    for attr_result in attrs {
        let attr = attr_result.map_err(PptxError::XmlAttr)?;
        match attr.key.as_ref() {
            b"type" => {
                ctype = String::from_utf8_lossy(&attr.value).into_owned();
            }
            b"srcId" => {
                src = String::from_utf8_lossy(&attr.value).into_owned();
            }
            b"destId" => {
                dest = String::from_utf8_lossy(&attr.value).into_owned();
            }
            _ => {}
        }
    }
    if ctype == "parOf" && !src.is_empty() && !dest.is_empty() {
        Ok(Some((src, dest)))
    } else {
        Ok(None)
    }
}

/// Parse `SmartArt` nodes from diagram data XML.
///
/// Reads `<dgm:pt>` elements from the `<dgm:ptLst>` in the diagram data
/// and extracts their text content from nested `<a:t>` elements. The
/// parent-child hierarchy is reconstructed from `<dgm:cxn>` connections
/// where `type="parOf"`.
///
/// Returns a list of root-level nodes with their children populated.
///
/// # Errors
///
/// Returns an error if the XML is malformed.
#[allow(clippy::similar_names)]
pub fn parse_smartart_nodes(data_xml: &[u8]) -> PptxResult<Vec<SmartArtNode>> {
    let mut reader = Reader::from_reader(data_xml);
    reader.config_mut().trim_text(true);

    // Collect points: (modelId, type, text)
    let mut points: Vec<(String, String, String)> = Vec::new();
    // Collect parOf connections: (srcId=parent, destId=child)
    let mut connections: Vec<(String, String)> = Vec::new();

    let mut buf = Vec::new();
    let mut in_pt = false;
    let mut pt_model_id = String::new();
    let mut pt_type = String::new();
    let mut pt_text = String::new();
    let mut in_t = false;
    let mut pt_depth = 0u32;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let ln = local_name(&name_bytes);
                if ln == b"pt" && !in_pt {
                    in_pt = true;
                    pt_depth = 1;
                    pt_model_id.clear();
                    pt_type.clear();
                    pt_text.clear();
                    for attr_result in e.attributes() {
                        let attr = attr_result.map_err(PptxError::XmlAttr)?;
                        match attr.key.as_ref() {
                            b"modelId" => {
                                pt_model_id = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                            b"type" => {
                                pt_type = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                            _ => {}
                        }
                    }
                } else if in_pt {
                    pt_depth += 1;
                    if ln == b"t" {
                        in_t = true;
                    }
                } else if ln == b"cxn" {
                    if let Some(pair) = parse_cxn_attrs(e.attributes())? {
                        connections.push(pair);
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let ln = local_name(&name_bytes);
                if ln == b"cxn" {
                    if let Some(pair) = parse_cxn_attrs(e.attributes())? {
                        connections.push(pair);
                    }
                }
            }
            Ok(Event::Text(ref t)) => {
                if in_t {
                    if let Ok(text) = t.decode() {
                        pt_text.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let ln = local_name(&name_bytes);
                if in_pt {
                    if ln == b"t" {
                        in_t = false;
                    }
                    pt_depth -= 1;
                    if pt_depth == 0 {
                        // Skip presentation, sibTrans, and parTrans nodes
                        if pt_type != "pres" && pt_type != "sibTrans" && pt_type != "parTrans" {
                            points.push((pt_model_id.clone(), pt_type.clone(), pt_text.clone()));
                        }
                        in_pt = false;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(PptxError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(build_tree(points, connections))
}

const MAX_SMARTART_DEPTH: usize = 100;

/// Recursively build a single `SmartArt` node from the maps.
///
/// `visited` tracks node IDs on the current path to detect cycles.
/// Returns `None` when a cycle is detected or `depth` exceeds `MAX_SMARTART_DEPTH`.
fn build_node(
    id: &str,
    node_map: &HashMap<String, SmartArtNode>,
    children_of: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    depth: usize,
) -> Option<SmartArtNode> {
    // Guard: depth limit
    if depth > MAX_SMARTART_DEPTH {
        return None;
    }
    // Guard: cycle detection
    if !visited.insert(id.to_owned()) {
        return None;
    }

    let default = SmartArtNode {
        text: String::new(),
        children: Vec::new(),
    };
    let base = node_map.get(id).unwrap_or(&default);
    let children = children_of
        .get(id)
        .map(|child_ids| {
            child_ids
                .iter()
                .filter_map(|cid| build_node(cid, node_map, children_of, visited, depth + 1))
                .collect()
        })
        .unwrap_or_default();

    visited.remove(id);

    Some(SmartArtNode {
        text: base.text.clone(),
        children,
    })
}

/// Build the `SmartArt` node tree from collected points and connections.
fn build_tree(
    points: Vec<(String, String, String)>,
    connections: Vec<(String, String)>,
) -> Vec<SmartArtNode> {
    // Build node map
    let mut node_map: HashMap<String, SmartArtNode> = HashMap::with_capacity(points.len());
    for (model_id, _pt_type, text) in points {
        node_map.insert(
            model_id,
            SmartArtNode {
                text,
                children: Vec::new(),
            },
        );
    }

    // Build parent -> children mapping from connections
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut has_parent: HashMap<String, bool> = HashMap::new();
    for (src_id, dest_id) in connections {
        if node_map.contains_key(&dest_id) && node_map.contains_key(&src_id) {
            children_of.entry(src_id).or_default().push(dest_id.clone());
            has_parent.insert(dest_id, true);
        }
    }

    // Root nodes: those that don't appear as a child in any connection
    node_map
        .keys()
        .filter(|id| !has_parent.contains_key(id.as_str()))
        .filter_map(|id| {
            let mut visited = HashSet::new();
            build_node(id, &node_map, &children_of, &mut visited, 0)
        })
        .collect()
}

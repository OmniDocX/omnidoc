use std::path::{Path, PathBuf};
use std::time::Instant;

use pptx::shapes::{Shape, ShapeTree};
use pptx::Presentation;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;
use unippt_core::{import_pptx_compact, ObjectKind, SceneObject};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileAudit {
    file: String,
    bytes: u64,
    import_ms: u128,
    slides: usize,
    raw_shape_elements: usize,
    source_shapes: usize,
    scene_objects: usize,
    painted_objects: usize,
    unknown_objects: usize,
    geometry_anomalies: usize,
    suspicious_slides: Vec<SlideAudit>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SlideAudit {
    slide: usize,
    raw_shape_elements: usize,
    source_shapes: usize,
    scene_objects: usize,
    painted_objects: usize,
    unknown_objects: usize,
    geometry_anomalies: usize,
    geometry_anomaly_details: Vec<GeometryAnomalyAudit>,
    top_level_source_shapes: usize,
    top_level_scene_objects: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeometryAnomalyAudit {
    id: String,
    name: String,
    width: f64,
    height: f64,
    child_span_area: f64,
    children: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: audit_corpus PPTX_OR_DIRECTORY")?,
    );
    let mut files = pptx_files(&root)?;
    files.sort_by_key(|path| numeric_stem(path).unwrap_or(u32::MAX));
    for path in files {
        println!("{}", serde_json::to_string(&audit_file(&path))?);
    }
    Ok(())
}

fn audit_file(path: &Path) -> FileAudit {
    let bytes_len = std::fs::metadata(path).map_or(0, |metadata| metadata.len());
    let started = Instant::now();
    let result = (|| {
        let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
        let presentation = Presentation::from_bytes(&bytes).map_err(|error| error.to_string())?;
        let refs = presentation.slides().map_err(|error| error.to_string())?;
        let compact = import_pptx_compact(&bytes).map_err(|error| error.to_string())?;
        let mut raw_shape_elements = 0usize;
        let mut source_shapes = 0usize;
        let mut scene_objects = 0usize;
        let mut painted_objects = 0usize;
        let mut unknown_objects = 0usize;
        let mut geometry_anomalies = 0usize;
        let mut suspicious_slides = Vec::new();
        for (index, slide_ref) in refs.iter().enumerate() {
            let xml = presentation
                .slide_xml(slide_ref)
                .map_err(|error| error.to_string())?;
            let tree = ShapeTree::from_slide_xml(xml).map_err(|error| error.to_string())?;
            let raw_count = count_raw_shape_elements(xml);
            let source_count = tree.shapes.iter().map(count_shape).sum::<usize>();
            let scene = &compact.deck.slides[index];
            let scene_roots = scene
                .objects
                .iter()
                .chain(&scene.layout_objects)
                .chain(&scene.master_objects);
            let scene_count = scene_roots.clone().map(count_scene_object).sum::<usize>();
            let painted_count = scene_roots.clone().map(count_painted_object).sum::<usize>();
            let unknown_count = scene_roots.clone().map(count_unknown_object).sum::<usize>();
            let anomaly_count = scene_roots.map(count_geometry_anomalies).sum::<usize>();
            let anomaly_details = scene
                .objects
                .iter()
                .chain(&scene.layout_objects)
                .chain(&scene.master_objects)
                .flat_map(geometry_anomaly_details)
                .collect::<Vec<_>>();
            raw_shape_elements += raw_count;
            source_shapes += source_count;
            scene_objects += scene_count;
            painted_objects += painted_count;
            unknown_objects += unknown_count;
            geometry_anomalies += anomaly_count;
            if (raw_count > 0 && painted_count == 0)
                || source_count.saturating_mul(2) < raw_count
                || anomaly_count > 0
            {
                suspicious_slides.push(SlideAudit {
                    slide: index + 1,
                    raw_shape_elements: raw_count,
                    source_shapes: source_count,
                    scene_objects: scene_count,
                    painted_objects: painted_count,
                    unknown_objects: unknown_count,
                    geometry_anomalies: anomaly_count,
                    geometry_anomaly_details: anomaly_details,
                    top_level_source_shapes: tree.shapes.len(),
                    top_level_scene_objects: scene.objects.len()
                        + scene.layout_objects.len()
                        + scene.master_objects.len(),
                });
            }
        }
        Ok::<_, String>((
            compact.deck.slides.len(),
            raw_shape_elements,
            source_shapes,
            scene_objects,
            painted_objects,
            unknown_objects,
            geometry_anomalies,
            suspicious_slides,
        ))
    })();
    match result {
        Ok((
            slides,
            raw_shape_elements,
            source_shapes,
            scene_objects,
            painted_objects,
            unknown_objects,
            geometry_anomalies,
            suspicious_slides,
        )) => FileAudit {
            file: path.display().to_string(),
            bytes: bytes_len,
            import_ms: started.elapsed().as_millis(),
            slides,
            raw_shape_elements,
            source_shapes,
            scene_objects,
            painted_objects,
            unknown_objects,
            geometry_anomalies,
            suspicious_slides,
            error: None,
        },
        Err(error) => FileAudit {
            file: path.display().to_string(),
            bytes: bytes_len,
            import_ms: started.elapsed().as_millis(),
            slides: 0,
            raw_shape_elements: 0,
            source_shapes: 0,
            scene_objects: 0,
            painted_objects: 0,
            unknown_objects: 0,
            geometry_anomalies: 0,
            suspicious_slides: Vec::new(),
            error: Some(error),
        },
    }
}

fn count_shape(shape: &Shape) -> usize {
    1 + match shape {
        Shape::GroupShape(group) => group.shapes.iter().map(count_shape).sum(),
        _ => 0,
    }
}

fn count_scene_object(object: &SceneObject) -> usize {
    1 + object
        .children
        .iter()
        .map(count_scene_object)
        .sum::<usize>()
}

fn count_painted_object(object: &SceneObject) -> usize {
    let own = usize::from(
        object.kind != ObjectKind::Group
            && object.kind != ObjectKind::Unknown
            && (object.frame.width.abs() > 0.01 || object.frame.height.abs() > 0.01),
    );
    own + object
        .children
        .iter()
        .map(count_painted_object)
        .sum::<usize>()
}

fn count_unknown_object(object: &SceneObject) -> usize {
    usize::from(object.kind == ObjectKind::Unknown)
        + object
            .children
            .iter()
            .map(count_unknown_object)
            .sum::<usize>()
}

fn count_geometry_anomalies(object: &SceneObject) -> usize {
    let own = if object.kind == ObjectKind::Group && !object.children.is_empty() {
        let group_area = (object.frame.width * object.frame.height).abs();
        let child_span_area = child_span_area(object);
        usize::from(
            group_area > 100.0 && child_span_area > 0.0 && child_span_area < group_area * 0.00001,
        )
    } else {
        0
    };
    own + object
        .children
        .iter()
        .map(count_geometry_anomalies)
        .sum::<usize>()
}

fn geometry_anomaly_details(object: &SceneObject) -> Vec<GeometryAnomalyAudit> {
    let mut details = Vec::new();
    if object.kind == ObjectKind::Group && !object.children.is_empty() {
        let group_area = (object.frame.width * object.frame.height).abs();
        let child_span_area = child_span_area(object);
        if group_area > 100.0 && child_span_area > 0.0 && child_span_area < group_area * 0.00001 {
            details.push(GeometryAnomalyAudit {
                id: object.id.clone(),
                name: object.name.clone(),
                width: object.frame.width,
                height: object.frame.height,
                child_span_area,
                children: object
                    .children
                    .iter()
                    .map(|child| {
                        format!("{}:{}x{}", child.id, child.frame.width, child.frame.height)
                    })
                    .collect(),
            });
        }
    }
    for child in &object.children {
        details.extend(geometry_anomaly_details(child));
    }
    details
}

fn child_span_area(object: &SceneObject) -> f64 {
    let min_x = object
        .children
        .iter()
        .map(|child| child.frame.x.min(child.frame.x + child.frame.width))
        .fold(f64::INFINITY, f64::min);
    let max_x = object
        .children
        .iter()
        .map(|child| child.frame.x.max(child.frame.x + child.frame.width))
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = object
        .children
        .iter()
        .map(|child| child.frame.y.min(child.frame.y + child.frame.height))
        .fold(f64::INFINITY, f64::min);
    let max_y = object
        .children
        .iter()
        .map(|child| child.frame.y.max(child.frame.y + child.frame.height))
        .fold(f64::NEG_INFINITY, f64::max);
    ((max_x - min_x) * (max_y - min_y)).abs()
}

fn count_raw_shape_elements(xml: &[u8]) -> usize {
    let mut reader = Reader::from_reader(xml);
    let mut count = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                let name = event.name();
                let local = name
                    .as_ref()
                    .rsplit(|byte| *byte == b':')
                    .next()
                    .unwrap_or_default();
                if matches!(
                    local,
                    b"sp" | b"pic" | b"graphicFrame" | b"cxnSp" | b"grpSp"
                ) {
                    count += 1;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    count
}

fn pptx_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    std::fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
        })
        .collect::<Vec<_>>()
        .pipe(Ok)
}

fn numeric_stem(path: &Path) -> Option<u32> {
    path.file_stem()?.to_str()?.parse().ok()
}

trait Pipe: Sized {
    fn pipe<T>(self, function: impl FnOnce(Self) -> T) -> T {
        function(self)
    }
}
impl<T> Pipe for T {}

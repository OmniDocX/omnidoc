//! Loss-aware native PowerPoint media discovery.
//!
//! PowerPoint stores an audio/video object as an ordinary picture (the poster
//! frame) and keeps the playable asset below `p:nvPr`.  The legacy
//! `a:audioFile`/`a:videoFile` relationship and the Office 2010 `p14:media`
//! relationship commonly point at the same package part.  We resolve both but
//! never mutate either relationship or the binary part; the exporter can thus
//! return the original package byte-for-byte when the scene is untouched.

use std::collections::HashMap;

use pptx::opc::{OpcPackage, Part};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::compact::AssetSink;
use crate::model::{MediaData, MediaKind};

#[derive(Debug, Default)]
struct PictureMediaRef {
    shape_id: Option<u32>,
    kind: Option<MediaKind>,
    legacy_relationship_id: Option<String>,
    embedded_relationship_id: Option<String>,
    trim_start_ms: Option<u64>,
    trim_end_ms: Option<u64>,
}

#[derive(Debug)]
struct PictureContext {
    depth: usize,
    media: PictureMediaRef,
}

#[derive(Debug)]
struct TimingMediaContext {
    depth: usize,
    shape_id: Option<u32>,
    volume: Option<f64>,
    loop_playback: bool,
    play_across_slides: bool,
    show_when_stopped: Option<bool>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TimingMediaSettings {
    volume: Option<f64>,
    loop_playback: bool,
    play_across_slides: bool,
    show_when_stopped: Option<bool>,
}

/// Parse media references for slide-local picture shapes.
///
/// The returned map is keyed by native `p:cNvPr/@id`, making it safe to apply
/// after the ordinary shape importer has built nested scene objects.
#[cfg(test)]
pub(crate) fn parse_slide_media(
    slide_xml: &[u8],
    slide_part: &Part,
    package: &OpcPackage,
) -> HashMap<u32, MediaData> {
    parse_slide_media_with_assets(slide_xml, slide_part, package, &mut AssetSink::inline())
}

pub(crate) fn parse_slide_media_with_assets(
    slide_xml: &[u8],
    slide_part: &Part,
    package: &OpcPackage,
    asset_sink: &mut AssetSink,
) -> HashMap<u32, MediaData> {
    let timing = parse_media_timing(slide_xml);
    let mut reader = Reader::from_reader(slide_xml);
    reader.config_mut().trim_text(true);
    let mut depth = 0usize;
    let mut picture: Option<PictureContext> = None;
    let mut output = HashMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                depth += 1;
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"pic" {
                    picture = Some(PictureContext {
                        depth,
                        media: PictureMediaRef::default(),
                    });
                } else if let Some(context) = picture.as_mut() {
                    update_picture_media(&mut context.media, &local, &element);
                }
            }
            Ok(Event::Empty(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if let Some(context) = picture.as_mut() {
                    update_picture_media(&mut context.media, &local, &element);
                }
            }
            Ok(Event::End(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if local == b"pic"
                    && picture
                        .as_ref()
                        .is_some_and(|context| context.depth == depth)
                {
                    if let Some(context) = picture.take() {
                        if let Some((shape_id, mut media)) =
                            resolve_picture_media(context.media, slide_part, package, asset_sink)
                        {
                            if let Some(settings) = timing.get(&shape_id) {
                                apply_timing_settings(&mut media, *settings);
                            }
                            output.insert(shape_id, media);
                        }
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    output
}

fn update_picture_media(media: &mut PictureMediaRef, local: &[u8], element: &BytesStart<'_>) {
    match local {
        b"cNvPr" if media.shape_id.is_none() => media.shape_id = attr_u32(element, b"id"),
        b"audioFile" => {
            media.kind = Some(MediaKind::Audio);
            media.legacy_relationship_id = attr(element, b"link");
        }
        b"videoFile" => {
            media.kind = Some(MediaKind::Video);
            media.legacy_relationship_id = attr(element, b"link");
        }
        b"media" => media.embedded_relationship_id = attr(element, b"embed"),
        b"trim" => {
            media.trim_start_ms = attr_f64(element, b"st").map(round_milliseconds);
            media.trim_end_ms = attr_f64(element, b"end").map(round_milliseconds);
        }
        _ => {}
    }
}

fn resolve_picture_media(
    reference: PictureMediaRef,
    slide_part: &Part,
    package: &OpcPackage,
    asset_sink: &mut AssetSink,
) -> Option<(u32, MediaData)> {
    let shape_id = reference.shape_id?;
    let relationship_id = reference
        .embedded_relationship_id
        .as_deref()
        .or(reference.legacy_relationship_id.as_deref())?;
    let relationship = slide_part.rels.get(relationship_id)?;

    let mut kind = reference.kind.unwrap_or(MediaKind::Audio);
    let mut asset = None;
    let mut mime_type = None;
    let mut source_part_name = None;
    if relationship.is_external {
        if !relationship.target_ref.eq_ignore_ascii_case("NULL")
            && !relationship.target_ref.trim().is_empty()
        {
            asset = Some(relationship.target_ref.clone());
            mime_type = mime_from_target(&relationship.target_ref).map(str::to_string);
        }
    } else if let Ok(partname) = relationship.target_partname(slide_part.partname.base_uri()) {
        if let Some(part) = package.part(&partname) {
            kind = kind_from_mime(&part.content_type).unwrap_or(kind);
            mime_type = Some(part.content_type.clone());
            source_part_name = Some(partname.to_string());
            asset = Some(asset_sink.insert(&part.content_type, &part.blob));
        }
    }

    Some((
        shape_id,
        MediaData {
            kind,
            asset,
            mime_type,
            playback_asset: None,
            playback_mime_type: None,
            source_part_name,
            relationship_id: reference.embedded_relationship_id,
            legacy_relationship_id: reference.legacy_relationship_id,
            trim_start_ms: reference.trim_start_ms,
            trim_end_ms: reference.trim_end_ms,
            volume: 1.0,
            loop_playback: false,
            play_across_slides: false,
            show_when_stopped: true,
        },
    ))
}

fn parse_media_timing(xml: &[u8]) -> HashMap<u32, TimingMediaSettings> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut depth = 0usize;
    let mut context: Option<TimingMediaContext> = None;
    let mut output = HashMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                depth += 1;
                let local = local_name(element.name().as_ref()).to_vec();
                if matches!(local.as_slice(), b"audio" | b"video") {
                    context = Some(TimingMediaContext {
                        depth,
                        shape_id: None,
                        volume: None,
                        loop_playback: false,
                        play_across_slides: false,
                        show_when_stopped: None,
                    });
                } else if let Some(context) = context.as_mut() {
                    update_media_timing(context, &local, &element);
                }
            }
            Ok(Event::Empty(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if let Some(context) = context.as_mut() {
                    update_media_timing(context, &local, &element);
                }
            }
            Ok(Event::End(element)) => {
                let local = local_name(element.name().as_ref()).to_vec();
                if matches!(local.as_slice(), b"audio" | b"video")
                    && context.as_ref().is_some_and(|value| value.depth == depth)
                {
                    if let Some(value) = context.take() {
                        if let Some(shape_id) = value.shape_id {
                            output.insert(
                                shape_id,
                                TimingMediaSettings {
                                    volume: value.volume,
                                    loop_playback: value.loop_playback,
                                    play_across_slides: value.play_across_slides,
                                    show_when_stopped: value.show_when_stopped,
                                },
                            );
                        }
                    }
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    output
}

fn update_media_timing(context: &mut TimingMediaContext, local: &[u8], element: &BytesStart<'_>) {
    match local {
        b"cMediaNode" => {
            context.volume =
                attr_f64(element, b"vol").map(|value| (value / 100_000.0).clamp(0.0, 1.0));
            context.play_across_slides =
                attr_u32(element, b"numSld").is_some_and(|value| value > 1);
            context.show_when_stopped = attr_bool(element, b"showWhenStopped");
        }
        b"cTn" => {
            context.loop_playback |= attr(element, b"repeatCount")
                .is_some_and(|value| value.eq_ignore_ascii_case("indefinite"));
        }
        b"spTgt" => context.shape_id = context.shape_id.or_else(|| attr_u32(element, b"spid")),
        _ => {}
    }
}

fn apply_timing_settings(media: &mut MediaData, settings: TimingMediaSettings) {
    if let Some(volume) = settings.volume {
        media.volume = volume;
    }
    media.loop_playback = settings.loop_playback;
    media.play_across_slides = settings.play_across_slides;
    if let Some(show_when_stopped) = settings.show_when_stopped {
        media.show_when_stopped = show_when_stopped;
    }
}

fn kind_from_mime(mime: &str) -> Option<MediaKind> {
    if mime.starts_with("audio/") {
        Some(MediaKind::Audio)
    } else if mime.starts_with("video/") {
        Some(MediaKind::Video)
    } else {
        None
    }
}

fn mime_from_target(target: &str) -> Option<&'static str> {
    let path = target.split(['?', '#']).next().unwrap_or(target);
    let extension = path.rsplit('.').next()?.to_ascii_lowercase();
    match extension.as_str() {
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "m4a" | "aac" => Some("audio/mp4"),
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "avi" => Some("video/x-msvideo"),
        _ => None,
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

fn attr_u32(element: &BytesStart<'_>, wanted: &[u8]) -> Option<u32> {
    attr(element, wanted)?.parse().ok()
}

fn attr_f64(element: &BytesStart<'_>, wanted: &[u8]) -> Option<f64> {
    attr(element, wanted)?.parse().ok()
}

fn attr_bool(element: &BytesStart<'_>, wanted: &[u8]) -> Option<bool> {
    match attr(element, wanted)?.as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn round_milliseconds(value: f64) -> u64 {
    value.max(0.0).round() as u64
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use pptx::opc::pack_uri::PackURI;
    use pptx::opc::relationship::Relationships;
    use pptx::opc::{OpcPackage, Part};

    use super::*;

    fn fixture_package() -> (OpcPackage, Part) {
        let mut package = OpcPackage::new().expect("default package");
        let media_partname = PackURI::new("/ppt/media/media1.mp3").unwrap();
        package.put_part(Part::new(
            media_partname,
            "audio/mpeg",
            b"ID3 fixture audio".to_vec(),
        ));
        let mut relationships = Relationships::new("/ppt/slides");
        relationships.add_relationship(
            "http://schemas.microsoft.com/office/2007/relationships/media",
            "../media/media1.mp3",
            false,
        );
        relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio",
            "../media/media1.mp3",
            false,
        );
        let slide = Part::with_rels(
            PackURI::new("/ppt/slides/slide1.xml").unwrap(),
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
            Vec::new(),
            relationships,
        );
        (package, slide)
    }

    #[test]
    fn resolves_audio_asset_trim_and_native_playback_settings() {
        let (package, slide) = fixture_package();
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r" xmlns:p14="p14"><p:cSld><p:spTree><p:pic><p:nvPicPr><p:cNvPr id="42" name="Audio 1"/><p:cNvPicPr/><p:nvPr><a:audioFile r:link="rId2"/><p:extLst><p:ext><p14:media r:embed="rId1"><p14:trim st="125.4" end="2863.0625"/></p14:media></p:ext></p:extLst></p:nvPr></p:nvPicPr></p:pic></p:spTree></p:cSld><p:timing><p:audio><p:cMediaNode vol="80000" numSld="999" showWhenStopped="0"><p:cTn repeatCount="indefinite"/><p:tgtEl><p:spTgt spid="42"/></p:tgtEl></p:cMediaNode></p:audio></p:timing></p:sld>"#;
        let parsed = parse_slide_media(xml, &slide, &package);
        let media = parsed.get(&42).expect("media");
        assert_eq!(media.kind, MediaKind::Audio);
        assert_eq!(media.mime_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(
            media.source_part_name.as_deref(),
            Some("/ppt/media/media1.mp3")
        );
        assert_eq!(media.relationship_id.as_deref(), Some("rId1"));
        assert_eq!(media.legacy_relationship_id.as_deref(), Some("rId2"));
        assert_eq!(media.trim_start_ms, Some(125));
        assert_eq!(media.trim_end_ms, Some(2863));
        assert!((media.volume - 0.8).abs() < f64::EPSILON);
        assert!(media.loop_playback);
        assert!(media.play_across_slides);
        assert!(!media.show_when_stopped);
        assert!(media
            .asset
            .as_deref()
            .unwrap()
            .starts_with("data:audio/mpeg;base64,"));
    }

    #[test]
    fn retains_broken_external_audio_as_relationship_evidence() {
        let package = OpcPackage::new().expect("default package");
        let mut relationships = Relationships::new("/ppt/slides");
        relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio",
            "NULL",
            true,
        );
        let slide = Part::with_rels(
            PackURI::new("/ppt/slides/slide1.xml").unwrap(),
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
            Vec::new(),
            relationships,
        );
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r"><p:cSld><p:spTree><p:pic><p:nvPicPr><p:cNvPr id="7"/><p:nvPr><a:audioFile r:link="rId1"/></p:nvPr></p:nvPicPr></p:pic></p:spTree></p:cSld></p:sld>"#;
        let media = parse_slide_media(xml, &slide, &package).remove(&7).unwrap();
        assert_eq!(media.legacy_relationship_id.as_deref(), Some("rId1"));
        assert!(media.asset.is_none());
    }

    #[test]
    fn resolves_slide_local_video_without_replacing_its_poster_picture() {
        let mut package = OpcPackage::new().expect("default package");
        package.put_part(Part::new(
            PackURI::new("/ppt/media/media2.mp4").unwrap(),
            "video/mp4",
            b"fixture mp4".to_vec(),
        ));
        let mut relationships = Relationships::new("/ppt/slides");
        relationships.add_relationship(
            "http://schemas.microsoft.com/office/2007/relationships/media",
            "../media/media2.mp4",
            false,
        );
        relationships.add_relationship(
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video",
            "../media/media2.mp4",
            false,
        );
        let slide = Part::with_rels(
            PackURI::new("/ppt/slides/slide1.xml").unwrap(),
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
            Vec::new(),
            relationships,
        );
        let xml = br#"<p:sld xmlns:p="p" xmlns:a="a" xmlns:r="r" xmlns:p14="p14"><p:cSld><p:spTree><p:pic><p:nvPicPr><p:cNvPr id="9"/><p:nvPr><a:videoFile r:link="rId2"/><p:extLst><p:ext><p14:media r:embed="rId1"/></p:ext></p:extLst></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rIdPoster"/></p:blipFill></p:pic></p:spTree></p:cSld></p:sld>"#;
        let media = parse_slide_media(xml, &slide, &package).remove(&9).unwrap();
        assert_eq!(media.kind, MediaKind::Video);
        assert_eq!(media.mime_type.as_deref(), Some("video/mp4"));
        assert!(media
            .asset
            .as_deref()
            .is_some_and(|asset| asset.starts_with("data:video/mp4;base64,")));
        // Poster image resolution belongs to the ordinary picture importer;
        // native media discovery only augments that same SceneObject.
        assert_eq!(media.relationship_id.as_deref(), Some("rId1"));
        assert_eq!(media.legacy_relationship_id.as_deref(), Some("rId2"));
    }
}

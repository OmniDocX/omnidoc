use md5::{Digest, Md5};
use pptx::Presentation;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use sha2::Sha256;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::compact::AssetSink;
use crate::model::EmbeddedFont;

const RT_OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

#[derive(Default)]
struct PendingFont {
    family: String,
    font_key: Option<String>,
    faces: Vec<PendingFace>,
}

struct PendingFace {
    relationship_id: String,
    weight: u16,
    style: &'static str,
    font_key: Option<String>,
}

struct PreparedFont {
    bytes: Vec<u8>,
    mime_type: &'static str,
    format: &'static str,
}

struct FontJob {
    family: String,
    relationship_id: String,
    source_part_name: String,
    font_key: Option<String>,
    weight: u16,
    style: &'static str,
    alias_seed: String,
    bytes: Vec<u8>,
}

/// Project PresentationML embedded font relationships into browser-ready font
/// faces. The native font parts remain untouched in the backing OPC package.
pub(crate) fn extract_with_assets(
    presentation: &Presentation,
    asset_sink: &mut AssetSink,
) -> Vec<EmbeddedFont> {
    let package = presentation.package();
    let Ok(presentation_part) = package.part_by_reltype(RT_OFFICE_DOCUMENT) else {
        return Vec::new();
    };
    let pending = parse_embedded_font_list(&presentation_part.blob);
    let mut jobs = Vec::new();

    for entry in pending {
        if entry.family.trim().is_empty() {
            continue;
        }
        for face in entry.faces {
            let Some(relationship) = presentation_part.rels.get(&face.relationship_id) else {
                continue;
            };
            let Ok(part_name) = relationship.target_partname(presentation_part.rels.base_uri())
            else {
                continue;
            };
            let Some(font_part) = package.part(&part_name) else {
                continue;
            };
            let font_key = face.font_key.or_else(|| entry.font_key.clone());
            let alias_seed = format!("{}:{}", entry.family, face.relationship_id);
            jobs.push(FontJob {
                family: entry.family.clone(),
                relationship_id: face.relationship_id,
                source_part_name: part_name.to_string(),
                font_key,
                weight: face.weight,
                style: face.style,
                alias_seed,
                bytes: font_part.blob.clone(),
            });
        }
    }

    let prepared = prepare_font_jobs(&jobs);
    let mut fonts = Vec::with_capacity(prepared.len());
    for (index, prepared) in prepared {
        let job = &jobs[index];
        let byte_size = prepared.bytes.len();
        let (sha256, md5) = font_fingerprints(&prepared.bytes);
        let data_uri = asset_sink.insert_owned(prepared.mime_type, prepared.bytes);
        fonts.push(EmbeddedFont {
            family: job.family.clone(),
            weight: job.weight,
            style: job.style.into(),
            data_uri,
            mime_type: prepared.mime_type.into(),
            format: prepared.format.into(),
            sha256: Some(sha256),
            md5: Some(md5),
            byte_size: Some(byte_size),
            postscript_name: None,
            face_index: None,
            source_part_name: Some(job.source_part_name.clone()),
            source_relationship_id: Some(job.relationship_id.clone()),
            font_key: job.font_key.clone(),
        });
    }
    fonts
}

fn prepare_font_jobs(jobs: &[FontJob]) -> Vec<(usize, PreparedFont)> {
    if jobs.is_empty() {
        return Vec::new();
    }
    let worker_count = std::env::var("UNIPPT_FONT_WORKERS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(2)
        })
        .clamp(1, 8)
        .min(jobs.len());
    let cursor = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(jobs.len()));
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            scope.spawn(|| loop {
                let index = cursor.fetch_add(1, Ordering::Relaxed);
                let Some(job) = jobs.get(index) else { break };
                let Some(prepared) = prepare_font_bytes(
                    &job.bytes,
                    job.font_key.as_deref(),
                    &job.family,
                    job.weight,
                    job.style == "italic",
                    &job.alias_seed,
                ) else {
                    continue;
                };
                results
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push((index, prepared));
            });
        }
    });
    let mut results = results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    results.sort_unstable_by_key(|(index, _)| *index);
    results
}

fn font_fingerprints(bytes: &[u8]) -> (String, String) {
    (
        format!("{:x}", Sha256::digest(bytes)),
        format!("{:x}", Md5::digest(bytes)),
    )
}

fn parse_embedded_font_list(xml: &[u8]) -> Vec<PendingFont> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut current: Option<PendingFont> = None;
    let mut fonts = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                if local_name(element.name().as_ref()) == b"embeddedFont" {
                    let font = PendingFont {
                        font_key: font_key_attr(&element),
                        ..Default::default()
                    };
                    current = Some(font);
                } else if let Some(font) = current.as_mut() {
                    capture_font_child(font, &element);
                }
            }
            Ok(Event::Empty(element)) => {
                if let Some(font) = current.as_mut() {
                    capture_font_child(font, &element);
                }
            }
            Ok(Event::End(element)) if local_name(element.name().as_ref()) == b"embeddedFont" => {
                if let Some(font) = current.take() {
                    fonts.push(font);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    fonts
}

fn capture_font_child(font: &mut PendingFont, element: &BytesStart<'_>) {
    match local_name(element.name().as_ref()) {
        b"font" => {
            if let Some(family) = xml_attr(element, b"typeface") {
                font.family = family;
            }
            if font.font_key.is_none() {
                font.font_key = font_key_attr(element);
            }
        }
        b"regular" | b"bold" | b"italic" | b"boldItalic" => {
            let Some(relationship_id) = xml_attr(element, b"id") else {
                return;
            };
            let is_bold = matches!(local_name(element.name().as_ref()), b"bold" | b"boldItalic");
            let is_italic = matches!(
                local_name(element.name().as_ref()),
                b"italic" | b"boldItalic"
            );
            font.faces.push(PendingFace {
                relationship_id,
                weight: if is_bold { 700 } else { 400 },
                style: if is_italic { "italic" } else { "normal" },
                font_key: font_key_attr(element),
            });
        }
        _ => {}
    }
}

fn prepare_font_bytes(
    source: &[u8],
    font_key: Option<&str>,
    family: &str,
    weight: u16,
    italic: bool,
    alias_seed: &str,
) -> Option<PreparedFont> {
    let mut bytes = source.to_vec();
    if let Some((mime_type, format)) = browser_font_kind(&bytes) {
        return Some(PreparedFont {
            bytes,
            mime_type,
            format,
        });
    }

    if is_eot(&bytes) {
        #[cfg(windows)]
        if let Some(decoded) = windows_eot_to_sfnt(&bytes, family, weight, italic, alias_seed) {
            if let Some((mime_type, format)) = browser_font_kind(&decoded) {
                return Some(PreparedFont {
                    bytes: decoded,
                    mime_type,
                    format,
                });
            }
        }

        // Uncompressed EOT stores the SFNT payload at the end of the container.
        if let Some(payload) = uncompressed_eot_payload(&bytes) {
            if let Some((mime_type, format)) = browser_font_kind(payload) {
                return Some(PreparedFont {
                    bytes: payload.to_vec(),
                    mime_type,
                    format,
                });
            }
        }
        return Some(PreparedFont {
            bytes,
            mime_type: "application/vnd.ms-fontobject",
            format: "embedded-opentype",
        });
    }

    if let Some(key) = font_key.and_then(guid_obfuscation_key) {
        deobfuscate_font_prefix(&mut bytes, &key);
        if let Some((mime_type, format)) = browser_font_kind(&bytes) {
            return Some(PreparedFont {
                bytes,
                mime_type,
                format,
            });
        }
    }
    None
}

fn browser_font_kind(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    match bytes.get(..4)? {
        [0x00, 0x01, 0x00, 0x00] | b"true" | b"typ1" | b"ttcf" => Some(("font/ttf", "truetype")),
        b"OTTO" => Some(("font/otf", "opentype")),
        b"wOFF" => Some(("font/woff", "woff")),
        b"wOF2" => Some(("font/woff2", "woff2")),
        _ => None,
    }
}

fn is_eot(bytes: &[u8]) -> bool {
    if bytes.len() < 82 {
        return false;
    }
    let declared_size = u32::from_le_bytes(bytes[0..4].try_into().unwrap_or_default()) as usize;
    let magic = u16::from_le_bytes(bytes[34..36].try_into().unwrap_or_default());
    magic == 0x504c && (declared_size == 0 || declared_size <= bytes.len())
}

fn uncompressed_eot_payload(bytes: &[u8]) -> Option<&[u8]> {
    let size = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    if size == 0 || size > bytes.len() {
        return None;
    }
    bytes.get(bytes.len() - size..)
}

/// ECMA-376 obfuscated fonts XOR the first 32 bytes with the GUID octets in
/// reverse textual order, repeated twice. This path is primarily used by
/// WordprocessingML but is accepted here for vendor PresentationML packages.
fn guid_obfuscation_key(value: &str) -> Option<[u8; 16]> {
    let hex = value
        .chars()
        .filter(char::is_ascii_hexdigit)
        .collect::<String>();
    if hex.len() != 32 {
        return None;
    }
    let mut key = [0u8; 16];
    for index in 0..16 {
        let byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
        key[15 - index] = byte;
    }
    Some(key)
}

fn deobfuscate_font_prefix(bytes: &mut [u8], key: &[u8; 16]) {
    for (index, byte) in bytes.iter_mut().take(32).enumerate() {
        *byte ^= key[index % key.len()];
    }
}

fn font_key_attr(element: &BytesStart<'_>) -> Option<String> {
    xml_attr(element, b"fontKey").or_else(|| xml_attr(element, b"key"))
}

fn xml_attr(element: &BytesStart<'_>, wanted: &[u8]) -> Option<String> {
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

#[cfg(windows)]
fn windows_eot_to_sfnt(
    eot: &[u8],
    family: &str,
    weight: u16,
    italic: bool,
    alias_seed: &str,
) -> Option<Vec<u8>> {
    use std::ffi::{c_void, CString};
    use std::ptr::{copy_nonoverlapping, null, null_mut};
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateFontW, DeleteDC, DeleteObject, GetFontData, SelectObject,
        TTDeleteEmbeddedFont, TTLoadEmbeddedFont, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
        DEFAULT_PITCH, DEFAULT_QUALITY, EMBED_EDITABLE, EMBED_INSTALLABLE, E_FONTNAMEALREADYEXISTS,
        E_NAMECHANGEFAILED, E_NONE, FF_DONTCARE, LICENSE_DEFAULT, OUT_DEFAULT_PRECIS,
        TTLOAD_PRIVATE,
    };

    struct ReadCursor<'a> {
        bytes: &'a [u8],
        position: usize,
    }

    unsafe extern "system" fn read_from_stream(
        stream: *mut c_void,
        buffer: *mut c_void,
        requested: u32,
    ) -> u32 {
        if stream.is_null() || buffer.is_null() {
            return 0;
        }
        let cursor = unsafe { &mut *stream.cast::<ReadCursor<'_>>() };
        let remaining = cursor.bytes.len().saturating_sub(cursor.position);
        let count = remaining.min(requested as usize);
        if count == 0 {
            return 0;
        }
        unsafe {
            copy_nonoverlapping(
                cursor.bytes.as_ptr().add(cursor.position),
                buffer.cast::<u8>(),
                count,
            );
        }
        cursor.position += count;
        count as u32
    }

    let checksum = alias_seed
        .bytes()
        .chain(eot.iter().copied().take(128))
        .fold(2_166_136_261u32, |value, byte| {
            value.wrapping_mul(16_777_619) ^ u32::from(byte)
        });
    let alias = format!("UniPPT{checksum:08X}");
    let wide_alias = alias
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mac_alias = CString::new(alias.as_str()).ok()?;
    let wide_family = family
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut cursor = ReadCursor {
        bytes: eot,
        position: 0,
    };
    let mut reference: HANDLE = null_mut();
    let mut privilege_status = 0u32;
    let mut load_status = 0u32;
    let mut loaded = unsafe {
        TTLoadEmbeddedFont(
            &mut reference,
            TTLOAD_PRIVATE,
            &mut privilege_status,
            LICENSE_DEFAULT,
            &mut load_status,
            Some(read_from_stream),
            (&mut cursor as *mut ReadCursor<'_>).cast::<c_void>(),
            wide_alias.as_ptr(),
            mac_alias.as_ptr().cast::<u8>(),
            null(),
        )
    };
    let mut face_name = wide_alias.as_ptr();
    if loaded == E_NAMECHANGEFAILED {
        // Some valid EOT packages cannot rewrite their internal name table.
        // Retry under the original PresentationML family while retaining
        // t2embed's normal fsType/license enforcement.
        cursor.position = 0;
        reference = null_mut();
        privilege_status = 0;
        load_status = 0;
        loaded = unsafe {
            TTLoadEmbeddedFont(
                &mut reference,
                TTLOAD_PRIVATE,
                &mut privilege_status,
                LICENSE_DEFAULT,
                &mut load_status,
                Some(read_from_stream),
                (&mut cursor as *mut ReadCursor<'_>).cast::<c_void>(),
                null(),
                null(),
                null(),
            )
        };
        face_name = wide_family.as_ptr();
    }
    let installed_collision = loaded == E_FONTNAMEALREADYEXISTS
        && matches!(privilege_status, EMBED_EDITABLE | EMBED_INSTALLABLE);
    if !installed_collision && (loaded != E_NONE || reference.is_null()) {
        return None;
    }

    let extracted = unsafe {
        let dc = CreateCompatibleDC(null_mut());
        if dc.is_null() {
            None
        } else {
            let font = CreateFontW(
                0,
                0,
                0,
                0,
                i32::from(weight),
                u32::from(italic),
                0,
                0,
                u32::from(DEFAULT_CHARSET),
                u32::from(OUT_DEFAULT_PRECIS),
                u32::from(CLIP_DEFAULT_PRECIS),
                u32::from(DEFAULT_QUALITY),
                u32::from(DEFAULT_PITCH | FF_DONTCARE),
                face_name,
            );
            if font.is_null() {
                DeleteDC(dc);
                None
            } else {
                let previous = SelectObject(dc, font);
                let size = GetFontData(dc, 0, 0, null_mut(), 0);
                let mut output = if size == u32::MAX || size == 0 {
                    None
                } else {
                    let mut bytes = vec![0u8; size as usize];
                    let written = GetFontData(dc, 0, 0, bytes.as_mut_ptr().cast(), size);
                    (written == size).then_some(bytes)
                };
                if !previous.is_null() {
                    SelectObject(dc, previous);
                }
                DeleteObject(font);
                DeleteDC(dc);
                output.take()
            }
        }
    };

    if !reference.is_null() {
        let mut delete_status = 0u32;
        unsafe {
            TTDeleteEmbeddedFont(reference, 0, &mut delete_status);
        }
    }
    extracted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_presentation_font_faces_and_variants() {
        let xml = br#"<p:presentation xmlns:p="p" xmlns:r="r"><p:embeddedFontLst>
          <p:embeddedFont><p:font typeface="Example"/><p:regular r:id="rId1"/>
          <p:bold r:id="rId2"/><p:italic r:id="rId3"/><p:boldItalic r:id="rId4"/></p:embeddedFont>
        </p:embeddedFontLst></p:presentation>"#;
        let fonts = parse_embedded_font_list(xml);
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0].family, "Example");
        assert_eq!(fonts[0].faces.len(), 4);
        assert_eq!(
            (fonts[0].faces[1].weight, fonts[0].faces[1].style),
            (700, "normal")
        );
        assert_eq!(
            (fonts[0].faces[3].weight, fonts[0].faces[3].style),
            (700, "italic")
        );
    }

    #[test]
    fn guid_deobfuscation_restores_sfnt_header() {
        let key = guid_obfuscation_key("{00112233-4455-6677-8899-AABBCCDDEEFF}").unwrap();
        assert_eq!(&key[..4], &[0xff, 0xee, 0xdd, 0xcc]);
        let mut bytes = vec![0u8; 64];
        bytes[..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);
        deobfuscate_font_prefix(&mut bytes, &key);
        assert_eq!(&bytes[..4], &[0xff, 0xef, 0xdd, 0xcc]);
        deobfuscate_font_prefix(&mut bytes, &key);
        assert_eq!(browser_font_kind(&bytes), Some(("font/ttf", "truetype")));
    }

    #[test]
    fn legacy_deck_json_defaults_to_no_embedded_fonts() {
        let mut value = serde_json::to_value(crate::model::Deck::demo()).unwrap();
        value.as_object_mut().unwrap().remove("fonts");
        let deck: crate::model::Deck = serde_json::from_value(value).unwrap();
        assert!(deck.fonts.is_empty());
    }

    #[test]
    fn browser_font_fingerprints_match_standard_vectors() {
        let (sha256, md5) = font_fingerprints(b"abc");
        assert_eq!(
            sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(md5, "900150983cd24fb0d6963f7d28e17f72");
    }
}

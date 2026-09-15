//! Media types (images, video, audio) for embedding in presentations.

mod audio;
mod image;
mod video;

pub use audio::Audio;
pub use image::Image;
pub use video::Video;

use sha1::{Digest, Sha1};

// ---------------------------------------------------------------------------
// Shared helper functions used by Image, Video, and Audio
// ---------------------------------------------------------------------------

/// Compute a lowercase hex SHA1 digest of the given bytes.
pub(super) fn compute_sha1(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex_encode(&result)
}

/// Encode bytes as lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Detect image format from magic bytes in the data.
/// Returns `(extension, content_type)` if recognized.
pub(super) fn detect_format_from_bytes(data: &[u8]) -> Option<(&'static str, &'static str)> {
    // SVG (text-based, check before binary formats)
    if is_svg_data(data) {
        return Some(("svg", "image/svg+xml"));
    }

    // PNG: 89 50 4E 47
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return Some(("png", "image/png"));
    }
    // JPEG: FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(("jpg", "image/jpeg"));
    }
    // GIF: "GIF8"
    if data.starts_with(b"GIF8") {
        return Some(("gif", "image/gif"));
    }
    // BMP: "BM"
    if data.starts_with(b"BM") {
        return Some(("bmp", "image/bmp"));
    }
    // TIFF little-endian: 49 49 2A 00
    if data.starts_with(&[0x49, 0x49, 0x2A, 0x00]) {
        return Some(("tiff", "image/tiff"));
    }
    // TIFF big-endian: 4D 4D 00 2A
    if data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return Some(("tiff", "image/tiff"));
    }
    // WEBP: "RIFF" + 4 bytes + "WEBP"
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some(("webp", "image/webp"));
    }
    // EMF: starts with 01 00 00 00 and bytes 40-44 are " EMF"
    if data.len() >= 44 && data.starts_with(&[0x01, 0x00, 0x00, 0x00]) && &data[40..44] == b" EMF" {
        return Some(("emf", "image/x-emf"));
    }
    // WMF: D7 CD C6 9A
    if data.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) {
        return Some(("wmf", "image/x-wmf"));
    }

    None
}

/// Check if the given bytes look like SVG content.
///
/// Detects `<?xml` or `<svg` near the start of the data, skipping
/// leading whitespace and BOM.
fn is_svg_data(data: &[u8]) -> bool {
    // Skip BOM and whitespace
    let trimmed = if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &data[3..]
    } else {
        data
    };
    let trimmed = trimmed
        .iter()
        .position(|&b| !b.is_ascii_whitespace())
        .map_or(trimmed, |pos| &trimmed[pos..]);

    // Check for <?xml followed by <svg, or just <svg directly
    if trimmed.starts_with(b"<svg") || trimmed.starts_with(b"<SVG") {
        return true;
    }
    if trimmed.starts_with(b"<?xml") {
        // Look for <svg within the first 1KB
        let check_len = trimmed.len().min(1024);
        let check = &trimmed[..check_len];
        if let Ok(s) = std::str::from_utf8(check) {
            let lower = s.to_lowercase();
            return lower.contains("<svg");
        }
    }
    false
}

/// Map a file extension to an image content type.
pub(super) fn ext_to_content_type(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" | "jpe" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "tif" | "tiff" => Some("image/tiff"),
        "emf" => Some("image/x-emf"),
        "wmf" => Some("image/x-wmf"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// Map an image content type to a canonical file extension.
pub(super) fn content_type_to_ext(ct: &str) -> Option<&'static str> {
    match ct {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/bmp" => Some("bmp"),
        "image/tiff" => Some("tiff"),
        "image/x-emf" => Some("emf"),
        "image/x-wmf" => Some("wmf"),
        "image/svg+xml" => Some("svg"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

/// Map a video file extension to a content type.
pub(super) fn video_ext_to_content_type(ext: &str) -> Option<&'static str> {
    match ext {
        "mp4" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "avi" => Some("video/x-msvideo"),
        "wmv" => Some("video/x-ms-wmv"),
        _ => None,
    }
}

/// Map a video content type to a canonical file extension.
pub(super) fn video_content_type_to_ext(ct: &str) -> Option<&'static str> {
    match ct {
        "video/mp4" => Some("mp4"),
        "video/quicktime" => Some("mov"),
        "video/x-msvideo" => Some("avi"),
        "video/x-ms-wmv" => Some("wmv"),
        _ => None,
    }
}

/// Map an audio file extension to a content type.
pub(super) fn audio_ext_to_content_type(ext: &str) -> Option<&'static str> {
    match ext {
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "m4a" => Some("audio/mp4"),
        _ => None,
    }
}

/// Map an audio content type to a canonical file extension.
pub(super) fn audio_content_type_to_ext(ct: &str) -> Option<&'static str> {
    match ct {
        "audio/mpeg" => Some("mp3"),
        "audio/wav" => Some("wav"),
        "audio/mp4" => Some("m4a"),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

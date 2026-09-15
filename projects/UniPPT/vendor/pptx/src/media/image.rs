//! Image media type for embedding in presentations.

use std::path::Path;

use crate::error::{PptxError, PptxResult};
use crate::units::Emu;

use super::{compute_sha1, content_type_to_ext, detect_format_from_bytes, ext_to_content_type};

const MAX_IMAGE_SIZE: u64 = 200 * 1024 * 1024; // 200 MB

/// An image that can be embedded in a presentation.
///
/// Stores the binary image data along with metadata like content type,
/// file extension, and a SHA1 hash for deduplication.
///
/// # Examples
///
/// ```no_run
/// use pptx::media::Image;
///
/// let img = Image::from_file("photo.png").unwrap();
/// assert_eq!(img.ext(), "png");
/// assert_eq!(img.content_type(), "image/png");
/// ```
#[derive(Debug, Clone)]
pub struct Image {
    blob: Vec<u8>,
    content_type: String,
    ext: String,
    sha1: String,
    filename: Option<String>,
}

impl Image {
    /// Create an Image from a file path.
    ///
    /// The image format is detected from file content (magic bytes).
    /// If that fails, the file extension is used as a fallback.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the image format is unrecognized.
    pub fn from_file(path: impl AsRef<Path>) -> PptxResult<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(PptxError::Io)?;
        if metadata.len() > MAX_IMAGE_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "image file size {} bytes exceeds the limit of {} bytes",
                    metadata.len(),
                    MAX_IMAGE_SIZE
                ),
            });
        }
        let blob = std::fs::read(path).map_err(PptxError::Io)?;

        let (ext, content_type) = detect_format_from_bytes(&blob)
            .map(|(e, ct)| (e.to_string(), ct.to_string()))
            .or_else(|| {
                // Fallback: detect from file extension
                let file_ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_lowercase)?;
                ext_to_content_type(&file_ext).map(|ct| (file_ext, ct.to_string()))
            })
            .ok_or_else(|| {
                PptxError::InvalidXml(format!(
                    "unsupported or unrecognized image format: {}",
                    path.display()
                ))
            })?;

        let sha1 = compute_sha1(&blob);

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(std::string::ToString::to_string);

        Ok(Self {
            blob,
            content_type,
            ext,
            sha1,
            filename,
        })
    }

    /// Create an Image from raw bytes with an explicit content type.
    ///
    /// The file extension is inferred from the content type.
    #[must_use]
    pub fn from_bytes(data: Vec<u8>, content_type: &str) -> Self {
        let ext = content_type_to_ext(content_type)
            .unwrap_or("bin")
            .to_string();
        let sha1 = compute_sha1(&data);

        Self {
            blob: data,
            content_type: content_type.to_string(),
            ext,
            sha1,
            filename: None,
        }
    }

    /// The original filename (just the file name, not the full path), if created from a file.
    ///
    /// Returns `None` if the image was created from bytes.
    #[must_use]
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// The raw image bytes.
    #[must_use]
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// The MIME content type, e.g. "image/png".
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The canonical file extension without a leading dot, e.g. "png".
    #[must_use]
    pub fn ext(&self) -> &str {
        &self.ext
    }

    /// The SHA1 hash digest of the image bytes (lowercase hex, 40 chars).
    #[must_use]
    pub fn sha1(&self) -> &str {
        &self.sha1
    }

    /// The pixel width of the image, or `None` if dimensions cannot be read.
    ///
    /// SVG images always return `None` since they have no intrinsic pixel size.
    #[must_use]
    pub fn width_px(&self) -> Option<u32> {
        self.dimensions().map(|(w, _)| w)
    }

    /// The pixel height of the image, or `None` if dimensions cannot be read.
    ///
    /// SVG images always return `None` since they have no intrinsic pixel size.
    #[must_use]
    pub fn height_px(&self) -> Option<u32> {
        self.dimensions().map(|(_, h)| h)
    }

    /// The DPI (dots per inch) of the image as `(x_dpi, y_dpi)`.
    ///
    /// Returns `(72, 72)` as the default if DPI cannot be determined from
    /// the image metadata.
    #[must_use]
    pub const fn dpi(&self) -> (u32, u32) {
        (72, 72)
    }

    /// The native size of the image in EMU, calculated from pixel dimensions and DPI.
    ///
    /// Returns `None` if pixel dimensions cannot be determined (e.g., for SVG).
    ///
    /// The formula is: `emu = pixels * 914400 / dpi`
    #[must_use]
    pub fn native_size(&self) -> Option<(Emu, Emu)> {
        let (w, h) = self.dimensions()?;
        let (dpi_x, dpi_y) = self.dpi();
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        // u32→f64, f64→i64 for EMU conversion
        let emu_x = (f64::from(w) * 914_400.0 / f64::from(dpi_x)) as i64;
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        // u32→f64, f64→i64 for EMU conversion
        let emu_y = (f64::from(h) * 914_400.0 / f64::from(dpi_y)) as i64;
        Some((Emu(emu_x), Emu(emu_y)))
    }

    /// Scale the image proportionally to fit within the given bounds.
    ///
    /// Returns the scaled `(width, height)` in EMU such that the image fits
    /// inside the bounding box defined by `max_width` and `max_height` while
    /// preserving the aspect ratio.
    ///
    /// Returns `None` if the native size cannot be determined.
    #[must_use]
    pub fn scale_to_fit(&self, max_width: Emu, max_height: Emu) -> Option<(Emu, Emu)> {
        let (native_w, native_h) = self.native_size()?;
        if native_w.0 <= 0 || native_h.0 <= 0 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)] // i64→f64 for ratio computation
        let scale_x = max_width.0 as f64 / native_w.0 as f64;
        #[allow(clippy::cast_precision_loss)] // i64→f64 for ratio computation
        let scale_y = max_height.0 as f64 / native_h.0 as f64;
        let scale = scale_x.min(scale_y).min(1.0);
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        // i64→f64, f64→i64 for EMU result
        let scaled_w = (native_w.0 as f64 * scale) as i64;
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        // i64→f64, f64→i64 for EMU result
        let scaled_h = (native_h.0 as f64 * scale) as i64;
        Some((Emu(scaled_w), Emu(scaled_h)))
    }

    /// Read pixel dimensions from the image data by parsing format headers.
    fn dimensions(&self) -> Option<(u32, u32)> {
        // SVG images have no intrinsic raster dimensions
        if self.content_type == "image/svg+xml" {
            return None;
        }
        dimensions_from_bytes(&self.blob)
    }
}

/// Extract pixel dimensions from raw image bytes by reading format headers.
fn dimensions_from_bytes(data: &[u8]) -> Option<(u32, u32)> {
    // PNG: width/height are at bytes 16..24 as big-endian u32
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) && data.len() >= 24 {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((w, h));
    }
    // GIF: width/height at bytes 6..10 as little-endian u16
    if data.starts_with(b"GIF8") && data.len() >= 10 {
        let w = u32::from(u16::from_le_bytes([data[6], data[7]]));
        let h = u32::from(u16::from_le_bytes([data[8], data[9]]));
        return Some((w, h));
    }
    // BMP: width at 18..22, height at 22..26 as little-endian i32
    if data.starts_with(b"BM") && data.len() >= 26 {
        let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]);
        let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
        return Some((w.unsigned_abs(), h.unsigned_abs()));
    }
    // JPEG: scan for SOF0-SOF15 markers (C0-CF, except C4 and CC)
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return jpeg_dimensions(data);
    }
    None
}

/// Parse JPEG SOF markers to extract image dimensions.
fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2; // skip SOI (FF D8)
    while i + 1 < data.len() {
        if data[i] != 0xFF {
            return None;
        }
        // skip padding FF bytes
        while i + 1 < data.len() && data[i + 1] == 0xFF {
            i += 1;
        }
        if i + 1 >= data.len() {
            return None;
        }
        let marker = data[i + 1];
        i += 2;
        // SOF markers: C0-CF except C4 (DHT) and CC (DAC)
        if (0xC0..=0xCF).contains(&marker) && marker != 0xC4 && marker != 0xCC {
            if i + 7 > data.len() {
                return None;
            }
            let h = u32::from(u16::from_be_bytes([data[i + 3], data[i + 4]]));
            let w = u32::from(u16::from_be_bytes([data[i + 5], data[i + 6]]));
            return Some((w, h));
        }
        // Skip segment (length includes the 2 length bytes)
        if i + 1 >= data.len() {
            return None;
        }
        let seg_len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if seg_len < 2 {
            return None;
        }
        i += seg_len;
    }
    None
}

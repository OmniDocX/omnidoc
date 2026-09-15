//! Video media type for embedding in presentations.

use std::path::Path;

use crate::error::{PptxError, PptxResult};

use super::{compute_sha1, video_content_type_to_ext, video_ext_to_content_type};

const MAX_VIDEO_SIZE: u64 = 500 * 1024 * 1024; // 500 MB

/// A video media file that can be embedded in a presentation.
#[derive(Debug, Clone)]
pub struct Video {
    blob: Vec<u8>,
    content_type: String,
    ext: String,
    sha1: String,
}

impl Video {
    /// Create a Video from a file path.
    ///
    /// The format is detected from the file extension.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the video format is unsupported.
    pub fn from_file(path: impl AsRef<Path>) -> PptxResult<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(PptxError::Io)?;
        if metadata.len() > MAX_VIDEO_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "video file size {} bytes exceeds the limit of {} bytes",
                    metadata.len(),
                    MAX_VIDEO_SIZE
                ),
            });
        }
        let blob = std::fs::read(path).map_err(PptxError::Io)?;

        let file_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .ok_or_else(|| {
                PptxError::InvalidXml(format!(
                    "cannot determine video format from path: {}",
                    path.display()
                ))
            })?;

        let content_type = video_ext_to_content_type(&file_ext)
            .ok_or_else(|| PptxError::InvalidXml(format!("unsupported video format: {file_ext}")))?
            .to_string();

        let sha1 = compute_sha1(&blob);

        Ok(Self {
            blob,
            content_type,
            ext: file_ext,
            sha1,
        })
    }

    /// Create a Video from raw bytes with an explicit content type.
    #[must_use]
    pub fn from_bytes(data: Vec<u8>, content_type: &str) -> Self {
        let ext = video_content_type_to_ext(content_type)
            .unwrap_or("bin")
            .to_string();
        let sha1 = compute_sha1(&data);

        Self {
            blob: data,
            content_type: content_type.to_string(),
            ext,
            sha1,
        }
    }

    /// The raw video bytes.
    #[must_use]
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// The MIME content type, e.g. "video/mp4".
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The canonical file extension without a leading dot, e.g. "mp4".
    #[must_use]
    pub fn ext(&self) -> &str {
        &self.ext
    }

    /// The SHA1 hash digest of the video bytes (lowercase hex, 40 chars).
    #[must_use]
    pub fn sha1(&self) -> &str {
        &self.sha1
    }
}

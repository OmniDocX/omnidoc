//! Audio media type for embedding in presentations.

use std::path::Path;

use crate::error::{PptxError, PptxResult};

use super::{audio_content_type_to_ext, audio_ext_to_content_type, compute_sha1};

const MAX_AUDIO_SIZE: u64 = 500 * 1024 * 1024; // 500 MB

/// An audio media file that can be embedded in a presentation.
///
/// Supports mp3, wav, and m4a formats.
#[derive(Debug, Clone)]
pub struct Audio {
    blob: Vec<u8>,
    content_type: String,
    ext: String,
    sha1: String,
}

impl Audio {
    /// Create an Audio from a file path.
    ///
    /// The format is detected from the file extension.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the audio format is unsupported.
    pub fn from_file(path: impl AsRef<Path>) -> PptxResult<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path).map_err(PptxError::Io)?;
        if metadata.len() > MAX_AUDIO_SIZE {
            return Err(PptxError::ResourceLimit {
                message: format!(
                    "audio file size {} bytes exceeds the limit of {} bytes",
                    metadata.len(),
                    MAX_AUDIO_SIZE
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
                    "cannot determine audio format from path: {}",
                    path.display()
                ))
            })?;

        let content_type = audio_ext_to_content_type(&file_ext)
            .ok_or_else(|| PptxError::InvalidXml(format!("unsupported audio format: {file_ext}")))?
            .to_string();

        let sha1 = compute_sha1(&blob);

        Ok(Self {
            blob,
            content_type,
            ext: file_ext,
            sha1,
        })
    }

    /// Create an Audio from raw bytes with an explicit content type.
    #[must_use]
    pub fn from_bytes(data: Vec<u8>, content_type: &str) -> Self {
        let ext = audio_content_type_to_ext(content_type)
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

    /// The raw audio bytes.
    #[must_use]
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// The MIME content type, e.g. "audio/mpeg".
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The canonical file extension without a leading dot, e.g. "mp3".
    #[must_use]
    pub fn ext(&self) -> &str {
        &self.ext
    }

    /// The SHA1 hash digest of the audio bytes (lowercase hex, 40 chars).
    #[must_use]
    pub fn sha1(&self) -> &str {
        &self.sha1
    }
}

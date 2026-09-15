use crate::error::PptxResult;
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;

use super::OpcPackage;

impl OpcPackage {
    /// Add an image part to the package, deduplicating by SHA1 hash.
    ///
    /// If an image with the same SHA1 already exists in `/ppt/media/`, it is
    /// reused. Otherwise a new image part is created.
    ///
    /// Returns `(partname, content_type)` of the image part (new or existing).
    ///
    /// # Errors
    ///
    /// Returns an error if the part cannot be created.
    pub fn or_add_image_part(
        &mut self,
        image: &crate::media::Image,
    ) -> PptxResult<(PackURI, String)> {
        let target_sha1 = image.sha1();
        if let Some(existing) =
            find_existing_media_by_sha1(&self.parts, "/ppt/media/image", target_sha1)
        {
            return Ok(existing);
        }

        let partname = self.next_image_partname(image.ext())?;
        let part = Part::new(
            partname.clone(),
            image.content_type(),
            image.blob().to_vec(),
        );
        self.put_part(part);
        Ok((partname, image.content_type().to_string()))
    }

    /// Add a media (video) part to the package, deduplicating by SHA1 hash.
    ///
    /// Returns `(partname, content_type)` of the media part (new or existing).
    ///
    /// # Errors
    ///
    /// Returns an error if the part cannot be created.
    pub fn or_add_media_part(
        &mut self,
        video: &crate::media::Video,
    ) -> PptxResult<(PackURI, String)> {
        let target_sha1 = video.sha1();
        if let Some(existing) =
            find_existing_media_by_sha1(&self.parts, "/ppt/media/media", target_sha1)
        {
            return Ok(existing);
        }

        let partname = self.next_media_partname(video.ext())?;
        let part = Part::new(
            partname.clone(),
            video.content_type(),
            video.blob().to_vec(),
        );
        self.put_part(part);
        Ok((partname, video.content_type().to_string()))
    }

    /// Add an audio part to the package, deduplicating by SHA1 hash.
    pub fn or_add_audio_part(
        &mut self,
        audio: &crate::media::Audio,
    ) -> PptxResult<(PackURI, String)> {
        let target_sha1 = audio.sha1();
        if let Some(existing) =
            find_existing_media_by_sha1(&self.parts, "/ppt/media/media", target_sha1)
        {
            return Ok(existing);
        }

        let partname = self.next_media_partname(audio.ext())?;
        let part = Part::new(
            partname.clone(),
            audio.content_type(),
            audio.blob().to_vec(),
        );
        self.put_part(part);
        Ok((partname, audio.content_type().to_string()))
    }
}

/// Search existing parts for a media file with matching SHA1.
fn find_existing_media_by_sha1(
    parts: &std::collections::HashMap<String, Part>,
    prefix: &str,
    target_sha1: &str,
) -> Option<(PackURI, String)> {
    for part in parts.values() {
        if part.partname.as_str().starts_with(prefix) {
            let existing_sha1 = compute_sha1_hex(&part.blob);
            if existing_sha1 == target_sha1 {
                return Some((part.partname.clone(), part.content_type.clone()));
            }
        }
    }
    None
}

/// Compute a lowercase hex SHA1 digest of the given bytes.
fn compute_sha1_hex(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    use std::fmt::Write;
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut s = String::with_capacity(40);
    for b in result.as_slice() {
        write!(s, "{b:02x}")
            .unwrap_or_else(|_| unreachable!("fmt::Write for String is infallible"));
    }
    s
}

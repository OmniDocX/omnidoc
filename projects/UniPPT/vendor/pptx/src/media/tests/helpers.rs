/// Build a minimal valid PNG file with given width/height (no pixel data, just IHDR + IEND).
/// Sufficient for dimension parsing; not a displayable image.
pub(super) fn minimal_png(width: u32, height: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    // PNG signature
    buf.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    // IHDR chunk: length=13, type="IHDR", data, CRC
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&width.to_be_bytes());
    ihdr_data.extend_from_slice(&height.to_be_bytes());
    ihdr_data.push(8); // bit depth
    ihdr_data.push(2); // color type (RGB)
    ihdr_data.push(0); // compression
    ihdr_data.push(0); // filter
    ihdr_data.push(0); // interlace
    let ihdr_len = ihdr_data.len() as u32;
    buf.extend_from_slice(&ihdr_len.to_be_bytes());
    buf.extend_from_slice(b"IHDR");
    buf.extend_from_slice(&ihdr_data);
    // CRC over type + data
    let crc = crc32(b"IHDR", &ihdr_data);
    buf.extend_from_slice(&crc.to_be_bytes());
    // IEND chunk
    buf.extend_from_slice(&0u32.to_be_bytes()); // length=0
    buf.extend_from_slice(b"IEND");
    let crc = crc32(b"IEND", &[]);
    buf.extend_from_slice(&crc.to_be_bytes());
    buf
}

/// Minimal CRC32 for PNG chunks (IEEE polynomial).
fn crc32(chunk_type: &[u8], data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in chunk_type.iter().chain(data.iter()) {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

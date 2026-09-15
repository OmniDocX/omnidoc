use crate::media::{
    audio_content_type_to_ext, audio_ext_to_content_type, content_type_to_ext,
    detect_format_from_bytes, ext_to_content_type, video_ext_to_content_type,
};

// --- Magic byte detection tests ---

#[test]
fn test_detect_format_png_magic() {
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let result = detect_format_from_bytes(&png_data);
    assert!(result.is_some());
    let (ext, ct) = result.unwrap();
    assert_eq!(ext, "png");
    assert_eq!(ct, "image/png");
}

#[test]
fn test_detect_format_jpeg_magic() {
    let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0];
    let (ext, ct) = detect_format_from_bytes(&jpeg_data).unwrap();
    assert_eq!(ext, "jpg");
    assert_eq!(ct, "image/jpeg");
}

#[test]
fn test_detect_format_gif_magic() {
    let gif_data = b"GIF89a\x02\x00\x03\x00\x00\x00\x00";
    let (ext, ct) = detect_format_from_bytes(gif_data).unwrap();
    assert_eq!(ext, "gif");
    assert_eq!(ct, "image/gif");
}

#[test]
fn test_detect_format_bmp_magic() {
    let mut bmp_data = vec![0u8; 30];
    bmp_data[0] = b'B';
    bmp_data[1] = b'M';
    let (ext, ct) = detect_format_from_bytes(&bmp_data).unwrap();
    assert_eq!(ext, "bmp");
    assert_eq!(ct, "image/bmp");
}

#[test]
fn test_detect_format_tiff_le_magic() {
    let data = vec![0x49, 0x49, 0x2A, 0x00, 0, 0, 0, 0];
    let (ext, ct) = detect_format_from_bytes(&data).unwrap();
    assert_eq!(ext, "tiff");
    assert_eq!(ct, "image/tiff");
}

#[test]
fn test_detect_format_tiff_be_magic() {
    let data = vec![0x4D, 0x4D, 0x00, 0x2A, 0, 0, 0, 0];
    let (ext, ct) = detect_format_from_bytes(&data).unwrap();
    assert_eq!(ext, "tiff");
    assert_eq!(ct, "image/tiff");
}

#[test]
fn test_detect_format_webp_magic() {
    let mut data = vec![0u8; 12];
    data[..4].copy_from_slice(b"RIFF");
    data[8..12].copy_from_slice(b"WEBP");
    let (ext, ct) = detect_format_from_bytes(&data).unwrap();
    assert_eq!(ext, "webp");
    assert_eq!(ct, "image/webp");
}

#[test]
fn test_detect_format_emf_magic() {
    let mut data = vec![0u8; 44];
    data[0] = 0x01;
    data[40..44].copy_from_slice(b" EMF");
    let (ext, ct) = detect_format_from_bytes(&data).unwrap();
    assert_eq!(ext, "emf");
    assert_eq!(ct, "image/x-emf");
}

#[test]
fn test_detect_format_wmf_magic() {
    let data = vec![0xD7, 0xCD, 0xC6, 0x9A, 0, 0, 0, 0];
    let (ext, ct) = detect_format_from_bytes(&data).unwrap();
    assert_eq!(ext, "wmf");
    assert_eq!(ct, "image/x-wmf");
}

#[test]
fn test_detect_format_unknown() {
    let data = vec![0x00, 0x01, 0x02, 0x03];
    assert!(detect_format_from_bytes(&data).is_none());
}

#[test]
fn test_ext_to_content_type() {
    assert_eq!(ext_to_content_type("png"), Some("image/png"));
    assert_eq!(ext_to_content_type("jpg"), Some("image/jpeg"));
    assert_eq!(ext_to_content_type("jpeg"), Some("image/jpeg"));
    assert_eq!(ext_to_content_type("gif"), Some("image/gif"));
    assert_eq!(ext_to_content_type("bmp"), Some("image/bmp"));
    assert_eq!(ext_to_content_type("tiff"), Some("image/tiff"));
    assert_eq!(ext_to_content_type("emf"), Some("image/x-emf"));
    assert_eq!(ext_to_content_type("wmf"), Some("image/x-wmf"));
    assert_eq!(ext_to_content_type("xyz"), None);
}

#[test]
fn test_content_type_to_ext() {
    assert_eq!(content_type_to_ext("image/png"), Some("png"));
    assert_eq!(content_type_to_ext("image/jpeg"), Some("jpg"));
    assert_eq!(content_type_to_ext("video/mp4"), None);
}

#[test]
fn test_video_ext_to_content_type() {
    assert_eq!(video_ext_to_content_type("mp4"), Some("video/mp4"));
    assert_eq!(video_ext_to_content_type("mov"), Some("video/quicktime"));
    assert_eq!(video_ext_to_content_type("avi"), Some("video/x-msvideo"));
    assert_eq!(video_ext_to_content_type("wmv"), Some("video/x-ms-wmv"));
    assert_eq!(video_ext_to_content_type("xyz"), None);
}

// --- SVG detection tests ---

#[test]
fn test_detect_svg_direct() {
    let svg_data = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>";
    let (ext, ct) = detect_format_from_bytes(svg_data).unwrap();
    assert_eq!(ext, "svg");
    assert_eq!(ct, "image/svg+xml");
}

#[test]
fn test_detect_svg_with_xml_decl() {
    let svg_data = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"></svg>"#;
    let (ext, ct) = detect_format_from_bytes(svg_data).unwrap();
    assert_eq!(ext, "svg");
    assert_eq!(ct, "image/svg+xml");
}

#[test]
fn test_detect_svg_with_bom() {
    let mut data = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    data.extend_from_slice(b"<svg></svg>");
    let (ext, _) = detect_format_from_bytes(&data).unwrap();
    assert_eq!(ext, "svg");
}

#[test]
fn test_ext_to_content_type_svg() {
    assert_eq!(ext_to_content_type("svg"), Some("image/svg+xml"));
}

#[test]
fn test_content_type_to_ext_svg() {
    assert_eq!(content_type_to_ext("image/svg+xml"), Some("svg"));
}

// --- Audio/Video content type helpers ---

#[test]
fn test_audio_ext_to_content_type() {
    assert_eq!(audio_ext_to_content_type("mp3"), Some("audio/mpeg"));
    assert_eq!(audio_ext_to_content_type("wav"), Some("audio/wav"));
    assert_eq!(audio_ext_to_content_type("m4a"), Some("audio/mp4"));
    assert_eq!(audio_ext_to_content_type("xyz"), None);
}

#[test]
fn test_audio_content_type_to_ext() {
    assert_eq!(audio_content_type_to_ext("audio/mpeg"), Some("mp3"));
    assert_eq!(audio_content_type_to_ext("audio/wav"), Some("wav"));
    assert_eq!(audio_content_type_to_ext("audio/mp4"), Some("m4a"));
    assert_eq!(audio_content_type_to_ext("video/mp4"), None);
}

use super::helpers::minimal_png;
use crate::media::Image;
use crate::units::Emu;

#[test]
fn test_image_from_bytes_png() {
    let img = Image::from_bytes(vec![1, 2, 3], "image/png");
    assert_eq!(img.ext(), "png");
    assert_eq!(img.content_type(), "image/png");
    assert_eq!(img.blob(), &[1, 2, 3]);
    assert!(!img.sha1().is_empty());
    assert_eq!(img.sha1().len(), 40);
}

#[test]
fn test_image_from_bytes_jpeg() {
    let img = Image::from_bytes(vec![0xFF, 0xD8, 0xFF], "image/jpeg");
    assert_eq!(img.ext(), "jpg");
    assert_eq!(img.content_type(), "image/jpeg");
}

#[test]
fn test_sha1_consistency() {
    let data = vec![1, 2, 3, 4, 5];
    let img1 = Image::from_bytes(data.clone(), "image/png");
    let img2 = Image::from_bytes(data, "image/png");
    assert_eq!(img1.sha1(), img2.sha1());
}

#[test]
fn test_sha1_different_data() {
    let img1 = Image::from_bytes(vec![1, 2, 3], "image/png");
    let img2 = Image::from_bytes(vec![4, 5, 6], "image/png");
    assert_ne!(img1.sha1(), img2.sha1());
}

#[test]
fn test_image_from_bytes_svg() {
    let img = Image::from_bytes(b"<svg></svg>".to_vec(), "image/svg+xml");
    assert_eq!(img.ext(), "svg");
    assert_eq!(img.content_type(), "image/svg+xml");
}

// --- Image dimension tests ---

#[test]
fn test_image_dimensions_png() {
    let buf = minimal_png(2, 3);
    let img = Image::from_bytes(buf, "image/png");
    assert_eq!(img.width_px(), Some(2));
    assert_eq!(img.height_px(), Some(3));
}

#[test]
fn test_image_dimensions_svg_returns_none() {
    let img = Image::from_bytes(b"<svg></svg>".to_vec(), "image/svg+xml");
    assert_eq!(img.width_px(), None);
    assert_eq!(img.height_px(), None);
}

#[test]
fn test_image_dpi_default() {
    let img = Image::from_bytes(vec![1, 2, 3], "image/png");
    assert_eq!(img.dpi(), (72, 72));
}

#[test]
fn test_image_native_size() {
    let buf = minimal_png(100, 200);
    let img = Image::from_bytes(buf, "image/png");
    let (w, h) = img.native_size().unwrap();
    assert_eq!(w.0, 1270000);
    assert_eq!(h.0, 2540000);
}

#[test]
fn test_image_native_size_svg_returns_none() {
    let img = Image::from_bytes(b"<svg></svg>".to_vec(), "image/svg+xml");
    assert!(img.native_size().is_none());
}

#[test]
fn test_image_scale_to_fit() {
    let buf = minimal_png(200, 100);
    let img = Image::from_bytes(buf, "image/png");
    let native = img.native_size().unwrap();

    let max_w = Emu(native.0 .0 / 2);
    let max_h = Emu(native.1 .0);
    let (scaled_w, scaled_h) = img.scale_to_fit(max_w, max_h).unwrap();
    assert_eq!(scaled_w.0, native.0 .0 / 2);
    assert_eq!(scaled_h.0, native.1 .0 / 2);
}

#[test]
fn test_image_scale_to_fit_no_upscale() {
    let buf = minimal_png(10, 10);
    let img = Image::from_bytes(buf, "image/png");
    let native = img.native_size().unwrap();

    let max_w = Emu(native.0 .0 * 10);
    let max_h = Emu(native.1 .0 * 10);
    let (scaled_w, scaled_h) = img.scale_to_fit(max_w, max_h).unwrap();
    assert_eq!(scaled_w.0, native.0 .0);
    assert_eq!(scaled_h.0, native.1 .0);
}

// --- Image.filename tests ---

#[test]
fn test_image_from_bytes_has_no_filename() {
    let img = Image::from_bytes(vec![1, 2, 3], "image/png");
    assert!(img.filename().is_none());
}

#[test]
fn test_image_from_file_has_filename() {
    let buf = minimal_png(2, 2);
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join("test_image_filename.png");
    std::fs::write(&tmp_path, &buf).unwrap();

    let img = Image::from_file(&tmp_path).unwrap();
    assert_eq!(img.filename(), Some("test_image_filename.png"));

    let _ = std::fs::remove_file(&tmp_path);
}

#[test]
fn test_image_from_file_filename_is_just_name() {
    let buf = minimal_png(2, 2);
    let tmp_dir = std::env::temp_dir().join("subdir_test");
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let tmp_path = tmp_dir.join("photo.png");
    std::fs::write(&tmp_path, &buf).unwrap();

    let img = Image::from_file(&tmp_path).unwrap();
    assert_eq!(img.filename(), Some("photo.png"));
    assert!(!img.filename().unwrap().contains('/'));

    let _ = std::fs::remove_file(&tmp_path);
    let _ = std::fs::remove_dir(&tmp_dir);
}

use crate::core_properties::CoreProperties;
use crate::presentation::Presentation;

#[test]
fn test_new_presentation() {
    let prs = Presentation::new().unwrap();
    assert!(prs.slide_count().unwrap() == 0);
}

#[test]
fn test_slide_masters() {
    let prs = Presentation::new().unwrap();
    let masters = prs.slide_masters().unwrap();
    assert_eq!(masters.len(), 1);
    assert_eq!(
        masters[0].partname.as_str(),
        "/ppt/slideMasters/slideMaster1.xml"
    );
}

#[test]
fn test_slide_layouts() {
    let prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    assert_eq!(layouts.len(), 11);
    assert_eq!(layouts[0].name, "Title Slide");
}

#[test]
fn test_slide_size() {
    let prs = Presentation::new().unwrap();
    let size = prs.slide_size().unwrap();
    assert_eq!(size, Some((9144000, 6858000)));
}

#[test]
fn test_round_trip_with_slides() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    prs.add_slide(&layouts[0]).unwrap();
    prs.add_slide(&layouts[1]).unwrap();

    // Save and reopen
    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();

    assert_eq!(prs2.slide_count().unwrap(), 2);
    let slides = prs2.slides().unwrap();
    assert_eq!(slides[0].partname.as_str(), "/ppt/slides/slide1.xml");
    assert_eq!(slides[1].partname.as_str(), "/ppt/slides/slide2.xml");

    // Verify layouts are still available
    let layouts2 = prs2.slide_layouts().unwrap();
    assert_eq!(layouts2.len(), 11);
}

#[test]
fn test_save_and_reopen() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    prs.add_slide(&layouts[0]).unwrap();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    prs.save(&path).unwrap();

    let prs2 = Presentation::open(&path).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 1);
}

#[test]
fn test_from_reader() {
    let prs = Presentation::new().unwrap();
    let bytes = prs.to_bytes().unwrap();
    let cursor = std::io::Cursor::new(bytes);

    let prs2 = Presentation::from_reader(cursor).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 0);
}

#[test]
fn test_write_to() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    prs.add_slide(&layouts[0]).unwrap();

    let mut buf = Vec::new();
    prs.write_to(&mut buf).unwrap();

    let prs2 = Presentation::from_bytes(&buf).unwrap();
    assert_eq!(prs2.slide_count().unwrap(), 1);
}

#[test]
fn test_set_slide_width() {
    let mut prs = Presentation::new().unwrap();
    prs.set_slide_width(12192000).unwrap();
    let size = prs.slide_size().unwrap();
    assert_eq!(size, Some((12192000, 6858000)));
}

#[test]
fn test_set_slide_height() {
    let mut prs = Presentation::new().unwrap();
    prs.set_slide_height(5000000).unwrap();
    let size = prs.slide_size().unwrap();
    assert_eq!(size, Some((9144000, 5000000)));
}

#[test]
fn test_core_properties_roundtrip() {
    let mut prs = Presentation::new().unwrap();

    let mut props = CoreProperties::new();
    props.set_title("Test Title");
    props.set_author("Test Author");
    prs.set_core_properties(&props).unwrap();

    let bytes = prs.to_bytes().unwrap();
    let prs2 = Presentation::from_bytes(&bytes).unwrap();
    let props2 = prs2.core_properties().unwrap();

    assert_eq!(props2.title(), "Test Title");
    assert_eq!(props2.author(), "Test Author");
}

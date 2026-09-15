use super::*;

#[test]
fn test_new_valid() {
    let uri = PackURI::new("/ppt/slides/slide1.xml").unwrap();
    assert_eq!(uri.as_str(), "/ppt/slides/slide1.xml");
}

#[test]
fn test_new_invalid() {
    assert!(PackURI::new("ppt/slides/slide1.xml").is_err());
}

#[test]
fn test_new_package_root_valid() {
    assert!(PackURI::new("/").is_ok());
}

#[test]
fn test_new_trailing_slash_invalid() {
    assert!(PackURI::new("/ppt/slides/").is_err());
}

#[test]
fn test_new_double_slash_invalid() {
    assert!(PackURI::new("//foo/bar").is_err());
    assert!(PackURI::new("/foo//bar").is_err());
}

#[test]
fn test_new_dot_segment_invalid() {
    assert!(PackURI::new("/./bar").is_err());
    assert!(PackURI::new("/foo/./bar").is_err());
}

#[test]
fn test_new_dotdot_segment_invalid() {
    assert!(PackURI::new("/../x").is_err());
    assert!(PackURI::new("/foo/../bar").is_err());
}

#[test]
fn test_new_backslash_invalid() {
    assert!(PackURI::new("/foo\\bar").is_err());
}

#[test]
fn test_base_uri() {
    let uri = PackURI::new("/ppt/slides/slide1.xml").unwrap();
    assert_eq!(uri.base_uri(), "/ppt/slides");
    assert_eq!(PackURI::package().base_uri(), "/");
    assert_eq!(PackURI::new("/presentation.xml").unwrap().base_uri(), "/");
}

#[test]
fn test_ext() {
    assert_eq!(PackURI::new("/ppt/slides/slide1.xml").unwrap().ext(), "xml");
    assert_eq!(
        PackURI::new("/ppt/printerSettings/printerSettings1.bin")
            .unwrap()
            .ext(),
        "bin"
    );
}

#[test]
fn test_filename() {
    assert_eq!(
        PackURI::new("/ppt/slides/slide1.xml").unwrap().filename(),
        "slide1.xml"
    );
    assert_eq!(PackURI::package().filename(), "");
}

#[test]
fn test_membername() {
    assert_eq!(
        PackURI::new("/ppt/slides/slide1.xml").unwrap().membername(),
        "ppt/slides/slide1.xml"
    );
    assert_eq!(PackURI::package().membername(), "");
}

#[test]
fn test_relative_ref() {
    let uri = PackURI::new("/ppt/slideLayouts/slideLayout1.xml").unwrap();
    assert_eq!(
        uri.relative_ref("/ppt/slides"),
        "../slideLayouts/slideLayout1.xml"
    );
    let uri2 = PackURI::new("/ppt/presentation.xml").unwrap();
    assert_eq!(uri2.relative_ref("/"), "ppt/presentation.xml");
}

#[test]
fn test_rels_uri() {
    let uri = PackURI::new("/ppt/slides/slide1.xml").unwrap();
    assert_eq!(uri.rels_uri().as_str(), "/ppt/slides/_rels/slide1.xml.rels");
    assert_eq!(PackURI::package().rels_uri().as_str(), "/_rels/.rels");
}

#[test]
fn test_from_rel_ref() {
    let uri = PackURI::from_rel_ref("/ppt/slides", "../slideLayouts/slideLayout1.xml").unwrap();
    assert_eq!(uri.as_str(), "/ppt/slideLayouts/slideLayout1.xml");
    let uri2 = PackURI::from_rel_ref("/", "ppt/presentation.xml").unwrap();
    assert_eq!(uri2.as_str(), "/ppt/presentation.xml");
}

#[test]
fn test_idx() {
    assert_eq!(
        PackURI::new("/ppt/slides/slide21.xml").unwrap().idx(),
        Some(21)
    );
    assert_eq!(PackURI::new("/ppt/presentation.xml").unwrap().idx(), None);
    assert_eq!(
        PackURI::new("/ppt/slides/slide1.xml").unwrap().idx(),
        Some(1)
    );
}

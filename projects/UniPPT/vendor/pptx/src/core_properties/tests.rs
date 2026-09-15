use super::*;

#[test]
fn test_new_core_properties() {
    let props = CoreProperties::new();
    assert_eq!(props.title(), "");
    assert_eq!(props.author(), "");
}

#[test]
fn test_setters_and_getters() {
    let mut props = CoreProperties::new();
    props.set_title("Test Title");
    props.set_author("Test Author");
    props.set_subject("Test Subject");
    props.set_keywords("key1, key2");
    props.set_comments("Some comments");
    props.set_category("Category");
    props.set_created("2024-01-01T00:00:00Z");
    props.set_modified("2024-06-15T12:00:00Z");
    props.set_last_modified_by("Modifier");
    props.set_revision("3");

    assert_eq!(props.title(), "Test Title");
    assert_eq!(props.author(), "Test Author");
    assert_eq!(props.subject(), "Test Subject");
    assert_eq!(props.keywords(), "key1, key2");
    assert_eq!(props.comments(), "Some comments");
    assert_eq!(props.category(), "Category");
    assert_eq!(props.created(), "2024-01-01T00:00:00Z");
    assert_eq!(props.modified(), "2024-06-15T12:00:00Z");
    assert_eq!(props.last_modified_by(), "Modifier");
    assert_eq!(props.revision(), "3");
}

#[test]
fn test_round_trip_xml() {
    let mut props = CoreProperties::new();
    props.set_title("My Presentation");
    props.set_author("Jane Doe");
    props.set_subject("Testing");
    props.set_keywords("rust, pptx");
    props.set_comments("A test presentation");
    props.set_category("Test");
    props.set_created("2024-01-01T00:00:00Z");
    props.set_modified("2024-06-15T12:00:00Z");
    props.set_last_modified_by("rust-pptx");
    props.set_revision("1");

    let xml = props.to_xml().unwrap();
    let xml_str = std::str::from_utf8(&xml).unwrap();

    // Verify key elements are present
    assert!(xml_str.contains("<dc:title>My Presentation</dc:title>"));
    assert!(xml_str.contains("<dc:creator>Jane Doe</dc:creator>"));
    assert!(xml_str.contains("xsi:type=\"dcterms:W3CDTF\""));

    // Parse back
    let props2 = CoreProperties::from_xml(&xml).unwrap();
    assert_eq!(props2.title(), "My Presentation");
    assert_eq!(props2.author(), "Jane Doe");
    assert_eq!(props2.subject(), "Testing");
    assert_eq!(props2.keywords(), "rust, pptx");
    assert_eq!(props2.comments(), "A test presentation");
    assert_eq!(props2.category(), "Test");
    assert_eq!(props2.created(), "2024-01-01T00:00:00Z");
    assert_eq!(props2.modified(), "2024-06-15T12:00:00Z");
    assert_eq!(props2.last_modified_by(), "rust-pptx");
    assert_eq!(props2.revision(), "1");
}

#[test]
fn test_parse_real_core_xml() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>PowerPoint Presentation</dc:title>
  <dc:creator>Author Name</dc:creator>
  <cp:lastModifiedBy>Editor</cp:lastModifiedBy>
  <cp:revision>2</cp:revision>
  <dcterms:created xsi:type="dcterms:W3CDTF">2024-01-01T00:00:00Z</dcterms:created>
  <dcterms:modified xsi:type="dcterms:W3CDTF">2024-06-15T12:30:00Z</dcterms:modified>
</cp:coreProperties>"#;

    let props = CoreProperties::from_xml(xml).unwrap();
    assert_eq!(props.title(), "PowerPoint Presentation");
    assert_eq!(props.author(), "Author Name");
    assert_eq!(props.last_modified_by(), "Editor");
    assert_eq!(props.revision(), "2");
    assert_eq!(props.created(), "2024-01-01T00:00:00Z");
    assert_eq!(props.modified(), "2024-06-15T12:30:00Z");
}

#[test]
fn test_content_status_language_version() {
    let mut props = CoreProperties::new();
    props.set_content_status("Draft");
    props.set_language("en-US");
    props.set_version("1.0");

    assert_eq!(props.content_status(), "Draft");
    assert_eq!(props.language(), "en-US");
    assert_eq!(props.version(), "1.0");

    // Round-trip through XML
    let xml = props.to_xml().unwrap();
    let xml_str = std::str::from_utf8(&xml).unwrap();
    assert!(xml_str.contains("<cp:contentStatus>Draft</cp:contentStatus>"));
    assert!(xml_str.contains("<dc:language>en-US</dc:language>"));
    assert!(xml_str.contains("<cp:version>1.0</cp:version>"));

    let parsed = CoreProperties::from_xml(&xml).unwrap();
    assert_eq!(parsed.content_status(), "Draft");
    assert_eq!(parsed.language(), "en-US");
    assert_eq!(parsed.version(), "1.0");
}

#[test]
fn test_parse_xml_with_content_status_language_version() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>Test</dc:title>
  <cp:contentStatus>Final</cp:contentStatus>
  <dc:language>ja-JP</dc:language>
  <cp:version>2.0</cp:version>
</cp:coreProperties>"#;

    let props = CoreProperties::from_xml(xml).unwrap();
    assert_eq!(props.title(), "Test");
    assert_eq!(props.content_status(), "Final");
    assert_eq!(props.language(), "ja-JP");
    assert_eq!(props.version(), "2.0");
}

#[test]
fn test_empty_new_fields_omitted_from_xml() {
    let props = CoreProperties::new();
    let xml = props.to_xml().unwrap();
    let xml_str = String::from_utf8(xml).unwrap();
    assert!(!xml_str.contains("<cp:contentStatus>"));
    assert!(!xml_str.contains("<dc:language>"));
    assert!(!xml_str.contains("<cp:version>"));
}

#[test]
fn test_to_xml_empty_fields_omitted() {
    let mut props = CoreProperties::new();
    props.set_title("Only Title");

    let xml = props.to_xml().unwrap();
    let xml_str = String::from_utf8(xml).unwrap();

    assert!(xml_str.contains("<dc:title>Only Title</dc:title>"));
    // Empty fields should not be present
    assert!(!xml_str.contains("<dc:creator>"));
    assert!(!xml_str.contains("<dc:subject>"));
}

#[test]
fn test_identifier_getter_setter() {
    let mut props = CoreProperties::new();
    assert_eq!(props.identifier(), None);
    props.set_identifier("urn:example:doc:123");
    assert_eq!(props.identifier(), Some("urn:example:doc:123"));
}

#[test]
fn test_last_printed_getter_setter() {
    let mut props = CoreProperties::new();
    assert_eq!(props.last_printed(), None);
    props.set_last_printed("2024-03-15T10:30:00Z");
    assert_eq!(props.last_printed(), Some("2024-03-15T10:30:00Z"));
}

#[test]
fn test_identifier_last_printed_roundtrip() {
    let mut props = CoreProperties::new();
    props.set_title("Test");
    props.set_identifier("doc-id-456");
    props.set_last_printed("2024-06-01T08:00:00Z");

    let xml = props.to_xml().unwrap();
    let xml_str = std::str::from_utf8(&xml).unwrap();
    assert!(xml_str.contains("<dc:identifier>doc-id-456</dc:identifier>"));
    assert!(xml_str.contains("<cp:lastPrinted>2024-06-01T08:00:00Z</cp:lastPrinted>"));

    let parsed = CoreProperties::from_xml(&xml).unwrap();
    assert_eq!(parsed.identifier(), Some("doc-id-456"));
    assert_eq!(parsed.last_printed(), Some("2024-06-01T08:00:00Z"));
}

#[test]
fn test_none_identifier_last_printed_omitted() {
    let props = CoreProperties::new();
    let xml = props.to_xml().unwrap();
    let xml_str = String::from_utf8(xml).unwrap();
    assert!(!xml_str.contains("<dc:identifier>"));
    assert!(!xml_str.contains("<cp:lastPrinted>"));
}

#[test]
fn test_parse_xml_with_identifier_last_printed() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:dcmitype="http://purl.org/dc/dcmitype/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>Test</dc:title>
  <dc:identifier>my-doc-id</dc:identifier>
  <cp:lastPrinted>2024-12-25T00:00:00Z</cp:lastPrinted>
</cp:coreProperties>"#;

    let props = CoreProperties::from_xml(xml).unwrap();
    assert_eq!(props.title(), "Test");
    assert_eq!(props.identifier(), Some("my-doc-id"));
    assert_eq!(props.last_printed(), Some("2024-12-25T00:00:00Z"));
}

use crate::opc::constants::content_type as CT;
use crate::opc::constants::relationship_type as RT;
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;
use crate::presentation::Presentation;
use crate::repair::validator::is_well_formed_xml;
use crate::repair::*;

#[test]
fn validate_default_presentation_is_clean() {
    let prs = Presentation::new().unwrap();
    let issues = PptxValidator::validate(&prs);
    assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
}

#[test]
fn validate_bytes_of_default_presentation() {
    let prs = Presentation::new().unwrap();
    let bytes = prs.to_bytes().unwrap();
    let issues = PptxValidator::validate_bytes(&bytes);
    assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
}

#[test]
fn validate_bytes_with_invalid_data() {
    let issues = PptxValidator::validate_bytes(b"not a zip file");
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity, Severity::Critical);
    assert_eq!(issues[0].category, IssueCategory::InvalidXml);
}

#[test]
fn detect_broken_relationship() {
    let mut prs = Presentation::new().unwrap();
    // Add a relationship pointing to a non-existent part
    let pres_partname = prs
        .package()
        .part_by_reltype(RT::OFFICE_DOCUMENT)
        .unwrap()
        .partname
        .clone();
    prs.package_mut()
        .part_mut(&pres_partname)
        .unwrap()
        .rels
        .add_relationship(RT::SLIDE, "slides/slideNONEXISTENT.xml", false);

    let issues = PptxValidator::validate(&prs);
    let broken = issues
        .iter()
        .filter(|i| i.category == IssueCategory::BrokenRelationship)
        .count();
    assert!(
        broken >= 1,
        "Expected broken relationship issue, got: {:?}",
        issues
    );
}

#[test]
fn detect_orphan_slide_relationship() {
    let mut prs = Presentation::new().unwrap();

    // Add a slide part and a relationship from presentation, but don't add to sldIdLst
    let slide_pn = PackURI::new("/ppt/slides/slide99.xml").unwrap();
    let slide_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#;
    let slide_part = Part::new(slide_pn, CT::PML_SLIDE, slide_xml.to_vec());
    prs.package_mut().put_part(slide_part);

    let pres_partname = prs
        .package()
        .part_by_reltype(RT::OFFICE_DOCUMENT)
        .unwrap()
        .partname
        .clone();
    prs.package_mut()
        .part_mut(&pres_partname)
        .unwrap()
        .rels
        .add_relationship(RT::SLIDE, "slides/slide99.xml", false);

    let issues = PptxValidator::validate(&prs);
    let orphans = issues
        .iter()
        .filter(|i| i.category == IssueCategory::OrphanSlide)
        .count();
    assert!(
        orphans >= 1,
        "Expected orphan slide issue, got: {:?}",
        issues
    );
}

#[test]
fn detect_malformed_xml() {
    let mut prs = Presentation::new().unwrap();
    let bad_pn = PackURI::new("/ppt/bad.xml").unwrap();
    let bad_part = Part::new(bad_pn, "application/xml", b"<broken><not-closed>".to_vec());
    prs.package_mut().put_part(bad_part);

    let issues = PptxValidator::validate(&prs);
    let xml_issues = issues
        .iter()
        .filter(|i| i.category == IssueCategory::InvalidXml)
        .count();
    assert!(
        xml_issues >= 1,
        "Expected invalid XML issue, got: {:?}",
        issues
    );
}

#[test]
fn detect_empty_content_type() {
    let mut prs = Presentation::new().unwrap();
    let pn = PackURI::new("/ppt/empty_ct.xml").unwrap();
    let part = Part::new(pn, "", b"<?xml version=\"1.0\"?><root/>".to_vec());
    prs.package_mut().put_part(part);

    let issues = PptxValidator::validate(&prs);
    let ct_issues = issues
        .iter()
        .filter(|i| i.category == IssueCategory::InvalidContentType)
        .count();
    assert!(
        ct_issues >= 1,
        "Expected content type issue, got: {:?}",
        issues
    );
}

#[test]
fn severity_and_category_are_debug_clone() {
    let s = Severity::Critical;
    let s2 = s;
    assert_eq!(s, s2);
    let _ = format!("{:?}", s);

    let c = IssueCategory::MissingPart;
    let c2 = c;
    assert_eq!(c, c2);
    let _ = format!("{:?}", c);
}

#[test]
fn validation_issue_fields_accessible() {
    let issue = ValidationIssue::new(
        Severity::Low,
        IssueCategory::MissingNamespace,
        "test description",
        Some("/ppt/test.xml".to_string()),
    );
    assert_eq!(issue.severity, Severity::Low);
    assert_eq!(issue.category, IssueCategory::MissingNamespace);
    assert_eq!(issue.description, "test description");
    assert_eq!(issue.location.as_deref(), Some("/ppt/test.xml"));
}

#[test]
fn well_formed_xml_check() {
    assert!(is_well_formed_xml(b"<?xml version=\"1.0\"?><root/>"));
    assert!(is_well_formed_xml(b"<a><b/></a>"));
    assert!(!is_well_formed_xml(b"<broken><not-closed>"));
    assert!(!is_well_formed_xml(b"<<<garbage"));
}

#[test]
fn validate_presentation_with_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    if let Some(layout) = layouts.first() {
        prs.add_slide(layout).unwrap();
    }
    let issues = PptxValidator::validate(&prs);
    assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
}

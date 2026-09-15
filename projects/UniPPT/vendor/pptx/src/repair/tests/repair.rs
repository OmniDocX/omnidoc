use crate::opc::constants::content_type as CT;
use crate::opc::constants::relationship_type as RT;
use crate::opc::pack_uri::PackURI;
use crate::opc::part::Part;
use crate::presentation::Presentation;
use crate::repair::*;

#[test]
fn repair_removes_broken_relationship() {
    let mut prs = Presentation::new().unwrap();
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
        .add_relationship(RT::SLIDE, "slides/slideGHOST.xml", false);

    let report = PptxRepairer::repair(&mut prs);
    assert!(!report.issues_found.is_empty());
    let broken_fixed = report
        .issues_fixed
        .iter()
        .filter(|i| i.category == IssueCategory::BrokenRelationship)
        .count();
    assert!(
        broken_fixed >= 1,
        "Expected broken rel fix, got: {:?}",
        report.issues_fixed
    );
}

#[test]
fn repair_removes_orphan_slide_relationship() {
    let mut prs = Presentation::new().unwrap();

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

    let report = PptxRepairer::repair(&mut prs);
    let orphan_fixed = report
        .issues_fixed
        .iter()
        .filter(|i| i.category == IssueCategory::OrphanSlide)
        .count();
    assert!(
        orphan_fixed >= 1,
        "Expected orphan fix, got: {:?}",
        report.issues_fixed
    );
}

#[test]
fn repair_adds_missing_office_document_rel() {
    let mut prs = Presentation::new().unwrap();

    // Remove the officeDocument relationship
    let rid = prs
        .package()
        .pkg_rels
        .iter()
        .find(|r| r.rel_type.as_ref() == RT::OFFICE_DOCUMENT)
        .map(|r| r.r_id.to_string());

    if let Some(rid) = rid {
        prs.package_mut().pkg_rels.remove(&rid);
    }

    let report = PptxRepairer::repair(&mut prs);
    let rel_fixed = report
        .issues_fixed
        .iter()
        .filter(|i| {
            i.category == IssueCategory::BrokenRelationship
                && i.description.contains("officeDocument")
        })
        .count();
    assert!(
        rel_fixed >= 1,
        "Expected officeDocument rel fix, got: {:?}",
        report.issues_fixed
    );
}

#[test]
fn repair_default_presentation_is_noop() {
    let mut prs = Presentation::new().unwrap();
    let report = PptxRepairer::repair(&mut prs);
    assert!(report.issues_found.is_empty());
    assert!(report.issues_fixed.is_empty());
    assert!(report.is_valid);
}

#[test]
fn repair_report_fields_accessible() {
    let report = RepairReport {
        issues_found: vec![],
        issues_fixed: vec![],
        is_valid: true,
    };
    assert!(report.is_valid);
    assert!(report.issues_found.is_empty());
    assert!(report.issues_fixed.is_empty());
}

#[test]
fn repair_after_validate_is_idempotent() {
    let mut prs = Presentation::new().unwrap();

    // Add a broken relationship
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
        .add_relationship(RT::SLIDE, "slides/nonexistent.xml", false);

    // First repair
    let report1 = PptxRepairer::repair(&mut prs);
    assert!(!report1.issues_fixed.is_empty());

    // Second repair should find nothing
    let report2 = PptxRepairer::repair(&mut prs);
    assert!(
        report2.issues_found.is_empty(),
        "Expected clean after first repair, got: {:?}",
        report2.issues_found
    );
    assert!(report2.issues_fixed.is_empty());
    assert!(report2.is_valid);
}

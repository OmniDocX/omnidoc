//! PPTX validation logic.

use std::collections::HashSet;

use quick_xml::Reader;

use crate::opc::constants::relationship_type as RT;
use crate::opc::package::OpcPackage;
use crate::presentation::Presentation;
use crate::slide::parse_slide_ids;

use super::{IssueCategory, Severity, ValidationIssue};

/// Validates PPTX files for structural correctness.
#[derive(Debug, Clone)]
pub struct PptxValidator;

impl PptxValidator {
    /// Validate a presentation and return all issues found.
    #[must_use]
    pub fn validate(prs: &Presentation) -> Vec<ValidationIssue> {
        let pkg = prs.package();
        let mut issues = Vec::new();

        Self::check_presentation_part_exists(pkg, &mut issues);
        Self::check_package_rels(pkg, &mut issues);
        Self::check_relationship_targets(pkg, &mut issues);
        Self::check_xml_wellformedness(pkg, &mut issues);
        Self::check_slide_references(pkg, &mut issues);
        Self::check_content_types(pkg, &mut issues);

        issues
    }

    /// Validate raw PPTX bytes and return all issues found.
    #[must_use]
    pub fn validate_bytes(data: &[u8]) -> Vec<ValidationIssue> {
        match Presentation::from_bytes(data) {
            Ok(prs) => Self::validate(&prs),
            Err(e) => vec![ValidationIssue::new(
                Severity::Critical,
                IssueCategory::InvalidXml,
                format!("Failed to open PPTX: {e}"),
                None,
            )],
        }
    }

    /// Check that ppt/presentation.xml exists.
    fn check_presentation_part_exists(pkg: &OpcPackage, issues: &mut Vec<ValidationIssue>) {
        if pkg.part_by_reltype(RT::OFFICE_DOCUMENT).is_err() {
            issues.push(ValidationIssue::new(
                Severity::Critical,
                IssueCategory::MissingPart,
                "Missing presentation part (ppt/presentation.xml)",
                Some("/ppt/presentation.xml".to_string()),
            ));
        }
    }

    /// Check _rels/.rels has the required officeDocument relationship.
    fn check_package_rels(pkg: &OpcPackage, issues: &mut Vec<ValidationIssue>) {
        if pkg.pkg_rels.is_empty() {
            issues.push(ValidationIssue::new(
                Severity::Critical,
                IssueCategory::BrokenRelationship,
                "Package relationships (_rels/.rels) are empty",
                Some("/_rels/.rels".to_string()),
            ));
            return;
        }

        let has_office_doc = pkg
            .pkg_rels
            .iter()
            .any(|r| r.rel_type.as_ref() == RT::OFFICE_DOCUMENT);
        if !has_office_doc {
            issues.push(ValidationIssue::new(
                Severity::Critical,
                IssueCategory::BrokenRelationship,
                "Package relationships missing officeDocument relationship",
                Some("/_rels/.rels".to_string()),
            ));
        }
    }

    /// Check that all internal relationship targets point to existing parts.
    fn check_relationship_targets(pkg: &OpcPackage, issues: &mut Vec<ValidationIssue>) {
        // Collect all part names for lookup
        let part_names: HashSet<&str> = pkg.parts().map(|p| p.partname.as_str()).collect();

        // Check package-level relationships
        for rel in pkg.pkg_rels.iter() {
            if rel.is_external {
                continue;
            }
            if let Ok(partname) = rel.target_partname(pkg.pkg_rels.base_uri()) {
                if !part_names.contains(partname.as_str()) {
                    issues.push(ValidationIssue::new(
                        Severity::High,
                        IssueCategory::BrokenRelationship,
                        format!(
                            "Package relationship {} targets missing part {}",
                            rel.r_id,
                            partname.as_str()
                        ),
                        Some("/_rels/.rels".to_string()),
                    ));
                }
            }
        }

        // Check part-level relationships
        for part in pkg.parts() {
            let base_uri = part.partname.base_uri();
            for rel in part.rels.iter() {
                if rel.is_external {
                    continue;
                }
                if let Ok(partname) = rel.target_partname(base_uri) {
                    if !part_names.contains(partname.as_str()) {
                        issues.push(ValidationIssue::new(
                            Severity::High,
                            IssueCategory::BrokenRelationship,
                            format!(
                                "Relationship {} in {} targets missing part {}",
                                rel.r_id,
                                part.partname,
                                partname.as_str()
                            ),
                            Some(part.partname.to_string()),
                        ));
                    }
                }
            }
        }
    }

    /// Check XML well-formedness of key XML parts.
    fn check_xml_wellformedness(pkg: &OpcPackage, issues: &mut Vec<ValidationIssue>) {
        for part in pkg.parts() {
            if !part.content_type.contains("xml") {
                continue;
            }
            if !is_well_formed_xml(&part.blob) {
                issues.push(ValidationIssue::new(
                    Severity::High,
                    IssueCategory::InvalidXml,
                    format!("Malformed XML in part {}", part.partname),
                    Some(part.partname.to_string()),
                ));
            }
        }
    }

    /// Check slide references: all slides in sldIdLst exist, no orphan slides.
    fn check_slide_references(pkg: &OpcPackage, issues: &mut Vec<ValidationIssue>) {
        let Ok(pres_part) = pkg.part_by_reltype(RT::OFFICE_DOCUMENT) else {
            return; // Already reported as missing
        };

        let Ok(slide_ids) = parse_slide_ids(&pres_part.blob) else {
            issues.push(ValidationIssue::new(
                Severity::High,
                IssueCategory::InvalidXml,
                "Failed to parse sldIdLst from presentation.xml",
                Some(pres_part.partname.to_string()),
            ));
            return;
        };

        // Collect rIds referenced in sldIdLst
        let referenced_rids: HashSet<&str> =
            slide_ids.iter().map(|(rid, _)| rid.as_str()).collect();

        let base_uri = pres_part.partname.base_uri();

        // Check each sldId references an existing slide
        for (r_id, _) in &slide_ids {
            match pres_part.rels.get(r_id.as_str()) {
                Some(rel) => {
                    if let Ok(partname) = rel.target_partname(base_uri) {
                        if pkg.part(&partname).is_none() {
                            issues.push(ValidationIssue::new(
                                Severity::High,
                                IssueCategory::MissingSlideRef,
                                format!(
                                    "Slide referenced in sldIdLst ({}) points to missing part {}",
                                    r_id,
                                    partname.as_str()
                                ),
                                Some(pres_part.partname.to_string()),
                            ));
                        }
                    }
                }
                None => {
                    issues.push(ValidationIssue::new(
                        Severity::High,
                        IssueCategory::MissingSlideRef,
                        format!(
                            "Slide referenced in sldIdLst with rId {r_id} has no matching relationship"
                        ),
                        Some(pres_part.partname.to_string()),
                    ));
                }
            }
        }

        // Check for orphan slide relationships (slide rels not in sldIdLst)
        for rel in pres_part.rels.iter() {
            if rel.rel_type.as_ref() == RT::SLIDE && !referenced_rids.contains(rel.r_id.as_str()) {
                issues.push(ValidationIssue::new(
                    Severity::Medium,
                    IssueCategory::OrphanSlide,
                    format!(
                        "Slide relationship {} (target: {}) not referenced in sldIdLst",
                        rel.r_id, rel.target_ref
                    ),
                    Some(pres_part.partname.to_string()),
                ));
            }
        }
    }

    /// Check that parts have valid content types.
    fn check_content_types(pkg: &OpcPackage, issues: &mut Vec<ValidationIssue>) {
        for part in pkg.parts() {
            if part.content_type.is_empty() {
                issues.push(ValidationIssue::new(
                    Severity::Medium,
                    IssueCategory::InvalidContentType,
                    format!("Part {} has an empty content type", part.partname),
                    Some(part.partname.to_string()),
                ));
            }
        }
    }
}

/// Check if a byte slice is well-formed XML.
pub(super) fn is_well_formed_xml(data: &[u8]) -> bool {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut depth: usize = 0;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(_)) => depth += 1,
            Ok(quick_xml::events::Event::End(_)) => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            Ok(quick_xml::events::Event::Eof) => return depth == 0,
            Err(_) => return false,
            _ => {}
        }
        buf.clear();
    }
}

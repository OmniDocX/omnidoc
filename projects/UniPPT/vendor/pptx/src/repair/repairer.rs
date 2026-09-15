//! PPTX repair logic.

use std::collections::HashSet;

use crate::opc::constants::relationship_type as RT;
use crate::opc::pack_uri::PackURI;
use crate::presentation::Presentation;
use crate::slide::parse_slide_ids;

use super::validator::PptxValidator;
use super::{IssueCategory, RepairReport, Severity, ValidationIssue};

/// Repairs structural issues in PPTX files.
#[derive(Debug, Clone)]
pub struct PptxRepairer;

impl PptxRepairer {
    /// Repair a presentation, returning a report of what was found and fixed.
    pub fn repair(prs: &mut Presentation) -> RepairReport {
        let issues_found = PptxValidator::validate(prs);

        let mut issues_fixed = Vec::new();

        Self::remove_broken_relationships(prs, &mut issues_fixed);
        Self::remove_orphan_slide_references(prs, &mut issues_fixed);
        Self::add_missing_office_document_rel(prs, &mut issues_fixed);

        // Re-validate after repairs to determine final validity
        let remaining = PptxValidator::validate(prs);
        let is_valid = remaining.is_empty();

        RepairReport {
            issues_found,
            issues_fixed,
            is_valid,
        }
    }

    /// Remove relationships whose internal targets don't exist.
    fn remove_broken_relationships(prs: &mut Presentation, fixed: &mut Vec<ValidationIssue>) {
        let pkg = prs.package_mut();

        // Collect all existing part names
        let part_names: HashSet<String> = pkg
            .parts()
            .map(|p| p.partname.as_str().to_string())
            .collect();

        // Fix package-level relationships
        let base_uri = pkg.pkg_rels.base_uri().to_string();
        let broken_pkg_rids: Vec<String> = pkg
            .pkg_rels
            .iter()
            .filter(|r| !r.is_external)
            .filter(|r| {
                r.target_partname(&base_uri)
                    .map(|pn| !part_names.contains(pn.as_str()))
                    .unwrap_or(false)
            })
            .map(|r| r.r_id.to_string())
            .collect();

        for r_id in broken_pkg_rids {
            if let Some(removed) = pkg.pkg_rels.remove(&r_id) {
                fixed.push(ValidationIssue::new(
                    Severity::High,
                    IssueCategory::BrokenRelationship,
                    format!(
                        "Removed broken package relationship {} -> {}",
                        removed.r_id, removed.target_ref
                    ),
                    Some("/_rels/.rels".to_string()),
                ));
            }
        }

        // Fix part-level relationships
        #[allow(clippy::similar_names)]
        let part_uris: Vec<String> = pkg
            .parts()
            .map(|p| p.partname.as_str().to_string())
            .collect();

        for partname_str in part_uris {
            let Ok(partname) = PackURI::new(&partname_str) else {
                continue;
            };

            let Some(part) = pkg.part(&partname) else {
                continue;
            };

            let part_base_uri = part.partname.base_uri().to_string();
            let broken_rids: Vec<String> = part
                .rels
                .iter()
                .filter(|r| !r.is_external)
                .filter(|r| {
                    r.target_partname(&part_base_uri)
                        .map(|pn| !part_names.contains(pn.as_str()))
                        .unwrap_or(false)
                })
                .map(|r| r.r_id.to_string())
                .collect();

            if broken_rids.is_empty() {
                continue;
            }

            let Some(part_mut) = pkg.part_mut(&partname) else {
                continue;
            };

            for r_id in broken_rids {
                if let Some(removed) = part_mut.rels.remove(&r_id) {
                    fixed.push(ValidationIssue::new(
                        Severity::High,
                        IssueCategory::BrokenRelationship,
                        format!(
                            "Removed broken relationship {} -> {} from {}",
                            removed.r_id, removed.target_ref, partname_str
                        ),
                        Some(partname_str.clone()),
                    ));
                }
            }
        }
    }

    /// Remove orphan slide references (slide rels not in sldIdLst).
    fn remove_orphan_slide_references(prs: &mut Presentation, fixed: &mut Vec<ValidationIssue>) {
        let Ok(pres_p) = prs.package().part_by_reltype(RT::OFFICE_DOCUMENT) else {
            return;
        };
        let pres_partname = pres_p.partname.clone();

        let Some(pres_part) = prs.package().part(&pres_partname) else {
            return;
        };

        let Ok(slide_ids) = parse_slide_ids(&pres_part.blob) else {
            return;
        };

        let referenced_rids: HashSet<String> =
            slide_ids.iter().map(|(rid, _)| rid.clone()).collect();

        let orphan_rids: Vec<String> = pres_part
            .rels
            .iter()
            .filter(|r| r.rel_type.as_ref() == RT::SLIDE)
            .filter(|r| !referenced_rids.contains(r.r_id.as_str()))
            .map(|r| r.r_id.to_string())
            .collect();

        if orphan_rids.is_empty() {
            return;
        }

        let Some(pres_part_mut) = prs.package_mut().part_mut(&pres_partname) else {
            return;
        };

        for r_id in orphan_rids {
            if let Some(removed) = pres_part_mut.rels.remove(&r_id) {
                fixed.push(ValidationIssue::new(
                    Severity::Medium,
                    IssueCategory::OrphanSlide,
                    format!(
                        "Removed orphan slide relationship {} (target: {})",
                        removed.r_id, removed.target_ref
                    ),
                    Some(pres_partname.to_string()),
                ));
            }
        }
    }

    /// Add missing officeDocument relationship if presentation.xml exists.
    fn add_missing_office_document_rel(prs: &mut Presentation, fixed: &mut Vec<ValidationIssue>) {
        let has_office_doc = prs
            .package()
            .pkg_rels
            .iter()
            .any(|r| r.rel_type.as_ref() == RT::OFFICE_DOCUMENT);

        if has_office_doc {
            return;
        }

        // Check if the presentation part exists by conventional name
        let Ok(pres_uri) = PackURI::new("/ppt/presentation.xml") else {
            return;
        };

        if prs.package().part(&pres_uri).is_some() {
            prs.package_mut().pkg_rels.add_relationship(
                RT::OFFICE_DOCUMENT,
                "ppt/presentation.xml",
                false,
            );
            fixed.push(ValidationIssue::new(
                Severity::Critical,
                IssueCategory::BrokenRelationship,
                "Added missing officeDocument relationship to package",
                Some("/_rels/.rels".to_string()),
            ));
        }
    }
}

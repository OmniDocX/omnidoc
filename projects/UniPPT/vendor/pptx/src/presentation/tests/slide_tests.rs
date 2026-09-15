use crate::opc::constants::relationship_type as RT;
use crate::presentation::Presentation;

#[test]
fn test_add_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let layout = &layouts[0];

    let slide_ref = prs.add_slide(layout).unwrap();
    assert_eq!(slide_ref.partname.as_str(), "/ppt/slides/slide1.xml");
    assert_eq!(prs.slide_count().unwrap(), 1);

    // Verify the slide has the correct layout relationship
    let slide_part = prs.package().part(&slide_ref.partname).unwrap();
    let layout_rels = slide_part.rels.all_by_reltype(RT::SLIDE_LAYOUT);
    assert_eq!(layout_rels.len(), 1);
}

#[test]
fn test_add_multiple_slides() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();

    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();
    let s3 = prs.add_slide(&layouts[0]).unwrap();

    assert_eq!(s1.partname.as_str(), "/ppt/slides/slide1.xml");
    assert_eq!(s2.partname.as_str(), "/ppt/slides/slide2.xml");
    assert_eq!(s3.partname.as_str(), "/ppt/slides/slide3.xml");
    assert_eq!(prs.slide_count().unwrap(), 3);
}

#[test]
fn test_slides_in_order() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();

    prs.add_slide(&layouts[0]).unwrap();
    prs.add_slide(&layouts[1]).unwrap();

    let slides = prs.slides().unwrap();
    assert_eq!(slides.len(), 2);
    assert_eq!(slides[0].partname.as_str(), "/ppt/slides/slide1.xml");
    assert_eq!(slides[1].partname.as_str(), "/ppt/slides/slide2.xml");
}

#[test]
fn test_slide_xml_access() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let xml = prs.slide_xml(&slide_ref).unwrap();
    let xml_str = String::from_utf8_lossy(xml);
    assert!(xml_str.contains("<p:sld"));
}

#[test]
fn test_delete_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();

    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();
    assert_eq!(prs.slide_count().unwrap(), 2);

    prs.delete_slide(&s1).unwrap();
    assert_eq!(prs.slide_count().unwrap(), 1);

    // Remaining slide should be s2
    let slides = prs.slides().unwrap();
    assert_eq!(slides[0].partname.as_str(), s2.partname.as_str());
}

#[test]
fn test_slides_get() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();

    let got0 = prs.slides_get(0).unwrap();
    assert_eq!(got0.partname.as_str(), s1.partname.as_str());

    let got1 = prs.slides_get(1).unwrap();
    assert_eq!(got1.partname.as_str(), s2.partname.as_str());
}

#[test]
fn test_slides_get_out_of_bounds() {
    let prs = Presentation::new().unwrap();
    let result = prs.slides_get(0);
    assert!(result.is_err());
}

#[test]
fn test_slide_index() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();
    let s3 = prs.add_slide(&layouts[0]).unwrap();

    assert_eq!(prs.slide_index(&s1).unwrap(), 0);
    assert_eq!(prs.slide_index(&s2).unwrap(), 1);
    assert_eq!(prs.slide_index(&s3).unwrap(), 2);
}

#[test]
fn test_slide_index_not_found() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    prs.delete_slide(&s1).unwrap();

    let result = prs.slide_index(&s1);
    assert!(result.is_err());
}

#[test]
fn test_move_slide_forward() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();
    let s3 = prs.add_slide(&layouts[0]).unwrap();

    // Move slide 0 to position 2
    prs.move_slide(0, 2).unwrap();

    let slides = prs.slides().unwrap();
    assert_eq!(slides[0].partname.as_str(), s2.partname.as_str());
    assert_eq!(slides[1].partname.as_str(), s3.partname.as_str());
    assert_eq!(slides[2].partname.as_str(), s1.partname.as_str());
}

#[test]
fn test_move_slide_backward() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let s2 = prs.add_slide(&layouts[1]).unwrap();
    let s3 = prs.add_slide(&layouts[0]).unwrap();

    // Move slide 2 to position 0
    prs.move_slide(2, 0).unwrap();

    let slides = prs.slides().unwrap();
    assert_eq!(slides[0].partname.as_str(), s3.partname.as_str());
    assert_eq!(slides[1].partname.as_str(), s1.partname.as_str());
    assert_eq!(slides[2].partname.as_str(), s2.partname.as_str());
}

#[test]
fn test_move_slide_noop() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let s1 = prs.add_slide(&layouts[0]).unwrap();
    let _s2 = prs.add_slide(&layouts[1]).unwrap();

    // Moving to same position is a no-op
    prs.move_slide(0, 0).unwrap();

    let slides = prs.slides().unwrap();
    assert_eq!(slides[0].partname.as_str(), s1.partname.as_str());
}

#[test]
fn test_move_slide_out_of_range() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    prs.add_slide(&layouts[0]).unwrap();

    let result = prs.move_slide(0, 5);
    assert!(result.is_err());
}

#[test]
fn test_theme_colors() {
    let prs = Presentation::new().unwrap();
    let colors = prs.theme_colors().unwrap();
    // The default template should have a theme with colors
    assert!(colors.is_some());
    let scheme = colors.unwrap();
    // The default Office theme has dk1 = black (windowText)
    assert_eq!(scheme.dk1.r, 0);
    assert_eq!(scheme.dk1.g, 0);
    assert_eq!(scheme.dk1.b, 0);
    // lt1 should be white (window)
    assert_eq!(scheme.lt1.r, 255);
    assert_eq!(scheme.lt1.g, 255);
    assert_eq!(scheme.lt1.b, 255);
}

#[test]
fn test_slide_name_default_none() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Default slides don't have a name attribute
    let name = prs.slide_name(&slide_ref).unwrap();
    assert_eq!(name, None);
}

#[test]
fn test_has_notes_slide_false() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    assert!(!prs.has_notes_slide(&slide_ref));
}

#[test]
fn test_has_notes_slide_true() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // Create notes slide
    prs.notes_slide_or_create(&slide_ref).unwrap();
    assert!(prs.has_notes_slide(&slide_ref));
}

#[test]
fn test_slide_layout_for() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let layout_name = layouts[0].name.clone();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    let layout = prs.slide_layout_for(&slide_ref).unwrap();
    assert!(layout.is_some());
    assert_eq!(layout.unwrap().name, layout_name);
}

#[test]
fn test_slide_layout_for_second_layout() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let layout_name = layouts[1].name.clone();
    let slide_ref = prs.add_slide(&layouts[1]).unwrap();

    let layout = prs.slide_layout_for(&slide_ref).unwrap();
    assert!(layout.is_some());
    assert_eq!(layout.unwrap().name, layout_name);
}

#[test]
fn test_slide_placeholders_blank_slide() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let slide_ref = prs.add_slide(&layouts[0]).unwrap();

    // A blank slide from new_slide_xml has no placeholders
    let placeholders = prs.slide_placeholders(&slide_ref).unwrap();
    assert!(placeholders.is_empty());
}

#[test]
fn test_remove_slide_layout_in_use_fails() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let layout_clone = layouts[0].clone();
    prs.add_slide(&layouts[0]).unwrap();

    // Trying to remove a layout used by a slide should fail
    let result = prs.remove_slide_layout(&layout_clone);
    assert!(result.is_err());
}

#[test]
fn test_remove_slide_layout_unused() {
    let mut prs = Presentation::new().unwrap();
    let layouts = prs.slide_layouts().unwrap();
    let initial_count = layouts.len();

    // Add a slide using layout[0], then try to remove layout[5] (unused)
    prs.add_slide(&layouts[0]).unwrap();
    let layout_to_remove = layouts[5].clone();
    prs.remove_slide_layout(&layout_to_remove).unwrap();

    let layouts_after = prs.slide_layouts().unwrap();
    assert_eq!(layouts_after.len(), initial_count - 1);

    // The removed layout should not be in the list
    assert!(!layouts_after
        .iter()
        .any(|l| l.partname.as_str() == layout_to_remove.partname.as_str()));
}

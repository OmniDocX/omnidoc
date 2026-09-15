//! Slide, layout, master, and notes types, parsing, and XML generation.

pub mod background;
mod parse;
#[allow(clippy::redundant_pub_crate)]
mod parse_pres;
#[allow(clippy::redundant_pub_crate)]
mod query;
mod types;
#[allow(clippy::redundant_pub_crate)]
mod xml_gen;

#[cfg(test)]
mod tests;

// Re-export types so external callers see the same public API.
pub use types::{
    NotesMasterRef, NotesSlide, NotesSlideRef, SlideLayoutRef, SlideMasterRef, SlideProperties,
    SlideRef,
};

// Re-export slide/notes parsing functions.
pub use parse::{
    parse_notes_slide, parse_notes_slide_text, parse_notes_slide_with_part_name, parse_slide_name,
};

// Re-export presentation-level parsing functions.
pub(crate) use parse_pres::{
    parse_layout_name, parse_slide_ids, parse_slide_master_ids, parse_slide_size,
};

// Re-export query/manipulation functions.
pub(crate) use query::extract_layout_r_ids;
pub use query::{get_layout_by_name, placeholder_shapes_from_layout};
pub(crate) use query::{layout_used_by_slides, remove_layout_from_master_xml};

// Re-export background functions.
pub(crate) use background::{
    set_follow_master_background, set_slide_background_gradient, set_slide_background_image,
    set_slide_background_solid,
};

// Re-export XML generation functions.
pub(crate) use xml_gen::{
    add_slide_id_to_presentation_xml, new_notes_master_xml, new_notes_slide_xml, new_slide_xml,
    next_slide_id, remove_slide_id_from_presentation_xml, reorder_slide_in_presentation_xml,
    set_slide_size_in_xml,
};

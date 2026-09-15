use super::*;
use crate::text::font::RgbColor;
use crate::text::TextFrame;

#[test]
fn test_data_labels_font() {
    let mut dl = DataLabels::new();
    assert!(dl.font().is_none());

    let font = dl.font_mut();
    font.bold = Some(true);
    font.size = Some(10.0);
    font.color = Some(RgbColor::new(0, 0, 0));
    assert!(dl.font().is_some());
    assert_eq!(dl.font().unwrap().size, Some(10.0));
}

#[test]
fn test_data_label_font() {
    let mut dl = DataLabel::new();
    assert!(dl.font().is_none());

    let font = dl.font_mut();
    font.italic = Some(true);
    assert!(dl.font().is_some());
    assert_eq!(dl.font().unwrap().italic, Some(true));
}

// -----------------------------------------------------------------------
// DataLabel.text_frame tests
// -----------------------------------------------------------------------

#[test]
fn test_data_label_text_frame_default_none() {
    let dl = DataLabel::new();
    assert!(!dl.has_text_frame());
    assert!(dl.text_frame().is_none());
}

#[test]
fn test_data_label_text_frame_mut_creates_default() {
    let mut dl = DataLabel::new();
    dl.text_frame_mut().set_text("Custom Label");
    assert!(dl.has_text_frame());
    assert!(dl.text_frame().is_some());
    assert_eq!(dl.text_frame().unwrap().text(), "Custom Label");
}

#[test]
fn test_data_label_set_text_frame() {
    let mut dl = DataLabel::new();
    let mut tf = TextFrame::new();
    tf.set_text("Explicit TF");
    dl.set_text_frame(tf);
    assert!(dl.has_text_frame());
    assert_eq!(dl.text_frame().unwrap().text(), "Explicit TF");
}

#[test]
fn test_data_label_text_frame_with_font() {
    let mut dl = DataLabel::new();
    let tf = dl.text_frame_mut();
    tf.set_text("Styled");
    // Also set font independently
    dl.font_mut().bold = Some(true);
    assert!(dl.has_text_frame());
    assert_eq!(dl.text_frame().unwrap().text(), "Styled");
    assert_eq!(dl.font().unwrap().bold, Some(true));
}

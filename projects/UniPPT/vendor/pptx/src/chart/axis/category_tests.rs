//! Tests for `CategoryAxis`.

#[cfg(test)]
mod tests {
    use crate::chart::axis::category::CategoryAxis;
    use crate::chart::axis::{AxisTitle, TickLabels};

    #[test]
    fn test_category_axis_reverse_order() {
        let mut axis = CategoryAxis::new();
        assert!(!axis.reverse_order());
        axis.set_reverse_order(true);
        assert!(axis.reverse_order());
    }

    #[test]
    fn test_category_axis_tick_labels() {
        let mut axis = CategoryAxis::new();
        assert!(axis.tick_labels().is_none());
        axis.tick_labels_mut().number_format = Some("0%".to_string());
        assert!(axis.tick_labels().is_some());
        assert_eq!(
            axis.tick_labels().unwrap().number_format.as_deref(),
            Some("0%")
        );
    }

    #[test]
    fn test_category_axis_set_tick_labels() {
        let mut axis = CategoryAxis::new();
        let mut tl = TickLabels::new();
        tl.offset = Some(50);
        axis.set_tick_labels(tl);
        assert_eq!(axis.tick_labels().unwrap().offset, Some(50));
    }

    #[test]
    fn test_category_axis_format() {
        let mut axis = CategoryAxis::new();
        assert!(axis.format().is_none());
        let _fmt = axis.format_mut();
        assert!(axis.format().is_some());
    }

    #[test]
    fn test_category_axis_gridline_formats() {
        let mut axis = CategoryAxis::new();
        assert!(axis.major_gridline_format().is_none());
        assert!(axis.minor_gridline_format().is_none());
        let _major = axis.major_gridline_format_mut();
        let _minor = axis.minor_gridline_format_mut();
        assert!(axis.major_gridline_format().is_some());
        assert!(axis.minor_gridline_format().is_some());
    }

    #[test]
    fn test_category_axis_set_title_creates_axis_title() {
        let mut axis = CategoryAxis::new();
        axis.set_title("X Axis");
        assert_eq!(axis.title(), Some("X Axis"));
        assert!(axis.axis_title().is_some());
        assert_eq!(
            axis.axis_title().unwrap().text(),
            Some("X Axis".to_string())
        );
    }

    #[test]
    fn test_category_axis_axis_title_mut() {
        let mut axis = CategoryAxis::new();
        axis.axis_title_mut().text_frame_mut().set_text("Custom");
        assert!(axis.has_title());
        assert!(axis.axis_title().is_some());
    }

    #[test]
    fn test_category_axis_set_axis_title() {
        let mut axis = CategoryAxis::new();
        let at = AxisTitle::from_text("Months");
        axis.set_axis_title(at);
        assert!(axis.has_title());
        assert_eq!(axis.title(), Some("Months"));
    }

    #[test]
    fn test_category_axis_set_has_title_false_clears_axis_title() {
        let mut axis = CategoryAxis::new();
        axis.set_title("Title");
        assert!(axis.axis_title().is_some());
        axis.set_has_title(false);
        assert!(axis.axis_title().is_none());
        assert!(axis.title().is_none());
    }
}

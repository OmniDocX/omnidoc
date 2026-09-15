//! Tests for `DateAxis`.

#[cfg(test)]
mod tests {
    use crate::chart::axis::date::DateAxis;
    use crate::chart::axis::{AxisTitle, TickLabels};
    use crate::chart::chart::ChartFormat;
    use crate::enums::chart::{XlAxisCrosses, XlTickLabelPosition, XlTickMark};

    #[test]
    fn test_date_axis_defaults() {
        let axis = DateAxis::new();
        assert!(!axis.has_title());
        assert!(axis.title().is_none());
        assert!(axis.axis_title().is_none());
        assert!(axis.visible());
        assert_eq!(axis.major_tick_mark(), XlTickMark::Outside);
        assert_eq!(axis.minor_tick_mark(), XlTickMark::None);
        assert_eq!(axis.tick_label_position(), XlTickLabelPosition::NextToAxis);
        assert!(!axis.has_major_gridlines());
        assert!(!axis.has_minor_gridlines());
        assert_eq!(axis.crosses(), XlAxisCrosses::Automatic);
        assert!(axis.crosses_at().is_none());
        assert_eq!(axis.number_format(), "General");
        assert!(axis.number_format_is_linked());
        assert!(!axis.reverse_order());
        assert!(axis.tick_labels().is_none());
        assert!(axis.format().is_none());
    }

    #[test]
    fn test_date_axis_set_title_creates_axis_title() {
        let mut axis = DateAxis::new();
        axis.set_title("Date");
        assert!(axis.has_title());
        assert_eq!(axis.title(), Some("Date"));
        assert!(axis.axis_title().is_some());
        assert_eq!(axis.axis_title().unwrap().text(), Some("Date".to_string()));
    }

    #[test]
    fn test_date_axis_set_has_title_false_clears() {
        let mut axis = DateAxis::new();
        axis.set_title("Date");
        assert!(axis.axis_title().is_some());
        axis.set_has_title(false);
        assert!(!axis.has_title());
        assert!(axis.title().is_none());
        assert!(axis.axis_title().is_none());
    }

    #[test]
    fn test_date_axis_axis_title_mut() {
        let mut axis = DateAxis::new();
        axis.axis_title_mut().text_frame_mut().set_text("Dates");
        assert!(axis.has_title());
        assert!(axis.axis_title().is_some());
    }

    #[test]
    fn test_date_axis_set_axis_title() {
        let mut axis = DateAxis::new();
        let at = AxisTitle::from_text("Timeline");
        axis.set_axis_title(at);
        assert!(axis.has_title());
        assert_eq!(axis.title(), Some("Timeline"));
    }

    #[test]
    fn test_date_axis_visibility() {
        let mut axis = DateAxis::new();
        assert!(axis.visible());
        axis.set_visible(false);
        assert!(!axis.visible());
    }

    #[test]
    fn test_date_axis_tick_marks() {
        let mut axis = DateAxis::new();
        axis.set_major_tick_mark(XlTickMark::Inside);
        axis.set_minor_tick_mark(XlTickMark::Cross);
        assert_eq!(axis.major_tick_mark(), XlTickMark::Inside);
        assert_eq!(axis.minor_tick_mark(), XlTickMark::Cross);
    }

    #[test]
    fn test_date_axis_tick_label_position() {
        let mut axis = DateAxis::new();
        axis.set_tick_label_position(XlTickLabelPosition::Low);
        assert_eq!(axis.tick_label_position(), XlTickLabelPosition::Low);
    }

    #[test]
    fn test_date_axis_gridlines() {
        let mut axis = DateAxis::new();
        assert!(!axis.has_major_gridlines());
        assert!(!axis.has_minor_gridlines());
        axis.set_has_major_gridlines(true);
        axis.set_has_minor_gridlines(true);
        assert!(axis.has_major_gridlines());
        assert!(axis.has_minor_gridlines());
    }

    #[test]
    fn test_date_axis_crosses() {
        let mut axis = DateAxis::new();
        axis.set_crosses(XlAxisCrosses::Minimum);
        assert_eq!(axis.crosses(), XlAxisCrosses::Minimum);
    }

    #[test]
    fn test_date_axis_crosses_at() {
        let mut axis = DateAxis::new();
        assert!(axis.crosses_at().is_none());
        axis.set_crosses_at(Some(42000.0));
        assert_eq!(axis.crosses_at(), Some(42000.0));
        axis.set_crosses_at(None);
        assert!(axis.crosses_at().is_none());
    }

    #[test]
    fn test_date_axis_number_format() {
        let mut axis = DateAxis::new();
        axis.set_number_format("yyyy-mm-dd");
        assert_eq!(axis.number_format(), "yyyy-mm-dd");
    }

    #[test]
    fn test_date_axis_number_format_is_linked() {
        let mut axis = DateAxis::new();
        assert!(axis.number_format_is_linked());
        axis.set_number_format_is_linked(false);
        assert!(!axis.number_format_is_linked());
    }

    #[test]
    fn test_date_axis_reverse_order() {
        let mut axis = DateAxis::new();
        assert!(!axis.reverse_order());
        axis.set_reverse_order(true);
        assert!(axis.reverse_order());
    }

    #[test]
    fn test_date_axis_tick_labels() {
        let mut axis = DateAxis::new();
        assert!(axis.tick_labels().is_none());
        axis.tick_labels_mut().number_format = Some("mm/dd".to_string());
        assert!(axis.tick_labels().is_some());
        assert_eq!(
            axis.tick_labels().unwrap().number_format.as_deref(),
            Some("mm/dd")
        );
    }

    #[test]
    fn test_date_axis_set_tick_labels() {
        let mut axis = DateAxis::new();
        let mut tl = TickLabels::new();
        tl.offset = Some(200);
        axis.set_tick_labels(tl);
        assert_eq!(axis.tick_labels().unwrap().offset, Some(200));
    }

    #[test]
    fn test_date_axis_format() {
        let mut axis = DateAxis::new();
        assert!(axis.format().is_none());
        let _fmt = axis.format_mut();
        assert!(axis.format().is_some());
    }

    #[test]
    fn test_date_axis_set_format() {
        let mut axis = DateAxis::new();
        let fmt = ChartFormat::new();
        axis.set_format(fmt);
        assert!(axis.format().is_some());
    }

    #[test]
    fn test_date_axis_gridline_formats() {
        let mut axis = DateAxis::new();
        assert!(axis.major_gridline_format().is_none());
        assert!(axis.minor_gridline_format().is_none());
        let _major = axis.major_gridline_format_mut();
        let _minor = axis.minor_gridline_format_mut();
        assert!(axis.major_gridline_format().is_some());
        assert!(axis.minor_gridline_format().is_some());
    }
}

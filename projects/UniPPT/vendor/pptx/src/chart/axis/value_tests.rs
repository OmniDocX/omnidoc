//! Tests for `ValueAxis`.

#[cfg(test)]
mod tests {
    use crate::chart::axis::value::ValueAxis;
    use crate::chart::axis::AxisTitle;
    use crate::chart::chart::ChartFormat;

    #[test]
    fn test_value_axis_crosses_at() {
        let mut axis = ValueAxis::new();
        assert!(axis.crosses_at().is_none());
        axis.set_crosses_at(Some(100.0));
        assert_eq!(axis.crosses_at(), Some(100.0));
        axis.set_crosses_at(None);
        assert!(axis.crosses_at().is_none());
    }

    #[test]
    fn test_value_axis_reverse_order() {
        let mut axis = ValueAxis::new();
        assert!(!axis.reverse_order());
        axis.set_reverse_order(true);
        assert!(axis.reverse_order());
    }

    #[test]
    fn test_value_axis_tick_labels() {
        let mut axis = ValueAxis::new();
        assert!(axis.tick_labels().is_none());
        axis.tick_labels_mut().number_format = Some("#,##0.00".to_string());
        assert_eq!(
            axis.tick_labels().unwrap().number_format.as_deref(),
            Some("#,##0.00")
        );
    }

    #[test]
    fn test_value_axis_format() {
        let mut axis = ValueAxis::new();
        assert!(axis.format().is_none());
        let fmt = ChartFormat::new();
        axis.set_format(fmt);
        assert!(axis.format().is_some());
    }

    #[test]
    fn test_value_axis_gridline_formats() {
        let mut axis = ValueAxis::new();
        assert!(axis.major_gridline_format().is_none());
        assert!(axis.minor_gridline_format().is_none());
        let _major = axis.major_gridline_format_mut();
        let _minor = axis.minor_gridline_format_mut();
        assert!(axis.major_gridline_format().is_some());
        assert!(axis.minor_gridline_format().is_some());
    }

    #[test]
    fn test_value_axis_set_title_creates_axis_title() {
        let mut axis = ValueAxis::new();
        axis.set_title("Y Axis");
        assert_eq!(axis.title(), Some("Y Axis"));
        assert!(axis.axis_title().is_some());
        assert_eq!(
            axis.axis_title().unwrap().text(),
            Some("Y Axis".to_string())
        );
    }

    #[test]
    fn test_value_axis_axis_title_mut() {
        let mut axis = ValueAxis::new();
        axis.axis_title_mut().text_frame_mut().set_text("Values");
        assert!(axis.has_title());
    }

    #[test]
    fn test_value_axis_set_axis_title() {
        let mut axis = ValueAxis::new();
        let at = AxisTitle::from_text("Sales");
        axis.set_axis_title(at);
        assert!(axis.has_title());
        assert_eq!(axis.title(), Some("Sales"));
    }

    #[test]
    fn test_value_axis_set_has_title_false_clears_axis_title() {
        let mut axis = ValueAxis::new();
        axis.set_title("Title");
        assert!(axis.axis_title().is_some());
        axis.set_has_title(false);
        assert!(axis.axis_title().is_none());
        assert!(axis.title().is_none());
    }
}

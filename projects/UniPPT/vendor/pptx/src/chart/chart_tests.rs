use super::*;
use crate::chart::data::CategoryChartData;
use crate::chart::datalabel::DataLabels;
use crate::chart::series::Series;
use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;
use crate::enums::chart::XlChartType;
use crate::text::font::{Font, RgbColor};
use crate::text::TextFrame;

#[test]
fn test_chart_format() {
    let mut chart = Chart::new(XlChartType::ColumnClustered);
    assert!(chart.chart_format().is_none());

    let fmt = chart.chart_format_mut();
    fmt.fill = Some(FillFormat::solid(ColorFormat::rgb(240, 240, 240)));
    assert!(chart.chart_format().is_some());
}

#[test]
fn test_chart_plot_properties() {
    let mut chart = Chart::new(XlChartType::BarClustered);
    chart.plot_properties_mut().set_gap_width(Some(200));
    chart.plot_properties_mut().set_overlap(Some(-25));
    assert_eq!(chart.plot_properties().gap_width(), Some(200));
    assert_eq!(chart.plot_properties().overlap(), Some(-25));
}

#[test]
fn test_replace_data() {
    let chart = Chart::new(XlChartType::ColumnClustered);

    let mut data = CategoryChartData::new();
    data.add_category("A");
    data.add_category("B");
    data.add_series("New", &[10.0, 20.0]);

    let xml = chart.replace_data(&data).unwrap();
    assert!(xml.contains("<c:barChart>"));
    assert!(xml.contains("<c:v>A</c:v>"));
    assert!(xml.contains("<c:v>B</c:v>"));
    assert!(xml.contains("<c:v>10</c:v>"));
    assert!(xml.contains("<c:v>20</c:v>"));
}

#[test]
fn test_3d_chart_no_axes_for_pie3d() {
    let chart = Chart::new(XlChartType::Pie3D);
    assert!(chart.category_axis().is_none());
    assert!(chart.value_axis().is_none());
}

#[test]
fn test_surface_chart_no_axes() {
    let chart = Chart::new(XlChartType::Surface);
    assert!(chart.category_axis().is_none());
    assert!(chart.value_axis().is_none());
}

#[test]
fn test_chart_title_new() {
    let ct = ChartTitle::new();
    assert!(!ct.has_text_frame());
    assert!(ct.text_frame().is_none());
    assert!(ct.text().is_none());
    assert!(ct.format.is_none());
}

#[test]
fn test_chart_title_from_text() {
    let ct = ChartTitle::from_text("Sales Report");
    assert!(ct.has_text_frame());
    assert_eq!(ct.text(), Some("Sales Report".to_string()));
}

#[test]
fn test_chart_title_text_frame_mut() {
    let mut ct = ChartTitle::new();
    ct.text_frame_mut().set_text("Dynamic Title");
    assert!(ct.has_text_frame());
    assert_eq!(ct.text(), Some("Dynamic Title".to_string()));
}

#[test]
fn test_chart_title_set_text_frame() {
    let mut ct = ChartTitle::new();
    let mut tf = TextFrame::new();
    tf.set_text("Custom");
    ct.set_text_frame(tf);
    assert!(ct.has_text_frame());
    assert_eq!(ct.text(), Some("Custom".to_string()));
}

#[test]
fn test_chart_title_format() {
    let mut ct = ChartTitle::from_text("Title");
    ct.format = Some(ChartFormat::new());
    assert!(ct.format.is_some());
}

#[test]
fn test_chart_set_title_creates_chart_title() {
    let mut chart = Chart::new(XlChartType::ColumnClustered);
    chart.set_title("My Chart");
    assert!(chart.has_title());
    assert_eq!(chart.title(), Some("My Chart"));
    assert!(chart.chart_title().is_some());
    assert_eq!(
        chart.chart_title().unwrap().text(),
        Some("My Chart".to_string())
    );
}

#[test]
fn test_chart_chart_title_mut() {
    let mut chart = Chart::new(XlChartType::Line);
    chart
        .chart_title_mut()
        .text_frame_mut()
        .set_text("Rich Title");
    assert!(chart.has_title());
    assert!(chart.chart_title().is_some());
}

#[test]
fn test_chart_set_chart_title() {
    let mut chart = Chart::new(XlChartType::BarClustered);
    let ct = ChartTitle::from_text("Overview");
    chart.set_chart_title(ct);
    assert!(chart.has_title());
    assert_eq!(chart.title(), Some("Overview"));
    assert_eq!(
        chart.chart_title().unwrap().text(),
        Some("Overview".to_string())
    );
}

#[test]
fn test_chart_set_has_title_false_clears_chart_title() {
    let mut chart = Chart::new(XlChartType::Pie);
    chart.set_title("Pie Title");
    assert!(chart.chart_title().is_some());
    chart.set_has_title(false);
    assert!(chart.chart_title().is_none());
    assert!(chart.title().is_none());
    assert!(!chart.has_title());
}

#[test]
fn test_chart_title_backward_compat() {
    // Verify set_title / title() still works the same as before
    let mut chart = Chart::new(XlChartType::ColumnClustered);
    assert!(chart.title().is_none());
    chart.set_title("Test");
    assert_eq!(chart.title(), Some("Test"));
    chart.set_has_title(false);
    assert!(chart.title().is_none());
}

// -----------------------------------------------------------------------
// Chart.font tests
// -----------------------------------------------------------------------

#[test]
fn test_chart_font_default_none() {
    let chart = Chart::new(XlChartType::ColumnClustered);
    assert!(chart.font().is_none());
}

#[test]
fn test_chart_font_mut_creates_default() {
    let mut chart = Chart::new(XlChartType::ColumnClustered);
    let font = chart.font_mut();
    font.size = Some(14.0);
    font.bold = Some(true);
    assert!(chart.font().is_some());
    assert_eq!(chart.font().unwrap().size, Some(14.0));
    assert_eq!(chart.font().unwrap().bold, Some(true));
}

#[test]
fn test_chart_set_font() {
    let mut chart = Chart::new(XlChartType::Line);
    let mut font = Font::new();
    font.name = Some("Calibri".to_string());
    font.size = Some(12.0);
    font.color = Some(RgbColor::new(0, 0, 0));
    chart.set_font(font);
    assert!(chart.font().is_some());
    let f = chart.font().unwrap();
    assert_eq!(f.name.as_deref(), Some("Calibri"));
    assert_eq!(f.size, Some(12.0));
}

// -----------------------------------------------------------------------
// Chart.plots tests
// -----------------------------------------------------------------------

#[test]
fn test_chart_has_default_plot() {
    let chart = Chart::new(XlChartType::BarClustered);
    assert_eq!(chart.plots().len(), 1);
    assert_eq!(chart.plots()[0].chart_type, XlChartType::BarClustered);
}

#[test]
fn test_chart_series_delegates_to_first_plot() {
    let mut chart = Chart::new(XlChartType::ColumnClustered);
    assert!(chart.series().is_empty());

    // Add series via the backward-compatible API

    chart
        .series_mut()
        .add(Series::new("S1", 0, XlChartType::ColumnClustered));
    assert_eq!(chart.series().len(), 1);

    // Verify it's actually in the first plot
    assert_eq!(chart.plots()[0].series().len(), 1);
}

#[test]
fn test_chart_plot_properties_delegates_to_first_plot() {
    let mut chart = Chart::new(XlChartType::BarClustered);
    chart.plot_properties_mut().set_gap_width(Some(250));
    assert_eq!(chart.plots()[0].plot_properties().gap_width(), Some(250));
}

#[test]
fn test_chart_add_plot() {
    let mut chart = Chart::new(XlChartType::ColumnClustered);
    let line_plot = Plot::new(XlChartType::Line);
    chart.add_plot(line_plot);
    assert_eq!(chart.plots().len(), 2);
    assert_eq!(chart.plots()[1].chart_type, XlChartType::Line);
}

#[test]
fn test_chart_plots_mut() {
    let mut chart = Chart::new(XlChartType::Pie);
    chart.plots_mut()[0].has_data_labels = true;
    assert!(chart.plots()[0].has_data_labels);
}

// -----------------------------------------------------------------------
// Plot tests
// -----------------------------------------------------------------------

#[test]
fn test_plot_new() {
    let plot = Plot::new(XlChartType::Line);
    assert_eq!(plot.chart_type, XlChartType::Line);
    assert!(plot.series().is_empty());
    assert!(plot.data_labels().is_none());
    assert!(!plot.has_data_labels);
    assert!(plot.categories.is_none());
}

#[test]
fn test_plot_data_labels_mut() {
    let mut plot = Plot::new(XlChartType::ColumnClustered);
    plot.data_labels_mut().set_show_value(true);
    assert!(plot.has_data_labels);
    assert!(plot.data_labels().unwrap().show_value());
}

#[test]
fn test_plot_set_data_labels() {
    let mut plot = Plot::new(XlChartType::Pie);
    let mut dl = DataLabels::new();
    dl.set_show_percent(true);
    plot.set_data_labels(dl);
    assert!(plot.has_data_labels);
    assert!(plot.data_labels().unwrap().show_percent());
}

#[test]
fn test_plot_series_access() {
    let mut plot = Plot::new(XlChartType::Line);
    plot.series_mut()
        .add(Series::new("A", 0, XlChartType::Line));
    plot.series_mut()
        .add(Series::new("B", 1, XlChartType::Line));
    assert_eq!(plot.series().len(), 2);
    assert_eq!(plot.series().get(0).unwrap().name(), "A");
}

#[test]
fn test_plot_properties_access() {
    let mut plot = Plot::new(XlChartType::BarClustered);
    plot.plot_properties_mut().set_gap_width(Some(300));
    plot.plot_properties_mut().set_overlap(Some(50));
    assert_eq!(plot.plot_properties().gap_width(), Some(300));
    assert_eq!(plot.plot_properties().overlap(), Some(50));
}

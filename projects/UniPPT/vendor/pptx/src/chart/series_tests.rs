use super::*;
use crate::dml::color::ColorFormat;
use crate::dml::fill::FillFormat;

#[test]
fn test_series_format() {
    let mut series = Series::new("Test", 0, XlChartType::ColumnClustered);
    assert!(series.format().is_none());

    let fmt = series.format_mut();
    fmt.fill = Some(FillFormat::solid(ColorFormat::rgb(255, 0, 0)));
    assert!(series.format().is_some());
    assert!(series.format().unwrap().fill.is_some());
}

#[test]
fn test_series_values() {
    let mut series = Series::new("Test", 0, XlChartType::Line);
    assert!(series.values().is_empty());

    series.set_values(vec![Some(1.0), Some(2.0), None, Some(4.0)]);
    assert_eq!(series.values().len(), 4);
    assert_eq!(series.values()[0], Some(1.0));
    assert_eq!(series.values()[2], None);
}

#[test]
fn test_series_points() {
    let mut series = Series::new("Test", 0, XlChartType::ColumnClustered);
    assert!(series.points().is_empty());

    let mut pt = Point::new(2);
    let mut fmt = SeriesFormat::new();
    fmt.fill = Some(FillFormat::solid(ColorFormat::rgb(0, 255, 0)));
    pt.set_format(fmt);
    series.add_point(pt);

    assert_eq!(series.points().len(), 1);
    assert_eq!(series.points()[0].index(), 2);
    assert!(series.points()[0].format().is_some());
}

#[test]
fn test_series_collection() {
    let mut coll = SeriesCollection::new();
    assert!(coll.is_empty());

    coll.add(Series::new("A", 0, XlChartType::Line));
    coll.add(Series::new("B", 1, XlChartType::Line));
    assert_eq!(coll.len(), 2);
    assert_eq!(coll.get(0).unwrap().name(), "A");
    assert_eq!(coll.get(1).unwrap().name(), "B");
}

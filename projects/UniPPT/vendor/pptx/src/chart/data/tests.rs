use super::*;

#[test]
fn test_hierarchical_categories() {
    let mut data = CategoryChartData::new();
    data.set_hierarchical_categories(vec![
        vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
        vec!["H1".into(), "H1".into(), "H2".into(), "H2".into()],
    ]);

    assert_eq!(data.category_depth(), 2);
    assert_eq!(data.categories(), &["Q1", "Q2", "Q3", "Q4"]);

    let hier = data.hierarchical_categories().unwrap();
    assert_eq!(hier.len(), 2);
    assert_eq!(hier[0], vec!["Q1", "Q2", "Q3", "Q4"]);
    assert_eq!(hier[1], vec!["H1", "H1", "H2", "H2"]);
}

#[test]
fn test_flat_categories_depth() {
    let mut data = CategoryChartData::new();
    data.add_category("A");
    data.add_category("B");
    assert_eq!(data.category_depth(), 1);
    assert!(data.hierarchical_categories().is_none());
}

#[test]
fn test_hierarchical_sets_leaf_as_categories() {
    let mut data = CategoryChartData::new();
    data.add_category("Old");
    data.set_hierarchical_categories(vec![
        vec!["New1".into(), "New2".into()],
        vec!["Parent".into(), "Parent".into()],
    ]);
    // The leaf level should replace previous categories
    assert_eq!(data.categories(), &["New1", "New2"]);
}

// -----------------------------------------------------------------------
// DateAxisChartData tests
// -----------------------------------------------------------------------

#[test]
fn test_date_axis_chart_data_new() {
    let data = DateAxisChartData::new();
    assert!(data.dates().is_empty());
    assert!(data.series().is_empty());
    assert_eq!(data.number_format(), "General");
    assert_eq!(data.date_format(), "yyyy-mm-dd");
}

#[test]
fn test_date_axis_chart_data_with_number_format() {
    let data = DateAxisChartData::with_number_format("#,##0.00");
    assert_eq!(data.number_format(), "#,##0.00");
}

#[test]
fn test_date_axis_chart_data_add_dates() {
    let mut data = DateAxisChartData::new();
    data.add_date("2024-01-01");
    data.add_date("2024-02-01");
    data.add_date("2024-03-01");
    assert_eq!(data.dates().len(), 3);
    assert_eq!(data.dates()[0], "2024-01-01");
    assert_eq!(data.dates()[2], "2024-03-01");
}

#[test]
fn test_date_axis_chart_data_add_series() {
    let mut data = DateAxisChartData::new();
    data.add_date("2024-01-01");
    data.add_date("2024-02-01");
    data.add_series("Revenue", &[100.0, 200.0]);
    assert_eq!(data.series().len(), 1);
    assert_eq!(data.series()[0].name(), "Revenue");
    assert_eq!(data.series()[0].values(), &[Some(100.0), Some(200.0)]);
}

#[test]
fn test_date_axis_chart_data_add_series_with_options() {
    let mut data = DateAxisChartData::new();
    data.add_date("2024-01-01");
    data.add_date("2024-02-01");
    data.add_series_with_options("Revenue", &[Some(100.0), None]);
    assert_eq!(data.series()[0].values(), &[Some(100.0), None]);
}

#[test]
fn test_date_axis_chart_data_set_date_format() {
    let mut data = DateAxisChartData::new();
    data.set_date_format("mm/dd/yyyy");
    assert_eq!(data.date_format(), "mm/dd/yyyy");
}

#[test]
fn test_date_axis_chart_data_to_category() {
    let mut data = DateAxisChartData::new();
    data.add_date("2024-01-01");
    data.add_date("2024-02-01");
    data.add_series("Revenue", &[100.0, 200.0]);

    let cat_data = data.to_category_chart_data();
    assert_eq!(cat_data.categories(), &["2024-01-01", "2024-02-01"]);
    assert_eq!(cat_data.series().len(), 1);
    assert_eq!(cat_data.series()[0].name(), "Revenue");
    assert_eq!(cat_data.series()[0].values(), &[Some(100.0), Some(200.0)]);
}

#[test]
fn test_date_axis_chart_data_multiple_series() {
    let mut data = DateAxisChartData::new();
    data.add_date("2024-Q1");
    data.add_date("2024-Q2");
    data.add_series("Sales", &[50.0, 60.0]);
    data.add_series("Costs", &[30.0, 40.0]);
    assert_eq!(data.series().len(), 2);
    assert_eq!(data.series()[0].index(), 0);
    assert_eq!(data.series()[1].index(), 1);
}

// -----------------------------------------------------------------------
// Categories object model tests
// -----------------------------------------------------------------------

#[test]
fn test_categories_from_flat_data() {
    let mut data = CategoryChartData::new();
    data.add_category("A");
    data.add_category("B");
    data.add_category("C");

    let cats = data.categories_object();
    assert_eq!(cats.depth(), 1);
    assert_eq!(cats.levels().len(), 1);
    assert_eq!(cats.levels()[0].len(), 3);
    assert_eq!(cats.levels()[0].categories()[0].label, "A");
    assert_eq!(cats.levels()[0].categories()[1].label, "B");
    assert_eq!(cats.levels()[0].categories()[2].label, "C");
}

#[test]
fn test_categories_from_hierarchical_data() {
    let mut data = CategoryChartData::new();
    data.set_hierarchical_categories(vec![
        vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
        vec!["H1".into(), "H1".into(), "H2".into(), "H2".into()],
    ]);

    let cats = data.categories_object();
    assert_eq!(cats.depth(), 2);
    assert_eq!(cats.levels().len(), 2);

    // Level 0 = leaf
    assert_eq!(cats.levels()[0].len(), 4);
    assert_eq!(cats.levels()[0].categories()[0].label, "Q1");
    assert_eq!(cats.levels()[0].categories()[3].label, "Q4");

    // Level 1 = root
    assert_eq!(cats.levels()[1].len(), 4);
    assert_eq!(cats.levels()[1].categories()[0].label, "H1");
    assert_eq!(cats.levels()[1].categories()[2].label, "H2");
}

#[test]
fn test_categories_iter() {
    let mut data = CategoryChartData::new();
    data.add_category("X");
    data.add_category("Y");

    let cats = data.categories_object();
    let labels: Vec<&str> = cats.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, vec!["X", "Y"]);
}

#[test]
fn test_categories_into_iter() {
    let mut data = CategoryChartData::new();
    data.add_category("P");
    data.add_category("Q");

    let cats = data.categories_object();
    let labels: Vec<&str> = (&cats).into_iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, vec!["P", "Q"]);
}

#[test]
fn test_categories_empty() {
    let data = CategoryChartData::new();
    let cats = data.categories_object();
    assert_eq!(cats.depth(), 1);
    assert_eq!(cats.levels()[0].len(), 0);
    assert!(cats.levels()[0].is_empty());
}

#[test]
fn test_flattened_labels_flat() {
    let mut data = CategoryChartData::new();
    data.add_category("A");
    data.add_category("B");
    data.add_category("C");

    let cats = data.categories_object();
    let labels = cats.flattened_labels();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0], vec!["A"]);
    assert_eq!(labels[1], vec!["B"]);
    assert_eq!(labels[2], vec!["C"]);
}

#[test]
fn test_flattened_labels_hierarchical() {
    let mut data = CategoryChartData::new();
    data.set_hierarchical_categories(vec![
        vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()],
        vec!["H1".into(), "H1".into(), "H2".into(), "H2".into()],
    ]);

    let cats = data.categories_object();
    let labels = cats.flattened_labels();
    assert_eq!(labels.len(), 4);
    assert_eq!(labels[0], vec!["Q1", "H1"]);
    assert_eq!(labels[1], vec!["Q2", "H1"]);
    assert_eq!(labels[2], vec!["Q3", "H2"]);
    assert_eq!(labels[3], vec!["Q4", "H2"]);
}

#[test]
fn test_flattened_labels_three_levels() {
    let mut data = CategoryChartData::new();
    data.set_hierarchical_categories(vec![
        vec!["Jan".into(), "Feb".into(), "Mar".into(), "Apr".into()],
        vec!["Q1".into(), "Q1".into(), "Q1".into(), "Q2".into()],
        vec!["2024".into(), "2024".into(), "2024".into(), "2024".into()],
    ]);

    let cats = data.categories_object();
    assert_eq!(cats.depth(), 3);
    let labels = cats.flattened_labels();
    assert_eq!(labels.len(), 4);
    assert_eq!(labels[0], vec!["Jan", "Q1", "2024"]);
    assert_eq!(labels[3], vec!["Apr", "Q2", "2024"]);
}

#[test]
fn test_flattened_labels_empty() {
    let data = CategoryChartData::new();
    let cats = data.categories_object();
    let labels = cats.flattened_labels();
    assert!(labels.is_empty());
}

#[test]
fn test_category_level_iter() {
    let mut data = CategoryChartData::new();
    data.add_category("A");
    data.add_category("B");

    let cats = data.categories_object();
    let level = &cats.levels()[0];
    let labels: Vec<&str> = level.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, vec!["A", "B"]);
}

#[test]
fn test_category_level_into_iter() {
    let mut data = CategoryChartData::new();
    data.add_category("A");

    let cats = data.categories_object();
    let level = &cats.levels()[0];
    let labels: Vec<&str> = level.into_iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, vec!["A"]);
}

#[test]
fn test_category_idx() {
    let mut data = CategoryChartData::new();
    data.add_category("First");
    data.add_category("Second");

    let cats = data.categories_object();
    assert_eq!(cats.levels()[0].categories()[0].idx, 0);
    assert_eq!(cats.levels()[0].categories()[1].idx, 1);
}

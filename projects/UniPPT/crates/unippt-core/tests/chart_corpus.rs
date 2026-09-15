use std::path::PathBuf;

use unippt_core::{export_pptx, import_pptx, ChartData, ChartKind, SceneObject};

fn chart_objects(objects: &[SceneObject]) -> Vec<(&SceneObject, &ChartData)> {
    let mut output = Vec::new();
    for object in objects {
        if let Some(chart) = object.chart.as_ref() {
            output.push((object, chart));
        }
        output.extend(chart_objects(&object.children));
    }
    output
}

#[test]
#[ignore = "requires the local PPT source corpus"]
fn imports_native_charts_and_keeps_untouched_package_byte_exact() {
    let path = std::env::var_os("UNIPPT_CHART_CORPUS")
        .map(PathBuf::from)
        .expect("Set UNIPPT_CHART_CORPUS to the external corpus fixture path");
    let source = std::fs::read(&path).expect("read 14.pptx chart fixture");
    let deck = import_pptx(&source).expect("import 14.pptx");

    for number in [8usize, 24, 25] {
        let slide = &deck.slides[number - 1];
        let charts = chart_objects(&slide.objects);
        eprintln!("slide {number}: {} native charts", charts.len());
        for (object, chart) in &charts {
            eprintln!(
                "  id={:?} frame={:?} kind={:?} title={:?} categories={} series={} part={:?}",
                object.source_shape_id,
                object.frame,
                chart.chart_type,
                chart.title,
                chart.categories.len(),
                chart.series.len(),
                chart.source_part_name
            );
        }
        assert!(
            !charts.is_empty(),
            "expected a native chart on slide {number}"
        );
        assert!(charts
            .iter()
            .all(|(_, chart)| chart.source_part_name.is_some()));
        assert!(charts.iter().all(|(_, chart)| !chart.series.is_empty()));
    }

    let slide_8 = chart_objects(&deck.slides[7].objects);
    assert_eq!(slide_8[0].1.chart_type, ChartKind::Line);
    assert_eq!(slide_8[0].1.categories.len(), 5);
    assert_eq!(slide_8[0].1.series.len(), 3);
    let slide_24 = chart_objects(&deck.slides[23].objects);
    assert_eq!(slide_24[0].1.chart_type, ChartKind::Bar);
    assert_eq!(slide_24[0].1.categories.len(), 4);
    assert_eq!(slide_24[0].1.series.len(), 3);
    let slide_25 = chart_objects(&deck.slides[24].objects);
    assert_eq!(slide_25[0].1.chart_type, ChartKind::Pie);
    assert_eq!(slide_25[0].1.categories.len(), 4);
    assert_eq!(slide_25[0].1.series.len(), 1);

    let output = export_pptx(&deck, Some(&source)).expect("no-op native export");
    assert_eq!(
        output, source,
        "untouched chart package must remain byte exact"
    );
}

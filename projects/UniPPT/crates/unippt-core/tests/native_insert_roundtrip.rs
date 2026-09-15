use unippt_core::{
    export_pptx, import_pptx, ChartData, ChartKind, ChartLegend, ChartSeries, Deck, MediaData,
    MediaKind, ObjectKind, TableCell, TableCellBorder, TableCellBorders, TableData, TableRow,
    TextFrameStyle, TextStyle,
};

fn inserted_object(deck: &Deck, kind: ObjectKind, name: &str) -> unippt_core::SceneObject {
    let mut object = deck.slides[0].objects[0].clone();
    object.id = format!("inserted-{name}");
    object.source_shape_id = None;
    object.kind = kind;
    object.name = name.into();
    object.text.clear();
    object.text_paragraphs.clear();
    object.formula = None;
    object.asset = None;
    object.media = None;
    object.table = None;
    object.chart = None;
    object.children.clear();
    object
}

#[test]
fn newly_inserted_table_chart_video_and_audio_remain_native_after_reimport() {
    let mut deck = Deck::demo();
    let border = TableCellBorder {
        color: "#808080".into(),
        width: 1.0,
        dash: None,
    };
    let cell = |text: &str| TableCell {
        text: text.into(),
        text_paragraphs: Vec::new(),
        text_frame: TextFrameStyle::default(),
        text_style: TextStyle::default(),
        fill: "#FFFFFF".into(),
        borders: TableCellBorders {
            left: Some(border.clone()),
            right: Some(border.clone()),
            top: Some(border.clone()),
            bottom: Some(border.clone()),
        },
        grid_span: 1,
        row_span: 1,
        h_merge: false,
        v_merge: false,
    };
    let mut table = inserted_object(&deck, ObjectKind::Table, "Native Table");
    table.table = Some(TableData {
        columns: vec![200.0, 200.0],
        rows: vec![
            TableRow {
                height: 60.0,
                cells: vec![cell("Name"), cell("Value")],
            },
            TableRow {
                height: 60.0,
                cells: vec![cell("Alpha"), cell("42")],
            },
        ],
        first_row: true,
        first_col: false,
        last_row: false,
        last_col: false,
        band_rows: true,
        band_cols: false,
        style_id: Some("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}".into()),
    });

    let mut chart = inserted_object(&deck, ObjectKind::Chart, "Native Chart");
    chart.chart = Some(ChartData {
        relationship_id: None,
        source_part_name: None,
        chart_type: ChartKind::Bar,
        title: "Sales".into(),
        legend: ChartLegend {
            visible: true,
            position: "right".into(),
            overlay: false,
        },
        categories: vec!["Q1".into(), "Q2".into()],
        series: vec![ChartSeries {
            source_index: None,
            name: "Revenue".into(),
            values: vec![Some(10.0), Some(20.0)],
            color: "#4472C4".into(),
            point_colors: Vec::new(),
        }],
        bar_direction: Some("column".into()),
        grouping: Some("clustered".into()),
        hole_size: None,
    });

    const POSTER: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxMDAiIGhlaWdodD0iMTAwIj48cmVjdCB3aWR0aD0iMTAwIiBoZWlnaHQ9IjEwMCIgZmlsbD0iIzQ0NzJDNCIvPjwvc3ZnPg==";
    let mut video = inserted_object(&deck, ObjectKind::Image, "Native Video");
    video.asset = Some(POSTER.into());
    video.media = Some(MediaData {
        kind: MediaKind::Video,
        asset: Some("data:video/mp4;base64,AAAAHGZ0eXBtcDQy".into()),
        mime_type: Some("video/mp4".into()),
        playback_asset: None,
        playback_mime_type: None,
        source_part_name: None,
        relationship_id: None,
        legacy_relationship_id: None,
        trim_start_ms: None,
        trim_end_ms: None,
        volume: 1.0,
        loop_playback: false,
        play_across_slides: false,
        show_when_stopped: true,
    });
    let mut audio = inserted_object(&deck, ObjectKind::Image, "Native Audio");
    audio.asset = Some(POSTER.into());
    audio.media = Some(MediaData {
        kind: MediaKind::Audio,
        asset: Some("data:audio/mpeg;base64,SUQzBAAAAAA=".into()),
        mime_type: Some("audio/mpeg".into()),
        playback_asset: None,
        playback_mime_type: None,
        source_part_name: None,
        relationship_id: None,
        legacy_relationship_id: None,
        trim_start_ms: None,
        trim_end_ms: None,
        volume: 1.0,
        loop_playback: false,
        play_across_slides: false,
        show_when_stopped: true,
    });

    deck.slides[0].objects.extend([table, chart, video, audio]);
    let pptx = export_pptx(&deck, None).expect("native insert export");
    let reopened = import_pptx(&pptx).expect("reimport native insert export");
    let objects = &reopened.slides[0].objects;

    let table = objects
        .iter()
        .find(|object| object.kind == ObjectKind::Table)
        .unwrap();
    assert_eq!(table.table.as_ref().unwrap().rows[1].cells[0].text, "Alpha");
    let chart = objects
        .iter()
        .find(|object| object.kind == ObjectKind::Chart)
        .unwrap();
    assert_eq!(chart.chart.as_ref().unwrap().categories, ["Q1", "Q2"]);
    assert!(objects.iter().any(|object| {
        object
            .media
            .as_ref()
            .is_some_and(|media| media.kind == MediaKind::Video)
    }));
    assert!(objects.iter().any(|object| {
        object
            .media
            .as_ref()
            .is_some_and(|media| media.kind == MediaKind::Audio)
    }));

    let mut edited = reopened;
    edited.slides[0]
        .objects
        .iter_mut()
        .find(|object| object.kind == ObjectKind::Table)
        .unwrap()
        .table
        .as_mut()
        .unwrap()
        .rows[1]
        .cells[1]
        .text = "84".into();
    edited.slides[0]
        .objects
        .iter_mut()
        .find(|object| object.kind == ObjectKind::Chart)
        .unwrap()
        .chart
        .as_mut()
        .unwrap()
        .series[0]
        .values[1] = Some(99.0);
    let edited_pptx = export_pptx(&edited, Some(&pptx)).expect("native edit export");
    let reopened = import_pptx(&edited_pptx).expect("reimport native edit export");
    let table = reopened.slides[0]
        .objects
        .iter()
        .find(|object| object.kind == ObjectKind::Table)
        .unwrap();
    assert_eq!(table.table.as_ref().unwrap().rows[1].cells[1].text, "84");
    let chart = reopened.slides[0]
        .objects
        .iter()
        .find(|object| object.kind == ObjectKind::Chart)
        .unwrap();
    assert_eq!(
        chart.chart.as_ref().unwrap().series[0].values[1],
        Some(99.0)
    );
}

//! Create a presentation with a clustered column chart.
//!
//! Run with: `cargo run --example add_chart`

use pptx::chart::data::CategoryChartData;
use pptx::enums::chart::XlChartType;
use pptx::units::Inches;
use pptx::Presentation;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prs = Presentation::new()?;
    let layouts = prs.slide_layouts()?;
    let slide = prs.add_slide(&layouts[0])?;

    let mut data = CategoryChartData::new();
    data.add_category("Q1");
    data.add_category("Q2");
    data.add_series("Revenue", &[100.0, 150.0]);

    prs.add_chart_to_slide(
        &slide,
        &data,
        XlChartType::ColumnClustered,
        Inches(1.0).into(),
        Inches(1.5).into(),
        Inches(6.0).into(),
        Inches(4.0).into(),
    )?;

    prs.save("chart_example.pptx")?;
    println!("Created chart_example.pptx with a column chart");
    Ok(())
}

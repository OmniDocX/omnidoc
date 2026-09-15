//! Media operations (images, charts, videos) on a [`Presentation`].

use crate::chart::data::CategoryChartData;
use crate::chart::xmlwriter::ChartXmlWriter;
use crate::enums::chart::XlChartType;
use crate::error::{PartNotFoundExt, PptxResult};
use crate::media::{Audio, Image, Video};
use crate::opc::constants::{content_type as CT, relationship_type as RT};
use crate::opc::part::Part;
use crate::shapes::shapetree::ShapeTree;
use crate::slide::SlideRef;
use crate::units::Emu;

use super::Presentation;

impl Presentation {
    /// Add an image to the package and return its partname.
    ///
    /// Uses SHA1 deduplication: if an image with the same hash already exists,
    /// the existing partname is returned.
    /// # Errors
    ///
    /// Returns an error if the image part cannot be created.
    pub fn add_image(&mut self, image: &Image) -> PptxResult<String> {
        let (partname, _ct) = self.package.or_add_image_part(image)?;
        Ok(partname.into_string())
    }

    /// Add a chart to a slide.
    ///
    /// Generates chart XML from the given `CategoryChartData` and chart type,
    /// creates a chart part in the package, and inserts a `<p:graphicFrame>`
    /// element into the slide referencing the chart.
    ///
    /// # Example
    /// ```no_run
    /// use pptx::presentation::Presentation;
    /// use pptx::chart::data::CategoryChartData;
    /// use pptx::enums::chart::XlChartType;
    /// use pptx::units::{Emu, Inches};
    ///
    /// let mut prs = Presentation::new().unwrap();
    /// let layouts = prs.slide_layouts().unwrap();
    /// let slide_ref = prs.add_slide(&layouts[0]).unwrap();
    ///
    /// let mut chart_data = CategoryChartData::new();
    /// chart_data.add_category("Q1");
    /// chart_data.add_category("Q2");
    /// chart_data.add_series("Sales", &[100.0, 150.0]);
    ///
    /// let left: Emu = Inches(1.0).into();
    /// let top: Emu = Inches(1.0).into();
    /// let width: Emu = Inches(6.0).into();
    /// let height: Emu = Inches(4.0).into();
    ///
    /// prs.add_chart_to_slide(
    ///     &slide_ref,
    ///     &chart_data,
    ///     XlChartType::ColumnClustered,
    ///     left, top, width, height,
    /// ).unwrap();
    /// ```
    /// # Errors
    ///
    /// Returns an error if the chart cannot be created or inserted.
    #[allow(clippy::too_many_arguments)]
    pub fn add_chart_to_slide(
        &mut self,
        slide_ref: &SlideRef,
        chart_data: &CategoryChartData,
        chart_type: XlChartType,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<()> {
        // 1. Generate chart XML
        let chart_xml_str = ChartXmlWriter::write_category(chart_data, chart_type)?;

        // 2. Create the chart part
        let chart_partname = self.package.next_partname("/ppt/charts/chart{}.xml")?;

        // 3. Generate and add the embedded xlsx part
        let xlsx_bytes = crate::chart::xlsx::generate_category_xlsx(chart_data)?;
        let xlsx_partname = self.package.next_partname("/ppt/charts/chart{}.xlsx")?;

        // Pre-compute relative refs before consuming partnames
        let xlsx_target_ref = xlsx_partname.relative_ref(chart_partname.base_uri());
        let chart_target_ref = chart_partname.relative_ref(slide_ref.partname.base_uri());
        let chart_partname_clone = chart_partname.clone();

        let chart_part = Part::new(chart_partname, CT::DML_CHART, chart_xml_str.into_bytes());
        self.package.put_part(chart_part);

        let xlsx_part = Part::new(xlsx_partname, CT::SML_SHEET, xlsx_bytes);
        self.package.put_part(xlsx_part);

        // 4. Add relationship from chart part to xlsx part
        let chart_part = self
            .package
            .part_mut(&chart_partname_clone)
            .or_part_not_found(chart_partname_clone.as_str())?;
        chart_part
            .rels
            .add_relationship(RT::PACKAGE, &xlsx_target_ref, false);
        let slide_part = self
            .package
            .part_mut(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        let r_id = slide_part
            .rels
            .add_relationship(RT::CHART, &chart_target_ref, false);

        // 6. Determine next shape ID from current slide content
        let shape_id = {
            let shapes = crate::shapes::shapetree::ShapeTree::from_slide_xml(&slide_part.blob)?;
            crate::units::ShapeId(shapes.max_shape_id().0 + 1)
        };

        // 7. Generate the graphicFrame XML
        let name = format!("Chart {shape_id}");
        let gf_xml = ShapeTree::new_chart_graphic_frame_xml(
            shape_id, &name, &r_id, left, top, width, height,
        );

        // 8. Insert into the slide's spTree
        let updated_xml = ShapeTree::insert_shape_xml(&slide_part.blob, &gf_xml)?;
        slide_part.blob = updated_xml;

        Ok(())
    }

    /// Add a video to a slide with a poster frame image.
    ///
    /// Adds the video as an external link relationship and the poster image
    /// as an embedded relationship, then inserts a `<p:pic>` element with
    /// `<a:videoFile>` into the slide.
    /// # Errors
    ///
    /// Returns an error if the video cannot be added to the slide.
    #[allow(clippy::too_many_arguments)]
    pub fn add_video_to_slide(
        &mut self,
        slide_ref: &SlideRef,
        video: &Video,
        poster: &Image,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<()> {
        // 1. Add the video media part to the package
        let (video_partname, _video_ct) = self.package.or_add_media_part(video)?;

        // 2. Add the poster image part to the package
        let (poster_partname, _poster_ct) = self.package.or_add_image_part(poster)?;

        // 3. Add relationship from the slide to the video (external link)
        let video_target_ref = video_partname.relative_ref(slide_ref.partname.base_uri());
        let slide_part = self
            .package
            .part_mut(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        let video_r_id = slide_part
            .rels
            .add_relationship(RT::VIDEO, &video_target_ref, false);

        // 4. Add relationship from the slide to the poster image (embedded)
        let poster_target_ref = poster_partname.relative_ref(slide_ref.partname.base_uri());
        let poster_r_id = slide_part
            .rels
            .add_relationship(RT::IMAGE, &poster_target_ref, false);

        // 5. Insert the movie shape XML into the slide's spTree
        let updated_xml = ShapeTree::add_movie(
            &slide_part.blob,
            &video_r_id,
            &poster_r_id,
            left,
            top,
            width,
            height,
        )?;
        slide_part.blob = updated_xml;

        Ok(())
    }

    /// Add an embedded audio file to a slide with a poster image.
    #[allow(clippy::too_many_arguments)]
    pub fn add_audio_to_slide(
        &mut self,
        slide_ref: &SlideRef,
        audio: &Audio,
        poster: &Image,
        left: Emu,
        top: Emu,
        width: Emu,
        height: Emu,
    ) -> PptxResult<()> {
        let (audio_partname, _) = self.package.or_add_audio_part(audio)?;
        let (poster_partname, _) = self.package.or_add_image_part(poster)?;
        let audio_target = audio_partname.relative_ref(slide_ref.partname.base_uri());
        let poster_target = poster_partname.relative_ref(slide_ref.partname.base_uri());
        let slide_part = self
            .package
            .part_mut(&slide_ref.partname)
            .or_part_not_found(slide_ref.partname.as_str())?;
        let audio_r_id = slide_part
            .rels
            .add_relationship(RT::AUDIO, &audio_target, false);
        let media_r_id = slide_part
            .rels
            .add_relationship(RT::MEDIA, &audio_target, false);
        let poster_r_id = slide_part
            .rels
            .add_relationship(RT::IMAGE, &poster_target, false);
        let shape_id = {
            let shapes = ShapeTree::from_slide_xml(&slide_part.blob)?;
            crate::units::ShapeId(shapes.max_shape_id().0 + 1)
        };
        let name = format!("Audio {shape_id}");
        let xml = ShapeTree::new_audio_xml(
            shape_id,
            &name,
            &audio_r_id,
            &media_r_id,
            &poster_r_id,
            left,
            top,
            width,
            height,
        );
        slide_part.blob = ShapeTree::insert_shape_xml(&slide_part.blob, &xml)?;
        Ok(())
    }
}

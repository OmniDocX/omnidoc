//! UniPPT-oriented, loss-aware helpers built on top of the upstream object model.

use std::collections::HashMap;
use std::sync::Arc;

use crate::dml::fill::FillFormat;
use crate::error::{PartNotFoundExt, PptxResult};
use crate::opc::pack_uri::PackURI;
use crate::opc::package::OpcPackage;
use crate::opc::part::Part;
use crate::shapes::{Shape, ShapeTree};
use crate::slide::SlideRef;

use super::Presentation;

/// Shared byte pool used while hydrating PresentationML scene trees.
///
/// A presentation can reference one image part hundreds of times from slides,
/// layouts, masters, groups, and picture fills. Keeping this cache for the
/// duration of an import makes every hydrated shape clone only an `Arc`, while
/// the underlying bytes are copied from the OPC package at most once per unique
/// target part.
#[derive(Debug, Default)]
pub struct SceneAssetCache {
    images: HashMap<PackURI, Arc<[u8]>>,
}

impl SceneAssetCache {
    /// Create an empty scene asset cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of unique OPC image parts held by the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Whether the cache currently holds no image parts.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    fn image_bytes(&mut self, partname: PackURI, bytes: &[u8]) -> Arc<[u8]> {
        self.images
            .entry(partname)
            .or_insert_with(|| Arc::from(bytes))
            .clone()
    }
}

impl Presentation {
    /// Parse any PresentationML scene part (slide, layout, or master) into a
    /// shape tree and resolve its embedded pictures against that part's own
    /// relationships.
    ///
    /// Layout and master furniture is visually part of a slide even though it
    /// lives outside `ppt/slides/slideN.xml`.  Resolving from the owning part
    /// is important because relationship ids are local to each OPC part.
    pub fn part_shape_tree(&self, partname: &PackURI) -> PptxResult<ShapeTree> {
        self.part_shape_tree_with_asset_cache(partname, &mut SceneAssetCache::new())
    }

    /// Parse a scene part while sharing image bytes with other hydrated trees.
    ///
    /// Keep one [`SceneAssetCache`] for the whole presentation import to avoid
    /// allocating the same image bytes for every slide, layout, and master that
    /// references them.
    pub fn part_shape_tree_with_asset_cache(
        &self,
        partname: &PackURI,
        cache: &mut SceneAssetCache,
    ) -> PptxResult<ShapeTree> {
        let part = self
            .package()
            .part(partname)
            .or_part_not_found(partname.as_str())?;
        let mut tree = ShapeTree::from_slide_xml(&part.blob)?;

        for shape in &mut tree.shapes {
            hydrate_picture(shape, part, self.package(), cache);
        }
        Ok(tree)
    }

    /// Parse a slide into a shape tree and resolve every embedded picture.
    ///
    /// Upstream's XML parser intentionally leaves `Picture::image_data` empty.
    /// An editor almost always needs the relationship target and bytes together,
    /// so this fork exposes one safe, recursive operation for that use case.
    pub fn slide_shape_tree(&self, slide_ref: &SlideRef) -> PptxResult<ShapeTree> {
        self.part_shape_tree(&slide_ref.partname)
    }

    /// Parse a slide while sharing hydrated image bytes with other scene trees.
    pub fn slide_shape_tree_with_asset_cache(
        &self,
        slide_ref: &SlideRef,
        cache: &mut SceneAssetCache,
    ) -> PptxResult<ShapeTree> {
        self.part_shape_tree_with_asset_cache(&slide_ref.partname, cache)
    }
}

fn hydrate_picture(
    shape: &mut Shape,
    slide_part: &Part,
    package: &OpcPackage,
    cache: &mut SceneAssetCache,
) {
    match shape {
        Shape::Picture(picture) => {
            let Some(r_id) = picture.image_r_id.as_deref() else {
                return;
            };
            let Ok(partname) = slide_part.related_partname(r_id) else {
                return;
            };
            let Some(image_part) = package.part(&partname) else {
                return;
            };
            picture.image_data = Some(cache.image_bytes(partname, &image_part.blob));
            picture.image_content_type = Some(image_part.content_type.clone());
        }
        Shape::AutoShape(auto_shape) => {
            hydrate_fill_picture(auto_shape.fill.as_mut(), slide_part, package, cache);
        }
        Shape::GroupShape(group) => {
            hydrate_fill_picture(group.fill.as_mut(), slide_part, package, cache);
            for child in &mut group.shapes {
                hydrate_picture(child, slide_part, package, cache);
            }
        }
        _ => {}
    }
}

fn hydrate_fill_picture(
    fill: Option<&mut FillFormat>,
    slide_part: &Part,
    package: &OpcPackage,
    cache: &mut SceneAssetCache,
) {
    let Some(FillFormat::Picture(fill)) = fill else {
        return;
    };
    let Ok(partname) = slide_part.related_partname(fill.image_r_id.as_str()) else {
        return;
    };
    let Some(image_part) = package.part(&partname) else {
        return;
    };
    fill.image_data = Some(cache.image_bytes(partname, &image_part.blob));
    fill.image_content_type = Some(image_part.content_type.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Image;
    use crate::opc::constants::relationship_type as RT;
    use crate::opc::pack_uri::PackURI;
    use crate::units::Emu;

    #[test]
    fn slide_shape_tree_hydrates_embedded_picture_data() {
        let mut presentation = Presentation::new().unwrap();
        let layout = presentation.slide_layouts().unwrap().remove(0);
        let slide_ref = presentation.add_slide(&layout).unwrap();
        let expected = vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4];
        let image_partname = presentation
            .add_image(&Image::from_bytes(expected.clone(), "image/png"))
            .unwrap();
        let image_partname = PackURI::new(image_partname).unwrap();
        let target_ref = image_partname.relative_ref(slide_ref.partname.base_uri());

        let slide_part = presentation
            .package_mut()
            .part_mut(&slide_ref.partname)
            .unwrap();
        let relationship_id = slide_part
            .rels
            .add_relationship(RT::IMAGE, &target_ref, false);
        slide_part.blob = ShapeTree::add_picture(
            &slide_part.blob,
            &relationship_id,
            Emu(0),
            Emu(0),
            Emu(914_400),
            Emu(914_400),
        )
        .unwrap();

        let tree = presentation.slide_shape_tree(&slide_ref).unwrap();
        let picture = tree
            .shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::Picture(picture) => Some(picture),
                _ => None,
            })
            .expect("picture should be parsed");
        assert_eq!(picture.image_data.as_deref(), Some(expected.as_slice()));
        assert_eq!(picture.image_content_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn slide_shape_tree_hydrates_auto_shape_picture_fill() {
        let mut presentation = Presentation::new().unwrap();
        let layout = presentation.slide_layouts().unwrap().remove(0);
        let slide_ref = presentation.add_slide(&layout).unwrap();
        let expected = vec![0x89, b'P', b'N', b'G', 9, 8, 7, 6];
        let image_partname = presentation
            .add_image(&Image::from_bytes(expected.clone(), "image/png"))
            .unwrap();
        let image_partname = PackURI::new(image_partname).unwrap();
        let target_ref = image_partname.relative_ref(slide_ref.partname.base_uri());

        let slide_part = presentation
            .package_mut()
            .part_mut(&slide_ref.partname)
            .unwrap();
        let relationship_id = slide_part
            .rels
            .add_relationship(RT::IMAGE, &target_ref, false);
        let shape_xml = format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="71" name="Picture-filled shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:blipFill><a:blip r:embed="{relationship_id}"/><a:srcRect l="1000" t="2000" r="3000" b="4000"/><a:stretch><a:fillRect/></a:stretch></a:blipFill></p:spPr></p:sp>"#
        );
        slide_part.blob = ShapeTree::insert_shape_xml(&slide_part.blob, &shape_xml).unwrap();

        let tree = presentation.slide_shape_tree(&slide_ref).unwrap();
        let fill = tree
            .shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::AutoShape(shape) => match shape.fill.as_ref() {
                    Some(FillFormat::Picture(fill)) => Some(fill),
                    _ => None,
                },
                _ => None,
            })
            .expect("picture fill should be parsed");
        assert_eq!(fill.image_data.as_deref(), Some(expected.as_slice()));
        assert_eq!(fill.image_content_type.as_deref(), Some("image/png"));
        assert_eq!(fill.source_rect.unwrap().left, 1000);
        assert!(fill.stretch);
        assert!(!fill.tile);
    }

    #[test]
    fn slide_shape_tree_hydrates_group_picture_fill() {
        let mut presentation = Presentation::new().unwrap();
        let layout = presentation.slide_layouts().unwrap().remove(0);
        let slide_ref = presentation.add_slide(&layout).unwrap();
        let expected = vec![0x89, b'P', b'N', b'G', 5, 4, 3, 2];
        let image_partname = presentation
            .add_image(&Image::from_bytes(expected.clone(), "image/png"))
            .unwrap();
        let image_partname = PackURI::new(image_partname).unwrap();
        let target_ref = image_partname.relative_ref(slide_ref.partname.base_uri());

        let slide_part = presentation
            .package_mut()
            .part_mut(&slide_ref.partname)
            .unwrap();
        let relationship_id = slide_part
            .rels
            .add_relationship(RT::IMAGE, &target_ref, false);
        let group_xml = format!(
            r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="81" name="Picture-filled group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/><a:chOff x="0" y="0"/><a:chExt cx="914400" cy="914400"/></a:xfrm><a:blipFill><a:blip r:embed="{relationship_id}"/><a:stretch><a:fillRect/></a:stretch></a:blipFill></p:grpSpPr></p:grpSp>"#
        );
        slide_part.blob = ShapeTree::insert_shape_xml(&slide_part.blob, &group_xml).unwrap();

        let tree = presentation.slide_shape_tree(&slide_ref).unwrap();
        let fill = tree
            .shapes
            .iter()
            .find_map(|shape| match shape {
                Shape::GroupShape(group) => match group.fill.as_ref() {
                    Some(FillFormat::Picture(fill)) => Some(fill),
                    _ => None,
                },
                _ => None,
            })
            .expect("group picture fill should be parsed");
        assert_eq!(fill.image_data.as_deref(), Some(expected.as_slice()));
        assert_eq!(fill.image_content_type.as_deref(), Some("image/png"));
    }

    #[test]
    fn shared_scene_asset_cache_reuses_image_bytes_across_slides() {
        let mut presentation = Presentation::new().unwrap();
        let layout = presentation.slide_layouts().unwrap().remove(0);
        let first_slide = presentation.add_slide(&layout).unwrap();
        let second_slide = presentation.add_slide(&layout).unwrap();
        let expected = vec![0x89, b'P', b'N', b'G', 4, 3, 2, 1];
        let image_partname = presentation
            .add_image(&Image::from_bytes(expected.clone(), "image/png"))
            .unwrap();
        let image_partname = PackURI::new(image_partname).unwrap();

        for slide_ref in [&first_slide, &second_slide] {
            let target_ref = image_partname.relative_ref(slide_ref.partname.base_uri());
            let slide_part = presentation
                .package_mut()
                .part_mut(&slide_ref.partname)
                .unwrap();
            let relationship_id = slide_part
                .rels
                .add_relationship(RT::IMAGE, &target_ref, false);
            slide_part.blob = ShapeTree::add_picture(
                &slide_part.blob,
                &relationship_id,
                Emu(0),
                Emu(0),
                Emu(914_400),
                Emu(914_400),
            )
            .unwrap();
        }

        let mut cache = SceneAssetCache::new();
        assert!(cache.is_empty());
        let first_tree = presentation
            .slide_shape_tree_with_asset_cache(&first_slide, &mut cache)
            .unwrap();
        let second_tree = presentation
            .slide_shape_tree_with_asset_cache(&second_slide, &mut cache)
            .unwrap();

        fn image_data(tree: &ShapeTree) -> &std::sync::Arc<[u8]> {
            tree.shapes
                .iter()
                .find_map(|shape| match shape {
                    Shape::Picture(picture) => picture.image_data.as_ref(),
                    _ => None,
                })
                .expect("picture should be hydrated")
        }
        let first_data = image_data(&first_tree);
        let second_data = image_data(&second_tree);

        assert_eq!(cache.len(), 1);
        assert_eq!(first_data.as_ref(), expected.as_slice());
        assert!(std::sync::Arc::ptr_eq(first_data, second_data));
    }
}

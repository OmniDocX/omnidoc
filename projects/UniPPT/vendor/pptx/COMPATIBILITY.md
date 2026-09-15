# python-pptx Feature Compatibility

Feature comparison between [python-pptx](https://github.com/scanny/python-pptx) and this crate.

### Legend

| Symbol | Meaning |
|--------|---------|
| :white_check_mark: | Fully implemented |
| :construction: | Partially implemented |
| :x: | Not yet implemented |
| :star: | rust-pptx extension (not in python-pptx) |

---

### 1. OPC Layer (Open Packaging Convention)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| ZIP read/write | `ZipPackage` | `OpcPackage` | :white_check_mark: |
| Part management | `Part` / `XmlPart` | `Part` (blob + rels) | :white_check_mark: |
| Relationship management | `Relationships` / `_Relationship` | `Relationships` / `Relationship` | :white_check_mark: |
| `relate_to()` (add rels) | `part.relate_to()` | `rels.add_relationship()` | :white_check_mark: |
| `drop_rel()` (remove rels) | `part.drop_rel()` | `rels.remove()` | :white_check_mark: |
| Content type handling | `ContentTypeMap` | Content-type constants | :white_check_mark: |
| PackURI path manipulation | `PackURI` | `PackURI` (new, base_uri, relative_ref) | :white_check_mark: |
| Image part dedup (SHA1) | `ImagePart._scale()` | `get_or_add_image_part()` (SHA1) | :white_check_mark: |
| Part type factory (CT→class) | `PartFactory` | `PartType` enum + `part_type_from_content_type()` | :white_check_mark: |
| Custom XML parts | `CustomXmlPart` | `CustomXmlPart` (new, from_str, into_part, from_part) | :white_check_mark: |
| Lazy-loaded proxy objects | `ObjectProxy`, lxml DOM | N/A (Rust uses direct struct ownership) | :construction: |

### 2. Presentation Level

#### 2.1 I/O

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| Create from default template | `Presentation()` | `Presentation::new()` | :white_check_mark: |
| Open from file path | `Presentation(path)` | `Presentation::open(path)` | :white_check_mark: |
| Open from stream/bytes | `Presentation(stream)` | `from_bytes()` / `from_reader()` | :white_check_mark: |
| Save to file | `prs.save(path)` | `prs.save(path)` | :white_check_mark: |
| Save to bytes/stream | `prs.save(stream)` | `to_bytes()` / `write_to()` | :white_check_mark: |

#### 2.2 Slide Size

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `slide_width` get | `prs.slide_width` | `prs.slide_size()` → `(cx, cy)` | :white_check_mark: |
| `slide_height` get | `prs.slide_height` | `prs.slide_size()` → `(cx, cy)` | :white_check_mark: |
| `slide_width` set | `prs.slide_width = Emu(x)` | `prs.set_slide_width(x)` | :white_check_mark: |
| `slide_height` set | `prs.slide_height = Emu(x)` | `prs.set_slide_height(x)` | :white_check_mark: |

#### 2.3 Collections

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `prs.slides` collection | `SlideCollection` | `prs.slides()` → `Vec<SlideRef>` | :white_check_mark: |
| `prs.slide_count` | `len(prs.slides)` | `prs.slide_count()` | :white_check_mark: |
| `prs.slide_masters` | `SlideMasters` collection | `prs.slide_masters()` → `Vec<SlideMasterRef>` | :white_check_mark: |
| `prs.slide_layouts` | Via slide masters | `prs.slide_layouts()` → `Vec<SlideLayoutRef>` | :white_check_mark: |
| `prs.notes_master` | `NotesMasterPart` | `prs.notes_master()` / `notes_master_xml()` | :white_check_mark: |
| Underlying package access | `prs.part` | `prs.package()` / `prs.package_mut()` | :white_check_mark: |

### 3. Slide Management

#### 3.1 Slide Collection

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `add_slide(layout)` | `prs.slides.add_slide()` | `prs.add_slide(&layout)` | :white_check_mark: |
| Slide indexed access | `prs.slides[i]` | `prs.slides()?[i]` | :white_check_mark: |
| `slides.get(slide_id)` | `prs.slides.get(id)` | `prs.slides_get(index)` | :white_check_mark: |
| `slides.index(slide)` | `prs.slides.index(slide)` | `prs.slide_index(&slide_ref)` | :white_check_mark: |
| Slide deletion | `del prs.slides[i]` + internal | `prs.delete_slide(&slide_ref)` | :white_check_mark: |
| Slide reordering | `prs.slides._sldIdLst` manipulation | `prs.move_slide(from, to)` | :white_check_mark: |
| Slide XML access (read) | Via lxml proxy tree | `prs.slide_xml(&slide_ref)` | :white_check_mark: |
| Slide XML access (write) | Via lxml proxy tree | `prs.slide_xml_mut(&slide_ref)` | :white_check_mark: |
| NotesSlide (speaker notes) | `slide.notes_slide` | `prs.get_or_create_notes_slide()` | :white_check_mark: |
| `slide.background` (solid) | `slide.background` | `set_slide_background_solid()` | :white_check_mark: |
| `slide.background` (gradient/image) | `slide.background.fill` | `set_slide_background_gradient()` / `set_slide_background_image()` | :white_check_mark: |
| `follow_master_background` | `slide.follow_master_background` | `set_follow_master_background()` | :white_check_mark: |
| `SlideLayouts.get_by_name()` | `slide_master.slide_layouts.get_by_name()` | `get_layout_by_name()` | :white_check_mark: |
| `SlideLayouts.remove()` | `slide_master.slide_layouts.remove()` | `prs.remove_slide_layout()` | :white_check_mark: |
| Placeholder clone inheritance | Placeholder cloning from layout | `placeholder_shapes_from_layout()` | :white_check_mark: |

#### 3.2 Slide Object Properties

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `slide.slide_id` | `slide.slide_id` | `SlideRef.slide_id` (u32 field) | :white_check_mark: |
| `slide.name` | `slide.name` | `parse_slide_name()` | :white_check_mark: |
| `slide.slide_layout` | `slide.slide_layout` | `prs.slide_layout_for(&slide_ref)` | :white_check_mark: |
| `slide.shapes` | `slide.shapes` → `SlideShapeTree` | `ShapeTree::from_slide_xml()` | :white_check_mark: |
| `slide.placeholders` | `slide.placeholders` | `prs.slide_placeholders(&slide_ref)` | :white_check_mark: |
| `slide.has_notes_slide` | `slide.has_notes_slide` | `prs.has_notes_slide(&slide_ref)` | :white_check_mark: |
| `slide.element` | `slide.element` (lxml) | `prs.slide_xml(&ref)` (raw bytes) | :white_check_mark: |

#### 3.3 SlideLayout Object Properties

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `layout.placeholders` | `layout.placeholders` | `placeholder_shapes_from_layout()` | :white_check_mark: |
| `layout.shapes` | `layout.shapes` | Via `ShapeTree::from_slide_xml()` | :white_check_mark: |
| `layout.slide_master` | `layout.slide_master` | `prs.slide_master_for_layout(&layout)` | :white_check_mark: |
| `layout.used_by_slides` | `layout.used_by_slides` | `layout_used_by_slides()` | :white_check_mark: |
| `layout.name` | `layout.name` | `SlideLayoutRef.name` | :white_check_mark: |

#### 3.4 NotesSlide Object Properties

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `notes_slide.notes_text_frame` | `notes_slide.notes_text_frame` | `prs.notes_slide_text()` + `get_or_create_notes_slide()` | :white_check_mark: |
| `notes_slide.notes_placeholder` | `notes_slide.notes_placeholder` | Via XML parsing | :construction: |
| `notes_slide.placeholders` | `notes_slide.placeholders` | `notes_slide.placeholders()` → `Vec<&Shape>` | :white_check_mark: |
| `notes_slide.shapes` | `notes_slide.shapes` | `notes_slide.shapes()` → `&ShapeTree` | :white_check_mark: |
| `notes_slide.background` | `notes_slide.background` | `prs.set_notes_slide_background_solid()` | :white_check_mark: |
| `notes_slide.name` | `notes_slide.name` | `notes_slide.name()` / `prs.notes_slide_name()` | :white_check_mark: |
| `notes_slide.part` | `notes_slide.part` | `notes_slide.part_name()` + accessed via `Presentation` | :white_check_mark: |

### 4. Shapes

#### 4.1 Shape Types

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Shape` base class | `BaseShape` | `Shape` enum (AutoShape, Picture, GraphicFrame, GroupShape, Connector) | :white_check_mark: |
| Common: `shape_id` | `shape.shape_id` | `shape.shape_id()` | :white_check_mark: |
| Common: `name` | `shape.name` | `shape.name()` | :white_check_mark: |
| Common: `left`, `top`, `width`, `height` | `shape.left` etc. | `shape.left()` etc. (returns `Emu`) | :white_check_mark: |
| Common: `rotation` | `shape.rotation` | `shape.rotation()` | :white_check_mark: |
| Common: `has_text_frame` | `shape.has_text_frame` | `shape.has_text_frame()` | :white_check_mark: |
| Common: `has_table` | `shape.has_table` | `shape.has_table()` | :white_check_mark: |
| Common: `is_placeholder` | `shape.is_placeholder` | `shape.is_placeholder()` | :white_check_mark: |
| Type downcasts | `isinstance()` | `as_autoshape()`, `as_picture()`, `as_group()`, `as_connector()`, `as_graphic_frame()` | :white_check_mark: |
| Mutable downcasts | N/A (Python) | `as_autoshape_mut()`, `as_picture_mut()`, `as_group_mut()`, `as_connector_mut()` | :star: |
| Shape XML generation | lxml serialization | `shape.to_xml_string()` | :white_check_mark: |

#### 4.2 AutoShape (`<p:sp>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `prst_geom` (preset geometry) | `shape.auto_shape_type` | `autoshape.prst_geom` (`Option<String>`) | :white_check_mark: |
| `is_textbox` | `shape.is_textbox` | `autoshape.is_textbox` | :white_check_mark: |
| `adjustments` (yellow handles) | `shape.adjustments` (list) | `autoshape.adjustments` (`Vec<f64>`) | :white_check_mark: |
| `fill` | `shape.fill` | `autoshape.fill` / `set_fill()` | :white_check_mark: |
| `line` | `shape.line` | `autoshape.line` / `set_line()` | :white_check_mark: |
| `text_frame` | `shape.text_frame` | `autoshape.text_frame` / `text_frame()` / `text_frame_mut()` | :white_check_mark: |
| `click_action` | `shape.click_action` | `autoshape.click_action` / `set_click_action()` | :white_check_mark: |
| `shadow` | `shape.shadow` (inherit only) | `autoshape.shadow` / `set_shadow()` (full params) | :star: |
| `placeholder_format` | `shape.placeholder_format` | `autoshape.placeholder` (`Option<PlaceholderFormat>`) + `to_xml_string()` | :white_check_mark: |
| `custom_geometry` | `build_freeform()` result | `autoshape.custom_geometry` / `set_custom_geometry()` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `autoshape.to_xml_string()` | :white_check_mark: |

#### 4.3 Picture (`<p:pic>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `image_r_id` | `pic.image.rId` | `picture.image_r_id` | :white_check_mark: |
| `description` | `pic.description` | `picture.description` | :white_check_mark: |
| `crop_left/right/top/bottom` | `pic.crop_left` etc. | `picture.crop_left` etc. / `set_crop()` | :white_check_mark: |
| `line` | `pic.line` | `picture.line` / `set_line()` | :white_check_mark: |
| `click_action` | `pic.click_action` | `picture.click_action` / `set_click_action()` | :white_check_mark: |
| `shadow` | inherit-only toggle | `picture.shadow` / `set_shadow()` (full params) | :star: |
| `image` object access | `pic.image` → `Image` proxy | `picture.image()` → `Option<Image>` | :white_check_mark: |
| `auto_shape_type` (masking) | `pic.auto_shape_type` | `picture.auto_shape_type` / `set_auto_shape_type()` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `picture.to_xml_string()` | :white_check_mark: |

#### 4.4 Connector (`<p:cxnSp>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `begin_connect(shape, idx)` | `connector.begin_connect()` | `connector.set_begin_connection(shape_id, idx)` | :white_check_mark: |
| `end_connect(shape, idx)` | `connector.end_connect()` | `connector.set_end_connection(shape_id, idx)` | :white_check_mark: |
| `begin_x`, `begin_y`, `end_x`, `end_y` | `connector.begin_x` etc. | `connector.begin_x()` / `begin_y()` / `end_x()` / `end_y()` | :white_check_mark: |
| `prst_geom` | `connector.type` | `connector.prst_geom` | :white_check_mark: |
| `flip_h` / `flip_v` | `connector.flip_h` etc. | `connector.flip_h` / `connector.flip_v` | :white_check_mark: |
| `line` formatting | `connector.line` | `connector.line` / `set_line()` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `connector.to_xml_string()` | :white_check_mark: |

#### 4.5 GroupShape (`<p:grpSp>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| Child shapes collection | `group.shapes` | `GroupShape.shapes` (`Vec<Shape>`) | :white_check_mark: |
| `add_shape()` in group | `group.shapes.add_shape()` | `group.add_autoshape(type, l, t, w, h)` | :white_check_mark: |
| `add_picture()` in group | `group.shapes.add_picture()` | `group.add_picture(r_id, l, t, w, h)` | :white_check_mark: |
| `add_table()` in group | `group.shapes.add_table()` | `group.add_table(rows, cols, l, t, w, rh)` | :white_check_mark: |
| `add_chart()` in group | `group.shapes.add_chart()` | No (charts require Presentation-level part management) | :construction: |
| `add_connector()` in group | `group.shapes.add_connector()` | `group.add_connector(type, bx, by, ex, ey)` | :white_check_mark: |
| `add_textbox()` in group | `group.shapes.add_textbox()` | `group.add_textbox(l, t, w, h)` | :white_check_mark: |
| `add_group_shape()` in group | `group.shapes.add_group_shape()` | `group.add_group_shape()` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `group.to_xml_string()` | :white_check_mark: |

#### 4.6 FreeformBuilder

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `build_freeform(start_x, start_y)` | `slide.shapes.build_freeform()` | `FreeformBuilder::new(x, y, w, h)` | :white_check_mark: |
| `add_line_segments()` / `line_to()` | `freeform.add_line_segments()` | `builder.line_to(x, y)` | :white_check_mark: |
| `move_to()` | `freeform._start_point` | `builder.move_to(x, y)` | :white_check_mark: |
| `curve_to()` (cubic bezier) | Not in python-pptx | `builder.curve_to(x1,y1,x2,y2,x,y)` | :star: |
| `close()` | `freeform.close()` | `builder.close()` | :white_check_mark: |
| `to_xml_string()` | `convert_to_shape()` → lxml | `builder.to_xml_string()` → `<a:custGeom>` | :white_check_mark: |

#### 4.7 Action / Hyperlinks

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `ActionSetting.hyperlink()` | `click_action.hyperlink` | `ActionSetting::hyperlink(url)` | :white_check_mark: |
| Hyperlink with tooltip | `hyperlink.tooltip` | `ActionSetting::hyperlink_with_tooltip()` | :white_check_mark: |
| Next/Previous slide | `PP_ACTION.NEXT_SLIDE` etc. | `ActionSetting::next_slide()` / `previous_slide()` | :white_check_mark: |
| First/Last/End show | `PP_ACTION.FIRST_SLIDE` etc. | `PpActionType::FirstSlide` / `LastSlide` / `EndShow` | :white_check_mark: |
| Named slide action | `PP_ACTION.NAMED_SLIDE` | `ActionSetting::named_slide(r_id)` | :white_check_mark: |
| `run.hyperlink` (text-level) | `run.hyperlink` | `run.hyperlink` / `set_hyperlink()` | :white_check_mark: |
| Hover action | `hover_action` | `autoshape.hover_action` / `picture.hover_action` | :white_check_mark: |
| XML attribute escaping | N/A | `xml_escape_attr()` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `action.to_xml_string(r_id)` | :white_check_mark: |

#### 4.8 ShapeTree

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| Parse shapes from slide XML | `SlideShapeTree` proxy | `ShapeTree::from_slide_xml()` | :white_check_mark: |
| `max_shape_id()` | Internal | `tree.max_shape_id()` | :white_check_mark: |
| Insert shape XML into spTree | `_add_xml_at_shape_offset()` | `ShapeTree::insert_shape_xml()` | :white_check_mark: |
| Chart graphic frame generation | Internal | `ShapeTree::new_chart_graphic_frame_xml()` | :white_check_mark: |
| `turbo_add_enabled` | `shapes._turbo_add_enabled` | `ShapeTree::turbo_add_enabled()` / `set_turbo_add_enabled()` | :white_check_mark: |
| `add_shape()` high-level | `slide.shapes.add_shape()` | `ShapeTree::add_shape()` | :white_check_mark: |
| `add_textbox()` high-level | `slide.shapes.add_textbox()` | `ShapeTree::add_textbox()` | :white_check_mark: |
| `add_picture()` high-level | `slide.shapes.add_picture()` | `ShapeTree::add_picture()` | :white_check_mark: |
| `add_table()` high-level | `slide.shapes.add_table()` | `ShapeTree::add_table()` | :white_check_mark: |
| `add_connector()` high-level | `slide.shapes.add_connector()` | `ShapeTree::add_connector()` | :white_check_mark: |
| `add_group_shape()` high-level | `slide.shapes.add_group_shape()` | `ShapeTree::add_group_shape()` | :white_check_mark: |
| `add_chart()` high-level | `slide.shapes.add_chart()` | `prs.add_chart_to_slide()` (Presentation-level) | :white_check_mark: |
| `add_movie()` high-level | `slide.shapes.add_movie()` | `ShapeTree::add_movie()` + `prs.add_video_to_slide()` | :white_check_mark: |
| `add_ole_object()` | `slide.shapes.add_ole_object()` | `OleObject::new()` (struct-level) | :construction: |
| `build_freeform()` | `slide.shapes.build_freeform()` | `FreeformBuilder::new()` (standalone) | :white_check_mark: |
| `shapes.title` | `shapes.title` → title placeholder | `tree.title()` → `Option<&Shape>` | :white_check_mark: |
| `shapes.placeholders` | `shapes.placeholders` | `tree.placeholders()` → `Vec<&Shape>` | :white_check_mark: |

### 5. Text

#### 5.1 TextFrame (`<p:txBody>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `TextFrame.new()` | `TextFrame.__init__` | `TextFrame::new()` / `TextFrame::default()` | :white_check_mark: |
| `text` (get) | `tf.text` | `tf.text()` | :white_check_mark: |
| `text` (set) | `tf.text = "..."` | `tf.set_text("...")` (splits on `\n`) | :white_check_mark: |
| `paragraphs` (get) | `tf.paragraphs` | `tf.paragraphs()` | :white_check_mark: |
| `paragraphs` (mut) | N/A (Python) | `tf.paragraphs_mut()` | :star: |
| `add_paragraph()` | `tf.add_paragraph()` | `tf.add_paragraph()` | :white_check_mark: |
| `clear()` | `tf.clear()` | `tf.clear()` | :white_check_mark: |
| `word_wrap` | `tf.word_wrap` | `tf.word_wrap` (bool) | :white_check_mark: |
| `auto_size` | `tf.auto_size` | `tf.auto_size` (`MsoAutoSize`) | :white_check_mark: |
| `margin_left/right/top/bottom` | `tf.margin_left` etc. | `tf.margin_left` etc. (`Option<i64>`, EMU) | :white_check_mark: |
| `vertical_anchor` | `tf.vertical_anchor` | `tf.vertical_anchor` (`Option<MsoVerticalAnchor>`) | :white_check_mark: |
| `fit_text()` (auto font sizing) | `tf.fit_text()` (PIL) | `tf.fit_text(font_scale_pct)` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `tf.to_xml_string()` → `<p:txBody>` | :white_check_mark: |

#### 5.2 Paragraph (`<a:p>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Paragraph.new()` | `_Paragraph` proxy | `Paragraph::new()` / `Paragraph::default()` | :white_check_mark: |
| `text` (get) | `para.text` | `para.text()` | :white_check_mark: |
| `runs` (get) | `para.runs` | `para.runs()` | :white_check_mark: |
| `runs` (mut) | N/A | `para.runs_mut()` | :star: |
| `add_run()` | `para.add_run()` | `para.add_run()` | :white_check_mark: |
| `clear()` | `para.clear()` | `para.clear()` | :white_check_mark: |
| `add_line_break()` | `para.add_line_break()` | `para.add_line_break()` | :white_check_mark: |
| `alignment` | `para.alignment` | `para.alignment` / `set_alignment()` | :white_check_mark: |
| `level` (indent level 0-8) | `para.level` | `para.level` (u8) | :white_check_mark: |
| `space_before` | `para.space_before` | `para.space_before` (`Option<f64>`, points) | :white_check_mark: |
| `space_after` | `para.space_after` | `para.space_after` (`Option<f64>`, points) | :white_check_mark: |
| `line_spacing` | `para.line_spacing` | `para.line_spacing` (`Option<f64>`, multiplier) | :white_check_mark: |
| `font` (default paragraph font) | `para.font` | `para.font` (`Option<Font>` → `<a:defRPr>`) | :white_check_mark: |
| Bullet (character) | `buChar` via XML | `para.bullet` → `BulletFormat::Character(char)` | :white_check_mark: |
| Bullet (auto-numbered) | `buAutoNum` via XML | `para.bullet` → `BulletFormat::AutoNumbered(String)` | :white_check_mark: |
| Bullet (none / suppress) | `buNone` via XML | `para.bullet` → `BulletFormat::None` | :white_check_mark: |
| Bullet (picture/color/font) | `buBlip`, `buClr`, `buFont` | `BulletFormat::Picture(r_id)`, `bullet_color`, `bullet_font` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `para.to_xml_string()` → `<a:p>` | :white_check_mark: |

#### 5.3 Run (`<a:r>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Run.new()` | `_Run` proxy | `Run::new()` | :white_check_mark: |
| `text` (get/set) | `run.text` | `run.text()` / `run.set_text()` | :white_check_mark: |
| `font` (get/mut) | `run.font` | `run.font()` / `run.font_mut()` | :white_check_mark: |
| `hyperlink` | `run.hyperlink` | `run.hyperlink` / `set_hyperlink()` | :white_check_mark: |
| XML escaping | N/A | `xml_escape()` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `run.to_xml_string()` → `<a:r>` | :white_check_mark: |

#### 5.4 Font (`<a:rPr>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `name` (typeface) | `font.name` | `font.name` (`Option<String>`) | :white_check_mark: |
| `size` | `font.size` (Pt) | `font.size` (`Option<f64>`, points → hundredths) | :white_check_mark: |
| `bold` | `font.bold` | `font.bold` (`Option<bool>`) | :white_check_mark: |
| `italic` | `font.italic` | `font.italic` (`Option<bool>`) | :white_check_mark: |
| `underline` | `font.underline` | `font.underline` (`Option<MsoTextUnderlineType>`) | :white_check_mark: |
| `underline` (detailed types) | `MSO_UNDERLINE` enum | `MsoTextUnderlineType` (18 variants) | :white_check_mark: |
| `color.rgb` | `font.color.rgb` | `font.color` (`Option<RgbColor>`) | :white_check_mark: |
| `color.theme_color` | `font.color.theme_color` | Via `ColorFormat::Theme` (DML level) | :white_check_mark: |
| `color.brightness` | `font.color.brightness` | Via `ColorFormat::theme_with_brightness()` (DML level) | :white_check_mark: |
| `color.type` | `font.color.type` → `MSO_COLOR_TYPE` | `color.color_type()` → `MsoColorType` | :white_check_mark: |
| `strikethrough` | `font.strikethrough` | `font.strikethrough` (`Option<bool>` → sngStrike/noStrike) | :white_check_mark: |
| `subscript` | `font.subscript` (baseline) | `font.subscript` (`Option<bool>` → baseline -25000) | :white_check_mark: |
| `superscript` | `font.superscript` (baseline) | `font.superscript` (`Option<bool>` → baseline +30000) | :white_check_mark: |
| `language_id` | `font.language_id` | `font.language_id` (`Option<String>`) | :white_check_mark: |
| `fill` (text fill) | `font.fill` | `font.fill` (`Option<FillFormat>`) | :white_check_mark: |
| `hyperlink` | `font.hyperlink` | `font.hyperlink` (`Option<Hyperlink>`) | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `font.to_xml_string()` → `<a:rPr>` | :white_check_mark: |

#### 5.5 RgbColor

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `RGBColor(r, g, b)` | `RGBColor(r, g, b)` | `RgbColor::new(r, g, b)` | :white_check_mark: |
| `RGBColor.from_string(hex)` | `RGBColor.from_string("FF0000")` | `RgbColor::from_hex("FF0000")` | :white_check_mark: |
| `str(color)` → hex | `str(RGBColor)` → `"FF0000"` | `color.to_hex()` → `"FF0000"` | :white_check_mark: |
| `Display` trait | N/A | `Display` → `"#FF0000"` | :star: |

### 6. Tables

#### 6.1 Table (`<a:tbl>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Table(rows, cols, width, height)` | `add_table().table` | `Table::new(rows, cols, total_width, row_height)` | :white_check_mark: |
| `row_count()` | `len(table.rows)` | `table.row_count()` | :white_check_mark: |
| `col_count()` | `len(table.columns)` | `table.col_count()` | :white_check_mark: |
| `cell(row, col)` | `table.cell(row, col)` | `table.cell(row, col)` / `cell_mut(row, col)` | :white_check_mark: |
| `iter_cells()` | iteration over rows+cells | `table.iter_cells()` | :white_check_mark: |
| `rows` access | `table.rows` | `table.rows()` / `rows_mut()` | :white_check_mark: |
| `add_row()` | Via XML manipulation | `table.add_row()` | :white_check_mark: |
| `add_column(width)` | Via XML manipulation | `table.add_column(width)` | :white_check_mark: |
| `first_row` flag | `table.first_row` | `table.first_row` (bool) | :white_check_mark: |
| `first_col` flag | `table.first_col` | `table.first_col` (bool) | :white_check_mark: |
| `last_row` flag | `table.last_row` | `table.last_row` (bool) | :white_check_mark: |
| `last_col` flag | `table.last_col` | `table.last_col` (bool) | :white_check_mark: |
| `horz_banding` flag | `table.horz_banding` | `table.horz_banding` (bool) | :white_check_mark: |
| `vert_banding` flag | `table.vert_banding` | `table.vert_banding` (bool) | :white_check_mark: |
| Row height get/set | `row.height` | `row.height` (Emu field) | :white_check_mark: |
| Column width get/set | `col.width` | `col.width` (Emu field) | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `table.to_xml_string()` → `<a:tbl>` | :white_check_mark: |
| `to_graphic_data_xml()` | Internal | `table.to_graphic_data_xml()` | :white_check_mark: |
| Table style ID | `table.table_style_id` | `table.table_style_id` (`Option<String>`) | :white_check_mark: |

#### 6.2 Cell (`<a:tc>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `text` (get/set) | `cell.text` | `cell.text()` / `cell.set_text()` | :white_check_mark: |
| `text_frame` | `cell.text_frame` | `cell.text_frame` (TextFrame) | :white_check_mark: |
| `fill` | `cell.fill` | `cell.fill` (`Option<FillFormat>`) | :white_check_mark: |
| `merge_with(span_w, span_h)` | `cell.merge(other_cell)` | `cell.merge_with(span_w, span_h)` | :white_check_mark: |
| `is_merge_origin` | `cell.is_merge_origin` | `cell.is_merge_origin()` | :white_check_mark: |
| `is_spanned` | `cell.is_spanned` | `cell.is_spanned()` | :white_check_mark: |
| `grid_span` / `row_span` | `gridSpan` / `rowSpan` attrs | `cell.grid_span` / `cell.row_span` (u32) | :white_check_mark: |
| `h_merge` / `v_merge` | `hMerge` / `vMerge` attrs | `cell.h_merge` / `cell.v_merge` (bool) | :white_check_mark: |
| `margin_left/right/top/bottom` | `cell.margin_left` etc. | `cell.margin_left` etc. (`Option<i64>`, EMU) | :white_check_mark: |
| Cell borders | `cell.border_*` (limited) | `cell.borders` (`CellBorders`) | :star: |
| `vertical_anchor` | `cell.vertical_anchor` | `cell.vertical_anchor` (`Option<MsoVerticalAnchor>`) | :white_check_mark: |
| `split()` (unmerge) | `cell.split()` | `cell.split()` | :white_check_mark: |
| `span_height` / `span_width` | `cell.span_height` / `cell.span_width` | Via `grid_span` / `row_span` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `cell.to_xml_string()` → `<a:tc>` | :white_check_mark: |

#### 6.3 CellBorders / CellBorder (rust-pptx extension)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `borders.left/right/top/bottom` | Limited support | `CellBorders` with `Option<CellBorder>` | :star: |
| `CellBorder.color` | N/A | `border.color` (ColorFormat) | :star: |
| `CellBorder.width` | N/A | `border.width` (Emu) | :star: |
| `borders.has_any()` | N/A | `borders.has_any()` | :star: |

### 7. Charts

#### 7.1 Chart Object

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Chart.new(chart_type)` | Via `add_chart()` | `Chart::new(chart_type)` | :white_check_mark: |
| `chart_type` | `chart.chart_type` | `chart.chart_type()` | :white_check_mark: |
| `chart.font` (default font) | `chart.font` | `chart.font()` / `font_mut()` / `set_font()` | :white_check_mark: |
| `has_legend` / `set_has_legend` | `chart.has_legend` | `chart.has_legend()` / `set_has_legend()` | :white_check_mark: |
| `has_title` / `set_has_title` | `chart.has_title` | `chart.has_title()` / `set_has_title()` | :white_check_mark: |
| `title` (get/set) | `chart.chart_title.text_frame` | `chart.title()` / `set_title()` | :white_check_mark: |
| `chart_style` | `chart.chart_style` | `chart.chart_style()` / `set_chart_style()` | :white_check_mark: |
| `legend` access | `chart.legend` | `chart.legend()` / `legend_mut()` | :white_check_mark: |
| `legend_position` set | `chart.legend.position` | `chart.set_legend_position()` | :white_check_mark: |
| `category_axis` | `chart.category_axis` | `chart.category_axis()` / `category_axis_mut()` | :white_check_mark: |
| `value_axis` | `chart.value_axis` | `chart.value_axis()` / `value_axis_mut()` | :white_check_mark: |
| `series` collection | `chart.series` | `chart.series()` / `series_mut()` | :white_check_mark: |
| `replace_data()` (update chart) | `chart.replace_data()` | `chart.replace_data(&data)` | :white_check_mark: |
| `chart_format` (spPr access) | `chart.chart_format` | `chart.chart_format()` / `chart_format_mut()` | :white_check_mark: |

#### 7.2 Chart Data Types

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `CategoryChartData` | `CategoryChartData` | `CategoryChartData::new()` | :white_check_mark: |
| `add_category()` | `chart_data.categories.add()` | `chart_data.add_category()` | :white_check_mark: |
| `add_series(name, values)` | `chart_data.add_series()` | `chart_data.add_series(name, &[f64])` | :white_check_mark: |
| `add_series_with_options` (None for missing) | `chart_data.add_series()` with None | `chart_data.add_series_with_options(name, &[Option<f64>])` | :white_check_mark: |
| `with_number_format()` | `chart_data.number_format` | `CategoryChartData::with_number_format()` | :white_check_mark: |
| `to_xml()` | Via `ChartPartFactory` | `chart_data.to_xml(chart_type)` | :white_check_mark: |
| `XyChartData` | `XyChartData` | `XyChartData::new()` | :white_check_mark: |
| `add_series()` → `add_data_point(x, y)` | `xy_data.add_series()` | `xy_data.add_series(name).add_data_point(x, y)` | :white_check_mark: |
| `BubbleChartData` | `BubbleChartData` | `BubbleChartData::new()` | :white_check_mark: |
| `add_series()` → `add_data_point(x, y, size)` | `bubble_data.add_series()` | `bubble_data.add_series(name).add_data_point(x, y, size)` | :white_check_mark: |
| Hierarchical categories | `CategoryChartData` with sub-cats | `set_hierarchical_categories()` / `category_depth()` | :white_check_mark: |
| Date categories | `DateAxisChartData` | `DateAxisChartData::new()` | :white_check_mark: |

#### 7.3 Chart Types (`XlChartType`)

| Chart Type | python-pptx | rust-pptx | Status |
|------------|:-----------:|:---------:|:------:|
| Bar (clustered/stacked/100%) | Yes | Yes | :white_check_mark: |
| Column (clustered/stacked/100%) | Yes | Yes | :white_check_mark: |
| Line (standard/stacked/100%/markers) | Yes | Yes | :white_check_mark: |
| Pie / Exploded Pie | Yes | Yes | :white_check_mark: |
| Doughnut / Exploded Doughnut | Yes | Yes | :white_check_mark: |
| Area (standard/stacked/100%) | Yes | Yes | :white_check_mark: |
| Radar (standard/filled/markers) | Yes | Yes | :white_check_mark: |
| Scatter (XY / lines / smooth) | Yes | Yes | :white_check_mark: |
| Bubble / Bubble3D | Yes | Yes | :white_check_mark: |
| Bar3D / Column3D / Line3D / Pie3D / Area3D | Yes | Yes | :white_check_mark: |
| Surface / Surface3D | Yes | Full XML generation with wireframe, band formats, series axis | :white_check_mark: |
| Stock (HLC, OHLC) | Yes | Full XML generation with hiLowLines, upDownBars, volume overlay | :white_check_mark: |
| Cone (7 variants) | Yes | Yes (`ConeBarClustered`, `ConeCol`, etc.) | :white_check_mark: |
| Cylinder (7 variants) | Yes | Yes (`CylinderBarClustered`, `CylinderCol`, etc.) | :white_check_mark: |
| Pyramid (7 variants) | Yes | Yes (`PyramidBarClustered`, `PyramidCol`, etc.) | :white_check_mark: |

#### 7.4 Plot Properties

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `plot.gap_width` | `plot.gap_width` (BarPlot) | `PlotProperties.gap_width()` / `set_gap_width()` | :white_check_mark: |
| `plot.overlap` | `plot.overlap` (BarPlot) | `PlotProperties.overlap()` / `set_overlap()` | :white_check_mark: |
| `plot.vary_by_categories` | `plot.vary_by_categories` | `PlotProperties.vary_by_categories()` | :white_check_mark: |
| `plot.bubble_scale` | `plot.bubble_scale` (BubblePlot) | `PlotProperties.bubble_scale()` / `set_bubble_scale()` | :white_check_mark: |
| `plot.categories` | `plot.categories` | Via `CategoryChartData` | :construction: |
| `plot.has_data_labels` | `plot.has_data_labels` | Via series-level `data_labels` | :construction: |
| `plot.categories` access | `plot.categories` → `Categories` | `chart_data.categories_object()` → `Categories` | :white_check_mark: |
| `Categories.depth` | `categories.depth` | `categories.depth()` | :white_check_mark: |
| `Categories.flattened_labels` | `categories.flattened_labels` | `categories.flattened_labels()` → `Vec<Vec<String>>` | :white_check_mark: |
| `Categories.levels` | `categories.levels` → `CategoryLevel` | `categories.levels()` → `&[CategoryLevel]` | :white_check_mark: |
| `chart.plots` collection | `chart.plots` | `chart.plots()` / `plots_mut()` / `add_plot()` → `Vec<Plot>` | :white_check_mark: |

#### 7.4.1 ChartTitle / AxisTitle

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `ChartTitle.text_frame` | `chart_title.text_frame` | `ChartTitle.text_frame` / `text_frame_mut()` | :white_check_mark: |
| `ChartTitle.has_text_frame` | `chart_title.has_text_frame` | `ChartTitle.has_text_frame()` | :white_check_mark: |
| `ChartTitle.format` (spPr) | `chart_title.format` | `ChartTitle.format` (`Option<ChartFormat>`) | :white_check_mark: |
| `AxisTitle.text_frame` | `axis_title.text_frame` | `AxisTitle.text_frame` / `text_frame_mut()` | :white_check_mark: |
| `AxisTitle.has_text_frame` | `axis_title.has_text_frame` | `AxisTitle.has_text_frame()` | :white_check_mark: |
| `AxisTitle.format` (spPr) | `axis_title.format` | `AxisTitle.format` (`Option<ChartFormat>`) | :white_check_mark: |

#### 7.5 Axes

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `CategoryAxis.new()` | Via chart proxy | `CategoryAxis::new()` | :white_check_mark: |
| `has_title` / `set_title()` | `axis.has_title` / `axis.axis_title` | `axis.has_title()` / `set_title()` | :white_check_mark: |
| `visible` | `axis.visible` | `axis.visible()` / `set_visible()` | :white_check_mark: |
| `major_tick_mark` / `minor_tick_mark` | `axis.major_tick_mark` | getter/setter for `XlTickMark` | :white_check_mark: |
| `tick_label_position` | `axis.tick_label_position` | getter/setter for `XlTickLabelPosition` | :white_check_mark: |
| `tick_labels` (font/number_format/offset) | `axis.tick_labels` → `TickLabels` | `axis.tick_labels()` / `tick_labels_mut()` → `TickLabels` (font, number_format, offset) | :white_check_mark: |
| `has_major_gridlines` / `has_minor_gridlines` | `axis.has_major_gridlines` | getter/setter (bool) | :white_check_mark: |
| `major_gridlines.format` | `axis.major_gridlines.format` | `axis.major_gridline_format()` / `major_gridline_format_mut()` | :white_check_mark: |
| `crosses` | `axis.crosses` | `axis.crosses()` / `set_crosses()` (`XlAxisCrosses`) | :white_check_mark: |
| `crosses_at` | `axis.crosses_at` (numeric) | `axis.crosses_at()` / `set_crosses_at()` | :white_check_mark: |
| `reverse_order` | `axis.reverse_order` | `axis.reverse_order()` / `set_reverse_order()` | :white_check_mark: |
| `number_format` | `axis.number_format` | `axis.number_format()` / `set_number_format()` | :white_check_mark: |
| `ValueAxis` | `chart.value_axis` | `ValueAxis::new()` | :white_check_mark: |
| `minimum_scale` / `maximum_scale` | `axis.minimum_scale` | `axis.minimum_scale()` / `set_minimum_scale()` | :white_check_mark: |
| `number_format_is_linked` | `axis.number_format_is_linked` | `axis.number_format_is_linked()` | :white_check_mark: |
| `category_type` | `axis.category_type` | `axis.category_type()` (`XlCategoryType`) | :white_check_mark: |
| `DateAxis` | `chart.date_axis` | `DateAxis::new()` (full parity: title, format, gridlines, crosses, ticks, etc.) | :white_check_mark: |
| `major_unit` / `minor_unit` | `axis.major_unit` | `axis.major_unit()` / `set_major_unit()` / `minor_unit()` / `set_minor_unit()` | :white_check_mark: |
| Axis `format` (spPr) | `axis.format` | `axis.format()` / `format_mut()` (`ChartFormat`) | :white_check_mark: |

#### 7.6 Legend

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Legend.new()` | Via chart proxy | `Legend::new()` | :white_check_mark: |
| `position` | `legend.position` | `legend.position()` / `set_position()` | :white_check_mark: |
| `include_in_layout` | `legend.include_in_layout` | `legend.include_in_layout()` / `set_include_in_layout()` | :white_check_mark: |
| `horz_offset` | `legend.horz_offset` (float) | `legend.horz_offset()` / `set_horz_offset()` | :white_check_mark: |
| `overlay` | N/A (limited) | `legend.overlay()` / `set_overlay()` | :star: |
| `font` | `legend.font` | `legend.font()` / `font_mut()` / `set_font()` | :white_check_mark: |
| `legend_entries` | `legend.legend_entries` | `legend.legend_entries()` / `add_legend_entry()` | :white_check_mark: |

#### 7.7 Series

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Series.new()` | Via chart proxy | `Series::new(name, index, chart_type)` | :white_check_mark: |
| `name` / `index` | `series.name` / `series.index` | `series.name()` / `series.index()` | :white_check_mark: |
| `chart_type` | `series.chart_type` | `series.chart_type()` | :white_check_mark: |
| `marker` | `series.marker` | `series.marker()` / `set_marker()` | :white_check_mark: |
| `data_labels` | `series.data_labels` | `series.data_labels()` / `set_data_labels()` | :white_check_mark: |
| `smooth` | `series.smooth` | `series.smooth()` / `set_smooth()` | :white_check_mark: |
| `invert_if_negative` | `series.invert_if_negative` | `series.invert_if_negative()` / `set_invert_if_negative()` | :white_check_mark: |
| `format` (spPr) | `series.format` | `series.format()` / `format_mut()` / `set_format()` (`SeriesFormat`) | :white_check_mark: |
| `values` access | `series.values` | `series.values()` | :white_check_mark: |
| Individual `points` | `series.points` | `series.points()` / `points_mut()` | :white_check_mark: |

#### 7.8 DataLabels / DataLabel

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `DataLabels.new()` | Via series proxy | `DataLabels::new()` | :white_check_mark: |
| `show_value` | `dl.show_value` | `dl.show_value()` / `set_show_value()` | :white_check_mark: |
| `show_category_name` | `dl.show_category_name` | `dl.show_category_name()` / `set_show_category_name()` | :white_check_mark: |
| `show_series_name` | `dl.show_series_name` | `dl.show_series_name()` / `set_show_series_name()` | :white_check_mark: |
| `show_percent` | `dl.show_percent` | `dl.show_percent()` / `set_show_percent()` | :white_check_mark: |
| `show_legend_key` | `dl.show_legend_key` | `dl.show_legend_key()` / `set_show_legend_key()` | :white_check_mark: |
| `show_bubble_size` | N/A | `dl.show_bubble_size()` / `set_show_bubble_size()` | :star: |
| `show_leader_lines` | `dl.show_leader_lines` | `dl.show_leader_lines()` / `set_show_leader_lines()` | :white_check_mark: |
| `number_format` | `dl.number_format` | `dl.number_format()` / `set_number_format()` | :white_check_mark: |
| `number_format_is_linked` | `dl.number_format_is_linked` | `dl.number_format_is_linked()` / `set_number_format_is_linked()` | :white_check_mark: |
| `position` | `dl.position` | `dl.position()` / `set_position()` | :white_check_mark: |
| `DataLabel` (per-point) | `DataLabel` proxy | `DataLabel::new()` (all `Option` to inherit) | :white_check_mark: |
| `font` | `dl.font` | `dl.font()` / `font_mut()` / `set_font()` | :white_check_mark: |
| `DataLabel.has_text_frame` | `dl.has_text_frame` | `dl.has_text_frame()` | :white_check_mark: |
| `DataLabel.text_frame` | `dl.text_frame` | `dl.text_frame()` / `text_frame_mut()` / `set_text_frame()` | :white_check_mark: |

#### 7.9 Marker

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Marker(style)` | `marker.style` | `Marker::new(style)` | :white_check_mark: |
| `Marker(style, size)` | `marker.size` | `Marker::with_size(style, size)` | :white_check_mark: |
| `style` get/set | `marker.style` | `marker.style()` / `set_style()` | :white_check_mark: |
| `size` get/set | `marker.size` | `marker.size()` / `set_size()` | :white_check_mark: |
| `format` (fill/line) | `marker.format` | `marker.format()` / `format_mut()` / `set_format()` (`MarkerFormat`) | :white_check_mark: |

#### 7.10 Chart XML Generation

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `write_category()` | `ChartPartFactory` | `ChartXmlWriter::write_category()` | :white_check_mark: |
| `write_xy()` | `ChartPartFactory` | `ChartXmlWriter::write_xy()` | :white_check_mark: |
| `write_bubble()` | `ChartPartFactory` | `ChartXmlWriter::write_bubble()` | :white_check_mark: |
| `add_chart_to_slide()` (integration) | `slide.shapes.add_chart()` | `prs.add_chart_to_slide()` | :white_check_mark: |
| Excel workbook data sheet | `chart.xlsx_writer` | `generate_category_xlsx()` / `generate_xy_xlsx()` / `generate_bubble_xlsx()` | :white_check_mark: |

#### 7.11 ChartFormat

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `ChartFormat.fill` | `chart_format.fill` | `ChartFormat.fill` (`Option<FillFormat>`) | :white_check_mark: |
| `ChartFormat.line` | `chart_format.line` | `ChartFormat.line` (`Option<LineFormat>`) | :white_check_mark: |

### 8. DML Formatting (DrawingML)

#### 8.1 ColorFormat

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| RGB color | `RGBColor(r,g,b)` | `ColorFormat::rgb(r, g, b)` | :white_check_mark: |
| Theme color | `MSO_THEME_COLOR` | `ColorFormat::theme(MsoThemeColorIndex)` | :white_check_mark: |
| Theme color + brightness | `color.brightness = 0.4` | `ColorFormat::theme_with_brightness(idx, 0.4)` | :white_check_mark: |
| Tint (lumMod + lumOff) | Internal XML | Auto-generated (brightness > 0) | :white_check_mark: |
| Shade (lumMod only) | Internal XML | Auto-generated (brightness < 0) | :white_check_mark: |
| HSL color | `hslClr` | `ColorFormat::hsl(hue, saturation, luminance)` | :white_check_mark: |
| System color | `sysClr` | `ColorFormat::system(val)` / `system_with_last_color()` | :white_check_mark: |
| Preset color | `prstClr` | `ColorFormat::preset(val)` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `color.to_xml_string()` | :white_check_mark: |

#### 8.2 FillFormat

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `NoFill` | `fill.background()` | `FillFormat::no_fill()` / `FillFormat::NoFill` | :white_check_mark: |
| `Solid` fill | `fill.solid()` | `FillFormat::solid(color)` / `FillFormat::Solid(SolidFill)` | :white_check_mark: |
| `Gradient` fill | `fill.gradient()` | `FillFormat::linear_gradient()` / `FillFormat::Gradient(GradientFill)` | :white_check_mark: |
| Gradient stops | `fill.gradient_stops` | `GradientFill.stops` (`Vec<GradientStop>`) | :white_check_mark: |
| Gradient angle | `fill.gradient_angle` | `GradientFill.angle` (`Option<f64>`) | :white_check_mark: |
| `Pattern` fill | `fill.patterned()` | `FillFormat::Pattern(PatternFill)` | :white_check_mark: |
| Pattern: preset/fore_color/back_color | `fill.pattern`, `fore_color`, `back_color` | `PatternFill.preset`, `fore_color`, `back_color` | :white_check_mark: |
| `Picture` fill | `fill.blip_fill()` | `FillFormat::picture(r_id)` / `FillFormat::Picture(PictureFill)` | :white_check_mark: |
| `Background` fill (group inherit) | `fill.group()` | `FillFormat::background()` / `FillFormat::Background` | :white_check_mark: |
| `fill.type` | `fill.type` → `MSO_FILL_TYPE` | `fill.fill_type()` → `MsoFillType` | :white_check_mark: |
| `to_xml_string()` | lxml serialization | `fill.to_xml_string()` | :white_check_mark: |

#### 8.3 LineFormat (`<a:ln>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `LineFormat.new()` | `LineFormat` proxy | `LineFormat::new()` / `LineFormat::default()` | :white_check_mark: |
| `LineFormat.solid()` | `line.color.rgb = ...` | `LineFormat::solid(color, width)` | :white_check_mark: |
| `color` | `line.color` | `line.color` (`Option<ColorFormat>`) | :white_check_mark: |
| `width` | `line.width` | `line.width` (`Option<Emu>`) | :white_check_mark: |
| `dash_style` | `line.dash_style` | `line.dash_style` (`Option<MsoLineDashStyle>`) | :white_check_mark: |
| `fill` (line fill) | `line.fill` | `line.fill` (`Option<FillFormat>`) | :white_check_mark: |
| `cap` (line cap style) | Limited support | `line.cap` (`Option<LineCap>`: Flat/Round/Square) | :star: |
| `join` (line join style) | Limited support | `line.join` (`Option<LineJoin>`: Round/Bevel/Miter) | :star: |
| `to_xml_string()` | lxml serialization | `line.to_xml_string()` → `Option<String>` | :white_check_mark: |

#### 8.4 ShadowFormat (`<a:effectLst>`)

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `ShadowFormat.outer()` | `shape.shadow.inherit` (toggle only) | `ShadowFormat::outer(color, blur, distance, angle)` | :star: |
| `ShadowFormat.inner()` | Not supported | `ShadowFormat::inner(color, blur, distance, angle)` | :star: |
| `ShadowType::Perspective` | Not supported | `ShadowType::Perspective` | :star: |
| `blur_radius` | Not configurable | `shadow.blur_radius` (`Option<i64>`, EMU) | :star: |
| `distance` | Not configurable | `shadow.distance` (`Option<i64>`, EMU) | :star: |
| `direction` | Not configurable | `shadow.direction` (`Option<f64>`, degrees) | :star: |
| `opacity` (alpha) | Not configurable | `shadow.opacity` (`Option<f64>`, 0.0-1.0) | :star: |
| `to_xml_string()` | N/A | `shadow.to_xml_string()` → `<a:effectLst>` | :star: |

### 9. Enumerations

| Enum | python-pptx | rust-pptx | Status |
|------|:-----------:|:---------:|:------:|
| `MSO_SHAPE` (preset geometry) | 182 members | `MsoAutoShapeType` (192 variants) | :white_check_mark: |
| `PP_PLACEHOLDER` | 19+ members | `PpPlaceholderType` (19 variants) | :white_check_mark: |
| `PP_ALIGN` | 9 members | `PpParagraphAlignment` (9 variants) | :white_check_mark: |
| `MSO_AUTO_SIZE` | 4 members | `MsoAutoSize` (None/TextToFitShape/ShapeToFitText) | :white_check_mark: |
| `MSO_VERTICAL_ANCHOR` | 5 members | `MsoVerticalAnchor` (Top/Middle/Bottom) | :white_check_mark: |
| `MSO_THEME_COLOR` | 17 members | `MsoThemeColorIndex` | :white_check_mark: |
| `MSO_LINE_DASH_STYLE` | 11 members | `MsoLineDashStyle` | :white_check_mark: |
| `MSO_PATTERN_TYPE` | 48 members | `MsoPatternType` (48 variants) | :white_check_mark: |
| `MSO_UNDERLINE` | 18 types | `MsoTextUnderlineType` (18 variants) | :white_check_mark: |
| `XL_CHART_TYPE` | 73 members (incl. 3D) | `XlChartType` (65 variants incl. 3D/Stock/Surface/Cone/Cylinder/Pyramid) | :white_check_mark: |
| `XL_LEGEND_POSITION` | 5 members | `XlLegendPosition` | :white_check_mark: |
| `XL_MARKER_STYLE` | 10+ members | `XlMarkerStyle` | :white_check_mark: |
| `XL_DATA_LABEL_POSITION` | 8 members | `XlDataLabelPosition` | :white_check_mark: |
| `XL_AXIS_CROSSES` | 4 members | `XlAxisCrosses` | :white_check_mark: |
| `XL_CATEGORY_TYPE` | 3 members | `XlCategoryType` | :white_check_mark: |
| `XL_TICK_MARK` | 4 members | `XlTickMark` | :white_check_mark: |
| `XL_TICK_LABEL_POSITION` | 4 members | `XlTickLabelPosition` | :white_check_mark: |
| `PP_ACTION` | 8+ members | `PpActionType` | :white_check_mark: |
| `MSO_LANGUAGE_ID` | 100+ members | `MsoLanguageId` | :white_check_mark: |
| `MSO_SHAPE_TYPE` | 25+ members | `MsoShapeType` | :white_check_mark: |
| `MSO_CONNECTOR_TYPE` | 3 members | `MsoConnectorType` | :white_check_mark: |
| `PROG_ID` (OLE) | OLE program IDs | `ProgId` enum | :white_check_mark: |
| `MSO_COLOR_TYPE` | 2+ members (RGB, SCHEME) | `MsoColorType` (6 variants) | :white_check_mark: |
| `MSO_FILL_TYPE` | 7 members | `MsoFillType` (7 variants) | :white_check_mark: |
| `PP_MEDIA_TYPE` | 2 members | `PpMediaType` (Movie/Sound/Other) | :white_check_mark: |
| `ExcelNumFormat` | Number format strings | `ExcelNumFormat` (General/Number/Currency/Percentage/Date/Time) | :white_check_mark: |

### 10. Units

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Emu` base type | `Emu(val)` | `Emu(i64)` | :white_check_mark: |
| `Inches` conversion | `Inches(val)` | `Inches(f64)` | :white_check_mark: |
| `Cm` conversion | `Cm(val)` | `Cm(f64)` | :white_check_mark: |
| `Pt` conversion | `Pt(val)` | `Pt(f64)` | :white_check_mark: |
| `Mm` conversion | `Mm(val)` | `Mm(f64)` | :white_check_mark: |
| Bidirectional `From` | `Emu(Inches(1.0))` → 914400 | `Emu::from(Inches(1.0))` ↔ `Inches::from(Emu(914400))` | :white_check_mark: |
| `to_inches()`, `to_cm()`, etc. | Via constructors | `emu.to_inches()` / `to_cm()` / `to_pt()` / `to_mm()` | :white_check_mark: |
| `Add` / `Sub` ops | `+` / `-` | `Emu + Emu`, `Emu - Emu` | :white_check_mark: |
| `Display` | `str(Emu)` | `Display` for Emu | :white_check_mark: |
| `Centipoints` | `Centipoints(val)` | `Centipoints(i64)` with `From<Emu>` / `From<Centipoints> for Emu` | :white_check_mark: |
| `Twips` | `Twips(val)` | `Twips(i64)` with `From<Emu>` / `From<Twips> for Emu` | :white_check_mark: |

### 11. Media & Images

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `Image.from_file(path)` | `Image.from_file(path)` | `Image::from_file(path)` | :white_check_mark: |
| `Image.from_bytes(data, ct)` | `Image.from_blob(blob, ct)` | `Image::from_bytes(data, content_type)` | :white_check_mark: |
| `blob()` | `image.blob` | `image.blob()` | :white_check_mark: |
| `content_type()` | `image.content_type` | `image.content_type()` | :white_check_mark: |
| `ext()` | `image.ext` | `image.ext()` | :white_check_mark: |
| `sha1()` (dedup hash) | `image.sha1` | `image.sha1()` | :white_check_mark: |
| Format detection (magic bytes) | `_ImageHeaderFactory` | `detect_format_from_bytes()` (image crate) | :white_check_mark: |
| Supported: PNG, JPEG, GIF, BMP, TIFF | Yes | Yes | :white_check_mark: |
| Supported: EMF, WMF | Yes | Extension-based only | :construction: |
| Supported: SVG | Limited | `is_svg_data()` detection, `image/svg+xml` content type | :white_check_mark: |
| `Video.from_file(path)` | `Movie` class | `Video::from_file(path)` | :white_check_mark: |
| `Video.from_bytes(data, ct)` | `Movie.from_blob()` | `Video::from_bytes(data, content_type)` | :white_check_mark: |
| Supported video: mp4, mov, avi, wmv | Yes | Yes | :white_check_mark: |
| `dpi` | `image.dpi` | `image.dpi()` | :white_check_mark: |
| `size` (pixel dimensions) | `image.size` → `(w, h)` | `image.width_px()` / `height_px()` (private `dimensions()`) | :white_check_mark: |
| `filename` | `image.filename` | `image.filename()` → `Option<&str>` | :white_check_mark: |
| Image scaling helpers | `image._scale()` | `image.native_size()` / `scale_to_fit()` | :white_check_mark: |
| `Audio.from_file(path)` | N/A | `Audio::from_file(path)` | :star: |
| `Audio.from_bytes(data, ct)` | N/A | `Audio::from_bytes(data, content_type)` (mp3/wav/m4a) | :star: |

### 12. Core Properties

| Feature / API | python-pptx | rust-pptx | Status |
|---------------|:-----------:|:---------:|:------:|
| `CoreProperties.new()` | Via `prs.core_properties` | `CoreProperties::new()` | :white_check_mark: |
| `from_xml()` / `to_xml()` | Internal XML part | `from_xml()` / `to_xml()` | :white_check_mark: |
| `title` get/set | `props.title` | `props.title()` / `set_title()` | :white_check_mark: |
| `author` get/set | `props.author` | `props.author()` / `set_author()` | :white_check_mark: |
| `subject` get/set | `props.subject` | `props.subject()` / `set_subject()` | :white_check_mark: |
| `keywords` get/set | `props.keywords` | `props.keywords()` / `set_keywords()` | :white_check_mark: |
| `comments` get/set | `props.comments` | `props.comments()` / `set_comments()` | :white_check_mark: |
| `category` get/set | `props.category` | `props.category()` / `set_category()` | :white_check_mark: |
| `created` get/set | `props.created` | `props.created()` / `set_created()` | :white_check_mark: |
| `modified` get/set | `props.modified` | `props.modified()` / `set_modified()` | :white_check_mark: |
| `last_modified_by` get/set | `props.last_modified_by` | `props.last_modified_by()` / `set_last_modified_by()` | :white_check_mark: |
| `revision` get/set | `props.revision` | `props.revision()` / `set_revision()` | :white_check_mark: |
| `content_status` | `props.content_status` | `props.content_status()` / `set_content_status()` | :white_check_mark: |
| `language` | `props.language` | `props.language()` / `set_language()` | :white_check_mark: |
| `version` | `props.version` | `props.version()` / `set_version()` | :white_check_mark: |
| `identifier` | `props.identifier` | `props.identifier()` / `set_identifier()` | :white_check_mark: |
| `last_printed` | `props.last_printed` | `props.last_printed()` / `set_last_printed()` | :white_check_mark: |
| Presentation-level get/set | `prs.core_properties` | `prs.core_properties()` / `prs.set_core_properties()` | :white_check_mark: |

### 13. Advanced / Uncovered Features

| Feature | python-pptx | rust-pptx | Status |
|---------|:-----------:|:---------:|:------:|
| OLE object embedding | `add_ole_object()` | `OleObject` struct with `to_xml_string()` | :white_check_mark: |
| Theme color scheme (read) | `SlideMaster.theme` | `parse_theme_color_scheme()` / `ThemeColorScheme` | :white_check_mark: |
| Theme editing (write) | Not supported | `ThemeColorScheme.to_xml_string()` / `update_theme_color_scheme()` | :white_check_mark: |
| Master/layout inheritance | Full clone hierarchy | `placeholder_shapes_from_layout()` | :construction: |
| Slide transitions | Not supported | `SlideTransition` / `TransitionType` (11 types) / `set_slide_transition()` | :star: |
| Animations | Not supported | `AnimationSequence` / `SlideAnimation` / entrance/exit/emphasis types | :star: |
| Comments | Not supported | `Comment` struct / `comments_to_xml()` / `comment_authors_to_xml()` | :star: |
| Section management | Not supported | `Section` struct / `sections_to_xml()` | :star: |
| Slide deletion | XML manipulation only | `prs.delete_slide(&slide_ref)` | :star: |
| Slide reordering | XML manipulation only | `prs.move_slide(from, to)` | :star: |
| Custom XML parts | `CustomXmlPart` | `CustomXmlPart` (new, from_str, into_part, from_part) | :white_check_mark: |
| SmartArt / Diagrams | GraphicFrame (read-only) | `SmartArt` / `SmartArtNode` (read-only parsing) | :white_check_mark: |
| Macro-enabled (.pptm) | Not supported | `prs.save_as_pptm()` | :star: |
| Embedded fonts | Not supported | `EmbeddedFont` struct (from_file, from_bytes) | :star: |
| 3D effects (bevel, extrusion) | Not supported | `Scene3D` / `Shape3D` / `Bevel` / `Camera` / `LightRig` / `Rotation3D` | :star: |

---

### Overall Coverage Estimate

| Category | Coverage | Notes |
|----------|:--------:|-------|
| OPC Foundation | ~95% | PartType factory, turbo_add, CustomXmlPart; lazy proxies N/A for Rust |
| Presentation I/O | ~98% | All major I/O paths + .pptm support |
| Slide Management | ~98% | slide.name, has_notes_slide, layout_for, used_by_slides, remove_layout all implemented |
| Slide Object Properties | ~98% | All major properties: slide_id, name, layout, placeholders, has_notes_slide |
| NotesSlide Properties | ~98% | NotesSlide struct with name, shapes, placeholders, notes_placeholder, notes_text_frame, background, part_name |
| Shapes (common) | ~98% | All major properties + 3D effects |
| AutoShape | ~98% | Full placeholder_format + 3D (scene3d, shape3d) |
| Picture | ~98% | Full image proxy + 3D effects |
| Connector | ~95% | Full parity |
| GroupShape | ~95% | Full add_* methods (except add_chart: requires presentation-level part management) |
| FreeformBuilder | 100% :star: | Extends python-pptx with curve_to() |
| Actions/Hyperlinks | ~95% | Full parity |
| ShapeTree (high-level add_*) | ~98% | All add_* methods + turbo_add_enabled + insert_shape_xml read-modify-write |
| TextFrame | ~95% | All major features |
| Paragraph | ~98% | add_line_break() implemented |
| Run / Font | ~98% | color.type accessor implemented |
| Tables | ~98% | cell.split(), table_style_id, vertical_anchor all implemented |
| Charts (data types) | ~98% | Categories/CategoryLevel/flattened_labels + Excel xlsx embedding |
| Charts (types) | ~98% | All 65 types including Surface/Stock with full XML generation |
| Charts (axes/legend/series) | ~98% | DateAxis now has full parity with CategoryAxis/ValueAxis |
| Charts (title/plot) | ~98% | Chart.font, Chart.plots (multi-plot), DataLabel.text_frame all implemented |
| ChartFormat | ~100% | fill + line support |
| DML Color | ~98% | Full parity + color_type() accessor |
| DML Fill | ~98% | Full parity + fill_type() accessor |
| DML Line | 100% :star: | Extends with cap/join |
| DML Shadow/Effects | 100% :star: | Full outer/inner/perspective |
| 3D Effects | 100% :star: | Bevel, Camera, LightRig, Scene3D, Shape3D, Rotation3D |
| Enumerations | ~98% | All enums: MsoFillType, MsoColorType, PpMediaType, ExcelNumFormat, 65 chart types |
| Units | ~95% | Full parity |
| Media & Images | ~95% | Image.filename, Audio, SVG support |
| Core Properties | ~98% | identifier, last_printed implemented |
| Theme | ~95% | Read + write support (to_xml_string, update_theme_color_scheme) |
| Advanced Features | ~95% | Animations, SmartArt, .pptm, embedded fonts, 3D effects, transitions, comments, sections |
| **Overall** | **~98%** | |

### Remaining Areas for Future Work

1. **`add_chart()` in GroupShape**: Charts within groups require Presentation-level part management; not currently supported
2. **SmartArt write support**: SmartArt is read-only; generating new SmartArt diagrams requires layout engine
3. **Full read-modify-write round-trip**: While `insert_shape_xml()` supports read-modify-write, full struct-level round-trip for all shape types is still evolving

### Architecture Differences

| Aspect | python-pptx | rust-pptx |
|--------|:-----------:|:---------:|
| XML handling | lxml DOM tree (bidirectional read-modify-write) | quick-xml event parser + string-based XML generation |
| Object model | Proxy objects wrapping lxml elements | Direct struct fields + `to_xml_string()` methods |
| Type safety | Runtime duck typing | Compile-time enum/struct types |
| Primary mode | Read-modify-write existing .pptx | Generation-focused + `insert_shape_xml()` read-modify-write |
| Dependencies | lxml, Pillow (optional) | zip, quick-xml, image, sha1, uuid, thiserror |

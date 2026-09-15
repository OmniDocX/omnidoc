//! PowerPoint-aware font resolution for browser rendering and native editing.
//!
//! DrawingML keeps separate Latin, East Asian, complex-script, and symbol
//! typefaces.  It also permits theme placeholders such as `+mj-lt` and
//! `+mn-ea`.  Collapsing those values into one CSS family loses information
//! and commonly makes CJK text disappear.  This module resolves the slots and
//! projects them as an ordered CSS fallback stack without mutating the source
//! package.

use std::collections::{HashMap, HashSet};

use pptx::text::Font;
use pptx::Presentation;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::model::EmbeddedFont;

const RT_THEME: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FontSlot {
    Latin,
    EastAsia,
    ComplexScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeFamily {
    Major,
    Minor,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ThemeFontCollection {
    latin: Option<String>,
    east_asia: Option<String>,
    complex_script: Option<String>,
    supplemental: HashMap<String, String>,
}

impl ThemeFontCollection {
    fn resolve(&self, slot: FontSlot, language: Option<&str>, text: &str) -> Option<&str> {
        match slot {
            FontSlot::Latin => non_empty(self.latin.as_deref()),
            FontSlot::EastAsia => non_empty(self.east_asia.as_deref()).or_else(|| {
                theme_script_for_language(language)
                    .filter(|script| is_east_asian_script(script))
                    .or_else(|| infer_east_asian_script(text))
                    .and_then(|script| self.supplemental.get(script))
                    .and_then(|face| non_empty(Some(face.as_str())))
            }),
            FontSlot::ComplexScript => non_empty(self.complex_script.as_deref()).or_else(|| {
                theme_script_for_language(language)
                    .filter(|script| is_complex_script(script))
                    .or_else(|| infer_complex_script(text))
                    .and_then(|script| self.supplemental.get(script))
                    .and_then(|face| non_empty(Some(face.as_str())))
            }),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ThemeFontScheme {
    major: ThemeFontCollection,
    minor: ThemeFontCollection,
}

impl ThemeFontScheme {
    fn from_presentation(presentation: &Presentation) -> Self {
        let Some(master) = presentation
            .slide_masters()
            .ok()
            .and_then(|masters| masters.into_iter().next())
        else {
            return Self::default();
        };
        let Some(master_part) = presentation.package().part(&master.partname) else {
            return Self::default();
        };
        let Some(theme_relationship) = master_part.rels.all_by_reltype(RT_THEME).first().copied()
        else {
            return Self::default();
        };
        let Ok(theme_part_name) =
            theme_relationship.target_partname(master_part.partname.base_uri())
        else {
            return Self::default();
        };
        presentation
            .package()
            .part(&theme_part_name)
            .map_or_else(Self::default, |part| parse_theme_font_scheme(&part.blob))
    }

    fn resolve_token<'a>(
        &'a self,
        token: &'a str,
        slot_hint: FontSlot,
        language: Option<&str>,
        text: &str,
    ) -> Option<&'a str> {
        let normalized = token.trim().to_ascii_lowercase();
        let (family, slot) = match normalized.as_str() {
            "+mj-lt" => (ThemeFamily::Major, FontSlot::Latin),
            "+mj-ea" => (ThemeFamily::Major, FontSlot::EastAsia),
            "+mj-cs" => (ThemeFamily::Major, FontSlot::ComplexScript),
            "+mn-lt" => (ThemeFamily::Minor, FontSlot::Latin),
            "+mn-ea" => (ThemeFamily::Minor, FontSlot::EastAsia),
            "+mn-cs" => (ThemeFamily::Minor, FontSlot::ComplexScript),
            _ if normalized.starts_with('+') => return None,
            _ => return non_empty(Some(token)),
        };
        let collection = match family {
            ThemeFamily::Major => &self.major,
            ThemeFamily::Minor => &self.minor,
        };
        // The token suffix is authoritative. `slot_hint` is retained for the
        // call-site contract and guards future theme aliases without a suffix.
        let _ = slot_hint;
        collection.resolve(slot, language, text)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FontPolicy {
    theme: ThemeFontScheme,
    embedded_families: HashMap<String, String>,
}

impl FontPolicy {
    pub(crate) fn from_presentation(
        presentation: &Presentation,
        embedded_fonts: &[EmbeddedFont],
    ) -> Self {
        Self {
            theme: ThemeFontScheme::from_presentation(presentation),
            embedded_families: embedded_family_map(embedded_fonts),
        }
    }

    /// Build a policy for one concrete theme part.
    ///
    /// Presentations may contain multiple slide masters and each master can
    /// point at a different theme. Importers should use this constructor after
    /// following the current slide's layout -> master -> theme relationship
    /// chain instead of resolving every slide through the first master.
    pub(crate) fn from_theme_xml(theme_xml: &[u8], embedded_fonts: &[EmbeddedFont]) -> Self {
        Self {
            theme: parse_theme_font_scheme(theme_xml),
            embedded_families: embedded_family_map(embedded_fonts),
        }
    }

    #[cfg(test)]
    fn with_theme_and_embedded(theme: ThemeFontScheme, embedded: &[&str]) -> Self {
        Self {
            theme,
            embedded_families: embedded
                .iter()
                .filter_map(|family| {
                    let family = clean_family(family)?;
                    Some((family.to_lowercase(), family))
                })
                .collect(),
        }
    }

    /// Resolve independently inherited DrawingML font slots into one CSS
    /// family stack. For mixed-script runs both Latin and CJK/complex faces
    /// are retained so the browser can select per Unicode codepoint.
    pub(crate) fn css_stack(
        &self,
        run: Option<&Font>,
        paragraph: Option<&Font>,
        text: &str,
        fallback: &str,
    ) -> String {
        let language = run
            .and_then(|font| non_empty(font.language_id.as_deref()))
            .or_else(|| paragraph.and_then(|font| non_empty(font.language_id.as_deref())));
        let latin = self.resolve_slot(
            font_slot_value(run, paragraph, FontSlot::Latin),
            FontSlot::Latin,
            language,
            text,
        );
        let east_asia = self.resolve_slot(
            font_slot_value(run, paragraph, FontSlot::EastAsia),
            FontSlot::EastAsia,
            language,
            text,
        );
        let complex = self.resolve_slot(
            font_slot_value(run, paragraph, FontSlot::ComplexScript),
            FontSlot::ComplexScript,
            language,
            text,
        );
        let symbol = run
            .and_then(|font| non_empty(font.symbol_name.as_deref()))
            .or_else(|| paragraph.and_then(|font| non_empty(font.symbol_name.as_deref())))
            .and_then(clean_family);

        let scripts = TextScripts::classify(text);
        let mut ordered = Vec::new();
        if scripts.east_asian && !scripts.latin && !scripts.complex {
            ordered.extend(east_asia);
            ordered.extend(latin);
            ordered.extend(complex);
        } else if scripts.complex && !scripts.latin && !scripts.east_asian {
            ordered.extend(complex);
            ordered.extend(latin);
            ordered.extend(east_asia);
        } else {
            ordered.extend(latin);
            ordered.extend(east_asia);
            ordered.extend(complex);
        }
        ordered.extend(symbol);
        ordered.extend(parse_font_stack(fallback));

        let mut families = Vec::new();
        let mut seen = HashSet::new();
        for family in ordered {
            self.push_aliases(&family, &mut families, &mut seen);
        }

        let east_script = theme_script_for_language(language)
            .filter(|script| is_east_asian_script(script))
            .or_else(|| infer_east_asian_script(text));
        if scripts.east_asian {
            for fallback in east_asian_fallbacks(east_script, families.first().map(String::as_str))
            {
                if !is_generic_family(fallback) {
                    push_unique(&mut families, &mut seen, fallback.to_string());
                }
            }
        }
        if scripts.complex {
            push_unique(&mut families, &mut seen, "Arial".into());
        }
        if families.is_empty() {
            push_unique(&mut families, &mut seen, "Aptos".into());
        }
        let generic = infer_generic_family(&families, scripts);
        push_unique(&mut families, &mut seen, generic.into());

        families
            .iter()
            .map(|family| css_family(family))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn resolve_slot(
        &self,
        raw: Option<&str>,
        slot: FontSlot,
        language: Option<&str>,
        text: &str,
    ) -> Option<String> {
        if let Some(raw) = non_empty(raw) {
            if let Some(resolved) = self.theme.resolve_token(raw, slot, language, text) {
                return clean_family(resolved);
            }
        }
        // DrawingML text without an explicit slot uses the minor theme
        // collection. This is especially common in table cells and body
        // placeholders whose `a:lstStyle` is otherwise empty.
        self.theme
            .minor
            .resolve(slot, language, text)
            .and_then(clean_family)
    }

    fn push_aliases(&self, family: &str, output: &mut Vec<String>, seen: &mut HashSet<String>) {
        let Some(clean) = clean_family(family) else {
            return;
        };
        if is_generic_family(&clean) {
            return;
        }
        if let Some(embedded) = self.embedded_families.get(&clean.to_lowercase()) {
            push_unique(output, seen, embedded.clone());
        }
        let aliases = aliases_for(&clean);
        if aliases.is_empty() {
            push_unique(output, seen, clean);
        } else {
            for alias in aliases {
                push_unique(output, seen, alias.to_string());
            }
        }
    }
}

fn embedded_family_map(embedded_fonts: &[EmbeddedFont]) -> HashMap<String, String> {
    embedded_fonts
        .iter()
        .filter_map(|font| {
            let family = clean_family(&font.family)?;
            Some((family.to_lowercase(), family))
        })
        .collect()
}

fn font_slot_value<'a>(
    run: Option<&'a Font>,
    paragraph: Option<&'a Font>,
    slot: FontSlot,
) -> Option<&'a str> {
    let from = |font: &'a Font| match slot {
        FontSlot::Latin => font.name.as_deref(),
        FontSlot::EastAsia => font.east_asia_name.as_deref(),
        FontSlot::ComplexScript => font.complex_script_name.as_deref(),
    };
    run.and_then(from)
        .and_then(|value| non_empty(Some(value)))
        .or_else(|| {
            paragraph
                .and_then(from)
                .and_then(|value| non_empty(Some(value)))
        })
}

fn parse_theme_font_scheme(xml: &[u8]) -> ThemeFontScheme {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut scheme = ThemeFontScheme::default();
    let mut active = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => match local_name(element.name().as_ref()) {
                b"majorFont" => active = Some(ThemeFamily::Major),
                b"minorFont" => active = Some(ThemeFamily::Minor),
                _ => apply_theme_font_element(&element, active, &mut scheme),
            },
            Ok(Event::Empty(element)) => {
                apply_theme_font_element(&element, active, &mut scheme);
            }
            Ok(Event::End(element))
                if matches!(
                    local_name(element.name().as_ref()),
                    b"majorFont" | b"minorFont"
                ) =>
            {
                active = None;
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    scheme
}

fn apply_theme_font_element(
    element: &BytesStart<'_>,
    active: Option<ThemeFamily>,
    scheme: &mut ThemeFontScheme,
) {
    let Some(active) = active else {
        return;
    };
    let collection = match active {
        ThemeFamily::Major => &mut scheme.major,
        ThemeFamily::Minor => &mut scheme.minor,
    };
    match local_name(element.name().as_ref()) {
        b"latin" => collection.latin = xml_attr(element, b"typeface"),
        b"ea" => collection.east_asia = xml_attr(element, b"typeface"),
        b"cs" => collection.complex_script = xml_attr(element, b"typeface"),
        b"font" => {
            if let (Some(script), Some(typeface)) =
                (xml_attr(element, b"script"), xml_attr(element, b"typeface"))
            {
                if !script.trim().is_empty() && !typeface.trim().is_empty() {
                    collection.supplemental.insert(script, typeface);
                }
            }
        }
        _ => {}
    }
}

fn xml_attr(element: &BytesStart<'_>, wanted: &[u8]) -> Option<String> {
    element
        .attributes()
        .with_checks(false)
        .filter_map(Result::ok)
        .find(|attribute| local_name(attribute.key.as_ref()) == wanted)
        .and_then(|attribute| attribute.unescape_value().ok())
        .map(|value| value.into_owned())
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn clean_family(value: &str) -> Option<String> {
    let value = value
        .trim()
        .trim_matches(['\'', '"'])
        .trim_start_matches('@')
        .trim();
    (!value.is_empty() && !value.starts_with('+')).then(|| value.to_string())
}

fn parse_font_stack(stack: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for character in stack.chars() {
        match (quote, character) {
            (Some(active), value) if value == active => quote = None,
            (None, '\'' | '"') => quote = Some(character),
            (None, ',') => {
                if let Some(value) = clean_family(&current) {
                    values.push(value);
                }
                current.clear();
            }
            _ => current.push(character),
        }
    }
    if let Some(value) = clean_family(&current) {
        values.push(value);
    }
    values
}

/// Return the first physical family from a CSS stack for DrawingML writeback.
pub(crate) fn primary_family(stack: &str) -> String {
    parse_font_stack(stack)
        .into_iter()
        .find(|family| !is_generic_family(family))
        .unwrap_or_else(|| "Aptos".into())
}

fn push_unique(output: &mut Vec<String>, seen: &mut HashSet<String>, family: String) {
    let key = family.to_lowercase();
    if !family.trim().is_empty() && seen.insert(key) {
        output.push(family);
    }
}

fn aliases_for(family: &str) -> &'static [&'static str] {
    match family.trim().to_lowercase().as_str() {
        "simsun" | "宋体" => &["SimSun", "宋体"],
        "nsimsun" | "新宋体" => &["NSimSun", "新宋体", "SimSun", "宋体"],
        "simhei" | "黑体" => &["SimHei", "黑体", "Microsoft YaHei", "微软雅黑"],
        "microsoft yahei" | "微软雅黑" => &["Microsoft YaHei", "微软雅黑", "DengXian", "等线"],
        "dengxian" | "等线" => &["DengXian", "等线", "Microsoft YaHei", "微软雅黑"],
        "kaiti" | "楷体" => &["KaiTi", "楷体", "STKaiti", "华文楷体"],
        "fangsong" | "仿宋" => &["FangSong", "仿宋", "STFangsong", "华文仿宋"],
        "pmingliu" | "new mingliu" | "新細明體" | "新细明体" => {
            &["PMingLiU", "新細明體", "MingLiU", "細明體"]
        }
        "mingliu" | "細明體" | "细明体" => &["MingLiU", "細明體", "PMingLiU", "新細明體"],
        "microsoft jhenghei" | "微軟正黑體" | "微软正黑体" => {
            &["Microsoft JhengHei", "微軟正黑體", "PMingLiU", "新細明體"]
        }
        "dfkai-sb" | "標楷體" | "标楷体" => &["DFKai-SB", "標楷體", "KaiTi", "楷体"],
        "yu gothic" | "游ゴシック" => &["Yu Gothic", "游ゴシック", "Meiryo", "メイリオ"],
        "meiryo" | "メイリオ" => &["Meiryo", "メイリオ", "Yu Gothic", "游ゴシック"],
        "ms pgothic" | "ｍｓ ｐゴシック" | "ms pゴシック" => {
            &["MS PGothic", "ＭＳ Ｐゴシック", "Meiryo", "メイリオ"]
        }
        "malgun gothic" | "맑은 고딕" => &["Malgun Gothic", "맑은 고딕"],
        _ => &[],
    }
}

fn theme_script_for_language(language: Option<&str>) -> Option<&'static str> {
    let normalized = language?.trim().replace('_', "-").to_ascii_lowercase();
    let primary = normalized.split('-').next().unwrap_or_default();
    match primary {
        "zh" if normalized.contains("hant")
            || normalized.contains("-tw")
            || normalized.contains("-hk")
            || normalized.contains("-mo") =>
        {
            Some("Hant")
        }
        "zh" => Some("Hans"),
        "ja" => Some("Jpan"),
        "ko" => Some("Hang"),
        "ar" | "fa" | "ur" | "ps" => Some("Arab"),
        "he" | "yi" => Some("Hebr"),
        "hi" | "mr" | "ne" => Some("Deva"),
        "th" => Some("Thai"),
        "bn" => Some("Beng"),
        "gu" => Some("Gujr"),
        "pa" => Some("Guru"),
        "ta" => Some("Taml"),
        "te" => Some("Telu"),
        "kn" => Some("Knda"),
        "ml" => Some("Mlym"),
        _ => None,
    }
}

fn is_east_asian_script(script: &str) -> bool {
    matches!(script, "Hans" | "Hant" | "Jpan" | "Hang")
}

fn is_complex_script(script: &str) -> bool {
    matches!(
        script,
        "Arab"
            | "Hebr"
            | "Deva"
            | "Thai"
            | "Beng"
            | "Gujr"
            | "Guru"
            | "Taml"
            | "Telu"
            | "Knda"
            | "Mlym"
    )
}

fn infer_east_asian_script(text: &str) -> Option<&'static str> {
    if text.chars().any(is_japanese) {
        Some("Jpan")
    } else if text.chars().any(is_hangul) {
        Some("Hang")
    } else if text.chars().any(is_han) {
        Some("Hans")
    } else {
        None
    }
}

fn infer_complex_script(text: &str) -> Option<&'static str> {
    if text.chars().any(is_arabic) {
        Some("Arab")
    } else if text.chars().any(is_hebrew) {
        Some("Hebr")
    } else if text.chars().any(is_devanagari) {
        Some("Deva")
    } else if text.chars().any(is_thai) {
        Some("Thai")
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TextScripts {
    latin: bool,
    east_asian: bool,
    complex: bool,
}

impl TextScripts {
    fn classify(text: &str) -> Self {
        let mut scripts = Self::default();
        for character in text.chars().filter(|character| !character.is_whitespace()) {
            if is_han(character) || is_japanese(character) || is_hangul(character) {
                scripts.east_asian = true;
            } else if is_arabic(character)
                || is_hebrew(character)
                || is_devanagari(character)
                || is_thai(character)
            {
                scripts.complex = true;
            } else if character.is_alphabetic() || character.is_ascii_digit() {
                scripts.latin = true;
            }
        }
        scripts
    }
}

fn is_han(character: char) -> bool {
    matches!(character as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x2FA1F)
}

fn is_japanese(character: char) -> bool {
    matches!(character as u32, 0x3040..=0x30FF | 0x31F0..=0x31FF)
}

fn is_hangul(character: char) -> bool {
    matches!(character as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF)
}

fn is_arabic(character: char) -> bool {
    matches!(character as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

fn is_hebrew(character: char) -> bool {
    matches!(character as u32, 0x0590..=0x05FF | 0xFB1D..=0xFB4F)
}

fn is_devanagari(character: char) -> bool {
    matches!(character as u32, 0x0900..=0x097F | 0xA8E0..=0xA8FF)
}

fn is_thai(character: char) -> bool {
    matches!(character as u32, 0x0E00..=0x0E7F)
}

fn east_asian_fallbacks(
    script: Option<&str>,
    first_family: Option<&str>,
) -> &'static [&'static str] {
    match script {
        Some("Jpan") => &["Yu Gothic", "Meiryo", "MS PGothic", "sans-serif"],
        Some("Hang") => &["Malgun Gothic", "sans-serif"],
        Some("Hant") => &["Microsoft JhengHei", "PMingLiU", "MingLiU", "sans-serif"],
        _ if first_family.is_some_and(is_cjk_serif) => {
            &["SimSun", "宋体", "Microsoft YaHei", "微软雅黑", "serif"]
        }
        _ => &[
            "Microsoft YaHei",
            "微软雅黑",
            "DengXian",
            "等线",
            "SimSun",
            "宋体",
            "sans-serif",
        ],
    }
}

fn is_cjk_serif(family: &str) -> bool {
    matches!(
        family.trim().to_lowercase().as_str(),
        "simsun"
            | "宋体"
            | "nsimsun"
            | "新宋体"
            | "pmingliu"
            | "mingliu"
            | "新細明體"
            | "細明體"
            | "kaiti"
            | "楷体"
            | "fangsong"
            | "仿宋"
    )
}

fn infer_generic_family(families: &[String], _scripts: TextScripts) -> &'static str {
    if families
        .iter()
        .any(|family| family.eq_ignore_ascii_case("monospace"))
    {
        "monospace"
    } else if families
        .iter()
        .any(|family| family.eq_ignore_ascii_case("serif") || is_cjk_serif(family))
    {
        "serif"
    } else {
        // Latin/East-Asian/complex scripts and unrecognized scripts alike fall
        // back to the same generic family.
        "sans-serif"
    }
}

fn is_generic_family(family: &str) -> bool {
    matches!(
        family.trim().to_ascii_lowercase().as_str(),
        "serif"
            | "sans-serif"
            | "monospace"
            | "cursive"
            | "fantasy"
            | "system-ui"
            | "-apple-system"
            | "blinkmacsystemfont"
            | "ui-serif"
            | "ui-sans-serif"
            | "ui-monospace"
            | "math"
            | "emoji"
    )
}

fn css_family(family: &str) -> String {
    if is_generic_family(family) {
        return family.to_ascii_lowercase();
    }
    if family
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        family.to_string()
    } else {
        format!("'{}'", family.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEME: &str = r#"
      <a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
        <a:themeElements><a:fontScheme name="Office">
          <a:majorFont>
            <a:latin typeface="Calibri Light"/><a:ea typeface=""/><a:cs typeface=""/>
            <a:font script="Hans" typeface="宋体"/><a:font script="Hant" typeface="新細明體"/>
            <a:font script="Jpan" typeface="MS PGothic"/><a:font script="Hang" typeface="Malgun Gothic"/>
            <a:font script="Arab" typeface="Times New Roman"/><a:font script="Hebr" typeface="Times New Roman"/>
          </a:majorFont>
          <a:minorFont>
            <a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/>
            <a:font script="Hans" typeface="等线"/><a:font script="Jpan" typeface="Yu Gothic"/>
            <a:font script="Arab" typeface="Arial"/>
          </a:minorFont>
        </a:fontScheme></a:themeElements>
      </a:theme>"#;

    #[test]
    fn parses_theme_slots_and_resolves_language_sensitive_tokens() {
        let theme = parse_theme_font_scheme(THEME.as_bytes());
        assert_eq!(theme.major.latin.as_deref(), Some("Calibri Light"));
        assert_eq!(
            theme.resolve_token("+mj-ea", FontSlot::EastAsia, Some("zh-TW"), "中文"),
            Some("新細明體")
        );
        assert_eq!(
            theme.resolve_token("+mj-ea", FontSlot::EastAsia, Some("ja-JP"), "日本語"),
            Some("MS PGothic")
        );
        assert_eq!(
            theme.resolve_token("+mj-ea", FontSlot::EastAsia, Some("ko-KR"), "한글"),
            Some("Malgun Gothic")
        );
        assert_eq!(
            theme.resolve_token("+mn-cs", FontSlot::ComplexScript, Some("ar-SA"), "العربية"),
            Some("Arial")
        );
        assert_eq!(
            theme.resolve_token("+mj-cs", FontSlot::ComplexScript, Some("he-IL"), "עברית"),
            Some("Times New Roman")
        );
    }

    #[test]
    fn keeps_latin_and_east_asian_faces_for_mixed_text() {
        let policy =
            FontPolicy::with_theme_and_embedded(parse_theme_font_scheme(THEME.as_bytes()), &[]);
        let mut font = Font::new();
        font.name = Some("Calibri".into());
        font.east_asia_name = Some("+mj-ea".into());
        font.language_id = Some("zh-CN".into());
        let stack = policy.css_stack(Some(&font), None, "Hello 中文", "Aptos, sans-serif");
        assert!(stack.starts_with("Calibri, SimSun, '宋体'"), "{stack}");
        assert!(stack.contains("Aptos"), "{stack}");
        assert!(!stack.contains("+mj"), "{stack}");
    }

    #[test]
    fn puts_east_asian_face_first_for_cjk_only_text() {
        let policy =
            FontPolicy::with_theme_and_embedded(parse_theme_font_scheme(THEME.as_bytes()), &[]);
        let mut font = Font::new();
        font.name = Some("Calibri".into());
        font.east_asia_name = Some("Microsoft YaHei".into());
        let stack = policy.css_stack(Some(&font), None, "中文", "Aptos, sans-serif");
        assert!(
            stack.starts_with("'Microsoft YaHei', '微软雅黑'"),
            "{stack}"
        );
        assert!(stack.contains("Calibri"), "{stack}");
    }

    #[test]
    fn embedded_custom_family_remains_exact_and_first() {
        let policy =
            FontPolicy::with_theme_and_embedded(ThemeFontScheme::default(), &["汉仪南宫体简"]);
        let mut font = Font::new();
        font.east_asia_name = Some("汉仪南宫体简".into());
        let stack = policy.css_stack(Some(&font), None, "中文", "Aptos, sans-serif");
        assert!(stack.starts_with("'汉仪南宫体简'"), "{stack}");
    }

    #[test]
    fn aliases_are_normalized_and_deduplicated() {
        let policy = FontPolicy::default();
        let mut font = Font::new();
        font.east_asia_name = Some("宋体".into());
        let stack = policy.css_stack(Some(&font), None, "中文", "SimSun, '宋体', serif");
        assert_eq!(stack.matches("SimSun").count(), 1, "{stack}");
        assert_eq!(stack.matches("'宋体'").count(), 1, "{stack}");
    }

    #[test]
    fn extracts_primary_physical_family_from_css_stack() {
        assert_eq!(
            primary_family("'Microsoft YaHei', SimSun, sans-serif"),
            "Microsoft YaHei"
        );
        assert_eq!(primary_family("sans-serif"), "Aptos");
        assert_eq!(
            primary_family("-apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif"),
            "Segoe UI"
        );
    }
}

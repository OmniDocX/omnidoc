//! Language identifier enumerations for text runs.

/// Top language identifiers used in OOXML `lang` attributes.
///
/// Alias: `MsoLanguage`
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsoLanguageId {
    EnUs,
    EnGb,
    EnAu,
    EnCa,
    JaJp,
    ZhCn,
    ZhTw,
    KoKr,
    DeDe,
    FrFr,
    FrCa,
    EsEs,
    EsMx,
    PtBr,
    PtPt,
    ItIt,
    RuRu,
    NlNl,
    PlPl,
    ArSa,
}

/// Alias matching the python-pptx `MSO_LANGUAGE_ID` name.
pub type MsoLanguage = MsoLanguageId;

impl MsoLanguageId {
    /// Return the BCP-47 / OOXML language tag for this language.
    #[must_use]
    pub const fn to_xml_str(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::EnGb => "en-GB",
            Self::EnAu => "en-AU",
            Self::EnCa => "en-CA",
            Self::JaJp => "ja-JP",
            Self::ZhCn => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::KoKr => "ko-KR",
            Self::DeDe => "de-DE",
            Self::FrFr => "fr-FR",
            Self::FrCa => "fr-CA",
            Self::EsEs => "es-ES",
            Self::EsMx => "es-MX",
            Self::PtBr => "pt-BR",
            Self::PtPt => "pt-PT",
            Self::ItIt => "it-IT",
            Self::RuRu => "ru-RU",
            Self::NlNl => "nl-NL",
            Self::PlPl => "pl-PL",
            Self::ArSa => "ar-SA",
        }
    }

    /// Parse an OOXML language tag string.
    #[must_use]
    pub fn from_xml_str(s: &str) -> Option<Self> {
        match s {
            "en-US" => Some(Self::EnUs),
            "en-GB" => Some(Self::EnGb),
            "en-AU" => Some(Self::EnAu),
            "en-CA" => Some(Self::EnCa),
            "ja-JP" => Some(Self::JaJp),
            "zh-CN" => Some(Self::ZhCn),
            "zh-TW" => Some(Self::ZhTw),
            "ko-KR" => Some(Self::KoKr),
            "de-DE" => Some(Self::DeDe),
            "fr-FR" => Some(Self::FrFr),
            "fr-CA" => Some(Self::FrCa),
            "es-ES" => Some(Self::EsEs),
            "es-MX" => Some(Self::EsMx),
            "pt-BR" => Some(Self::PtBr),
            "pt-PT" => Some(Self::PtPt),
            "it-IT" => Some(Self::ItIt),
            "ru-RU" => Some(Self::RuRu),
            "nl-NL" => Some(Self::NlNl),
            "pl-PL" => Some(Self::PlPl),
            "ar-SA" => Some(Self::ArSa),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_roundtrip() {
        let langs = [
            MsoLanguageId::EnUs,
            MsoLanguageId::JaJp,
            MsoLanguageId::ZhCn,
            MsoLanguageId::KoKr,
            MsoLanguageId::DeDe,
            MsoLanguageId::FrFr,
            MsoLanguageId::EsEs,
            MsoLanguageId::PtBr,
            MsoLanguageId::ItIt,
            MsoLanguageId::RuRu,
            MsoLanguageId::ArSa,
        ];
        for lang in langs {
            let xml = lang.to_xml_str();
            assert_eq!(MsoLanguageId::from_xml_str(xml), Some(lang));
        }
    }

    #[test]
    fn test_unknown_language() {
        assert_eq!(MsoLanguageId::from_xml_str("xx-YY"), None);
    }

    #[test]
    fn test_en_us() {
        assert_eq!(MsoLanguageId::EnUs.to_xml_str(), "en-US");
    }

    #[test]
    fn test_ja_jp() {
        assert_eq!(MsoLanguageId::JaJp.to_xml_str(), "ja-JP");
    }
}

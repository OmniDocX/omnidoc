#![cfg(windows)]

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use unippt_core::import_pptx;

#[test]
#[ignore = "requires the local 1.pptx embedded-font corpus"]
fn extracts_embedded_powerpoint_fonts_as_browser_sfnt() {
    let path = std::env::var_os("UNIPPT_FONT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/zindex-audit/1.pptx")
        });
    let bytes = std::fs::read(path).expect("read 1.pptx embedded-font fixture");
    let deck = import_pptx(&bytes).expect("import 1.pptx");

    assert_eq!(
        deck.fonts.len(),
        17,
        "all embedded faces should be retained"
    );
    assert!(deck.fonts.iter().any(|font| font.family == "汉仪南宫体简"));
    assert!(deck.fonts.iter().any(|font| font.family == "个性印章"));
    assert!(deck
        .fonts
        .iter()
        .any(|font| { font.family == "Calibri" && font.weight == 700 && font.style == "italic" }));

    let mut non_sfnt = Vec::new();
    for font in &deck.fonts {
        let encoded = font
            .data_uri
            .split_once(",")
            .map(|(_, data)| data)
            .expect("font data URI payload");
        let decoded = STANDARD.decode(encoded).expect("valid font base64");
        if !matches!(
            decoded.get(..4),
            Some([0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"true" | b"ttcf")
        ) {
            non_sfnt.push(format!("{} {}", font.family, font.style));
        }
    }
    assert!(
        non_sfnt.is_empty(),
        "faces not converted to SFNT: {}",
        non_sfnt.join(", ")
    );
}

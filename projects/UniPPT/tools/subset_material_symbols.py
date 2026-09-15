"""Build the tiny Material Symbols font used by the UniPPT editor shell.

The upstream font contains thousands of symbols and costs almost 900 KiB on
the first visit. UniPPT uses a small, explicit ligature set. This script keeps
those glyphs and rebuilds a compact ``liga`` table so existing HTML can retain
readable icon names such as ``save`` and ``picture_as_pdf``.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from fontTools import subset
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.ttLib import TTFont


ICONS = (
    "save",
    "undo",
    "redo",
    "html",
    "picture_as_pdf",
    "photo_library",
    "speed",
    "movie",
    "screen_record",
    "add",
    "content_copy",
    "arrow_upward",
    "arrow_downward",
    "transition_slide",
    "delete",
    "dashboard",
    "widgets",
    "perm_media",
    "drive_folder_upload",
    "convert_to_text",
    "cloud_off",
    "rotate_right",
)


def build(source: Path, output: Path) -> None:
    font = TTFont(source)
    options = subset.Options()
    options.flavor = "woff2"
    options.layout_features = ["liga"]
    options.layout_closure = False
    options.glyph_names = True
    options.symbol_cmap = True
    options.legacy_cmap = True
    options.recommended_glyphs = True

    sub = subset.Subsetter(options=options)
    sub.populate(text=" ".join(ICONS), glyphs=ICONS)
    sub.subset(font)

    cmap = {
        codepoint: glyph
        for table in font["cmap"].tables
        for codepoint, glyph in table.cmap.items()
    }
    rules = ["feature liga {"]
    for icon in ICONS:
        characters = " ".join(cmap[ord(character)] for character in icon)
        rules.append(f"  sub {characters} by {icon};")
    rules.append("} liga;")
    addOpenTypeFeaturesFromString(font, "\n".join(rules))

    output.parent.mkdir(parents=True, exist_ok=True)
    # Keep content hashing stable across machines and repeated builds. FontTools
    # otherwise rewrites head.modified on every save.
    font.recalcTimestamp = False
    font.save(output)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    build(args.source, args.output)
    print(f"Material Symbols subset: {args.source.stat().st_size} -> {args.output.stat().st_size} bytes")


if __name__ == "__main__":
    main()

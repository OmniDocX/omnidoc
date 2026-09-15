#!/usr/bin/env python
"""Batch LaTeX <-> OMML conversion used by the UniPPT Rust service.

Input JSON:
  {"direction":"latex-to-omml","items":[{"key":"1","value":"E=mc^2"}]}

Output JSON:
  {"values":{"1":"<m:oMathPara>...</m:oMathPara>"},"errors":{}}
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from lxml import etree

HERE = Path(__file__).resolve().parent
MATH_NS = "http://schemas.openxmlformats.org/officeDocument/2006/math"

_mml_to_omml = None
_omml_to_mml = None
_mml_to_tex = None


def _load_latex_to_omml():
    global _mml_to_omml
    if _mml_to_omml is None:
        _mml_to_omml = etree.XSLT(etree.parse(str(HERE / "MML2OMML.XSL")))
    return _mml_to_omml


def _load_omml_to_latex():
    global _omml_to_mml, _mml_to_tex
    if _omml_to_mml is None:
        _omml_to_mml = etree.XSLT(etree.parse(str(HERE / "omml2mml.xsl")))
        _mml_to_tex = etree.XSLT(etree.parse(str(HERE / "mmltex" / "mmltex.xsl")))
    return _omml_to_mml, _mml_to_tex


def latex_to_omml(latex: str) -> str:
    import latex2mathml.converter

    source = latex.strip()
    source = re.sub(r"^\$\$?|\$\$?$", "", source).strip()
    if not source:
        raise ValueError("empty LaTeX")
    mathml = latex2mathml.converter.convert(source)
    transformed = _load_latex_to_omml()(etree.fromstring(mathml.encode("utf-8")))
    root = transformed.getroot()
    if etree.QName(root.tag).localname == "oMath":
        para = etree.Element(f"{{{MATH_NS}}}oMathPara", nsmap={"m": MATH_NS})
        para.append(root)
        root = para
    etree.cleanup_namespaces(root)
    return etree.tostring(root, encoding="unicode")


def omml_to_latex(omml: str) -> str:
    source = omml.strip()
    if not source:
        raise ValueError("empty OMML")
    root = etree.fromstring(source.encode("utf-8"))
    to_mml, to_tex = _load_omml_to_latex()
    mathml = to_mml(root)
    latex = str(to_tex(mathml)).strip()
    return re.sub(r"^\$|\$$", "", latex).strip()


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: convert.py <input.json> <output.json>", file=sys.stderr)
        return 2
    input_path, output_path = map(Path, sys.argv[1:])
    payload = json.loads(input_path.read_text(encoding="utf-8-sig"))
    direction = payload.get("direction")
    converter = {
        "latex-to-omml": latex_to_omml,
        "omml-to-latex": omml_to_latex,
    }.get(direction)
    if converter is None:
        raise ValueError(f"unsupported direction: {direction}")

    values = {}
    errors = {}
    for item in payload.get("items", []):
        key = str(item.get("key", ""))
        try:
            values[key] = converter(str(item.get("value", "")))
        except Exception as exc:  # Batch conversion must degrade per formula.
            errors[key] = str(exc)
    output_path.write_text(
        json.dumps({"values": values, "errors": errors}, ensure_ascii=False),
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

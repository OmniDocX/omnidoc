# Formula conversion assets

This directory adapts the formula round-trip used by the parent UniDoc and
UniCell projects:

- LaTeX → `latex2mathml` → MathML → `MML2OMML.XSL` → OMML
- OMML → `omml2mml.xsl` → MathML → `mmltex` → LaTeX

`convert.py` provides a batch JSON protocol for the Rust server. Conversion is
kept outside the browser so the exact OMML source can be stored in `.uppt` and
later inserted into PowerPoint OOXML without scraping KaTeX's display markup.

The XSL files are shared with the UniDoc formula conversion tools. `mmltex/README` contains the upstream permissive license notice.

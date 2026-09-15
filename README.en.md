# OmniDoc · Project Source Collection

[OmniDoc main site](https://omnidoc.top) · [中文说明](README.md)

This repository contains actual reviewed source snapshots of OmniDoc's independently developed Chinese application projects:

- [UniPPT](projects/UniPPT): local presentation editor and PPTX conversion.
- [UniCell](projects/unicell): local spreadsheet editor, formulas and format conversion.
- [vecmeta](projects/vecmeta): vector conversion components.

PolyglotPDF is intentionally excluded. Each project retains its own README, build instructions, licenses and third-party notices. These are ordinary copied files, not submodules or automatic mirrors. Open the project directory and follow its README to run it.

[sources.lock.json](sources.lock.json) pins each public upstream commit and every file's SHA-256. Run `python tools/verify_copies.py` from this root to verify snapshots. Snapshot date: 2026-09-16.

The application projects are developed in China; independently authored dependencies retain their own origins and terms. First-party collection documentation uses the [OmniDoc Non-Commercial Source License 1.0](LICENSE); project and third-party licenses remain unchanged. Qualifying non-commercial use is free. Commercial use requires written permission. Source-available is not the same as OSI-approved open source.

Contact: cc@omnidoc.top; WeChat 13184071590. Normally replies within 48 hours.

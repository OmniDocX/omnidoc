# Upstream provenance

- Project: `rust-pptx`
- Repository: <https://github.com/hidemi-ito/rust-pptx>
- Upstream commit: `0e19bcf4d60617bc20f26400ad9c432892660e22`
- License: MIT (see `LICENSE`)
- Vendored: 2026-08-05

UniPPT changes the package name to `unippt-pptx`, removes an unused ZIP time
feature that broke the stated MSRV, and adds `Presentation::slide_shape_tree()`
to recursively resolve embedded picture relationships into the parsed scene.
The fork also allows `clippy::large_enum_variant` on `Shape` so the workspace
clippy gate passes on newer toolchains without boxing upstream variants.

Keep application-specific document semantics in `unippt-core`; this directory
should remain a reviewable fork of the generic OOXML library.

# Source-image precision reconstruction

The web image-to-editable-slide entry uses `image_precision_runtime.js`. It accepts only the uploaded raster, fresh OCR anchors and the active model configuration. It has no reviewed-blueprint input or fixture coordinates.

## Native renderer setup

Configure absolute `UNIPPT_NODE_PATH` and `UNIPPT_PRESENTATION_NODE_MODULES` paths, or put a local `.unippt-quality-runtime.json` in the application root:

```json
{"nodeExecutable":"/absolute/path/to/node","nodeModules":"/absolute/path/to/node_modules"}
```

The modules directory must contain the installed `@oai/artifact-tool` package. This host uses the provided document runtime, not a desktop LibreOffice installation. The local configuration file is ignored by Git. The web app checks `/api/ai/reconstruction/status` before paid recognition.

## Workflow and timing

Original and 90-degree OCR run concurrently. The model transcribes source text while up to four automatically detected bounded modules are independently grouped. Fan layers and alpha are recovered from source color islands. Remaining geometry uses explicit native source contours, never a whole-page image background. A native font atlas measures typography, a native source-contour baseline checks model geometry, and four actual PPTX renders fit text and verify the result. Geometry that increases measured source error falls back to the source contours. The native package is read, never re-saved by the rendering library. XML rotation and subscript properties are independently counted.

The UI reports time from starting recognition to the prepared preview, including OCR, excluding the user's approval wait. Reports also separate model calls and native rendering stages. After approval, the single reconstructed slide can be downloaded without the rest of the open document, and its export duration is shown separately. This is a measured-quality candidate, not a claim of PowerPoint application acceptance. Remaining text errors and local fallbacks are shown before approval, which still owns the sole document mutation. Cancellation stops browser/model requests; a server render already in progress is bounded by its timeout and concurrency guard.

## Boundaries

Symmetric fans and flat-color modules have explicit applicability tests. Failed local grouping preserves source contours and reports the fallback. OCR omissions, split math and geometry ambiguities still require visual verification; the source-only route must be benchmarked against a reviewed reference without reading that reference during generation. A good renderer score alone does not establish semantic accuracy or native PowerPoint parity.

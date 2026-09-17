# From documents to applications

The way people record ideas has evolved with the medium: inscriptions on stone and bone, writing on paper, printed publications, and eventually digital documents and office software.

Documents are now entering another stage. They can present changing data, let readers adjust a model, and show the result of a simulation. AI can work with their content, structure and layout.

**With UniDoc, our ambition is to take “what you see is what you get” further: what you see can be an application.**

## Why we built UniDoc

The project began with a practical question: how could one document combine Word-style layout, Markdown-style writing and LaTeX mathematical notation?

Early experiments with Markdown and formulas in conventional editors exposed differences in format handling and rendering. Interactive content was also difficult to edit and use as part of the document. Those experiences led us to reconsider how documents should be represented.

Our approach uses text to describe content and structure, retains layout information, and uses Web technology for rendering and interaction. The aim is a document that people can read and edit and that AI can understand, generate and revise.

## What a document can contain

- **Rich expression:** text, images, tables, LaTeX formulas, flexible layouts and reusable templates.
- **Live content:** data-driven charts, adjustable calculations and simulations that readers can explore directly.
- **Understandable structure:** content, object relationships, layout and interaction logic available to AI workflows.
- **Portable files:** a representation designed for storage, migration and version control, balancing size and fidelity.

Interactive documents also need clear permission boundaries for scripts, network requests and local file access. These controls are part of making the format suitable for everyday work.

## The OmniDoc direction

OmniDoc is an office software project developed in China. With UniDoc at its core, we are building tools for documents, spreadsheets, presentations, images and email, serving the everyday needs addressed by Microsoft 365 (Office 365) and WPS Office.

Our long-term priorities are product engineering, document formats, compatibility and performance. We take responsibility for our core implementation while respecting and documenting the third-party technology we use.

Cross-platform access is central to this work. Web technology lets the same document capabilities reach different devices and operating systems. Windows, macOS, Linux, HarmonyOS and Kylin are part of our compatibility direction; supported configurations depend on the product version and actual testing.

## Turning the vision into products

The [UniDoc online editor](https://app.unidoc.top/) is available today. The product family also includes UniPPT, UniCell, UniPic and UniMail. This repository currently publishes source code, build instructions and benchmarks for UniPPT, UniCell and vecmeta.

Size targets and platform ideas in the original project notes are research goals. Implemented features, compatibility and performance are documented through product releases and reproducible measurements.

We are working toward documents that people can read, edit, calculate with and interact with as a single workspace.

[OmniDoc](https://omnidoc.top/) · [Try UniDoc](https://app.unidoc.top/)

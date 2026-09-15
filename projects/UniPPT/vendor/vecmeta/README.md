# vecmeta · UniPPT 集成 / UniPPT integration

[简体中文完整说明](https://github.com/OmniDocX/vecmeta/blob/main/README.md) · [English documentation](https://github.com/OmniDocX/vecmeta/blob/main/README_EN.md) · [源码](https://github.com/OmniDocX/vecmeta)

vecmeta 是 OmniDoc 旗下的原生 Rust SVG ↔ EMF 矢量转换引擎。本目录包含 UniPPT 集成的五个库组件快照：`vector-ir`、`emf-core`、`glyph2path`、`emf2svg`、`svg2emf`。独立项目及命令行工具的源码、构建方式和 API 说明见上述官方仓库。

组件提供矢量场景中间表示、原生 EMF 读写、受支持的 SVG/CSS 解析、字形轮廓及嵌入源数据恢复。源数据恢复与第三方渲染一致性采用独立验证标准；详细支持范围见官方项目文档。

vecmeta is OmniDoc's native Rust SVG ↔ EMF conversion engine. This directory contains five library crates integrated into UniPPT. The standalone command-line tool, API definitions, build instructions, and compatibility specifications are available in the official repository linked above.

当前组件采用 [OmniDoc 非商业源码许可](LICENSE)。第三方组件及旧版本的既有许可保持不变，详见 [来源与许可说明](LICENSE-NOTICE.md)。

Non-commercial use is free under [LICENSE](LICENSE). Commercial use requires permission, including internal business use by companies in China and elsewhere. Earlier valid grants remain unaffected.

**商业联系 / Commercial contact:** [cc@omnidoc.top](mailto:cc@omnidoc.top) · 微信 / WeChat: **13184071590**。完整申请 48 小时内审核答复 / Complete applications reviewed and answered within 48 hours; no automatic approval.

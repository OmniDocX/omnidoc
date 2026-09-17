<div align="center">

<img src="web/unippt-logo.svg" width="72" height="72" alt="UniPPT" />

# UniPPT

**在浏览器中创作演示文稿，导出原生可编辑 PPTX。**

[English](README.md) | **简体中文**

[官网](https://omnidoc.top/) · [在线体验](https://unippt.unidoc.top/) · [快速开始](#快速开始) · [文档](#文档) · [测评](benchmarks/README.zh-CN.md)

[![CI](https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml/badge.svg)](https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml)
[![License: PolyForm Noncommercial](https://img.shields.io/badge/license-PolyForm_Noncommercial-315EFB?style=flat-square)](LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/OmniDocX/UniPPT?style=flat-square)](https://github.com/OmniDocX/UniPPT/issues)

</div>

UniPPT 将浏览器编辑器、Rust 文档引擎和本机 AI 自动化连接起来，覆盖课件、报告、技术演示与日常幻灯片创作。文本、形状、图片、公式和动画围绕同一份结构化文稿工作，可导入 PPTX，继续编辑，再导出为 PPTX、UDOC 或 HTML。

**OmniDoc 自主研发的纯国产应用项目（app project）**，公开源码提供可独立运行的本机版。

![UniPPT presentation editor](docs/images/editor.png)

<sub>公开本机版实际界面：原生文本、图形与 Office 数学公式。</sub>

## 核心能力

- **原生对象编辑** — 直接操作文本、形状、图片、连接线与公式，支持对齐、分组、撤销和重做。
- **PPTX 工作流** — 导入现有演示文稿，编辑支持的对象并导出原生 PPTX；格式引擎保留可回写的未修改部件。
- **演示与动画** — 在浏览器中放映，编排受支持的动画、运动路径、时间和演讲备注。
- **公式创作** — 结合 LaTeX 编辑与 Office 数学公式转换，用于教学与技术内容。
- **AI 与 MCP** — 编辑器内配置 U AI，或通过本机 stdio MCP 让外部 AI 客户端读取、修改和预览文稿。
- **多种交付方式** — 保存为 PPTX、UDOC 或可再导入的 HTML；按需接入 PDF、图片和视频导出工具链。

## 快速开始

准备 Git、Rust 1.88+ 与现代浏览器，执行：

```sh
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
cargo run --release --locked -p unippt-server
```

访问 **http://127.0.0.1:8141**，即可开始编辑。前端随服务提供，无需单独构建；端口可通过 `UNIPPT_PORT` 配置。

<details>
<summary>可选依赖</summary>

数学公式转换使用 Python 3.10+：`python -m pip install -r tools/math/requirements.txt`。MCP 图像处理需 Node.js 22+ 并执行 `npm ci`。Chromium 与 FFmpeg 用于对应的渲染和视频导出流程，详见[运行说明](docs/LOCAL_RUNTIME.md)。

</details>

## 连接 AI 工作流

启动 UniPPT 后，将以下配置加入支持 stdio 的 MCP 客户端，并把路径替换为实际安装路径：

```json
{
  "mcpServers": {
    "unippt": {
      "command": "node",
      "args": ["/absolute/path/to/UniPPT/tools/unippt_mcp.cjs"]
    }
  }
}
```

连接当前本机文稿后，可读取对象、批量创建页面、修改布局并检查预览。[MCP 接入](docs/AI_MCP_INTEGRATION.md) · [可编辑幻灯片 Skill](skills/unippt-editable-slides/SKILL.md)。

编辑器内的 **U AI** 支持自定义模型服务：配置 `UNIPPT_AI_BASE`、`UNIPPT_AI_MODEL` 和 `UNIPPT_AI_KEY`。启用时，所选内容发送至你配置的模型服务。

## 性能

<!-- BENCHMARK:START -->
| 操作 | 工作负载 | 中位数 |
| --- | --- | --- |
| PPTX 导出 | 100 页 | **139.41 ms** |
| PPTX 导入，缓存未命中 | 100 页 | **813.36 ms** |

2026-09-16 · Windows 10 · Intel i7-1165G7 · 31.7 GiB · Rust release · 预热 2 次 / 测量 7 次。

[完整测评、原始数据与复现方法](benchmarks/README.zh-CN.md) — 以上为合成工作负载的本机测试，不含浏览器渲染，未与其他办公软件做速度对测。
<!-- BENCHMARK:END -->

## 版本与文件兼容性

本仓库面向单人本机使用，文件由用户在本地保存。在线产品可通过首屏链接体验；R2、云存储、共享链接、协作编辑和在线账户不包含在此版本。

PPTX 对象的编辑与保留范围取决于对象类型；图表、SmartArt、OLE、母版及高级动画的支持详情见[兼容性说明](docs/COMPATIBILITY.md)。

## 文档

| 入口 | 内容 |
| --- | --- |
| [用户指南](docs/USER_GUIDE.md) | 编辑、演示与导出 |
| [MCP](docs/MCP_PROTOCOL.md) | 工具协议与开发接入 |
| [文档格式](docs/PORTABLE_FORMATS.md) | PPTX、UDOC 与 HTML |
| [架构](docs/ARCHITECTURE.md) | 引擎、服务与编辑器 |
| [贡献指南](CONTRIBUTING.md) | 开发环境、测试与提交要求 |

## OmniDoc 产品与社区

[OmniDoc 主站](https://omnidoc.top/) · [UniPPT](https://github.com/OmniDocX/UniPPT) · [UniCell](https://github.com/OmniDocX/unicell) · [vecmeta](https://github.com/OmniDocX/vecmeta)

问题与建议请提交 [GitHub Issue](https://github.com/OmniDocX/UniPPT/issues)，附上复现步骤和可公开的最小示例。欢迎参与功能开发、兼容性改进和文档建设。

办公体验对标 Microsoft 365（Office 365）与 WPS Office；公开办公项目参照 ONLYOFFICE、Univer。[项目定位与能力对照](docs/COMPARISON.zh-CN.md)。

## 许可证与商业合作

自有代码采用 [PolyForm Noncommercial 1.0.0](LICENSE)。标准许可允许的非商业及特定机构用途免费；超出允许范围的商业用途，须[申请付费商业授权](docs/COMMERCIAL_LICENSE.md)并取得书面许可。

**商业联系：[cc@omnidoc.top](mailto:cc@omnidoc.top) · 微信：13184071590**

本项目采用源码可见许可（非 OSI 开源许可）。第三方组件及旧版本有效授权保持独立，详见[许可范围](docs/LICENSING.md)。

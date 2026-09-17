<div align="center">

# OmniDoc

**面向 AI 时代的国产办公平台。所见即所得，所见即应用。**

[English](README.md) | **简体中文**

[官网](https://omnidoc.top/) · [快速开始](#快速开始) · [文档](#文档) · [测评](benchmarks/README.zh-CN.md)

[![CI](https://github.com/OmniDocX/omnidoc/actions/workflows/ci.yml/badge.svg)](https://github.com/OmniDocX/omnidoc/actions/workflows/ci.yml)
[![License: PolyForm Noncommercial](https://img.shields.io/badge/license-PolyForm_Noncommercial-315EFB?style=flat-square)](LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/OmniDocX/omnidoc?style=flat-square)](https://github.com/OmniDocX/omnidoc/issues)

</div>

OmniDoc 致力于建设 AI 时代的国产办公平台。以 UniDoc 动态文档为核心，我们将文字、布局、公式、数据与交互带入同一份文档，建设覆盖文档、表格、演示、图片和邮件的产品体系。

从石头与甲骨，到纸张、印刷和数字文档，人类表达与传播思想的方式不断演进。UniDoc 希望推动下一步：让文档能够承载实时数据、计算与交互，成为可以直接使用的应用。

**所见即所得，所见即应用。**

这是一个纯国产办公软件项目。我们长期投入核心产品与文件格式研发，面向 Microsoft 365 和 WPS Office 所服务的办公需求，持续推进跨平台使用体验。

本仓库公开 **UniPPT、UniCell 和 vecmeta** 三个项目的源码，附独立构建说明与性能测评。UniDoc 及完整产品可在官网体验。

[体验 UniDoc](https://app.unidoc.top/) · [OmniDoc 官网](https://omnidoc.top/) · [我们的产品愿景](docs/VISION.zh-CN.md)

![OmniDoc project family](docs/images/overview.svg)

## 选择你的工具

| 项目 | 用途 | 开始使用 |
| --- | --- | --- |
| **UniPPT** | 原生可编辑幻灯片、PPTX 转换、AI 编排 | [文档](projects/UniPPT/README.zh-CN.md) · [GitHub](https://github.com/OmniDocX/UniPPT) |
| **UniCell** | 本机电子表格、公式计算、Excel / CSV 文件处理 | [文档](projects/unicell/README.zh-CN.md) · [GitHub](https://github.com/OmniDocX/unicell) |
| **vecmeta** | Rust SVG ↔ EMF 库与命令行工具 | [文档](projects/vecmeta/README.zh-CN.md) · [GitHub](https://github.com/OmniDocX/vecmeta) |

<table>
  <tr>
    <td width="50%"><a href="projects/UniPPT/README.md"><img src="projects/UniPPT/docs/images/editor.png" alt="UniPPT presentation editor" /></a></td>
    <td width="50%"><a href="projects/unicell/README.md"><img src="projects/unicell/docs/images/editor.png" alt="UniCell spreadsheet editor" /></a></td>
  </tr>
  <tr><td align="center"><strong>UniPPT</strong></td><td align="center"><strong>UniCell</strong></td></tr>
</table>

## 我们公开了什么

- **可以运行的应用**：UniPPT 和 UniCell 保留本机编辑、文件导入导出与核心计算能力。
- **可以复用的组件**：vecmeta 提供矢量格式转换库和命令行工具；各项目另有对应的 AI、MCP 或 Rust 接口说明。
- **可以核验的结果**：性能报告列出测试环境、工作负载、原始数据和适用范围。
- **可以追溯的源码**：仓库记录各项目的来源版本与文件哈希，便于构建和核对。

## 快速开始

获取仓库后，选择所需项目启动。各组件独立运行：

```sh
git clone https://github.com/OmniDocX/omnidoc.git
cd omnidoc
```

**演示文稿 · UniPPT**

```sh
cd projects/UniPPT
cargo run --release --locked -p unippt-server
```

http://127.0.0.1:8141

<details>
<summary>启动 UniCell 或构建 vecmeta</summary>

以下命令均从合集根目录开始：

```sh
cd projects/unicell
cargo run --release --locked --manifest-path server/Cargo.toml
```

http://127.0.0.1:8143

```sh
cd projects/vecmeta
cargo build --release --locked -p emfsvg-cli
```

</details>

需要 Git、Rust 和现代浏览器；具体工具链版本、平台依赖与 AI 配置见各项目文档。

## 性能一览

<!-- BENCHMARK:START -->
| 项目 | 工作负载 | 中位数 |
| --- | --- | --- |
| UniPPT | 100 页 PPTX 导出 | **139.41 ms** |
| UniCell | 1 万行 CSV 导入与计算，2 万公式 | **660.92 ms** |
| vecmeta | 1 万图元 SVG → EMF | **105.80 ms** |

2026-09-16 · Windows 10 · Intel i7-1165G7 · 31.7 GiB · Rust release · 预热 2 次 / 测量 7 次。合成工作负载，本轮未做竞品速度对测。

[完整测评与各组件原始数据](benchmarks/README.zh-CN.md)
<!-- BENCHMARK:END -->

## 版本与收录范围

本合集收录单人本机版 UniPPT、UniCell 和 vecmeta，不包含 PolyglotPDF。公开本机版保留基础编辑与格式处理，不含 R2、云存储、协作编辑、共享链接和统一账户。

`projects/` 中是可独立构建的源码副本。[sources.lock.json](sources.lock.json) 记录上游仓库、提交和文件哈希；开发者可运行 `python tools/verify_copies.py` 核对副本。

## 文档

| 入口 | 内容 |
| --- | --- |
| [UniPPT](projects/UniPPT/README.zh-CN.md) | 编辑器、MCP、公式与导出 |
| [UniCell](projects/unicell/README.zh-CN.md) | 工作簿、计算、AI 与文件格式 |
| [vecmeta](projects/vecmeta/README.zh-CN.md) | 命令行、Rust API 与兼容性 |
| [OmniDoc](https://omnidoc.top/) | 产品主站与旗下应用 |

## 旗下在线产品

[UniDoc](https://app.unidoc.top/) · [UniPPT](https://unippt.unidoc.top/) · [UniCell](https://unicell.unidoc.top/) · [UniMail](https://unimail.omnidoc.top/) · [UniPic](https://pic.unidoc.top/)

在线产品的功能与服务范围以各产品说明为准。

## OmniDoc 产品与社区

[OmniDoc 主站](https://omnidoc.top/) · [UniPPT](https://github.com/OmniDocX/UniPPT) · [UniCell](https://github.com/OmniDocX/unicell) · [vecmeta](https://github.com/OmniDocX/vecmeta)

问题与建议请提交 [GitHub Issue](https://github.com/OmniDocX/omnidoc/issues)，附上复现步骤和可公开的最小示例。欢迎参与功能开发、兼容性改进和文档建设。

办公体验对标 Microsoft 365（Office 365）与 WPS Office；公开办公项目参照 ONLYOFFICE、Univer。[项目定位与能力对照](docs/COMPARISON.zh-CN.md)。

## 许可证与商业合作

自有代码采用 [PolyForm Noncommercial 1.0.0](LICENSE)。标准许可允许的非商业及特定机构用途免费；超出允许范围的商业用途，须[申请付费商业授权](docs/COMMERCIAL_LICENSE.md)并取得书面许可。

**商业联系：[cc@omnidoc.top](mailto:cc@omnidoc.top) · 微信：13184071590**

本项目采用源码可见许可（非 OSI 开源许可）。第三方组件及旧版本有效授权保持独立，详见[许可范围](docs/LICENSING.md)。

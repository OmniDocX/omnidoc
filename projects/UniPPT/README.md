<h1 align="center">UniPPT</h1>
<p align="center"><strong>原生演示文稿引擎与 AI 自动化平台</strong></p>
<p align="center">OmniDoc 旗下演示文稿产品，支持原生 PPTX 编辑、MCP 集成、Skills 工作流及自定义模型。</p>

<p align="center">
  <a href="https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml"><img alt="Verify" src="https://github.com/OmniDocX/UniPPT/actions/workflows/verify.yml/badge.svg"></a>
  <img alt="Rust 1.88+" src="https://img.shields.io/badge/Rust-1.88%2B-222222">
  <img alt="MCP and Skills" src="https://img.shields.io/badge/AI-MCP%20%2B%20Skills-2563eb">
  <a href="LICENSE"><img alt="Non-commercial license" src="https://img.shields.io/badge/License-Non--Commercial%20%2B%20Commercial-orange"></a>
</p>

<p align="center"><strong>简体中文</strong> · <a href="README_EN.md">English</a></p>
<p align="center"><a href="https://unippt.unidoc.top/">在线体验</a> · <a href="docs/USER_GUIDE.md">使用文档</a> · <a href="docs/MCP_PROTOCOL.md">MCP 协议</a> · <a href="https://omnidoc.top/">OmniDoc 主站</a> · <a href="docs/COMMERCIAL_LICENSE.md">商业授权</a></p>

---

## 本机版范围

本仓库提供本地单机版：保留基础编辑、动画放映、格式转换、导入导出、自定义 U AI 和本机 MCP。R2、云存储、云文稿、共享编辑、分享链接、多人协作与账户网关不包含在此源码中。

## 项目概述

UniPPT 是 OmniDoc 旗下的演示文稿编辑与自动化平台，为 PowerPoint 文档工作流提供浏览器端及本地部署方案。系统由 Rust 文稿引擎、HTML/SVG 编辑界面和 MCP 服务组成，覆盖文稿导入、内容创作、对象编辑、公式排版、动画配置、放映与多格式导出。

文本、形状、连接线、公式、图片和动画采用统一的结构化文稿模型。浏览器编辑器与外部 AI 客户端操作相同的对象及属性，支持将编辑结果导出为包含原生对象的 PPTX 文件。

| 核心能力 | 技术说明 |
| --- | --- |
| **原生 PPTX 对象** | 将文本、形状、图片、连接线、公式和支持的动画映射为原生 OOXML 结构 |
| **保留优先的格式往返** | 基于原始 OPC 包定点修改，未编辑部件、未知扩展和原始时间树按保留策略处理 |
| **LaTeX ↔ Office 公式** | 同时保留 LaTeX 与 OMML，衔接网页预览和 PowerPoint 原生公式 |
| **动画编排** | 进入、强调、退出、时长、延迟、顺序、动作路径和媒体节点结构化编辑 |
| **MCP + Skills** | 外部 AI 读取对象、批量生成模块、局部修复、配置动画、预览、检查和导出 |
| **自定义模型与部署** | MCP 不绑定模型供应商；内置 U AI 可接 OpenAI-compatible 接口 |
| **多格式交付** | PPTX、UDOC、无损 HTML、PDF、图片包与视频 |

## 目录

[能力与兼容性](#能力与兼容性) · [AI、MCP 与 Skills](#aimcp-与-skills) · [快速开始](#快速开始) · [OmniDoc 旗下业务](#omnidoc-旗下业务) · [开发](#架构与开发) · [许可](#许可与商业使用)

## 能力与兼容性

### 编辑器功能

编辑器提供 Ribbon 功能区、缩略图、大纲、备注、格式窗格、动画窗格与放映视图。对象操作包括选择、移动、缩放、旋转、层级调整及撤销/重做；内容插入支持文本、形状、图片和公式。文本格式工具栏基于字符选区显示，并将格式修改应用于对应范围。

### 原生引擎与便携格式

- **PPTX**：Rust 负责 OOXML/OPC 解析与导出；普通对象编辑与源文件保留策略结合。
- **公式**：LaTeX / MathML / OMML 转换，KaTeX 实时预览，Office Math 对象写回。
- **动画**：常见效果可编辑并重新生成原生 `p:timing`；未建模自定义动画按保留策略处理。
- **UDOC / 无损 HTML**：携带可再次导入的原生数据，HTML 也可用于便携放映。
- **SVG / EMF**：使用 OmniDoc 的 [vecmeta](https://github.com/OmniDocX/vecmeta) 原生 Rust 矢量转换组件。
- **视频**：浏览器录屏与 Chromium + FFmpeg 逐帧渲染，适配即时录制和固定帧率交付。

原生实现范围包括文稿对象模型、OOXML 处理及矢量转换。公式转换依赖 Python/XSLT，预览和视频处理依赖浏览器及相应工具链。

| 内容 | 处理方式 |
| --- | --- |
| 文本、基础形状、图片、OMML、常见动画 | 原生编辑与导出 |
| 未编辑 OPC 部件、未知 Office 扩展 | 保留优先，配合字节级回归检查 |
| 图表、SmartArt、OLE、母版和复杂对象 | 导入/保留能力与结构化编辑能力分别评估 |
| 复杂动画、受保护文件和特殊 Office 功能 | 按具体格式与对象支持范围处理 |

具体格式及对象的支持范围见 [兼容性说明](docs/COMPATIBILITY.md)。兼容性验收分别检查文件结构、视觉呈现和对象语义，不以导出成功替代完整验证。

## AI、MCP 与 Skills

```text
宿主模型分析输入 → Skill 定义工作流
        ↓
MCP 查询对象与能力 → 批量编排 / 原子修改 / 动画配置
        ↓
预览与验证 → 局部修订 → 原生 PPTX / 其他格式导出
```

- **模型中立**：使用具备相应工具能力的 ChatGPT、Codex、Claude 等客户端，模型由宿主选择。
- **模块级批量操作**：将重复层片、箭头与标签等模块描述展开为文稿对象，支持单次批量提交。
- **版本与并发保护**：写入遵循工具要求的 `sessionId`、`expectedRevision` 与稳定 `requestId`。
- **局部修订**：基于对象 ID 更新指定区域，并结合预览、结构验证与导出完成检查。
- **图片转可编辑 PPT**：网页 U AI 提供图像重建流程；外部模型可完成视觉分析后，通过 MCP 生成原生文稿对象。重建结果需与源图进行文字、公式和几何校验。

**本机 MCP：`node /absolute/path/to/UniPPT/tools/unippt_mcp.cjs`**

支持 stdio 的客户端可接入已打开的本机文稿。编辑器右上角设置菜单控制 MCP 自动连接；无需账户授权。接口契约以运行时 `tools/list` 与 Schema 为准。

仓库保留 [editable-slides Skill](skills/unippt-editable-slides/SKILL.md)，用于本机原生对象编辑与图片重建。

[完整协议](docs/MCP_PROTOCOL.md) · [本机接入](docs/AI_MCP_INTEGRATION.md)

### 自定义模型

网页 U AI 设置或本机 `.env.local` 可配置 OpenAI-compatible 服务：

```ini
UNIPPT_AI_BASE=https://api.example.com/v1
UNIPPT_AI_MODEL=your-model-name
UNIPPT_AI_KEY=<your-local-key>
```

模型密钥应保存在本机配置或部署环境中，不应提交至版本库。坐标 OCR 配置项 `UNIPPT_OCR_MODEL` 要求服务兼容 DashScope 原生协议。MCP 宿主模型与网页 U AI 使用独立配置。

## 快速开始

环境：**Rust 1.88+、Git**。公式转换需要 **Python 3.10+**；测试使用 Node.js 22，音视频回归使用 FFmpeg/ffprobe。

### Windows / PowerShell

```powershell
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
python -m venv .venv
.venv\Scripts\python.exe -m pip install -r tools/math/requirements.txt
$env:UNIPPT_PYTHON = (Resolve-Path .venv\Scripts\python.exe).Path
cargo run -p unippt-server --locked
```

### Linux / macOS

```sh
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
python3 -m venv .venv
.venv/bin/python -m pip install -r tools/math/requirements.txt
UNIPPT_PYTHON="$PWD/.venv/bin/python" cargo run -p unippt-server --locked
```

服务启动后访问 **http://127.0.0.1:8141**。监听端口可通过进程环境变量 `UNIPPT_PORT` 配置；前端静态资源无需额外执行 npm 构建。

上述命令启动本地单机版。文稿通过文件保存到本机，服务器监听回环地址。

本机 MCP 图片处理与完整测试需 Node.js 22+，在仓库根目录执行 `npm ci`。原生预览与额外交付验证的可选依赖见 [本机工具配置](docs/LOCAL_RUNTIME.md)。

## OmniDoc 旗下业务

OmniDoc 提供文档处理、内容创作、演示文稿、电子表格、邮件管理及图像处理产品，并提供配套的格式转换组件。

| 产品 / 组件 | 业务方向 | 官方网站 / 源码 |
| --- | --- | --- |
| OmniDoc | 主站、文档服务与账户基础设施 | [omnidoc.top](https://omnidoc.top/) |
| UniDoc | 文档创作与编辑 | [app.unidoc.top](https://app.unidoc.top/) |
| UniPPT | 原生演示文稿与 AI 编排 | [unippt.unidoc.top](https://unippt.unidoc.top/) · [源码](https://github.com/OmniDocX/UniPPT) |
| UniCell | 浏览器电子表格 | [unicell.unidoc.top](https://unicell.unidoc.top/) |
| UniMail | 多账户邮件、日历、联系人与 AI 邮件辅助 | [unimail.omnidoc.top](https://unimail.omnidoc.top/) |
| UniPic | 图片与矢量编辑 | [pic.unidoc.top](https://pic.unidoc.top/) |
| vecmeta | 原生 Rust SVG ↔ EMF 引擎 | [源码与使用说明](https://github.com/OmniDocX/vecmeta) |
| PolyglotPDF | 多语种电子书与 PDF 翻译 | [源码](https://github.com/OmniDocX/PolyglotPDF) |

各产品的部署方式、账户要求及许可条款以对应官方文档为准。vecmeta 以 Rust 库和命令行工具形式交付。

## 架构与开发

```text
crates/unippt-core/      文稿模型、导入/导出、公式和动画
crates/unippt-server/    本机服务、渲染、AI 宿主
web/                    HTML/SVG 编辑器与放映
vendor/pptx/            保留上游署名的 rust-pptx 分支
vendor/vecmeta/         OmniDoc 原生矢量组件
skills/                 本机 MCP 工作流
docs/                   协议、兼容性、格式及接入
```

```sh
cargo test --workspace --locked
node --check web/app.js
npm ci
node --test web/*.test.js tools/*.test.cjs
python tools/test_public_boundary.py -v
python tools/check_public_boundary.py
```

发布检查针对 Git 暂存区执行，运行前需暂存待提交文件。测试覆盖文稿往返、原生对象、动画、公式、网页交互、音视频、授权及发布边界；涉及视觉输出的变更还应进行渲染与对象级验证。

[架构](docs/ARCHITECTURE.md) · [便携格式](docs/PORTABLE_FORMATS.md) · [维护指南](docs/OPEN_SOURCE.md) · [贡献规范](CONTRIBUTING.md) · [安全报告](SECURITY.md)

## 许可与商业使用

**非商业用途免费；商业用途须事先取得书面许可。**

当前自有源码采用 [OmniDoc Non-Commercial Source License 1.0](LICENSE)，属于源码可见许可，不是 MIT，也不属于 OSI 定义的开源许可。个人学习、非商业研究与教学可按协议使用、修改及非商业分发。

**中国公司及其他国家或地区的企业，将本项目用于内部业务、商业产品、SaaS、API、客户交付或商业集成时，均须取得商业许可。** 修改源码、私有部署或不直接收费，不自动免除授权要求。

- **邮箱：** [cc@omnidoc.top](mailto:cc@omnidoc.top)
- **微信：** 添加手机号 **13184071590**
- **审核时限：** 完整申请材料收到后 **48 小时内完成审核并答复**。申请或超时未回复不构成授权，实际权利以书面许可为准。

[商业申请流程](docs/COMMERCIAL_LICENSE.md) · [许可范围与历史版本](docs/LICENSING.md)。第三方组件保留各自协议；不追溯撤销旧版本已授予的 MIT/GPL 等权利。

PowerPoint 是 Microsoft 的商标；UniPPT 是独立项目，不代表 Microsoft 产品、认证或背书。

# UniPPT

[English](README.md) | **简体中文**

[OmniDoc](https://omnidoc.top/) · [GitHub](https://github.com/OmniDocX)

**原生演示文稿编辑、PPTX 转换与本机 AI 自动化。**

UniPPT 是 OmniDoc 自主研发的国产演示文稿应用项目。系统由 Rust 文稿引擎与 HTML/SVG 浏览器编辑器组成，将文本、形状、图片、公式及支持的动画表示为结构化对象，并导出为原生 PPTX 元素。

## 项目定位

以 **Microsoft 365（Office 365）和 WPS Office** 为同类产品对标，致力于建设功能最完善、工程资料最完整的国产开放源码办公项目。该表述是发展目标；当前功能范围与证据见 [产品对照](docs/COMPARISON.zh-CN.md)。自有代码使用非商业源码许可，具体条件见下文。

## 性能测评

<!-- BENCHMARK:START -->
2026-09-16 在 Windows 10 / Intel Core i7-1165G7 / 31.70 GiB 内存上实测。每项预热 2 次、测量 7 次，顺序执行。

| 操作 | 规模（页） | 中位数 ms | P95 ms |
| --- | ---: | ---: | ---: |
| PPTX 导出 | 100 | 139.41 | 316.91 |
| PPTX 导入，缓存未命中 | 100 | 813.36 | 1721.34 |
| UDOC 导出 | 100 | 669.36 | 1189.64 |
| UDOC 导入，缓存未命中 | 100 | 72.49 | 85.58 |
| HTML 导出 | 100 | 325.71 | 369.41 |
| HTML 导入，缓存未命中 | 100 | 83.82 | 278.04 |

[全部规模、复现方法与限制](benchmarks/README.zh-CN.md) · [原始数据](benchmarks/results/2026-09-16-windows-x64.json)

以下耗时不包含浏览器渲染；Office 365 和 WPS 本轮未进行速度对测。
<!-- BENCHMARK:END -->

## 功能范围

| 领域 | 公开本机版 |
| --- | --- |
| 内容编辑 | 文本与形状、图片、连接线、公式、对象变换、备注、撤销与重做 |
| 演示播放 | 放映、支持的动画效果、时序和动作路径 |
| 文稿格式 | PPTX 导入导出；便携 UDOC 与可重新导入的 HTML |
| 格式保留 | 往返处理时保留未编辑 OPC 部件及支持的未知扩展 |
| 自动化 | 本机 stdio MCP、对象级操作、可编辑幻灯片工作流与可选 U AI |
| 可选交付 | 通过文档所列浏览器与渲染工具链生成预览、PDF、图片和视频 |

本仓库提供单用户本机版。R2、云存储、共享编辑、分享链接、远程账户和托管协作不属于本版范围。

## 快速开始

环境要求：Git、Rust 1.88+ 和现代浏览器。前端资源直接由服务提供，无需单独构建。

```sh
git clone https://github.com/OmniDocX/UniPPT.git
cd UniPPT
cargo run --release --locked -p unippt-server
```

访问 **http://127.0.0.1:8141**。通过 `UNIPPT_PORT` 修改端口。服务绑定本机回环地址，文稿通过本机文件保存。

LaTeX/Office 公式转换需要 Python 3.10+，执行 `python -m pip install -r tools/math/requirements.txt`，并可通过 `UNIPPT_PYTHON` 指定解释器。MCP 图片处理与完整 JavaScript 检查需要 Node.js 22+ 和 `npm ci`。部分导出工作流需额外安装 Chromium、FFmpeg，见 [本机运行环境](docs/LOCAL_RUNTIME.md)。

## AI 与本机 MCP

支持 stdio 的 MCP 客户端可运行 `node /absolute/path/to/UniPPT/tools/unippt_mcp.cjs`，接入已打开的本机文稿。参见 [接入指南](docs/AI_MCP_INTEGRATION.md)、[协议](docs/MCP_PROTOCOL.md) 和 [可编辑幻灯片工作流](skills/unippt-editable-slides/SKILL.md)。

U AI 通过 `UNIPPT_AI_BASE`、`UNIPPT_AI_MODEL`、`UNIPPT_AI_KEY` 或本机设置界面配置模型服务。密钥应保存在本机；所选文稿上下文会发送至配置的服务。基础编辑不依赖 AI。坐标 OCR 要求 DashScope 兼容的原生协议，与 MCP 宿主模型配置相互独立。

## 兼容性

原生编辑与源文件保留属于不同能力。图表、SmartArt、OLE、母版和高级动画存在各自的支持边界，导出成功不代表完整兼容 PowerPoint。实际文稿应分别核对对象语义与渲染结果。参见 [兼容性](docs/COMPATIBILITY.md)、[便携格式](docs/PORTABLE_FORMATS.md) 和 [使用指南](docs/USER_GUIDE.md)。

## 架构与验证

| 目录 | 职责 |
| --- | --- |
| `crates/unippt-core` | 文稿模型、格式、公式及动画 |
| `crates/unippt-server` | 本机 HTTP 服务、渲染及 AI 接入 |
| `web` | 浏览器编辑器与放映界面 |
| `vendor/pptx`、`vendor/vecmeta` | 保留署名的第三方分支及独立版本矢量组件 |
| `benchmarks` | 合成工作负载、实测结果与复现说明 |

```sh
cargo test --workspace --locked
npm ci
node --test web/*.test.js tools/*.test.cjs
python tools/test_public_boundary.py -v
python tools/check_public_boundary.py
```

发布检查读取 Git 暂存区，运行前应暂存计划发布的修改。[架构说明](docs/ARCHITECTURE.md) · [贡献规范](CONTRIBUTING.md) · [安全报告](SECURITY.md) · [许可说明](docs/LICENSING.md)。

## OmniDoc 产品体系

| 项目 | 方向 | 官网 / 源码 |
| --- | --- | --- |
| OmniDoc | 产品主站 | [omnidoc.top](https://omnidoc.top/) |
| UniDoc | 文档创作 | [app.unidoc.top](https://app.unidoc.top/) |
| UniPPT | 演示文稿 | [编辑器](https://unippt.unidoc.top/) · [源码](https://github.com/OmniDocX/UniPPT) |
| UniCell | 电子表格 | [编辑器](https://unicell.unidoc.top/) · [源码](https://github.com/OmniDocX/unicell) |
| UniMail | 邮件、日历与联系人 | [unimail.omnidoc.top](https://unimail.omnidoc.top/) |
| UniPic | 图像与矢量编辑 | [pic.unidoc.top](https://pic.unidoc.top/) |
| vecmeta | SVG ↔ EMF 转换 | [源码](https://github.com/OmniDocX/vecmeta) |
| 源码合集 | 三个已公开组件的固定版本副本 | [omnidoc](https://github.com/OmniDocX/omnidoc) · [omnidocx](https://github.com/OmniDocX/omnidocx) |

在线产品可能提供超出公开本机版范围的功能，其可用性与条款以各产品说明为准。

## 许可证与商业授权

自有代码及文档采用 [OmniDoc 非商业源码许可 1.0](LICENSE)。符合条款的非商业使用免费；商业使用，包括中国及其他地区企业的内部业务使用，须事先取得书面授权。本许可属于源码可见许可，不是 OSI 批准的开源许可。第三方组件保持各自条款，旧版本已合法授予的权利不受追溯影响。

商业联系：[cc@omnidoc.top](mailto:cc@omnidoc.top) · 微信：**13184071590**。完整申请材料收到后 48 小时内答复；提交申请或未获回复均不构成授权。

# 本机可选工具依赖

基础编辑器、PPTX/UDOC/HTML 导入导出使用 Rust 服务和浏览器。以下功能另外需要本机工具。

## 图片处理与测试

Node.js 22 或更新版，在仓库根目录运行 `npm ci` 安装锁定的 `sharp`。它用于本机 MCP 图片解码、裁剪与区域预览，不调用模型或远程存储。运行 `npm test` 验证网页和 stdio 工具。

公式依赖见 `tools/math/requirements.txt`；使用 `UNIPPT_PYTHON` 指定安装这些依赖的 Python。音视频回归测试需要 FFmpeg/ffprobe。

## 原生 PPTX 预览

原生预览需要独立渲染器。Linux 可安装 LibreOffice、Poppler 和 coreutils，并设置 `UNIPPT_QUALITY_RENDERER=libreoffice`。该适配器使用 `/usr/bin/soffice`、`/usr/bin/pdftoppm`、`/usr/bin/timeout`。

具有演示文稿运行时的环境也可配置 `UNIPPT_NODE_PATH` 和 `UNIPPT_PRESENTATION_NODE_MODULES`，后者须含 `@oai/artifact-tool`。该外部运行时不随仓库分发。

本机 MCP 的额外交付验证使用 `.unippt-quality-runtime.json` 中的 `nodeExecutable`、`nodeModules`、`pythonExecutable` 和 `skillDir`。`skillDir` 指向已安装的 Presentations Skill，包含 `container_tools` 中的完整验证工具。配置缺失时会保留已提交的文稿修改并报告交付验证未通过，不应视作验证成功；仍可通过编辑器导出文件。

## 视频

使用仓库 `tools/html-video-renderer` 提供的本机 Chromium 捕获工具和 FFmpeg。渲染器、公式和浏览器的可用性应按实际安装检查；普通编辑不要求配置模型服务。

所有运行时路径和密钥属于本机配置，已被 Git 忽略，不得提交到公开仓库。

# UniPPT 便携格式

## UDOC

UniPPT 的 `.udoc` 使用母项目相同的 UDOC 混合压缩逻辑容器。新文件直接以 `UDOC3PKG` 开头，不再用覆盖完整文档的外层 ZIP：

- 8 字节文件头：`UDOC3PKG`。
- 独立部件载荷；JSON、XML 和其他适合压缩的结构化部件使用 Brotli q9（16 MiB 窗口）。
- 图片、音视频、PDF、字体和其他二进制资源各自封装为只含同名文件的 ZIP 部件；PNG/JPEG/MP4/WOFF2 等已压缩格式使用 ZIP Store，BMP/WAV/TTF/OTF 和其他可压缩二进制使用 ZIP Deflate level 6。
- 每个目录项记录原始/压缩尺寸、MIME、codec 和 SHA-256。
- 中央目录固定 Brotli 压缩。
- 64 字节尾部以 `UD3DIR01` 开始，记录目录范围、目录 SHA-256、版本和原始尺寸。

UDoc 不内嵌一份完整 `.pptx` ZIP，也不依赖外部 PPTX 文件或服务器缓存。PPTX 作为 OPC 包被拆分为有序的原生 member 和内容寻址 blob；`document/document.json` 中的当前 UniPPT 场景与这份 OPC 快照同步，重开后直接作为差量回写基线。打开 `.udoc` 时恢复场景与资源表，不需要再次执行 PPTX 全量解析。

包内核心部件：

| 路径 | 作用 |
| --- | --- |
| `manifest.json` | 包入口、类型、特性和关系路径 |
| `document/document.json` | 与 OPC 快照同步的可编辑场景、动画/切换结构及重开后的定点回写基线 |
| `pptx/package.json` | OPC 包索引；按原 ZIP 顺序记录 member 名、blob 路径、压缩方式、尺寸、校验值和可携带的 ZIP 元数据 |
| `blobs/sha256/<sha256>` | OPC member 或场景资源的原始字节；以 SHA-256 内容寻址并在包内跨类型去重 |
| `rels/relationships.json` | 场景文档到 OPC 包索引和资源索引的内部关系 |
| `assets/index.json` | 场景资源 ID 到内容寻址 blob 的映射；可直接复用同一个 `blobs/sha256/<sha256>` |

打开器要求 manifest 与 document 同时满足 `format`/`version` 约束，并明确声明 `unidoc_type="pptx"` 和 `app="UniPPT"`。为兼容既有文件，导入可接受 manifest 与 document 同时使用旧值 `ppt`；新导出及结构查看只产生 `pptx`。打开器会验证路径安全、边界、重叠、解压尺寸和每个 SHA-256，并验证 `pptx/package.json` 中每个 member 引用的 blob、尺寸与摘要。

每个唯一内容只写入一个 blob。多个 OPC member 或场景资源如果 SHA-256 和尺寸一致，它们的索引直接引用同一个 `blobs/sha256/<sha256>`；编辑后新增且尚未成为 OPC member 的资源也进入同一内容寻址空间。容器中不存在第二份完整 PPTX，也不使用 `sourcePartName` 穿透回指某个内嵌 ZIP，因此不会形成“完整 PPTX + 展开资源”的体积翻倍。

浏览器无法直接播放的原生媒体（例如 AVI）在运行缓存与无损 HTML 中可以同时携带浏览器兼容的 MP4 派生资源。UDoc 不再重复保存这类服务器生成的播放副本：原生 AVI 仍作为 OPC blob 自包含保存，打开 UDoc 时一次性重建 MP4，并同时回填当前场景和无损回写基线。若运行环境没有可用的 FFmpeg，原生 PPTX 回写仍保持完整，只是该次浏览器放映无法播放 AVI。用户显式插入、且不能由 OPC 原生媒体重建的 MP4 不会被删除。

压缩按部件语义选择且不改变原始字节：JSON、XML、RELS、VML、SVG 和纯文本部件独立尝试 Brotli；每个二进制资源使用独立的单文件 ZIP Store/Deflate，ZIP 内路径必须与尾目录声明路径完全一致。完整 `.udoc` 不再套 Brotli 或 Deflate，因而尾目录仍可直接随机寻址任意部件，修改一个部件也不要求解压整个文档。解码后的 OPC member 字节必须与目录中的原始尺寸和 SHA-256 一致。打开器继续兼容旧的 Store/Brotli 部件，也兼容只含 `package.udoc3` 的旧外层 ZIP，但新导出只产生直接 `UDOC3PKG` 容器。

当 current Deck 与导入 baseline 完全一致时，`document/document.json` 不再重复保存 baseline；缺省语义就是“baseline 等于 current”。发生编辑后才携带独立 baseline，保证重开后仍能 loss-aware 回写。

### 无损边界

“原生部件无损”和“整个 ZIP 字节一致”是两个不同的保证：

- **OPC 结构/部件无损**：无编辑时，必须保留全部 ZIP member 的名称、顺序和解压后原始字节，包括 `[Content_Types].xml`、所有 `.rels`、母版/版式/主题、媒体、字体、图表、SmartArt、批注、自定义 XML、嵌入对象、宏和厂商扩展。导出时只改写被结构化编辑触及的 member，其余 blob 直接复用。
- **ZIP 外壳字节一致**：要求重建后 `.pptx` 的整体 SHA-256 与原文件相同，还必须保留每个 local header、原压缩流、general-purpose flags、extra fields、data descriptor、时间戳、权限/属性、entry comment、填充/偏移、central directory、ZIP64/EOCD 和 archive comment 的确切字节。当前 OPC parts 容器不存储完整 PPTX 外壳，因此不宣称重建 ZIP 字节一致；在未编辑导出时保证的是 OPC member 字节与顺序无损，重建 ZIP 的压缩流、头部和中央目录可以不同。

因此，“PowerPoint 可打开、OPC 图与所有未编辑 member 原字节不变”是当前的结构无损目标；“重建的 `.pptx` 与导入文件逐字节相同”属于另一级 ZIP 外壳保真，尚未实现，不应与 OPC 结构无损混用。

## 无损 HTML

无损 HTML 是无需服务器即可放映的单文件，同时也是可再次导入的编辑载体。页面内的数据岛为：

```html
<script type="application/x-unippt+json" id="unippt-data">…</script>
```

v4 payload 包含：

- `format: "unippt-html"`
- `version: 4`
- `documentFormat: "udoc"` 与 `unidocVersion: 3`：兼容母项目的公共容器外壳
- `unidocType: "pptx"`（导入兼容旧值 `ppt`）
- `app: "UniPPT"`
- `deck`：当前可编辑场景、结构化动画和母版/版式信息
- `baselineDeck`：仅在当前场景已经编辑时携带，用于重开后的 loss-aware 差量回写
- `opcPackage`：与 UDoc 相同的有序 OPC member 索引
- `blobs`：`blobs/sha256/<digest>` 内容寻址字节表，OPC 与场景资产共享同一份 Base64
- `assets`：场景资源 ID 到 blob 的映射，不再重复内嵌资源 Base64
- `sceneSha256`、`baselineSha256` 与 `opcSha256`：导入前校验当前场景、不可变基线和有序 OPC 索引

导入器继续兼容早期 v1/v2/v3（包括 `pptxBase64`/`pptxBr`）和旧 `unidocType: "ppt"`，但新的导出一律写 v4 与 `unidocType: "pptx"`。HTML 运行时按动画顺序处理单击、与上一动画同时和上一动画之后三种触发关系，并提供与编辑器放映一致的整页导航、动画节点导航、音量/静音和快捷键。

## 与母项目 UniDoc 的兼容边界

UniPPT 与 UniDoc 共享 UDOC 文件头、尾目录、部件寻址、Brotli/SHA-256、`app`/`unidoc_type` 判别和版本规则；不把演示文稿强行转换成 Word 的 `blocks/page/wordCache` 文档树。`unidoc_type="docs"` 使用文字文档 payload，`unidoc_type="pptx"` 使用 `deck/masters/layouts/slides/animations/transitions` payload。也就是说需要统一的是容器协议与工具链，不是两种内容模型。

## API

| 方法 | 路径 | 内容 |
| --- | --- | --- |
| `POST` | `/api/export-udoc` | `{ "deck": … }` → `.udoc` |
| `POST` | `/api/import-udoc` | `.udoc` bytes → `Deck` JSON |
| `POST` | `/api/export-html` | `{ "deck": … }` → 无损 `.html` |
| `POST` | `/api/import-html` | HTML bytes → `Deck` JSON |

UDoc 与无损 HTML 现在共享“当前 Deck + baseline Deck + OPC package index + 内容寻址 blobs”语义；两者都不内嵌完整 PPTX，也不依赖外部 PPTX。区别只在传输外壳：UDoc 使用 Brotli 部件和尾目录，HTML 使用可离线执行的 JSON 数据岛与共享播放器运行时。

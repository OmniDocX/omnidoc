# UniPPT 架构边界

## 核心原则

UniPPT 的长期资产是自己的场景模型，而不是某个 PPTX crate 的公开结构。解析库负责理解 OOXML，`unippt-core` 负责把它转换成稳定、适合浏览器编辑和多格式导出的语义对象。

这一边界吸收了相邻母项目的经验：

- `unicell`：Rust 服务、纯 HTML 交互、对象层、OOXML 资源解析和单文件 HTML 导出。
- `unidoc`：UDoc、HTML 沙箱对象、放映体验和 LaTeX/OMML 公式互转。
- UniPPT 自身按解析内核、场景模型、服务和 UI 分层，不复制母项目的大型单文件前端。

## 数据流

1. `unippt-pptx` 读取 OPC ZIP，保留 part、relationship 和未知二进制/XML。
2. `Presentation::slide_shape_tree()` 生成可直接渲染的对象树，并递归水合图片。
3. `unippt-core` 将 EMU 坐标归一到 1280 像素宽的画布，生成可序列化的 `Deck`。
4. 导入器从形状文本体中识别 `m:oMath` / `m:oMathPara`，保留原始 OMML。
5. `unippt-server` 调用公式转换脚本补齐 LaTeX，浏览器以 LaTeX 实时预览并同时保存两种源格式。
6. 浏览器只消费 `Deck`，所有编辑都写回该模型。
7. `p:timing` 同时进入结构化动画模型和原始 XML fallback；普通编辑保留原 XML，动画编辑生成新的原生时间树。
8. UDOC 保存当前场景、不可变 baseline 和内容寻址 OPC parts，不嵌套完整 PPTX；PPTX exporter 仅在导出边界重建 OPC ZIP，并只改写发生变化的 shape/slide 结构。
9. UniDoc 自由 HTML 进入禁用脚本的布局沙箱，由浏览器冻结最终 DOM/CSS 坐标；静态文本、形状、图片和简单矢量被编译为 `SceneObject`，不支持项进入显式保真报告。

## HTML-first / AI-first 主模型

UniPPT 与母项目 UniDoc 的核心一致性不在于把演示文稿伪装成 Word blocks，而在于把可编辑语义从 Office ZIP 中独立出来：

- `Deck → Slide → SceneObject` 是权威编辑模型；文本、富文本、表格、图表、公式、媒体、墨迹、动画和换页都有稳定 JSON 字段。
- HTML/SVG renderer 直接消费该模型，编辑器、缩略图、放映页和无损 HTML 尽量复用同一运行时，不维护第二套简化 DOM。
- AI 读取或修改的是小而明确的场景 JSON、关系和内容寻址资源，不需要生成或猜测 OOXML 标签，也不需要解压二进制 PPTX。
- `baseline Deck + OPC snapshot` 是 loss-aware 证据层，不是编辑主模型。未知 OOXML、厂商扩展和未建模内容由它兜底保留。
- 导出 PPTX 是边界操作：比较 current 与 baseline，只 patch 发生变化的原生节点；导入 UDoc 则直接恢复场景和 Arc 资源池，无需重跑 PPTX parser。

因此架构可以实现母项目式的 HTML/AI 友好体验，同时仍保留 PowerPoint 原生兼容。长期重构目标是继续把尚未结构化的 SmartArt、复杂图表、批注/审阅和高级墨迹属性投影进场景模型，而不是移除 OPC 证据层。

## Office UI 分层

界面采用 PowerPoint 的桌面工作流和信息层级：

- 顶部标题栏承载快速访问、文稿标题、搜索、放映和窗口命令。
- 功能区选项卡控制 Ribbon 命令组；“开始”“插入”“设计”等命令按 PowerPoint 的心智模型组织。
- 左侧幻灯片/大纲窗格、中部画布与备注、右侧格式窗格构成编辑区。
- 底部状态栏提供导入状态、视图、缩放和适配画布。
- 动画选项卡提供效果库、预览、动画窗格和计时命令；右侧窗格展示顺序、触发关系、时间条和结构化属性。

DOM 和 CSS 是项目自己的实现，不依赖 Office WebView；场景对象与 Ribbon 命令之间通过统一编辑状态连接。

## 公式双源模型

公式对象包含：

- `latex`：适合浏览器编辑、KaTeX 预览和文本交换。
- `omml`：用于无损保留和原生 PPTX 回写；导出时承载于 PowerPoint 支持的 `a14:m` / `m:oMathPara` 结构。
- `display`：区分行内与展示公式。

转换管线复用母项目的 XSLT 资产：

```text
LaTeX -> latex2mathml -> MML2OMML.XSL -> OMML
OMML -> omml2mml.xsl -> mmltex.xsl -> LaTeX
```

转换失败时不会丢弃原始 OMML；导入仍可完成，并把缺失的 LaTeX 留给后续重试。

## PPTX 回写策略

回写不能简单重建整个包，否则会丢失供应商扩展、未知 XML、动画或嵌入对象。当前 exporter 采用 loss-aware patch：

- 导入时在场景对象中保留 slide part 与 `shape_id` 的来源标识，并在本地服务中以不透明导入 ID 关联原始包。
- 未编辑 part 原样写回；无操作往返时所有 part、content type 和 relationship 逐字节一致。
- 只对发生变化的 shape XML 做定点更新；文本变化仅替换 `p:txBody`，视觉或几何变化才替换 `p:spPr`。
- 新对象通过生成 API 插入，并同步 relationship、content type 和媒体 part。
- 公式优先写回对象中保存的 OMML；修改 LaTeX 后重新生成 OMML，并生成文本 fallback。
- shape 定点更新不触碰 `p:timing` 与 `p:transition`；真实动画文档的编辑回归会比较这两个 XML 块的原始字节。
- 动画字段发生变化时，exporter 为常见效果生成 PowerPoint 原生 `p:timing` 根时间容器、主序列、触发条件和行为节点；重新导入必须得到同样的目标、效果与时序。
- 未编辑的自定义/未知动画继续使用原始 `p:timing`，避免把尚未建模的 Office 扩展降级。显式结构化编辑某页动画后，该页改用规范化时间树。
- 保存后运行包一致性检查、Open XML SDK 渲染和真实文档集回归。

## UDOC 与无损 HTML

UDOC 沿用母项目的 `UDOC3PKG` + `UD3DIR01` 尾目录协议：结构部件独立 Brotli，二进制静态资源各自使用单文件 ZIP Store/Deflate，所有部件带原始/编码尺寸、MIME 与 SHA-256，目录始终 Brotli 压缩。新导出不再增加覆盖完整文档的外层 ZIP，因此仍可从尾目录随机读取单个部件；导入端保留旧 Store/Brotli 部件和旧 `package.udoc3` 外层 ZIP 兼容。UniPPT 包内保存场景文档、baseline、`pptx/package.json` 与内容寻址 OPC blobs，不包含完整 `presentation.pptx`，也不依赖外部 PPTX。新导出的 manifest 与 document 同时声明 `unidoc_type="pptx"`，导入端兼容二者同时使用旧值 `ppt`。

无损 HTML v4 使用 `application/x-unippt+json` 数据岛，直接保存 current Deck、按需保存 baseline Deck，并携带与 UDoc 相同的 OPC package index 和内容寻址 blobs。它不再嵌入 Brotli+Base64 的完整 PPTX；浏览器直接消费 Deck 离线放映，再次导入时 hydrate Arc 资源池与 OPC 快照，无需重建或重解析 PPTX。旧 v1-v3 的完整 PPTX 载荷仅作为兼容导入路径保留。

## 字体身份与首开路径

字体加载沿用 UniDoc 的本机优先协议，但不按“字体名称相同”直接信任本机文件：导入器为可供浏览器使用的字体字节记录 SHA-256、MD5 和精确长度；用户从打开命令进入文件选择器时，浏览器可在同一个用户手势中请求 Local Font Access，只读取文稿实际引用字体的同名候选并在本地计算身份。SHA-256、MD5 与长度全部一致才安装本机字节，否则从当前文档缓存的内容寻址字体 URL 发起同源 GET，并在安装前再次校验。MD5 只承担与既有文档/UniDoc 的兼容身份，安全完整性由 SHA-256 决定。权限被拒绝、浏览器不支持或候选不匹配均是普通网络回退，不会阻断文稿。

`web/font_runtime.js` 在文稿首帧之后加载；字体安装由空闲任务处理，只覆盖当前场景实际用到的 family，并用 epoch 取消过时文稿任务。一个批次完成后只做一次字体度量刷新，避免每个 `FontFace` 到达都触发全画布抖动。顶层入口只授权 `Permissions-Policy: local-fonts=(self)`，不会把本机字体元数据或字节发送给服务端。

PPTX 服务端导入把“场景投影 + 嵌入字体准备”与 OPC 快照构建并行执行。字体 part 先收集后以有界工作线程完成 EOT 解压、去混淆、浏览器字体转换与双哈希计算，默认线程数取可用并行度并限制为 8，可由 `UNIPPT_FONT_WORKERS` 覆盖。完成结果按原始序号稳定排序；场景与快照任一失败都会终止本次导入，不会缓存半成品。

## HTML 沙箱对象

与 `unidoc` 的互操作分成两个边界，不能把“自由 HTML”笼统当作一张截图：

- **可编辑 PPT 编译**：`web/html_to_ppt_runtime.js` 把 HTML 放进仅允许同源读取、但不允许脚本执行的临时 iframe。浏览器完成字体、Grid/Flex、媒体查询和最终几何计算后，转换器按直接文本节点和可见元素边界生成文本、形状、图片、SVG/Canvas 对象；`.omnidoc-page`、`data-page-size` 和常见 slide 容器保留分页语义。线性渐变保存结构化 stop，外链图片在可能时转为 Data URL，整个 deck 通过 `presentation.importFreeHtml` 一次提交。
- **确定性 Skill 路由**：`web/local_skill_runtime.js` 在任何模型请求之前匹配“单个 HTML 附件 + 明确转 PPT 意图”。命中后只申请一次整批授权，完整 HTML 源码在授权后由附件运行时直接注入宿主，模型既不接收源码也不参与坐标推理。这个路径只依赖 UniPPT 本地页面/宿主，不依赖外部模型通道；聊天模式、多 HTML 歧义和用户拒绝均保持非修改状态。
- **U AI 自由整稿路由**：明确的新建/替换整稿意图不再进入固定 `presentation.compose` 网格。宿主先用无工具 completion 让模型生成自包含的多页静态 HTML，源码只存在于本轮闭包；随后检查 2 MiB UTF-8、2—24 页、8,000 节点、互不嵌套的 `.omnidoc-page`、固定 1280×720、首尾角色、逐页标题与 `[Sources]`，并拒绝脚本、事件、跳转和非 data 资源。预检成功后才显示一张不含源码的授权卡，批准后由宿主私有 WeakMap 签发按工具名/次数限权的一次性 capability，再把源码内部注入同一个 `presentation.importFreeHtml` 编译事务；全局 `UniPpt/UniDoc.ai.callTool` 没有票据时只能读取，不能写稿。生成式 iframe 的 CSP 放在 head 首位，编译器在该路径禁用主动网络抓取；完整 HTML 不进入聊天历史、全局事件或工具参数。
- **整稿提交门禁**：生成前捕获目标 Deck 引用与 revision，模型生成、等待确认或浏览器编译期间只要用户切换/编辑文稿就拒绝覆盖；异步转换在最终 commit 还会再次核对原 Deck 对象身份，避免新旧文稿都为 revision 0 时被陈旧任务串写。编译页数必须与预检一致，`unsupported` 或 high fidelity risk、低于 80 分的整稿质量、无真实来源的指标页、结构错误都会在 commit 前失败；首次失败的结构化 findings 会在同一次用户授权中驱动一次完整 HTML 自动修订并复检，第二次仍不合格才终止。成功替换只提交一次，并保存 Deck、文档缓存和选择状态的撤销快照；替换过程异常会回滚，Ctrl+Z/Redo 会重新安装对应稿件字体。
- **有界局部保真**：径向渐变背景、Canvas 可表达的 CSS filter/clip-path 和可读取的静态同源 iframe 由同一浏览器按最多 8M 像素绘制为局部 PNG；滤镜阴影会扩张自己的捕获边界，但不会覆盖已成功提取的页外文字/形状。全页背景进入 `Slide.backgroundAsset`，PPTX exporter 写成原生 `<p:bg><a:blipFill>`，不会成为可拖拽的全页对象。报告用 `fallbacks`/`localizedFallbacks` 与真正失败的 `unsupported` 分流。
- **CSS 合成语义**：浏览器 computed style 中的八位 Hex、RGBA 和 `color(... / alpha)` 会把颜色与透明度共同带入场景；零宽轮廓不写 PowerPoint hairline，半透明轮廓在页面底色上预合成。栅格化渐变时先绘制 computed `background-color` 再叠加图像层，使透明 stop 在 PPT 背景图片里仍保持原页面底色，而不是透出 Office 白色画布。
- **动态 HTML 保真**：脚本、WebComponent 隔离文档、无 poster 视频，以及依赖页面上下文的 `backdrop-filter`、混合模式、mask 和 3D 不会进入编辑器宿主页执行，也不会被伪报为已保真。带脚本原版放映仍需独立来源域的隔离 HTML 对象。

这个边界使 UniDoc 可以继续负责自由 HTML 的内容生成与浏览器视觉上限，UniPPT 则负责受约束的 `HTML → Deck → PPTX` 编译、对象编辑、质量审计和原生导出。带脚本的原版 HTML 放映仍应走隔离的动态 HTML 对象/无损 HTML 通道，不能把脚本执行权混入普通 `SceneObject`。

# UniPPT 本机 MCP 协议

## 传输与能力

入口是 `tools/unippt_mcp.cjs`，使用 stdio JSON-RPC。客户端调用 `tools/list` 获取完整工具 Schema；编辑器通过本机桥接与当前文稿交互。静态快照见 [工具目录](/mcp-tools.json) 和 [原生结构](/mcp-runtime-schemas.json)。运行时 Schema 优先。

保留对象查询、模块编排、图片重建、动画、原子编辑、预览、验证、撤销和导出。没有账户文件管理或远程 HTTP MCP 入口。

## 会话与版本

`sessionId` 来自已连接会话，`slideId` 和对象 ID 来自读取结果。不要猜测 ID。`expectedRevision` 必须是最新桥接版本；写操作使用稳定 `requestId`。以下示例中的会话和版本必须替换为读取到的值。

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"unippt_list_sessions","arguments":{}}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"unippt_prepare","arguments":{"sessionId":"session-from-list"}}}
```

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"unippt_get_animation_schema","arguments":{"sessionId":"session-from-list"}}}
```

```json
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"unippt_validate","arguments":{"sessionId":"session-from-list","expectedRevision":0}}}
```

```json
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"unippt_export","arguments":{"sessionId":"session-from-list","expectedRevision":0,"format":"pptx"}}}
```

## 编辑与图片

`apply_patch` 支持 14 类原生操作，单次至多 100 操作。`apply_scene` 接收最多 64 模块，编译后最多 512 操作。修改事务通过当前编辑器撤销机制提交。

图片路径只用于用户显式指定的本机输入。低层修改不接受任意脚本、外部 URL 或文件路径；插入图片可使用受限 PNG 数据（最大 750000 base64 字符、单边 4096 像素、总计 6 百万像素，并受单批 1 MB 限制）。

`begin_image` 返回源像素行协议；`apply_image_layout` 支持整行和稀疏 `patches` 修复。实际行类型、动画参数和原生操作以当前 Schema 为准。

## 预览、导出与错误处理

预览和导出使用本机渲染/转换工具。支持 PPTX、UDOC、无损 HTML、PDF、图片 ZIP 和视频；渲染、公式和视频依赖见 README。

MCP 导出结果上限 20 MB，命令期限 60 秒。较长视频用编辑器导出。导出产生新文件，不覆盖输入。

写入超时先查询 `command_status`。`writeCommitted:true` 表示文稿已修改，即使后续预览失败，也不能重复写入。视觉验证需比较实际原生预览与目标；结构通过和计时通过都不能代替视觉结论。

## 本机连接边界

桥接仅允许本机编辑器来源，使用本机客户端令牌和浏览器会话凭据分隔读写路径。切换文稿或关闭 MCP 会撤销旧连接。MCP 调用方使用自己的模型，工具不调用付费模型代理。

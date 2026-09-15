# 本机 AI 与 MCP 接入

## 两条独立路径

调用方使用自己的模型，通过 MCP 读取结构、提交原生对象修改和查看预览。MCP 不调用 U AI 模型，也不传递 U AI 密钥。

U AI 是编辑器内的助手，可在设置或本机 `.env.local` 配置 `UNIPPT_AI_BASE`、`UNIPPT_AI_MODEL`、`UNIPPT_AI_KEY`。坐标 OCR 的 `UNIPPT_OCR_MODEL` 需要兼容原生 DashScope 协议。使用外部模型时，调用方应按该模型配置理解数据处理范围。

## 配置 stdio

先启动本机 UniPPT（默认 8141 端口），安装 Node.js 22 或更新版。将以下配置中的绝对路径替换为实际安装路径，加入支持 stdio 的 MCP 客户端后重连：

```json
{"mcpServers":{"unippt":{"command":"node","args":["/absolute/path/to/UniPPT/tools/unippt_mcp.cjs"]}}}
```

桥接进程默认监听回环地址的 8142 端口。本机编辑器的设置菜单控制连接。无需账号、在线令牌或远程服务。

## 图像工作流

`list_sessions → begin_image → apply_image_layout → 比较原图与原生预览 → 局部修正 → finish_image`。

`begin_image` 只接受用户指定的本机图片绝对路径。返回原图与像素坐标行协议；模型自行识图。`apply_image_layout` 确定性编译原生对象并检查交付文件。检查返回的原生 PNG 后，才用 `finish_image` 记录视觉结论及未解决问题。

现有文稿可用 `prepare → apply_scene` 批量创建模块，或使用 `inspect → get_edit_schema → apply_patch` 精细修改。Skill 见仓库 `skills/unippt-editable-slides/SKILL.md`。

## 并发与恢复

多个客户端可读取各自获准的本机浏览器会话。同一文稿的写入须按最新 `expectedRevision` 串行提交；版本冲突先读取，再决定修改。写入使用稳定 `requestId`，超时后先 `command_status`，不能盲目重复写入。

预览失败可能发生在写入成功之后。检查 `writeCommitted` 和返回版本，再重试预览。结构、时间和视觉质量分别报告。

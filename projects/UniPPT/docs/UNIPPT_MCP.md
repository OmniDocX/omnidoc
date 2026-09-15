# UniPPT 本机 MCP

启动本机编辑器后，在支持 stdio 的客户端配置 `node` 加仓库中 `tools/unippt_mcp.cjs` 的绝对路径。默认编辑器端口为 8141，本机桥接端口为 8142。

右上角设置菜单可以关闭/恢复 MCP 自动连接。先调用 `unippt_list_sessions`，使用返回的会话、页面 ID 和版本操作当前文稿。

- [接入与模型配置](AI_MCP_INTEGRATION.md)
- [协议与调用示例](MCP_PROTOCOL.md)
- [图片与原生对象工作流](../skills/unippt-editable-slides/SKILL.md)

所有文件编辑、预览和导出在本机执行。MCP 不提供云存储、在线账户或多人协作服务。

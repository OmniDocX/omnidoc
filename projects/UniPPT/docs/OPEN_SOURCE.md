# 公开本机版开发说明

本仓库发布 UniPPT 本地单机版的独立源码快照。保留编辑器、Rust 文稿引擎、原生格式往返、动画放映、导入导出、自定义 U AI 和本机 MCP。

本版不包含 R2、云存储、账户网关、云文稿、分享链接、共享编辑、多人协作或在线 OAuth MCP。相关实现及集成文档不进入公开源码或 Git 历史。

## 目录

| 路径 | 内容 |
| --- | --- |
| `crates/unippt-core` | 原生文稿与格式转换 |
| `crates/unippt-server` | 回环地址 HTTP 服务、本机缓存和模型配置 |
| `web` | 浏览器编辑器、放映、本机 AI 和 MCP 桥接 |
| `tools` | 本机工具、stdio MCP、文档生成与验证 |
| `skills` | 原生幻灯片工作流 |
| `vendor` | 格式引擎、第三方组件与许可 |

## 验证

```sh
cargo test --workspace --locked
node --test web/*.test.js tools/*.test.cjs
python tools/test_public_boundary.py -v
python tools/check_public_boundary.py --history
```

测试所需公式、图像与音视频依赖见 README 与 `tools/math/requirements.txt`。修改工具契约后运行 `node tools/build_mcp_docs.cjs` 更新所有本机文档快照。

公开发布前运行上述验证和源码边界检查。不得向公开分支合并私有服务分支或历史。许可和第三方声明见仓库 LICENSE 与 `docs/LICENSING.md`。

# Contributing to UniPPT

感谢参与 UniPPT。项目目标是提供可审计、可编辑、可离线运行的演示文稿工具，并让 AI 通过受限 MCP/Skills 安全地操作原生对象。

## 开始之前

请先阅读 [`README.md`](README.md)、[`docs/OPEN_SOURCE.md`](docs/OPEN_SOURCE.md) 和 [`SECURITY.md`](SECURITY.md)。讨论功能时请明确它属于核心对象模型、网页编辑器、本机 MCP、Skill 还是使用文档。不要在 issue、日志或补丁中粘贴用户文稿、令牌、Cookie、API Key、本机凭据或主机信息。

## 提交改动

- 保持 Rust 核心、网关和网页层的职责边界；优先复用现有 schema 与能力查询，不为单一模型写硬编码分支。
- 新增 MCP 工具时，更新运行时 schema、`docs/MCP_PROTOCOL.md`、本机连接说明、Skill 和回归测试；写操作必须带 `sessionId`、`expectedRevision` 与幂等 `requestId`。
- 新增模型配置时，只接受 OpenAI-compatible 的 provider/model/key 分离配置；密钥只能来自本地忽略文件或受控部署变量，不能进入 Skill、MCP 参数、截图或提交。
- 修改 PPTX、公式、动画、视频或浮窗交互时，提供最小可复现样例和渲染/导出检查说明。
- 提交信息使用清晰的动词和范围，例如 `mcp: validate MCP revision before patch`。

## 必跑检查

```powershell
cargo test --workspace --locked
node --check web/app.js
node --test web/*.test.js
git diff --check
python tools/check_public_boundary.py
python tools/test_public_boundary.py -v
```

环境安装见 README（Rust 1.88+、Python 公式依赖、Node.js 22、FFmpeg/ffprobe）。公开边界扫描针对 Git 索引；暂存改动后再执行，可检测工作区已覆盖但仍留在暂存区的凭据。

Rust 改动建议再运行 `cargo fmt --all -- --check` 与
`cargo clippy --workspace --all-targets -- -D warnings`，并在 PR 中记录结果。

若某项依赖私有文稿、浏览器登录或外部服务，请在提交说明中写明复现条件；不要为了让 CI 通过而提交凭据或跳过安全检查。

## Pull request 内容

PR 描述应包含：改动目的、涉及的包/文档、兼容性影响、测试命令及结果、是否改变 本机 MCP 连接或文件访问边界，以及必要的截图或渲染产物。涉及公共协议的改动需要同时更新中英文 README 或链接文档。

维护者会检查对象可编辑性、版本冲突处理、未授权访问、公开边界和导出兼容性。CI 通过不代表所有文稿的视觉兼容性已完成验收。

## 许可证

自有源码采用 [`LICENSE`](LICENSE) 的 PolyForm Noncommercial 1.0.0；第三方依赖继续遵循各自协议。提交者应确认有权提供贡献。需要将贡献纳入商业授权时，维护者须另行取得明确的商业再许可授权，不以提交 PR 或贡献代码推定版权转让。旧版本既有许可不追溯撤销。

# OmniDoc · 国产应用项目源码合集

**这是纯国产 app project。** OmniDoc 汇集 OmniDoc 旗下自主开发应用的公开源码版本，方便了解项目、下载源码与本机体验。

## 主站与旗下项目

**官方网站：[omnidoc.top](https://omnidoc.top)** · [English](README.en.md) · [另一合集入口](https://github.com/OmniDocX/omnidocx)

| 项目 | 功能 | 本仓库源码副本 | 独立仓库 |
| --- | --- | --- | --- |
| UniPPT | 本机演示文稿编辑、PPTX 导入导出、可选 AI 与 MCP | [projects/UniPPT](projects/UniPPT) | [OmniDocX/UniPPT](https://github.com/OmniDocX/UniPPT) |
| UniCell | 本机电子表格、公式、格式与 XLSX/CSV/UDOC/HTML 转换 | [projects/unicell](projects/unicell) | [OmniDocX/unicell](https://github.com/OmniDocX/unicell) |
| vecmeta | SVG / EMF 等矢量格式转换组件 | [projects/vecmeta](projects/vecmeta) | [OmniDocX/vecmeta](https://github.com/OmniDocX/vecmeta) |

这是包含真实源码文件的快照合集，可直接克隆或下载 ZIP。每个项目的 README、源码、构建文件和许可证都保留在对应子目录。项目使用的第三方组件保持独立署名和许可证；国产应用项目定位不表示所有依赖都为国产。

## 快速使用

```sh
git clone https://github.com/OmniDocX/omnidoc.git
cd omnidoc
```

进入对应 `projects/` 子目录，按照该项目 README 安装依赖和运行。本仓库不提供一个统一应用进程：例如 UniCell 可运行：

```sh
cd projects/unicell
cargo run --locked --manifest-path server/Cargo.toml
```

UniPPT 的公开副本使用本机基础版；UniCell 副本已移除 R2、云存储、共享编辑与托管账号。合集**不收录 PolyglotPDF**。

## 版本与同步

本次快照日期：2026-09-16。完整来源提交与逐文件 SHA-256 记录在 [sources.lock.json](sources.lock.json)。副本不是子模块，不会自动随独立仓库更新；需要更新时应重新取得公开版本，并一并更新清单。

在仓库根目录执行 `python tools/verify_copies.py` 可确认源码副本与记录的快照一致。合集 CI 检查副本完整性与发布边界，各应用的构建与功能测试见其独立仓库。

## 许可与联系

自有合集文档采用 [OmniDoc 非商业源码许可 1.0](LICENSE)。各子项目以其目录内 LICENSE 及第三方声明为准；复制不会改变已有授权。符合协议的非商业使用免费，商业使用需事先书面许可。这是源码可见模式，不是 OSI 批准的开源许可。

商业授权：**cc@omnidoc.top** · 微信 **13184071590**。授权范围以书面确认为准，通常 48 小时内答复。

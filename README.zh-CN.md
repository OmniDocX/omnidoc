# OmniDoc — 源码合集

[English](README.md) | **简体中文**

[OmniDoc](https://omnidoc.top/) · [GitHub](https://github.com/OmniDocX)

**OmniDoc 国产办公项目的固定版本源码合集。**

OmniDoc 收录 UniPPT、UniCell 与 vecmeta 的可构建源码副本，为本机演示文稿、电子表格和矢量转换工作流提供统一入口。项目由 OmniDoc 自主研发，第三方组件保留其来源与独立许可。

## 项目定位

以 **Microsoft 365（Office 365）和 WPS Office** 为同类产品对标，致力于建设功能最完善、工程资料最完整的国产开放源码办公项目。该表述是发展目标；当前功能范围与证据见 [产品对照](docs/COMPARISON.zh-CN.md)。自有代码使用非商业源码许可，具体条件见下文。

## 性能测评

<!-- BENCHMARK:START -->
测试环境：Windows 10 10.0.19045，Intel Core i7-1165G7（4 核 8 线程），31.70 GiB 内存，Python 3.13.11；组件为 Rust 1.97.1 release 构建。各项预热 2 次、测量 7 次，串行执行。P95 采用最近秩法，在 7 个样本下等于最大值。全部正式观测均保留。

| 项目 | 操作 | 规模 | 中位数 ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX 导出 | 100 页 | 139.41 | 316.91 |
| UniPPT | PPTX 导入，缓存未命中 | 100 页 | 813.36 | 1721.34 |
| UniCell | CSV 导入与计算 | 10,000 行 | 660.92 | 707.79 |
| UniCell | XLSX 导出 | 10,000 行 | 562.30 | 618.65 |
| UniCell | 批量编辑与重算 | 10,000 行 | 120.68 | 135.50 |
| vecmeta | SVG → EMF，CLI | 10,000 图元 | 105.80 | 118.47 |
| vecmeta | EMF → SVG，CLI | 10,000 图元 | 57.10 | 95.53 |

UniCell 已收录 SUM 优化：公开本机版相同 1 万行 CSV 导入并计算，从 20.217 秒降至 0.661 秒，约快 31 倍；新版每轮全部 2 万公式及结果均核验正确。此前完整版优化实验的 0.368 秒（约 55 倍）和旧基线均保留在 [UniCell 报告](projects/unicell/benchmarks/README.zh-CN.md)，两次优化后测量使用不同构建，不能混为一次测试。

[完整报告与源码完整性校验](benchmarks/README.zh-CN.md)

共享工作站的后台负载未完全受控，结果仅适用于所列合成工作负载。HTTP 测评不含浏览器渲染；vecmeta 包含 CLI 启动和文件读写。本轮未实测 Office 365、Excel 或 WPS 的性能，不发布竞品速度排名。
<!-- BENCHMARK:END -->

## 收录项目

| 项目 | 用途 | 源码与文档 | 性能报告 |
| --- | --- | --- | --- |
| UniPPT | 本机演示文稿编辑与 PPTX 转换 | [中文说明](projects/UniPPT/README.zh-CN.md) · [English](projects/UniPPT/README.md) | [测评](projects/UniPPT/benchmarks/README.zh-CN.md) |
| UniCell | 本机表格编辑、计算与格式转换 | [中文说明](projects/unicell/README.zh-CN.md) · [English](projects/unicell/README.md) | [测评](projects/unicell/benchmarks/README.zh-CN.md) |
| vecmeta | SVG/EMF 转换库与命令行工具 | [中文说明](projects/vecmeta/README.zh-CN.md) · [English](projects/vecmeta/README.md) | [测评](projects/vecmeta/benchmarks/README.zh-CN.md) |

PolyglotPDF 不在收录范围。`projects/` 中为普通源码文件；各组件独立构建，版本更新时应同步提交来源清单。

## 快速开始

```sh
git clone https://github.com/OmniDocX/omnidoc.git
cd omnidoc
python tools/verify_copies.py
cd projects/UniPPT
cargo run --release --locked -p unippt-server
```

访问 http://127.0.0.1:8141。UniCell 的启动命令及可选运行环境见其独立文档；本合集没有统一的套件服务进程。

## 来源与完整性

[sources.lock.json](sources.lock.json) 固定公开来源仓库、提交版本和每个文件的 SHA-256。`python tools/verify_copies.py` 比对已跟踪文件集合与内容哈希。保留各项目许可证、第三方声明和测评原始记录。

基础应用使用本机文件和回环地址服务，公开版本不包含 R2、云存储、共享编辑、分享链接或统一账户服务。

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

自有代码及文档采用未经修改的标准 [PolyForm Noncommercial 1.0.0](LICENSE)。符合该许可允许用途的使用免费。超出其允许范围的商业用途须另行申请付费商业授权：使用前请联系我们，协商费用并取得书面许可。标准许可对特定机构的允许条款完整保留。本许可属于源码可见许可，不是 OSI 批准的开源许可。第三方条款及旧版本已合法授予的权利保持不变。

[许可范围与允许用途](docs/LICENSING.md) · [商业授权与申请流程](docs/COMMERCIAL_LICENSE.md)。

商业联系：[cc@omnidoc.top](mailto:cc@omnidoc.top) · 微信：**13184071590**。完整申请材料收到后 48 小时内答复；提交申请或未获回复均不构成授权。

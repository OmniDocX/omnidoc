# OmniDoc — 源码合集

[English](README.md) | **简体中文**

[OmniDoc](https://omnidoc.top/) · [GitHub](https://github.com/OmniDocX)

**OmniDoc 国产办公项目的固定版本源码合集。**

OmniDoc 收录 UniPPT、UniCell 与 vecmeta 的可构建源码副本，为本机演示文稿、电子表格和矢量转换工作流提供统一入口。项目由 OmniDoc 自主研发，第三方组件保留其来源与独立许可。

## 项目定位

以 **Microsoft 365（Office 365）和 WPS Office** 为同类产品对标，致力于建设功能最完善、工程资料最完整的国产开放源码办公项目。该表述是发展目标；当前功能范围与证据见 [产品对照](docs/COMPARISON.zh-CN.md)。自有代码使用非商业源码许可，具体条件见下文。

## 性能测评

<!-- BENCHMARK:START -->
测量环境：Windows 10 10.0.19045，Intel Core i7-1165G7（4 核 8 线程），31.70 GiB 内存，Python 3.13.11。组件为 Rust 1.97.1 release 构建。每项预热 2 次、测量 7 次，串行运行；P95 采用最近秩法，在 7 个样本下等于最大值。原始数据包含全部样本，不剔除慢样本。

| 项目 | 操作 | 规模 | 中位数 ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX 导出 | 100 页 | 139.41 | 316.91 |
| UniPPT | PPTX 导入，缓存未命中 | 100 页 | 813.36 | 1721.34 |
| UniCell | CSV 导入与计算 | 10,000 行 | 20217.27 | 28130.84 |
| UniCell | XLSX 导出 | 10,000 行 | 452.73 | 557.95 |
| UniCell | 批量编辑与重算 | 10,000 行 | 19081.38 | 23292.38 |
| vecmeta | SVG → EMF，CLI | 10,000 图元 | 105.80 | 118.47 |
| vecmeta | EMF → SVG，CLI | 10,000 图元 | 57.10 | 95.53 |

[完整报告与源码校验测评](benchmarks/README.zh-CN.md)

结果来自共享工作站，后台负载未完全受控；不同规模可能出现非单调耗时。HTTP 项不含浏览器渲染，vecmeta 项包含 CLI 启动与文件读写。大表 CSV 导入是当前已观测到的瓶颈；不能以 XLSX 导出耗时代替导入或重算性能。本轮未实测 Office 365 与 WPS 的速度，不发布竞品速度比值。
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

自有代码及文档采用 [OmniDoc 非商业源码许可 1.0](LICENSE)。符合条款的非商业使用免费；商业使用，包括中国及其他地区企业的内部业务使用，须事先取得书面授权。本许可属于源码可见许可，不是 OSI 批准的开源许可。第三方组件保持各自条款，旧版本已合法授予的权利不受追溯影响。

商业联系：[cc@omnidoc.top](mailto:cc@omnidoc.top) · 微信：**13184071590**。完整申请材料收到后 48 小时内答复；提交申请或未获回复均不构成授权。

# OmniDoc — 性能测评

[English](README.md) | **简体中文**

本合集提供三个组件的独立实测，以及本仓库源码完整性校验的实测。合集没有独立办公运行时，源码校验速度不能代表文档编辑或计算速度。

测试环境：Windows 10 10.0.19045，Intel Core i7-1165G7（4 核 8 线程），31.70 GiB 内存，Python 3.13.11；组件为 Rust 1.97.1 release 构建。各项预热 2 次、测量 7 次，串行执行。P95 采用最近秩法，在 7 个样本下等于最大值。全部正式观测均保留。

## 组件测评

| 项目 | 操作 | 规模 | 中位数 ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX 导出 | 100 页 | 139.41 | 316.91 |
| UniPPT | PPTX 导入，缓存未命中 | 100 页 | 813.36 | 1721.34 |
| UniCell | CSV 导入与计算 | 10,000 行 | 660.92 | 707.79 |
| UniCell | XLSX 导出 | 10,000 行 | 562.30 | 618.65 |
| UniCell | 批量编辑与重算 | 10,000 行 | 120.68 | 135.50 |
| vecmeta | SVG → EMF，CLI | 10,000 图元 | 105.80 | 118.47 |
| vecmeta | EMF → SVG，CLI | 10,000 图元 | 57.10 | 95.53 |

UniCell 已收录 SUM 优化：公开本机版相同 1 万行 CSV 导入并计算，从 20.217 秒降至 0.661 秒，约快 31 倍；新版每轮全部 2 万公式及结果均核验正确。此前完整版优化实验的 0.368 秒（约 55 倍）和旧基线均保留在 [UniCell 报告](../projects/unicell/benchmarks/README.zh-CN.md)，两次优化后测量使用不同构建，不能混为一次测试。

共享工作站的后台负载未完全受控，结果仅适用于所列合成工作负载。HTTP 测评不含浏览器渲染；vecmeta 包含 CLI 启动和文件读写。本轮未实测 Office 365、Excel 或 WPS 的性能，不发布竞品速度排名。

| 项目 | 收录提交 | 运行时构建提交 | 报告与数据 |
| --- | --- | --- | --- |
| UniPPT | `14521e76459ac6236c484a0092e84da6e7b71f8b` | `e66c1e0eab10a93afd776a755e0973149dedcd73` | [Report](../projects/UniPPT/benchmarks/README.zh-CN.md) · [JSON](../projects/UniPPT/benchmarks/results/2026-09-16-windows-x64.json) |
| unicell | `9a45c5f94da22cc78cd9654258c334ebef8ff15c` | `dc6f6c6a5c5de532e93b3adbf6a00336562a1a9f` | [Report](../projects/unicell/benchmarks/README.zh-CN.md) · [JSON](../projects/unicell/benchmarks/results/2026-09-16-sum-optimized-windows-x64.json) |
| vecmeta | `5e186640fc943720635c2bc69e068078bc6d42a7` | `96acc4608d644d8137fd334412e82bb2ba7c5a0a` | [Report](../projects/vecmeta/benchmarks/README.zh-CN.md) · [JSON](../projects/vecmeta/benchmarks/results/2026-09-16-windows-x64.json) |

## 当前源码完整性校验

当前源码集合校验包含 Python/Git 启动、暂存区文件集合检查、全部文件读取和 SHA-256 校验。操作系统缓存已预热，不包含网络和应用运行。两个合集逐个测量。

1,333 files · 18,518,350 bytes · manifest SHA-256: `892fdc03696104800d2c2a3227c3e81598fd0596d174ca9b02531a2033d3e44d`

| 操作 | 规模（文件） | 中位数 ms | P95 ms |
| --- | ---: | ---: | ---: |
| 源码完整性校验 | 1,333 | 375.54 | 408.44 |

[JSON](results/2026-09-16-polyform-snapshot-windows-x64.json)

## 保留的旧快照结果

以下为上一版源码集合的测量，文件数及哈希对应旧快照，不代表当前集合，也不适合仅按时延变化推导优化收益。

| 操作 | 规模（文件） | 中位数 ms | P95 ms |
| --- | ---: | ---: | ---: |
| 源码完整性校验 | 1,321 | 1557.28 | 3363.75 |

[JSON](results/2026-09-16-windows-x64.json)

## 复现

```sh
python tools/verify_copies.py
python benchmarks/run.py --warmups 2 --samples 7 --output benchmarks/results/local.json
python benchmarks/verify_results.py
```
组件测评应在各自 `projects/` 子目录运行，具体编译配置与命令见对应报告。

# OmniDoc — 性能测评

[English](README.md) | **简体中文**

本合集的性能报告包含三个组件的独立实测，以及本仓库源码完整性校验的实测。本合集没有独立办公运行时，源码校验速度不代表编辑或计算速度。

## 环境与统计方法

测量环境：Windows 10 10.0.19045，Intel Core i7-1165G7（4 核 8 线程），31.70 GiB 内存，Python 3.13.11。组件为 Rust 1.97.1 release 构建。每项预热 2 次、测量 7 次，串行运行；P95 采用最近秩法，在 7 个样本下等于最大值。原始数据包含全部样本，不剔除慢样本。

## 组件结果摘要

| 项目 | 操作 | 规模 | 中位数 ms | P95 ms |
| --- | --- | ---: | ---: | ---: |
| UniPPT | PPTX 导出 | 100 页 | 139.41 | 316.91 |
| UniPPT | PPTX 导入，缓存未命中 | 100 页 | 813.36 | 1721.34 |
| UniCell | CSV 导入与计算 | 10,000 行 | 20217.27 | 28130.84 |
| UniCell | XLSX 导出 | 10,000 行 | 452.73 | 557.95 |
| UniCell | 批量编辑与重算 | 10,000 行 | 19081.38 | 23292.38 |
| vecmeta | SVG → EMF，CLI | 10,000 图元 | 105.80 | 118.47 |
| vecmeta | EMF → SVG，CLI | 10,000 图元 | 57.10 | 95.53 |

结果来自共享工作站，后台负载未完全受控；不同规模可能出现非单调耗时。HTTP 项不含浏览器渲染，vecmeta 项包含 CLI 启动与文件读写。大表 CSV 导入是当前已观测到的瓶颈；不能以 XLSX 导出耗时代替导入或重算性能。本轮未实测 Office 365 与 WPS 的速度，不发布竞品速度比值。

| 项目 | 收录提交 | 构建所用提交 | 测评与原始记录 |
| --- | --- | --- | --- |
| UniPPT | `6ff38470e22de19dd822f4ff616e06cbb13148b5` | `e66c1e0eab10a93afd776a755e0973149dedcd73` | [Report](../projects/UniPPT/benchmarks/README.zh-CN.md) · [JSON](../projects/UniPPT/benchmarks/results/2026-09-16-windows-x64.json) |
| unicell | `44bfc75e3c49eda4ac408f50c711ce70b0e34e7a` | `51e007388d702f23290bb44df7270d164daa0e2d` | [Report](../projects/unicell/benchmarks/README.zh-CN.md) · [JSON](../projects/unicell/benchmarks/results/2026-09-16-windows-x64.json) |
| vecmeta | `2d2ce09aad1778b52a5af4bbc1902d166c286d08` | `96acc4608d644d8137fd334412e82bb2ba7c5a0a` | [Report](../projects/vecmeta/benchmarks/README.zh-CN.md) · [JSON](../projects/vecmeta/benchmarks/results/2026-09-16-windows-x64.json) |

## 合集完整性校验

计时包含 Python 与 Git 进程启动、暂存区文件集合检查、全部源码文件读取和 SHA-256 校验。操作系统文件缓存已预热；不包含网络或应用启动。预检查曾出现两个测评进程重叠，已排除该次试跑，公开数据来自随后逐个执行的复测。

1,321 files · 18,463,820 bytes · SHA-256 manifest: `f12679d3c36f3e9da0f3104c210fd0b307316869ef0edc3ed76f4edc2b88fb79`

| 操作 | 规模（文件） | 中位数 ms | P95 ms |
| --- | ---: | ---: | ---: |
| 源码完整性校验 | 1,321 | 1557.28 | 3363.75 |

[原始数据](results/2026-09-16-windows-x64.json). 该报告用来源清单哈希及组件提交标识实际测试的源码集合；环境字段中的合集提交为执行时的基线提交。

## 复现

```sh
python tools/verify_copies.py
python benchmarks/run.py --warmups 2 --samples 7 --output benchmarks/results/local.json
python benchmarks/verify_results.py
```

组件测评应在对应 `projects/` 子目录运行，具体命令与构建配置见各自报告。

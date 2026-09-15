# vecmeta — 性能测评

[English](README.md) | **简体中文**

2026-09-16 在 Windows 10 / Intel Core i7-1165G7 / 31.70 GiB 内存上实测。每项预热 2 次、测量 7 次，顺序执行。

## 测试环境

| 字段 | 记录 |
| --- | --- |
| OS | Microsoft Windows 10 专业版 (10.0.19045, AMD64) |
| CPU | 11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz (4 cores / 8 threads) |
| RAM | 31.70 GiB |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Python | 3.13.11 |
| 构建源提交 | `96acc4608d644d8137fd334412e82bb2ba7c5a0a` |
| 构建配置 | `cargo build --release --locked -p emfsvg-cli; default opt-level=3, lto=true` |
| 程序大小 | 1,203,712 bytes |
| 测量时间 UTC | 2026-09-15T23:03:36.508770+00:00 |

## 工作负载与计时边界

交替使用实色矩形与三角形路径，位置和颜色由确定性规则生成。不包含文本、字体查找、渐变、图片或原文封装。使用默认缩放参数，不启用 `--lossless`。检查 EMF 签名与声明长度、SVG 路径数量，以及容差为 1e-6 的 CLI 几何固定点测试。几何检查不等于像素级渲染验证。

计时包含完整 CLI 进程启动、文件读写、转换和退出；结果检查在计时区间外。操作系统文件缓存经过预热。

## 完整结果

| 操作 | 规模（图元） | 中位数 ms | P95 ms |
| --- | ---: | ---: | ---: |
| SVG → EMF，CLI | 100 | 25.10 | 25.85 |
| EMF → SVG，CLI | 100 | 22.36 | 24.29 |
| 几何往返检查，CLI | 100 | 23.83 | 25.95 |
| SVG → EMF，CLI | 1,000 | 26.62 | 46.84 |
| EMF → SVG，CLI | 1,000 | 25.75 | 26.39 |
| 几何往返检查，CLI | 1,000 | 30.40 | 47.85 |
| SVG → EMF，CLI | 10,000 | 105.80 | 118.47 |
| EMF → SVG，CLI | 10,000 | 57.10 | 95.53 |
| 几何往返检查，CLI | 10,000 | 206.04 | 224.29 |

[原始数据](results/2026-09-16-windows-x64.json) · [测评脚本](run.py)

## 统计方法与限制

预热样本不计入统计。中位数为第 4 个排序样本；P95 使用最近秩法 `ceil(0.95 × n)`，在 n=7 时等于本次最大值，不能视为稳定尾延迟估计。原始 JSON 保留全部观测值、输入/输出大小或哈希、校验结果、二进制与脚本 SHA-256。此处的源提交是构建时运行时代码的版本；本次新增文档与脚本未修改运行时代码。

测试在共享工作站上进行，未控制所有后台负载、功耗或文件缓存状态；没有剔除慢样本，也没有执行并发压力测试。结果仅适用于这些合成工作负载；未测峰值内存、完整应用启动时间、浏览器帧率、真实复杂文档、外部 AI 和云服务。Microsoft 365（Office 365）与 WPS Office 本轮未实测，不能从这些数据推导相对速度排名。

## 复现

```sh
cargo build --release --locked -p emfsvg-cli
python benchmarks/run.py --binary target/release/emfsvg --sizes 100,1000,10000 --warmups 2 --samples 7 --output benchmarks/results/local.json
python benchmarks/verify_results.py
```

Windows 下为程序路径追加 `.exe`；若使用 `--target-dir`，应相应修改 `--binary`。脚本只使用 Python 标准库，输入完全由脚本生成；服务型测评会启动和关闭自己的临时回环服务进程。使用相同源码、锁文件、编译器与构建配置复现；时间因机器与负载而异。

[Office 365 / WPS 对照与后续同机测试协议](../docs/COMPARISON.zh-CN.md)

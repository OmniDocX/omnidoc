# vecmeta

[English](README.md) | **简体中文**

[OmniDoc](https://omnidoc.top/) · [GitHub](https://github.com/OmniDocX)

**采用原生 Rust 实现的 SVG ↔ EMF 转换库与命令行工具。**

vecmeta 是 OmniDoc 自主研发的国产矢量转换组件，通过统一场景表示衔接 SVG 解析、EMF 记录回放与矢量序列化。转换过程无需安装 Microsoft Office、LibreOffice 或 C 语言转换库。

## 项目定位

以 **Microsoft 365（Office 365）和 WPS Office** 为同类产品对标，致力于建设功能最完善、工程资料最完整的国产开放源码办公项目。该表述是发展目标；当前功能范围与证据见 [产品对照](docs/COMPARISON.zh-CN.md)。自有代码使用非商业源码许可，具体条件见下文。

## 性能测评

<!-- BENCHMARK:START -->
2026-09-16 在 Windows 10 / Intel Core i7-1165G7 / 31.70 GiB 内存上实测。每项预热 2 次、测量 7 次，顺序执行。

| 操作 | 规模（图元） | 中位数 ms | P95 ms |
| --- | ---: | ---: | ---: |
| SVG → EMF，CLI | 10,000 | 105.80 | 118.47 |
| EMF → SVG，CLI | 10,000 | 57.10 | 95.53 |
| 几何往返检查，CLI | 10,000 | 206.04 | 224.29 |

[全部规模、复现方法与限制](benchmarks/README.zh-CN.md) · [原始数据](benchmarks/results/2026-09-16-windows-x64.json)

以下耗时不包含浏览器渲染；Office 365 和 WPS 本轮未进行速度对测。
<!-- BENCHMARK:END -->

## 功能范围

| 领域 | 实现 |
| --- | --- |
| EMF | 二进制记录、绘图状态、变换、画笔、画刷、路径与图元 |
| SVG | 支持的形状、路径、样式、继承、CSS 优先级及引用 |
| 文本 | 通过 fontdb/ttf-parser 查找字体并生成字形轮廓 |
| 绘制属性 | 支持的填充、描边、线性渐变及嵌入 EMF+ 绘制信息 |
| 原文恢复 | 可选 `--lossless` 封装原始数据，用于反向精确恢复 |
| 诊断信息 | 报告跳过的内容、兼容性警告与嵌入原文恢复状态 |

## 快速开始

安装 Rust 与 Cargo；CI 使用 Rust 1.88。文本转轮廓还需要安装对应字体。

```sh
git clone https://github.com/OmniDocX/vecmeta.git
cd vecmeta
cargo build --release --locked
```

生成的程序为 `target/release/emfsvg`，Windows 下为 `emfsvg.exe`。

```sh
cargo run --release --locked -p emfsvg-cli -- to-emf input.svg -o output.emf
cargo run --release --locked -p emfsvg-cli -- to-svg input.emf -o output.svg
cargo run --release --locked -p emfsvg-cli -- roundtrip input.emf --tolerance 1e-6
cargo run --release --locked -p emfsvg-cli -- to-emf input.svg -o archive.emf --lossless
```

`--verbose` 输出诊断，`--help` 列出支持的参数。

## Rust 接入

```rust
use emf2svg::{emf_to_svg_with, Emf2SvgOptions};
use svg2emf::{svg_to_emf, EmitOptions};

let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><circle cx="50" cy="50" r="25"/></svg>"#;
let emf = svg_to_emf(svg, EmitOptions {
    lossless: true,
    ..Default::default()
}).expect("SVG conversion");
let recovered = emf_to_svg_with(&emf, Emf2SvgOptions { lossless: true })
    .expect("Source recovery");
assert_eq!(svg, recovered);
```

`emf_to_svg_with_report` 与 `svg_to_emf_with_report` 同时返回兼容性诊断。UniPPT 内置组件采用独立版本快照，接入时应以目标快照的 API 为准。

## 保真度与限制

几何转换与原文精确恢复属于不同机制。`--lossless` 在目标文件内封装原始数据，不保证第三方应用渲染一致，也不会自动将中间格式的编辑合并到封装原文。字形轮廓保留矢量几何，不保留可编辑文本语义。

不支持的位图、滤镜或裁剪语义可能被跳过，应检查报告。径向渐变采用线性近似；传统 EMF 查看器可能平坦化渐变或忽略透明度。文本依赖字体，未实现完整的复杂文字塑形和双向排版。Office 与浏览器的像素一致性需另外开展渲染验证。

## 架构与验证

| Crate | 职责 |
| --- | --- |
| `vector-ir` | 统一场景、路径、变换与绘制属性 |
| `emf-core` | EMF 二进制读写与记录 |
| `glyph2path` | 字体查找与字形轮廓 |
| `emf2svg` | EMF 回放与 SVG 输出 |
| `svg2emf` | SVG 解析与 EMF 输出 |
| `emfsvg-cli` | 命令行转换与几何回归 |

```sh
cargo test --workspace --locked
```

四项外部语料测试默认忽略，需另行取得相应素材使用权，见 [来源说明](docs/PROVENANCE.md)。本次性能测评使用脚本生成的合成输入，无需外部语料。[商业授权](docs/COMMERCIAL_LICENSE.md)。

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

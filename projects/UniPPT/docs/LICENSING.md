# 许可范围与历史版本 / License scope and earlier releases

## 当前自有源码

自本次许可变更提交起，本仓库当前 UniPPT 自有源码及附有同一协议的 vecmeta 组件采用 [PolyForm Noncommercial 1.0.0](../LICENSE)。非商业使用按协议免费开放；超出标准许可允许范围的商业用途须另行申请付费书面授权。申请方式见 [商业授权](COMMERCIAL_LICENSE.md)。这是源码可见模式，不使用 OSI 开源认证措辞。

维护者于 2026-09-15 明确确认 vecmeta 为自研原创并具有重新许可权。因此修正其遗留的 GPL-2.0 与不对应本项目的 repository 元数据；这项权属说明来自项目维护者，不是第三方版权审计结论。

## 第三方边界

| 内容 | 适用规则 |
| --- | --- |
| UniPPT 自有核心、网页、本机 MCP 和 Skills | 本次发布所附非商业源码许可 |
| `vendor/vecmeta` 当前自有代码 | 该目录 [LICENSE](../vendor/vecmeta/LICENSE) 和 [来源说明](../vendor/vecmeta/LICENSE-NOTICE.md) |
| `vendor/pptx` | 保留上游 MIT 许可及 [溯源](../vendor/pptx/UPSTREAM.md) |
| 网页依赖、字体与图标 | 保留 [前端声明](../web/vendor/NOTICE.md) 及对应资产的许可 |
| 其他依赖与另附声明的资产 | 各自许可，不因顶层协议而改变 |

## 已经发布的版本

UniPPT 提交 `1d1caf665a0f77e9c66719c366e829e795b983f3` 及之前相应发布中的自有源码曾附 MIT 许可；vecmeta 的历史元数据曾声明 GPL-2.0。公开发行包使用独立源码快照；历史授权说明保留，使用者已经依法取得的权利不受影响。相同代码存在其他有效授权时，使用者仍可依该授权使用。

商业授权覆盖的是许可方有权授予的权利；不能据此限制第三方组件或撤回旧版本许可。源码许可不包含公网账户、模型额度、第三方字体或内容的使用权。面向实际交易的商业合同应由具有相应资质的法律专业人士审核。

## English

From the license-change commit onward, current first-party UniPPT source and vecmeta code carrying this license use the [PolyForm Noncommercial 1.0.0](../LICENSE). Qualifying non-commercial use is free; commercial uses outside its permitted purposes require a separate paid written license. This is source-available, not OSI-approved open source.

On 2026-09-15 the maintainer expressly confirmed independent authorship and relicensing authority for vecmeta. Its legacy GPL-2.0 and unrelated repository metadata are corrected on that basis; this is a maintainer statement, not an independent copyright audit.

The MIT license and attribution of `vendor/pptx`, browser dependency notices, and all other independently licensed components remain intact. UniPPT at commit `1d1caf665a0f77e9c66719c366e829e795b983f3` and applicable earlier releases carried MIT for its own source; vecmeta historically declared GPL-2.0. Earlier valid grants are not retroactively revoked, and another valid license for identical code remains usable.

Commercial licensing grants only rights controlled by the licensor. It does not override third-party terms or include hosted accounts, model credits, fonts, or content rights. Have qualified legal counsel review contracts for actual commercial transactions.

## 标准许可与单独商业授权 / Standard and commercial licenses

当前发行采用未经修改的 PolyForm Noncommercial 1.0.0，SPDX 标识为 `PolyForm-Noncommercial-1.0.0`。根目录 `LICENSE` 保留标准原文；本说明及商业申请流程不修改该许可。非商业用途及原文明确列出的机构用途按标准条款免费。慈善、教育、公共研究、公共安全或卫生、环保和政府机构的使用允许条款，不受资金来源或资金义务影响。

超出标准许可允许范围的商业用途，请联系 [cc@omnidoc.top](mailto:cc@omnidoc.top)，申请付费商业授权，协商费用并在使用前取得书面许可，详见 [商业授权流程](COMMERCIAL_LICENSE.md)。不能以此说明限制第三方代码、另附许可的资产或使用者已经合法取得的旧版本权利。此前按 MIT、GPL 或自定义非商业许可等条款有效授予的权利，不因本次变更而撤销。

分发时须遵守标准许可的 Notices 条款，提供许可原文或其链接，并保留所附 `Required Notice:` 行。贡献者保留版权；提交 PR 不自动授予商业再许可权，维护者须另行取得所需的明确授权。此许可属于源码可见许可，不是 OSI 批准的开源许可。

Current releases use the unmodified PolyForm Noncommercial 1.0.0, SPDX identifier `PolyForm-Noncommercial-1.0.0`. The root `LICENSE` contains the standard text. This guide and the commercial application process do not amend it. Noncommercial purposes and the institutional uses expressly listed in that text are free under its terms, regardless of the listed institutions' funding sources or resulting obligations.

For commercial uses outside the standard license's permitted purposes, contact [cc@omnidoc.top](mailto:cc@omnidoc.top) to apply for a paid commercial license, agree on fees and obtain written permission before use. See the [application process](COMMERCIAL_LICENSE.md). Third-party code, separately licensed assets and valid earlier grants remain independent. Rights validly granted under earlier MIT, GPL or custom noncommercial terms are not retroactively revoked.

Distributions must comply with the standard Notices clause, providing the terms or their URL and any supplied `Required Notice:` lines. Contributors retain copyright; submitting a PR does not automatically grant commercial relicensing rights, which require explicit additional authorization. This license is source-available and is not OSI-approved open source.

Standard text: [PolyForm Project](https://polyformproject.org/licenses/noncommercial/1.0.0) · [SPDX](https://spdx.org/licenses/PolyForm-Noncommercial-1.0.0.html).

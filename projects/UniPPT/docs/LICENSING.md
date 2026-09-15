# 许可范围与历史版本 / License scope and earlier releases

## 当前自有源码

自本次许可变更提交起，本仓库当前 UniPPT 自有源码及附有同一协议的 vecmeta 组件采用 [OmniDoc 非商业源码许可 1.0](../LICENSE)。非商业使用按协议免费开放；商业使用须书面许可。申请方式见 [商业授权](COMMERCIAL_LICENSE.md)。这是源码可见模式，不使用 OSI 开源认证措辞。

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

From the license-change commit onward, current first-party UniPPT source and vecmeta code carrying this license use the [OmniDoc Non-Commercial Source License 1.0](../LICENSE). Qualifying non-commercial use is free; commercial use requires written permission. This is source-available, not OSI-approved open source.

On 2026-09-15 the maintainer expressly confirmed independent authorship and relicensing authority for vecmeta. Its legacy GPL-2.0 and unrelated repository metadata are corrected on that basis; this is a maintainer statement, not an independent copyright audit.

The MIT license and attribution of `vendor/pptx`, browser dependency notices, and all other independently licensed components remain intact. UniPPT at commit `1d1caf665a0f77e9c66719c366e829e795b983f3` and applicable earlier releases carried MIT for its own source; vecmeta historically declared GPL-2.0. Earlier valid grants are not retroactively revoked, and another valid license for identical code remains usable.

Commercial licensing grants only rights controlled by the licensor. It does not override third-party terms or include hosted accounts, model credits, fonts, or content rights. Have qualified legal counsel review contracts for actual commercial transactions.

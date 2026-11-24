# RPCore 研究资料与算法溯源
状态：研究阅读清单；链接用于后续研读，不代表已逐篇验证或实现。

## 规范
- PDF 2.0：ISO 32000-2（PDF Association）：https://pdfa.org/resource/iso-32000-2/
- PDF 1.7：ISO 32000-1 / Adobe PDF Reference（历史兼容）
- OpenType 规范：https://learn.microsoft.com/en-us/typography/opentype/spec/
- Unicode UAX #9（双向文本）：https://www.unicode.org/reports/tr9/
- Unicode UAX #29（文本边界）：https://www.unicode.org/reports/tr29/

## 参考工程（研究与测试对照）
- PDFium：https://pdfium.googlesource.com/pdfium/ — 对象解析、字体、显示列表、字符定位；不作为运行时依赖。
- Hayro：https://github.com/LaurenzV/hayro — Rust PDF 解释与渲染的模块设计参考。
- PDF.js：https://github.com/mozilla/pdf.js — 现有引擎的比较基准及兼容测试参考。
- MuPDF：https://mupdf.com/ — 可选测试对照；尤其注意 AGPL/商业授权，不直接复制代码。

## 科研文档理解阅读清单
| 论文/项目 | 用途 |
|---|---|
| PubLayNet | 科学论文版面检测；https://arxiv.org/abs/1908.07836 |
| DocLayNet | 多类型文档布局数据；https://arxiv.org/abs/2206.01062 |
| LayoutLM | 文字与空间布局建模；https://arxiv.org/abs/1912.13318 |
| TableFormer | 表格结构恢复；https://arxiv.org/abs/2203.01017 |
| Nougat | 学术文档转结构化文本/数学；https://arxiv.org/abs/2308.13418 |
| Docling | 文档解析流水线；https://arxiv.org/abs/2408.09869 |

先以几何规则、字符坐标、行聚类、列间空白和区域关系实现可解释基线；再用数据集评估学习模型的增益。公式识别及表格重建不是 PDF 标准解析本身，必须允许低置信度和人工纠正。研究资料不等于模型/数据可自由商业使用。

## 溯源制度
每个算法记录：问题、规范章节、论文/代码版本及链接、许可证、独立实现思路、测试样本、结果、局限、评审者。依赖登记 SBOM、版本、许可证及再分发义务；直接复用代码需明确出处、许可与必要版权声明。研究笔记和测试数据不得包含私人论文、用户数据、密钥或未授权材料。公开仓库内容应具备发布授权。

# RPCore 功能需求与系统架构
状态：设计提案，以下均为待实现功能。

## 分层
```text
React / Electron + Chromium UI（计划迁移）
    | 隔离 preload / main process / worker IPC
    | 类型化 IPC：文档 ID、页号、视口、任务 ID
Rust RPCore facade / worker / cache
    | selection / search / annotation / layout
    | display list / text model / renderer
    | graphics interpreter / fonts / image decoder
    | parser / xref / streams / page tree
PDF 原始文件
```
统一页面对象和字符坐标为渲染、选择、搜索、高亮提供数据；不依赖 HTML TextLayer。字体缺字、不可逆编码和非逻辑内容顺序须显式处理。UI 可显示原生生成的位图，未来可独立尝试 GPU 或原生窗口；不要求全应用改写。

## 计划目录
```text
crates/
  rpcore-parser/      # lexer, objects, xref, resolution, limits
  rpcore-decoder/     # stream filters
  rpcore-document/    # page tree, resources, metadata
  rpcore-font/        # encoding, CMap, glyph outlines
  rpcore-graphics/    # operators, matrices, display list
  rpcore-render/      # raster backend, image, compositing
  rpcore-text/        # glyph/character model, lines
  rpcore-selection/   # hit testing, ranges, search, quads
  rpcore-layout/      # reading order, columns, regions
  rpcore-math/        # equation recognition
  rpcore-table/       # table reconstruction
  rpcore/            # public facade, errors, cache, jobs
benchmarks/
test-pdfs/
research/
```
目录为计划，不代表已创建。可先在一个 crate 内按模块实现，稳定后再拆分，避免早期过度工程化。

## 功能清单
| 模块 | 功能 | 阶段 |
|---|---|---|
| 解析 | PDF 1.x/2.0 支持矩阵、间接对象、xref、压缩对象、增量更新 | 0.1+ |
| 文档 | 页树、页面盒、旋转、资源继承、目录、元数据、加密状态 | 0.1+ |
| 流 | Flate、ASCIIHex/85、RunLength、LZW、预测器；图像过滤器按能力扩展 | 0.1–0.2 |
| 图形 | q/Q、cm、路径、填充/描边、裁剪、Form XObject、色彩空间、透明组 | 0.2+ |
| 字体 | Type1/TrueType/CFF、Type0/CID、嵌入字体、ToUnicode、CMap、字宽、替代字体 | 0.2–0.3+ |
| 渲染 | 页面尺寸、DPI、抗锯齿、图片与遮罩、旋转、分块、颜色管理 | 0.2–0.5 |
| 文字 | Unicode、字符/字形对应、连字、竖排、上下标、行/段落及双向文本 | 0.3+ |
| 选择 | 字符命中、拖动、跨行跨页、双栏顺序、复制、稳定选区 | 0.3+ |
| 搜索 | 文本索引、大小写与 Unicode 规范化、跨文本块匹配、结果坐标 | 0.3+ |
| 批注 | 高亮、下划线、笔记、原始页坐标；默认不改源 PDF | 0.3+ |
| 科研布局 | 双栏、标题、摘要、正文、参考文献、图表区域及图注关联 | 0.4 |
| 科研语义 | 数学结构/LaTeX 候选、表格行列/合并单元格、结构化输出 | 0.4+ |
| OCR | 扫描页检测，可选本地 OCR，置信度标记 | 后续扩展 |
| 性能 | 按需解析、页面缓存、可取消任务、虚拟化、内存上限 | 0.5 |
| 集成 | 文献库、AI 提示词、科研记录、Word、翻译；旧批注迁移 | 0.5–1.0 |

加密 PDF、复杂透明度、字体及 PDF/A 等按实际测试逐项列出支持情况；不宣称完整标准兼容。OCR、数学和表格结构恢复是独立推断任务，不能保证无误。

## 公共接口草案
open_document(authorized_source, password?) -> DocumentId + metadata
page_info(document_id, page_index) -> boxes + rotation
render_page(document_id, page_index, viewport, scale, task_id) -> bitmap
page_text(document_id, page_index) -> text model + glyph quads
hit_test(document_id, page_index, point) -> character anchor
select_range(document_id, anchors) -> text + page quads
search(document_id, query, options, task_id) -> matches
cancel(task_id); close_document(document_id)
接口版本化；页号内部零基、UI 一基，明确 PDF 用户空间和屏幕空间转换。选区锚点关联文档指纹、页索引、字符范围和四边形；不同缩放下不重写原始坐标。区分原始字符顺序、逻辑阅读顺序和显示字形；一对多 Unicode 映射不能用单字符假设。

## 基础库候选
flate2、ttf-parser、skrifa、tiny-skia、kurbo、image、rayon、wgpu（可选）。仅候选，使用前做版本、许可证、维护状态与安全审计，不引入完整 PDF 解析器/渲染器作为运行时依赖。压缩、字体等基础算法可复用通用库；PDF 特有语义与核心控制逻辑独立实现。

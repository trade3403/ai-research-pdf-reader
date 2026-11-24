# Codex 开发路线：从 PDF Reader 到科研阅读工作站

## V0.1：基础 PDF 阅读（当前源码）

目标：无需网络即可打开本机 PDF，具备完整的基本阅读、搜索、缩放、目录与书签功能。

验收：使用示例 5 页 PDF 跑通 README 中的全部验收步骤；在 Windows 11 x64 `npm run tauri:build` 成功产出 NSIS 安装包。

## V0.2：专业阅读体验

- 多页连续滚动 + 页面虚拟化，限制同时渲染的页面数量。
- 持久化高亮、下划线、批注；每条批注保留页码、四边形坐标、原文、颜色。
- SQLite 文献库：documents, file_paths, reading_positions, annotations, collections, tags。
- 搜索索引缓存，避免每次 Ctrl+F 扫描全篇。
- 精确文字选择与批注锚点（考虑 PDF.js TextLayer 字符位置）。
- 测试大型论文 PDF、中文 CMap、旋转页面、加密 PDF、扫描 PDF、表格和公式。

## V0.3：Zotero 风格文献管理

- 文件夹/标签、分类树、元数据表单、DOI/ISBN 元数据导入。
- BibTeX/CSL JSON 导入导出，跨语言文献关键词搜索。
- 仅通过合法文件/API 接口做 Zotero 互操作；避免直接修改 Zotero SQLite 数据库。

## V0.4：AI 论文分析（独立可替换模块）

- 文本分块、页码和坐标溯源、向量搜索。
- 摘要、研究问题、方法、创新点、实验设置提取。
- 关键结论必须显示 PDF 原始页码、选区及来源段落。
- 模型运行时可配置本地推理或获得用户授权后接入云端 API，绝不默认上传 PDF。

## V0.5：科研工作站集成

- 与 AI MATLAB Bridge 通信：论文方法参数提取、实验任务结构化、结果归档。
- PDF 阅读器与论文编辑器、实验结果、笔记的双向引用。

## 代码质量要求

- 所有 I/O 执行错误须有明确 UI 提示。
- PDF 加载/渲染切换可取消，避免关闭标签后残留 Worker 作业。
- 阅读器功能在断网状态下仍应正常工作。
- 新增文件访问权限遵循最小权限原则。
- 维护许可证 SBOM 和第三方声明文件。


## V0.6.x 及以后：自主 Rust PDF 核心 RPCore

新增独立研发主线，核心版本与应用版本分别管理。先完成 PDF 解析、图形渲染和字符坐标闭环，再研发科研布局、公式、表格及性能优化。现有 V0.5.9 应用作为迁移与测试基线；最终不依赖完整第三方 PDF 引擎。详细计划见 [RPCore 研发路线](docs/RPCORE_ROADMAP.md) 和 [功能架构](docs/RPCORE_ARCHITECTURE.md)。各阶段须通过 [验收门槛](docs/RPCORE_VALIDATION.md)，未验证能力不得标注为已实现。

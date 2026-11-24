# AI Research PDF Reader / AI Research Scientist Studio

## 新研发方向：RPCore 自主 Rust PDF 核心

当前应用基线 V0.5.9，RPCore 尚处于规划阶段。计划依据 PDF 国际标准自主研发解析、渲染、字符坐标、划词、搜索及科研布局算法，不在最终产品中依赖 PDFium、MuPDF 或 PDF.js；现有应用功能在过渡期继续保留。

- [研发路线与里程碑](docs/RPCORE_ROADMAP.md)
- [功能需求与系统架构](docs/RPCORE_ARCHITECTURE.md)
- [测试、安全与验收](docs/RPCORE_VALIDATION.md)
- [规范、论文与算法研究](docs/RPCORE_RESEARCH.md)

---

## V0.5.3 增量功能

- PDF 划词后选择科研分析：自动准备带来源和页码的 ChatGPT 提示词。
- 本地可信 WebView 经 Rust 命令仅向 chatgpt.com 的空白输入框尝试插入文本；不自动发送，不提取消息、Cookie 或聊天记录。
- 页面未加载或结构变化时有手动“重试填写”和复制提示词回退。
- ChatGPT 右上角可“隐藏 PDF / 显示 PDF”：隐藏时连同 PDF 工具栏一起收起，网页占满工作区；恢复保留之前的页码和分屏比例。
- 保持 V0.5.2 的拖动分栏、关闭 ChatGPT、Word 编辑、科研记录、Azure 翻译和 SQLite 备份。

# AI Research PDF Reader V0.5.1

V0.5.1 adds dynamic SCI equation/figure/table numbering, cross-references, snapshot-backed bibliography, Word headers/footers with page numbers, line spacing, and GitHub release/rollback controls.

# AI Research PDF Reader V0.3 — Literature Library

**Windows desktop app** using Tauri 2 + Rust + React/TypeScript + PDF.js + SQLite. V0.3 adds a Zotero-style **local literature library** while keeping the existing PDF reading and annotation functions.

Features include multiple PDF/folder import, SHA-256 deduplication, folders/tags, favorites, title/author/journal/DOI editing, manual PDF metadata detection, optional user-approved Crossref DOI lookup, per-PDF offline text indexing, BibTeX/RIS citation exchange and metadata-only JSON export.

**Privacy:** Imported PDFs stay at their original file paths. The application does not upload PDF contents. Crossref lookup contacts `api.crossref.org` only after the user opts in. **Backup warning:** JSON export does not include PDF bytes or original SQLite annotations; a full backup must also preserve the application's SQLite database and the original PDF files.

**V0.3 limitations:** No official Zotero integration or sync; incomplete PDF recognition and no scanned-PDF OCR. Bulk 1,000-PDF throughput has not been validated on real hardware. Source: `V0_3_RELEASE_NOTES.md`.

**Build:** `npm install` then `npm run tauri:build`; GitHub Actions builds an NSIS installer automatically when this repository is updated. Requires Windows for native installer compilation.

---

# AI Research PDF Reader V0.2

**V0.2 changes:** Research annotations/highlights, notes, SQLite persistence, reading session restore and Markdown export. See [V0_2_RELEASE_NOTES.md](V0_2_RELEASE_NOTES.md) for details. The Windows app remains fully offline for PDF viewing and annotations.

---

# AI Research PDF Reader V0.1 — Windows 安装包构建版

Windows 优先的科研 PDF 阅读器 MVP，使用 **Tauri 2 + Rust + React + TypeScript + Mozilla PDF.js**。项目包含**一键生成 Windows NSIS 安装包的脚本与 GitHub Actions 构建工作流**。由于当前交付环境无法完成 Windows 编译，压缩包中**不含已经编译好的 EXE**。如只想安装，请先按照 `WINDOWS_INSTALLER_GUIDE.md` 生成安装文件。

第一阶段只开发 PDF 阅读。**不接入 AI API、Zotero、云同步或付费 PDF SDK**。

## V0.1 已实现

| 功能 | 说明 |
| --- | --- |
| 本地 PDF 打开 | Windows 原生选择文件对话框，支持一次选多个文件；`npm run dev` 浏览器预览时使用 HTML 文件选择器 |
| PDF 渲染 | PDF.js Worker + Canvas，支持高 DPI 渲染 |
| 多标签 | 多 PDF 同时打开，单独维护页码、缩放和旋转 |
| 页面导航 | 上/下一页、指定页码、缩略图点击跳转 |
| 缩放 | 50%–300%，适应阅读区域宽度 |
| 页面旋转 | 每次旋转 90° |
| 文本选择 | PDF.js TextLayer，支持鼠标选择与复制文字 |
| 全文搜索 | `Ctrl+F`，扫描全文并显示命中页数、命中次数，页面内高亮匹配文字；同词跨文本块命中高亮能力有限 |
| 章节目录 | PDF 内置 Outline，大纲条目跳转 |
| 书签 | 收藏当前页；使用浏览器/Tauri WebView 的 localStorage，重启后仍在本机保留 |
| 深色模式 | 本地保存主题偏好 |
| PDF 输入验证 | PDF 头校验，默认单文件最大 100 MB |
| 加密 PDF | 基础密码提示与重试（由 PDF.js 处理） |
| 中文 PDF 资源 | `postinstall` 自动复制 PDF.js CMaps、标准字体与 WebAssembly 资源 |

### 暂未开发

- Zotero 数据库、文献条目、DOI 元数据、云同步
- AI 论文总结、翻译、问答、公式解析
- 高亮批注持久化、侧边笔记编辑、批注写回 PDF
- OCR、编辑 PDF 页面、合并/拆分/签名
- 连续滚动和双页视图（首版为单页阅读）
- 大文件流式读取、打开上次文件时自动恢复其内容

## Windows 开发环境

建议 Windows 10/11 x64，并安装：

1. Node.js 20.19+ 或 22 LTS：https://nodejs.org/
2. Rust 工具链（MSVC）：https://rustup.rs/
3. Visual Studio 2022 Build Tools，安装 **Desktop development with C++** 工作负载：https://visualstudio.microsoft.com/visual-cpp-build-tools/
4. Microsoft Edge WebView2 Runtime（Windows 11 通常预装）：https://developer.microsoft.com/microsoft-edge/webview2/
5. 可以联网安装 npm/Rust 依赖的网络环境，应用运行时不需要使用外部 API。

> 首次构建 Rust/Tauri 会下载依赖并编译本地二进制。请勿把“软件本地离线使用”和“首次安装构建依赖需要下载”混淆。

## 运行步骤

解压 ZIP 后，在项目根目录 PowerShell 中运行：

```powershell
npm install
npm run tauri:dev
```

也可以双击 `scripts/start-dev.bat`。

启动后点击 **打开 PDF**，选中任意 PDF。项目包含示例：`demo/Research_Reader_Test.pdf`。

### 只测试 Web 前端（无需 Rust）

```powershell
npm install
npm run dev
```

然后在 Chrome 打开 http://127.0.0.1:5173 。浏览器模式通过本地文件选择器打开 PDF（不同于 Windows 桌面选择器），用于检查阅读界面和 PDF.js 渲染。

### 编译 Windows 安装包（推荐双击 BUILD_WINDOWS_INSTALLER.bat）

```powershell
npm install
npm run tauri:build
```

也可双击 `scripts/build-windows.bat`。

生成目录通常为：

```text
src-tauri\target\release\bundle\nsis\
```

输出文件通常以 `-setup.exe` 结尾。**本压缩包没有预编译 EXE**，需在 Windows 中实际执行上述命令。

## 项目结构

```text
AI_Research_PDF_Reader_V0_1/
├─ src/
│  ├─ App.tsx                     # 主界面、标签页、搜索、书签、导航
│  ├─ components/
│  │  ├─ PdfPage.tsx              # PDF.js 渲染与文本层
│  │  └─ Thumbnail.tsx            # 懒加载页面缩略图
│  ├─ utils/
│  │  ├─ pdf.ts                   # PDF.js Worker、解析、搜索、大纲
│  │  └─ bookmarks.ts             # 本地书签持久化
│  ├─ main.tsx
│  └─ styles.css
├─ src-tauri/
│  ├─ src/lib.rs                  # Tauri、文件对话框与 fs 插件
│  ├─ src/main.rs
│  ├─ capabilities/default.json  # 最小化读文件权限
│  ├─ icons/
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ scripts/
│  ├─ copy-pdfjs-assets.mjs      # 自动复制 PDF.js CMap/字体/wasm
│  ├─ start-dev.bat
│  └─ build-windows.bat
├─ demo/Research_Reader_Test.pdf
├─ package.json
└─ README.md
```

## 测试验收

用 `demo/Research_Reader_Test.pdf` 进行以下操作：

1. 打开 PDF，显示 **5 页**。
2. 通过左侧缩略图跳到第 3 页，应看到蓝色曲线图。
3. 切换 **大纲**，点击 `4. Discussion`，定位第 4 页。
4. 按 `Ctrl+F`，输入 `reproducibility`，应返回包含该单词的页面。
5. 在 PDF 上拖动鼠标选择英文段落并复制。
6. 点击旋转按钮，页面每次旋转 90°。
7. 放大、缩小、使用“适应宽度”。
8. 在第 3 页添加书签，进入左侧书签列表跳转。
9. 重新打开软件，已存储的书签应在再次打开**同一 PDF**时出现。
10. 打开第二份 PDF，测试两个标签独立导航。

## 设计与安全说明

- 默认不上传 PDF、文字、书签或任何文件内容到互联网。
- 书签保存在 WebView 本地 `localStorage` 中，不写回原 PDF。
- Tauri dialog 插件在用户手动选择文件后临时将所选路径加入文件访问 scope；fs 插件只开放 `allow-read-file`，没有提供任意路径写入权限。
- 本版本不开放执行外部系统命令的前端能力。
- PDF.js 的文字层对扫描图像 PDF 不会自动 OCR；若 PDF 无文字层，搜索不会识别图片中的文字。
- 仅通过本地文件对话框打开，暂不支持外部远程 URL 和文档注入脚本。
- 本项目为源码示范。处理来自互联网的不可信 PDF 时，应保持 PDF.js、WebView2 和操作系统最新，并加强生产级安全测试。

## 开源组件和商业使用

项目主要依赖 Tauri 2（MIT/Apache-2.0）、React（MIT）、Mozilla PDF.js（Apache-2.0）。闭源商业软件通常可以集成这些组件，但仍需在发布时遵守许可证与相关第三方版权声明，并进行依赖许可证审计。**本源码不集成 MuPDF/PyMuPDF**。

## 已验证与尚未验证

已检查源码结构、配置文件、示例 PDF 的可读取/可搜索性，示例 PDF 的五页渲染。

因为当前构建环境没有 Rust/Cargo 工具链，也无法下载 npm 依赖，所以 **尚未实际编译 Windows EXE、尚未做 Tauri 运行时的端到端验证**。Windows 编译或依赖 API 兼容性问题需要在目标 Windows 环境中进一步验证。

## 下一版规划

V0.2 建议增加：真正的高亮批注及保存、本地 SQLite 文献库、阅读位置恢复、文件拖拽与目录扫描、连续滚动模式。之后再引入 Zotero 互操作和本地 AI 论文分析。

## V0.4 — Native ChatGPT research workspace

Choose **ChatGPT 网页** to open an actual WebView2 child view beside the current PDF inside the Tauri desktop app (no Chrome extension or OpenAI API required). Select text in ChatGPT and press **Ctrl+C**, then click the app's **保存选中内容** toolbar. Review it, optionally link a literature item and page, and confirm save. Browse, edit, search and export saved notes via **科研记录**.

**Privacy:** ChatGPT runs in an unprivileged webview identified separately from the local `main` webview. The remote page has no local Tauri IPC/file/SQLite permissions. Clipboard text is read only after a user click. No session scraping, cookie reads, DOM injection, or automatic bulk history extraction.

**Compatibility warning:** ChatGPT authentication in embedded WebView2 depends on the website's policies and may fail. Use **外部打开** when needed. A successful NSIS build does not prove that ChatGPT accepts embedded authentication. See [V0.4 release notes](V0_4_RELEASE_NOTES.md).

### V0.4.2 — Azure Translator F0 translations (optional)

The app adds a **翻译设置** button, PDF text-selection **翻译** action, and saved research-record **翻译 / 中英对照** action. Optional Azure API keys are stored in Windows Credential Manager; only the trusted Rust backend calls Microsoft. Translation caching and a conservative monthly local budget are persisted in SQLite. LaTeX and code are shielded from translation and typeset locally in side-by-side comparison. See [V0_4_2_AZURE_TRANSLATION_RELEASE_NOTES.md](V0_4_2_AZURE_TRANSLATION_RELEASE_NOTES.md) for configuration, privacy, limitations and acceptance steps.

## V0.5.0 — Word 科研论文编辑器

新增 **Word 论文** 标签：创建草稿、导入/编辑常见 DOCX、排版标题/段落/表格/图片、插入 LaTeX 数学公式、保存草稿及导出 DOCX。草稿自动保存到现有 SQLite 数据库，数据库备份随之涵盖 Word 草稿。可以从 PDF 选中文字或科研记录直接插入 Word。SCI 模板和文献库中的作者年份引用也可插入。

详情见 `V0_5_RELEASE_NOTES.md`。不能保证复杂 DOCX 文件的往返保真，原 DOCX 必须保留。无需安装 Microsoft Word，但安装的 Word/兼容办公软件可用于检查导出排版。

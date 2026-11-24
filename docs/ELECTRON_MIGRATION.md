# Electron + Chromium 桌面架构迁移
状态：已选定研发方向，尚未实施。迁移基线：V0.5.9 Tauri。
## 架构决策
正式桌面版本采用 Electron 自带 Chromium（不依赖系统 WebView2），保留 React/TypeScript/Tiptap，Rust RPCore 独立研发。当前 Tauri 版本保持可构建、可回滚。Electron 本身不是 PDF 解析方案，内置 PDF 查看器不得作为正式阅读路径；最终应用不调用 PDF.js、PDFium、MuPDF 等完整 PDF 引擎实现阅读。Chromium 随 Electron 分发可能包含 PDFium 相关代码，此约束指应用 PDF 功能不调用该引擎，而非未经验证地宣称二进制完全不含相关代码。
## 进程架构
- Main：窗口生命周期、菜单、授权文件选择、IPC 路由、数据库服务、任务管理。
- 本地 Renderer：React UI、位图显示及交互；不直接访问 Node、文件系统或数据库。
- Preload：contextBridge 暴露窄类型 API；不暴露通用 IPC、shell 或任意文件读写。
- ChatGPT：独立 WebContentsView、独立持久化 session partition；禁止 Node、特权 preload、任意本地 IPC；导航及弹窗白名单、外部打开回退。登录兼容性必须实测；用户确认后传递选区，禁止自动发送、读取 Cookie 或批量提取聊天记录。
- Rust worker：独立可执行进程，版本化消息协议（请求 ID、长度限制、超时、取消、错误码）；只处理获授权文档句柄，默认无网络；崩溃可重启。进程隔离并不自动等同 OS 沙箱，后续研究 Windows 受限令牌及 Job Object。N-API 只作为经过评估的可选优化，避免解析不可信 PDF 导致主进程崩溃。
- 渲染：先 Rust CPU 位图 + Chromium 显示层，二进制缓冲传输而非大 JSON/base64；统一页面坐标和字符模型。后期独立评估 wgpu、共享纹理及原生视图嵌入；不把 GPU 支持当作已实现功能。
## 迁移清单
| 功能 | 迁移目标 | 验收 |
|---|---|---|
| 文件选择和导出 | Electron dialog + 授权路径/句柄 | 多文件、目录、取消、非法路径、覆盖确认 |
| PDF 阅读 | 初期保留 PDF.js 适配器，后期 RPCore | 页码、缩放、旋转、密码、搜索、划词、批注 |
| ChatGPT 分屏 | WebContentsView | DPI、拖动、焦点、隐藏恢复、会话、外部打开 |
| SQLite | 主进程数据库服务，选型后固定版本 | 迁移旧库、事务、备份/恢复、完整性校验 |
| Azure 翻译 | 受信服务与 OS 凭据存储 | 旧凭据兼容或用户重新授权；不写入日志 |
| 文献库 | 保持数据结构和本地路径 | 去重、标签、索引、导入导出、引用 |
| Word / 科研记录 | 保留 Tiptap 和业务组件 | DOCX、公式、引用、草稿、笔记回归 |
| 安装更新 | electron-builder + NSIS | 干净 Windows x64 安装/卸载、离线启动、数据保留 |
## 安全与性能
contextIsolation=true、sandbox=true、nodeIntegration=false、webSecurity=true；严格 CSP、IPC sender/frame 校验、协议与路径白名单；第三方网页不共享本地权限。固定 Electron 安全维护版本，定期升级 Chromium；npm/Cargo 锁文件及 SBOM。跨进程位图有界缓存、取消过期任务、避免主线程阻塞。安装包大小和内存以实测为准，不承诺提速。
## 分支与版本
feature/electron-migration：先建立最小桌面壳、平台适配接口、打包与 smoke test，再迁移业务；feature/rpcore-bootstrap：独立 Rust 核心，不依赖 Electron；通过评审合并。V0.6.0 Electron 预览，V0.6.1 功能及数据迁移验收，V0.7.x RPCore 实验接入（取决于核心成熟度），V0.8.x 精准交互和科研布局，V0.9.x 性能/GPU，V1.0 全面验收。阶段编号是计划，非发布日期或已完成状态。默认引擎切换需通过 RPCore 验收门槛。
## 参考
https://www.electronjs.org/docs/latest/tutorial/security
https://www.electronjs.org/docs/latest/api/web-contents-view
https://www.electron.build/

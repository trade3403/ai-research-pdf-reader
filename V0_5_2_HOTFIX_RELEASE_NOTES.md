# V0.5.2 — ChatGPT 侧栏关闭、拖动分隔线与 WebView2 覆盖修复

## 问题

V0.5.1 的嵌入式 ChatGPT 是原生 WebView2 子窗口，与 React Word 编辑器并非同一渲染层。切换到 Word 时异步 close() 可能未及时移除子窗口，导致右侧 ChatGPT 继续覆盖 Word，既不能关闭也不能调整左右宽度。

## 修复

- 在 Word 论文工作区可通过顶部「打开/关闭 ChatGPT 侧栏」独立控制右侧网页（默认关闭，Word 独占全宽）。
- 在「ChatGPT 网页」工作区和 Word 侧栏顶部增加「关闭 ChatGPT 侧栏」按钮。
- 在左、右面板之间增加可拖动的分隔条；双击恢复默认比例，方向键微调；宽度保存在本地配置。
- 拖动时临时隐藏原生 WebView2，避免子窗口拦截分隔线拖动事件；结束后按最新尺寸定位并显示。
- 退出 ChatGPT 工作区时先 hide() 再 close()；清理带 chatgpt-embedded- 前缀的遗留原生窗口。
- `hide/show` 权限仅授予可信主 WebView。远程 ChatGPT 页面没有访问 Tauri/SQLite/本地文件的权限。
- V0.5.1 及之前的 Word 文稿、PDF 记录、翻译缓存和 SQLite 结构不改动。

## 验收

1. 启动 V0.5.2，进入 Word 论文，右侧无残留 ChatGPT。
2. Word 顶栏点击「打开 ChatGPT 侧栏」，右侧出现网页。
3. 拖动中间的分隔线，松开后两侧宽度与网页位置一致。
4. 点击右侧 X 关闭，Word 填满可用区域。
5. 切换文献库、科研记录、PDF，右侧网页不会覆盖其他工作区。
6. 切换回 ChatGPT 网页，PDF + ChatGPT 分屏可独立调整宽度；拖动和关闭均有效。
7. 重启软件后分屏比例可恢复，原有 Word/PDF/科研记录保留。

## 限制

WebView2 子视图创建和显示仍依赖 Tauri 2 的不稳定 child-webview API；正式用户 Windows 10/11 UI 验收仍然必要。

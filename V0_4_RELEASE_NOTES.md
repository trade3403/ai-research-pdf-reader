# AI Research PDF Reader V0.4 — ChatGPT Embedded Research Workspace

## Included features

- A **native WebView2 child view** in the same Windows/Tauri window, alongside the PDF reader; **no Chrome extension or OpenAI API key** required for the web view.
- ChatGPT's own webpage is opened at `https://chatgpt.com/`; no fake chat interface or reverse-engineered ChatGPT API is used.
- A native app toolbar lets users **explicitly** read selected/copied text from the clipboard (`Ctrl+C` on the ChatGPT page), preview and edit it, and save to the existing on-device SQLite database.
- Optional paper, PDF page, ChatGPT source URL and title are saved with each note.
- Research Records screen for local keyword search, editing, deletion, linked PDF opening, Markdown export.
- V0.1 reading, V0.2 annotations and V0.3 literature management remain intact.

## Technical architecture

- `src/components/ChatBrowserDock.tsx`: Tauri 2 `Webview` child (`unstable` multiwebview API), positioned inside a React container with `ResizeObserver`.
- `src/utils/researchClips.ts`: validated record data, SQLite migrations (`research_clips` table), and Markdown exporter.
- `src/components/ResearchRecords.tsx`: research archive UI.
- `src-tauri/capabilities/default.json`: `webviews: ["main"]` **without** a `windows` selector. The remote child webview has **no Tauri capabilities**. It cannot call SQLite, file-system, clipboard or Rust invoke commands. No remote source is authorized in capability `remote`.
- `src-tauri/Cargo.toml`: enables Tauri's `unstable` feature for native child webviews; adds Tauri clipboard and opener plugins.

## Limitations and safety

1. Loading the ChatGPT webpage in WebView2 **does not guarantee that OpenAI will permit login or all browser features**. The app provides an explicit “open in system browser” fallback. Verify with a real Windows desktop session.
2. The app **does not automatically intercept ChatGPT responses, network traffic, credentials or cookies**. It does not inject scripts into ChatGPT's DOM. Only user-selected text copied via Ctrl+C is imported via a deliberate action.
3. Clipboard text is not necessarily Markdown. For maximum formatting fidelity, copy Markdown text or original code blocks before saving; mathematical notation may need light manual correction.
4. Switching away from the ChatGPT tab closes its native webview. The ChatGPT site may remember authentication cookies, but the exact current conversation location is not automatically restored. Save the source conversation URL when saving important notes.
5. Automated build and static checks do not replace Windows GUI acceptance testing of login, Webview2 focus, selection, resizing, clipboard and local data migration.
6. This is an unsigned developer installer. Install only after reviewing source and Windows Security warnings.
7. Existing `research-reader-v02.db` file is reused, with an additive table migration. No existing PDF source or annotation is rewritten by the new module.

## Manual Windows acceptance checklist

- [ ] Install V0.4 over V0.3, preserving documents, annotations and library metadata.
- [ ] Click **ChatGPT 网页**; confirm a native ChatGPT web page appears next to the PDF, or the external fallback is offered.
- [ ] Test account login in WebView2; if blocked, use **外部打开** rather than attempting to bypass authentication.
- [ ] Select an answer in ChatGPT; Ctrl+C; click **保存选中内容**; preview and confirm save.
- [ ] Link a paper and page number; switch to **科研记录**; verify the record remains after application restart.
- [ ] Click **打开关联论文**, verify the linked page is opened.
- [ ] Search records; edit, delete, and export Markdown.
- [ ] Resize/maximize the Windows app: remote webview should track the ChatGPT region without covering the app toolbar.

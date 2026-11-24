# AI Research Scientist Studio V0.5.3

## 范围与行为

1. PDF 划词分析：在用户单次点击后准备科研提示词，打开 ChatGPT 网页并尝试自动填写。
2. 只填写、不发送：Rust 命令仅定位 chatgpt.com 对应的 WebView2，在用户输入框为空时填入文本；已存在内容不覆盖。
3. 状态与回退：返回填写状态（filled/occupied/not_ready等），失败时使用“重试自动填写”或“复制提示词”。登录状态和 ChatGPT 网页 DOM 不保证稳定，用户需实机验收。
4. 单独的“隐藏 PDF / 显示 PDF”按钮：隐藏 PDF 缩略图、阅读画布、工具栏与分割条，使 ChatGPT 占满工作区域；展开后恢复原论文页码和之前的宽度比例。
5. 不改变原有 PDF、Word、批注、文献库、科研记录、Azure 翻译、SQLite 及 Windows 凭据管理器数据结构。

## 安全限制

- 仅允许信任的 main WebView 调用本地 Rust 命令；远程 ChatGPT WebView 仍没有 Tauri 能力。
- 后端校验 WebView 标签和 HTTPS chatgpt.com 域名，不读取/传出聊天记录或 Cookie；只尝试向空白输入框插入用户明确选择的 PDF 文本。
- 不点击发送、表单提交或调用 ChatGPT 内部 API。
- 没有 OpenAI API 费用；ChatGPT 网页输入方式并非稳定官方接口。

## 实机验收

- 已登录 ChatGPT 的 WebView2 中测试空白输入框自动填入、人工确认发送。
- 输入框原有文字时保持原样，不自动覆盖。
- 登录页和网页布局变化时显示错误并复制回退。
- 分屏隐藏/展开操作不重载 ChatGPT，PDF 页码和滚动状态应合理恢复。
- Word、翻译和备份回归。

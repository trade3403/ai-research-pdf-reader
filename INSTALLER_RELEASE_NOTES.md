# Windows EXE 安装包 — 发布说明

## 当前文件是什么
本文件夹为 Windows 安装包 **可构建并自动发布的完整源码**，不是已经生成的 EXE。

## 无需本机安装 Rust 的方式
将本目录内容上传到你已授权的 GitHub 仓库根目录，GitHub Actions 的 Windows Runner 将自动运行构建。

- 推送到 `main`：Actions → Build and Publish Windows Installer → Artifacts → 下载 Windows 安装包。
- 推送 `v0.1.0` 标签：若 Actions 成功，将生成 GitHub Release，附带 Windows `-setup.exe` 和 SHA256SUMS.txt。
- 项目构建已配置为 Tauri 2 + Rust + React + TypeScript + PDF.js，不需要最终用户安装 Rust、Node.js。
- 必须以 Windows Runner 实际成功构建为准；目前该流程尚未由此环境验证通过。

## 验收要求
1. Release 有 Windows `.exe` 文件（PE MZ 文件头、SHA256 可验）。
2. Windows 10/11 x64 安装成功，界面能启动。
3. 打开 demo/Research_Reader_Test.pdf，确认文字选择、分页、缩略图、缩放、搜索、书签。
4. 脱机加载 PDF 正常（WebView2 Runtime 需存在，或按安装程序策略安装）。

注意：未签名开发版在 Windows SmartScreen 可能提示未知发布者；正式商业发布需代码签名和安装验收。

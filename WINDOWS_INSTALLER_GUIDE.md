# AI Research PDF Reader — Windows 安装包构建说明

本包是 **可构建 Windows EXE 安装程序的完整源码**，不是已编译安装包。

## 方法 A：在自己的 Windows 电脑上生成安装包

### 首次准备（只需做一次）

在 Windows 10/11 x64 上安装：

1. Node.js 22 LTS：https://nodejs.org/
2. Rust（MSVC 版本）：https://rustup.rs/
3. Visual Studio 2022 Build Tools：https://visualstudio.microsoft.com/visual-cpp-build-tools/
   - 必选 **Desktop development with C++** 工作负载
   - 包含 MSVC x64/x86 编译器和 Windows SDK
4. WebView2 Runtime：https://developer.microsoft.com/microsoft-edge/webview2/ （运行安装后的应用需要，Windows 11 通常自带）

安装工具链后重新打开终端或重启电脑，以更新 PATH。第一次构建需要互联网下载 npm、Rust 依赖。

### 生成安装包

解压源码，**双击项目根目录的 `BUILD_WINDOWS_INSTALLER.bat`**。

脚本会自动：

1. 检查 Node/Rust/MSVC 工具链
2. 下载并安装所需 npm 依赖
3. 编译 React/TypeScript 页面
4. 编译 Tauri/Rust 桌面程序
5. 生成 Windows NSIS `-setup.exe`
6. 把安装包复制到项目根目录下 `INSTALLER_OUTPUT/`

初次构建取决于网络和编译器性能，会比后续增量构建慢。**构建成功的必要验收是 `INSTALLER_OUTPUT/` 中存在可安装的 EXE**。

如果失败，请提供 `build-logs/windows-installer-*.log` 以及红色报错，而不只是桌面截图。

### 可选命令行方式

```powershell
npm install
npm run tauri:build
```

Tauri 原始输出位置：

```text
src-tauri\target\release\bundle\nsis\
```

## 方法 B：使用 GitHub Actions 的 Windows 构建环境

本项目包含 `.github/workflows/build-windows-installer.yml`。

1. 在 GitHub 创建一个仓库（可选 Private）。
2. 将**项目内容**上传到仓库根目录（包含 `.github`、`src`、`src-tauri` 等）。
3. 打开仓库 **Actions** 选项卡。
4. 运行 **Build Windows PDF Reader Installer** 工作流（Run workflow）。
5. 构建通过后，从工作流页面 **Artifacts** 下载 `AI-Research-PDF-Reader-Windows-Installer`。
6. 解压 GitHub 的 Artifact ZIP，即可获取 `-setup.exe`。

GitHub Actions 的使用额度、权限和网络由你的 GitHub 账户决定。此工作流不会自动发布 Release，也不会上传你的私人 PDF 文件。

## 安装和验证

生成 EXE 后在 Windows 电脑双击安装。

验证：
- 启动软件
- 点击 **打开 PDF**
- 选择 `demo/Research_Reader_Test.pdf`
- 应显示 5 页
- 测试缩略图、页码跳转、查找、文字选择、缩放、书签、多标签

## 说明

- Windows 安装包默认**未进行代码签名**，Windows SmartScreen 可能会弹出未知发布者提示。仅在你信任自己构建的安装程序时继续。
- 本项目不使用收费 PDF SDK，也不使用 AI API。
- 如果 Tauri 或 PDF.js 的具体依赖版本更新引入不兼容，需要根据构建日志调整。提供的源码未在此 Linux 环境完整编译成 Windows EXE。
- GitHub Actions 源码构建的实际成功与否应以 Windows runner 构建日志及 EXE 安装验收为准。

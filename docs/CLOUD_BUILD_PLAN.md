# GitHub Actions 官方云端编译方案
状态：计划；当前 Tauri 构建仍使用既有工作流，尚未配置 Electron/RPCore CI。
## Runner
全部常规编译使用 GitHub 官方托管 runner，不依赖用户电脑或 self-hosted runner。ubuntu-latest 与 windows-latest 用于 Rust 核心 CI；windows-latest 用于 Electron x64 NSIS 打包；实际发布记录 runner 镜像与工具链版本。macOS 后续按需求增加。
## 计划工作流
| 文件 | 触发 | 任务 |
|---|---|---|
| rpcore-ci.yml | PR/push（核心路径）、手动 | cargo fmt --check、clippy、test、build --locked；Linux/Windows 矩阵 |
| electron-ci.yml | PR/push（应用路径）、手动 | npm ci、类型检查、单测、IPC 安全回归、构建 |
| build-electron-windows.yml | main、手动、经验证的版本发布 | Rust worker release、Electron/Chromium 打包、NSIS EXE、冒烟测试、SHA256、artifact |
| rpcore-security.yml | PR/定期/手动 | 依赖审计、限时 fuzz、恶意样本回归 |
| rpcore-benchmark.yml | 手动/阶段验收 | 固定语料、输出机器及版本、P50/P95、内存报告；托管机器波动需注明 |
工作流在实际 workspace 和锁文件建立后启用，避免空配置假通过。PR 仅最小只读权限，不接触发布密钥，不运行来自不受信 PR 的特权发布任务。构建缓存以锁文件、平台、工具链为键。固定第三方 action 版本或提交，生成 SBOM 与构建来源记录。
## 发布门槛
Windows x64 构建通过、PE 文件校验、SHA256、安装/启动冒烟测试、完整功能与数据回归；构建产物上传 Actions artifact，正式发布由审核过的版本标签/手动发布流程完成，下载链接与提交 SHA 对应。未签名包标注 SmartScreen 风险；签名另行配置。真实用户环境的 PDF 交互、ChatGPT 登录、GPU 性能仍需要实机验收，云端编译成功不代表所有功能通过。公开仓库标准托管 runner 一般免费，遵守平台额度/政策；大型 runner 不默认启用。
## 过渡
保留现有 build-windows-installer.yml 供 V0.5.9 基线使用；原本地 runner 诊断不是新流程依赖，后续迁移时取消自动触发或退役。禁止公开仓库把不受信代码交给私人电脑的 runner。Electron 发布流程稳定后再退役 Tauri 构建。

# D 线路 · RPTranslate / RPInfer / RPModel / RPData

D1.0：独立 Rust workspace；仅 CPU、合成测试数据、无第三方 crate、无在线翻译 API。禁止将尚未训练的模型或固定测试译文当作真实翻译结果。A/B/C 线路和主程序不修改。

## 运行
在 rptranslate 目录执行：
```sh
cargo test --locked --offline --workspace
cargo run --locked --offline -p rptranslate -- self-check
```
首次安装 Rust 工具链需要联网；上述 Cargo 命令不下载依赖。CI 使用标准 GitHub 托管 Ubuntu Runner，限时 10 分钟，不使用 GPU、付费 Runner 或自托管 Runner。

## 里程碑
| 阶段 | 任务 |
| --- | --- |
| D1.0 | 工程、语料元数据、标量数值基准、模型体积预算、CI |
| D1.1 | Safetensors 权重解析及边界检查 |
| D1.2 | SentencePiece/Tokenizer 数值对照 |
| D1.3–D1.7 | 张量、矩阵、SIMD、Attention、Transformer、Beam Search |
| D1.8–D1.11 | 合法开放模型、INT8、术语/公式保护、真实论文评估 |
| D1.12–D1.13 | 小规模训练与断点恢复验证、Windows 工具 |
| D2 | 本地 8840HS / 780M / 64 GB：CPU/Vulkan 基准、教师语料、微调和蒸馏 |
| D3 | 通过质量验收后集成 Electron/RPCore |

## 验收原则
- 每阶段必须有真实可执行测试，不用伪造译文或性能数字。
- 对照成熟框架验证逐层数值；真实论文按文档划分训练与测试，检查数值、公式、漏译、术语及人工评估。
- 语料、教师与学生权重须确认各自许可证、来源及可再分发条件。公共仓库不上传私人论文、凭据、机器本地路径或未授权数据。
- 大模型和训练数据独立管理，不进入应用安装包。目标 50–120 MB INT8 学生权重是待验证目标，不是测量结果。
- 免费 CI 只做有限研发测试；正式训练不使用反复运行的免费 Runner。Windows GPU/NPU 兼容性须在用户机器实测。

D1.0 暂不提供真正翻译、模型加载、训练、GUI 或 GPU 加速。

## 正式研发文档（D 线路）
- [开发规范与技术架构](docs/DEVELOPMENT.md)
- [研发路线与里程碑](docs/ROADMAP.md)
- [研发目标与验收](docs/TARGETS.md)
- [实际开发进度与 CI 证据](docs/PROGRESS.md)

阶段完成以进度文档及对应 GitHub Actions 记录为准。

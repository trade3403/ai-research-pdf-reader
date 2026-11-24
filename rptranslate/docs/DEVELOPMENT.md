# D 线路开发规范与技术架构

状态：D1.0 基线；2026-11-24。本文是研发约定，不代表功能已经实现。

## 1. 范围与分支
D 线路负责离线科研翻译。独立分支 feature/rptranslate，独立 Rust workspace rptranslate/。A（Electron 应用）、B（PDF 核心）、C（Word 核心）独立推进。D 阶段不修改它们；集成通过明确接口和独立 PR，禁止直接覆盖 main 或未经验证发布安装包。

## 2. 四个核心及边界
| 工程 | 职责 | 不承担 |
| --- | --- | --- |
| RPData | 语料来源与许可证、清洗、双语对齐、去重、划分数据集 | 网络上传私人论文 |
| RPInfer | 权重读取、分词适配、张量、Transformer 数值计算、解码、量化和可选硬件后端 | 业务排版与网络翻译 |
| RPModel | 模型元数据、训练配置、教师生成、学生微调、模型评估与权重发布 | 将未训练权重称为可用模型 |
| RPTranslate | 翻译任务编排、上下文、术语/公式保护、段落映射、质量校验 | 直接接管 PDF/Word 核心 |
训练脚本后续独立于 Rust 发布运行时；D1.0 仅有四个 crate 的基础实现。

预期数据流：PDF/Word 文本块（文档 ID、页码/段落、字符范围）→ 语义整理/保护占位符 → Tokenizer → RPInfer 加载 RPModel 本地权重 → 解码 → 恢复/校验 → 带原文定位的译文块 → B/C/A 输出。接口尚未定型，后续采用有版本的 JSON IPC 或 Rust 接口；错误不得以原文或固定译文假装成功。

## 3. 技术路线与自主边界
| 技术 | 用途 | 难度（1–5，主观） |
| --- | --- | --- |
| Rust stable、Cargo | 独立 CPU 核心、测试、交付 | 2 |
| Safetensors | 权重头、dtype、shape、offset、字节序、安全长度检查 | 3 |
| SentencePiece/Tokenizer | Unicode、词表、特殊 token、编码/解码一致性 | 4 |
| Tensor、FP32 标量基准 | shape、stride、矩阵、广播、有限数 | 4 |
| SIMD / AVX2 | CPU 特性检测、可回退标量实现 | 4 |
| Transformer encoder-decoder | embedding、attention mask、softmax、归一化、FFN、cross-attention | 5 |
| Beam search | EOS、长度惩罚、候选排序、缓存、重复性 | 4 |
| INT8 | scale、零点/对称方案、反量化、误差和性能验证 | 4 |
| Python / PyTorch（研发端） | 微调、蒸馏、训练及成熟框架对照 | 5 |
| Vulkan（可选） | AMD 核显推理、算子一致性、CPU 回退 | 5 |
| AMD NPU（研究项） | 可选模型转换和推理 | 5 |
| Electron / RPCore / Word 接口 | 文档结构、翻译显示与导出 | 4 |

自主实现优先覆盖权重解析、数值算子、推理流程、科研业务算法；Candle、PyTorch、llama.cpp 可作对照或研发教师推理工具，不强制进入最终运行包。采用开放标准和预训练权重不等于从零自研模型；基于外部权重微调的模型必须保留来源、许可证和修改说明。训练框架不要求从零重写。GPU/NPU 不作为首版硬依赖，不假设 ROCm 或 CUDA 在 AMD 核显可用。

## 4. 工作流程与测试
每阶段按需求/算法说明 → 实现 → 边界测试 → 对照数值/质量 → CI → 更新 PROGRESS 的顺序完成。
- 只使用标准 GitHub 托管 Runner 进行 D1 的有限测试；不使用付费 GPU、自托管机器或重复运行规避额度。当前 Linux job 限时 10 分钟，离线 Cargo、无第三方 crate；后续增加依赖必须更新锁文件、许可证、缓存策略及离线测试边界。
- 先通过 Linux CI，D1.13 再进行 Windows 标准 Runner 编译和工具 smoke test；不宣称 Linux 成功即 Windows 成功。
- 训练工具通过 D1 验收后才转到本地机器；不自动连接或在用户电脑执行任务。使用本地 GUI/CLI 显式选择数据、目录、预算、开始、暂停/取消和断点恢复；GPU 不可用可回退 CPU。
- 每阶段记录 commit SHA、Actions run URL、真实测试结论、未测范围和下一阶段；不可虚构 PASS、速度或译文。

当前命令（从 rptranslate/ 运行）：
    cargo fmt --all -- --check
    cargo clippy --locked --offline --workspace --all-targets -- -D warnings
    cargo test --locked --offline --workspace
    cargo build --locked --offline --release -p rptranslate
    cargo run --locked --offline -p rptranslate -- self-check

## 5. 安全和数据管理
模型头长度、维度乘积、偏移和内存分配须设上限且 checked 运算；拒绝不支持格式、越界、非有限数据。后续模型包应有 SHA-256 清单、版本、来源、许可和完整性检查，不执行权重携带代码。默认不联网；下载模型由用户明确触发，也支持离线导入。敏感论文、真实训练数据、权重、检查点、密钥不提交公开仓库。合成 fixture 可进入 CI。语料分训练/验证/测试按文档来源隔离，防止重复/泄漏。

## 6. 参考与变更管理
参考方向：Transformer、Marian、SentencePiece、Safetensors、序列级知识蒸馏、量化感知/训练后量化。后续算法阶段需在研究记录列出实际引用的论文、规范版本、公式、适用范围和对照实现，不复制不兼容许可证的源码。路线见 [ROADMAP](ROADMAP.md)，目标验收见 [TARGETS](TARGETS.md)，实际状态见 [PROGRESS](PROGRESS.md)。

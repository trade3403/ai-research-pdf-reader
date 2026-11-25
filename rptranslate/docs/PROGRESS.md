# D 线路实际开发进度

更新：2026-11-25。当前工程版本 0.1.0；D1.0–D1.3 已通过阶段 CI。此文件只记录可核验进度，不将路线目标计为实现。

## 状态总览
| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| D1.0 | 已通过 | 独立工程、基础数值/语料/元数据及 CPU CI |
| D1.1 | 已通过 | 自主 Safetensors 子集解析、参考样本及 CPU CI |
| D1.2a | 已通过 | Unigram Viterbi 内核、identity profile、参考样本 |
| D1.2b1 | 已通过 | SPM protobuf 读取、ID 映射与真实文件探测 |
| D1.2b2a | 已通过 | 自主 compiled charsmap 与真实归一化对照 |
| D1.2b2b1 | 已通过 | 真实 SPM 端到端 ID 编解码对照 |
| D1.2b2b2 | 已通过 | Marian JSON、网络词表 ID、特殊 token |
| D1.3 | 已通过 | FP32 Tensor shape/stride、reshape、permute、广播与基本运算 |
| D1.4a | 已通过 | 标量与分块 FP32 矩阵乘法、数值对照及性能基准 |
| D1.4b | 待验证 | AVX2 自动检测、向量化、标量回退、多线程 |
| D1.5–D1.7 | 未开始 | Attention、Transformer 和解码 |
| D1.8–D1.11 | 未开始 | 真实模型、量化、科研保护和评估 |
| D1.12–D1.13 | 未开始 | 小规模训练和 Windows 工具 |
| D2 | 未开始 | 尚未使用本地电脑训练或测试 GPU/NPU |
| D3 | 未开始 | 未集成 A/B/C、无发布包及实际 GUI |

D1 里程碑完成 4/14（按阶段数统计，不是工作量百分比）。

## D1.0 证据
- 分支：feature/rptranslate
- 代码提交：[0a6e12ee5fadd1f69093decd7d34867e30cf4cb1](https://github.com/trade3403/ai-research-pdf-reader/commit/0a6e12ee5fadd1f69093decd7d34867e30cf4cb1)
- [GitHub Actions run 45415794319](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45415794319)：已完成，结论 success。
- Runner：标准托管 Ubuntu，CPU。
- 已通过工具链检测、cargo fmt、Clippy、单元测试、release 编译、自检。
- RPData：句对必填/长度检查、文本精确去重、来源和许可元数据。
- RPInfer：标量 FP32 向量点积、shape 及非有限数异常检查。
- RPModel：参数量/位宽/理想权重字节预算，错误和溢出检查。
- RPTranslate：命令行 self-check、真实翻译接口显式返回 ModelNotReady。
- 无外部 Rust crate，未使用在线翻译 API；合成 fixture 不是真实论文模型结果。

## 当前限制
尚未加载真实预训练模型（已有 FP32/FP16/BF16 Safetensors 子集读取器），已有受限 Unigram 分词子集（CPU CI 已通过），已通过选定 Marian 模型底层分词样本验收，有基础 FP32 张量引擎（CI 已通过），未实现通用 Marian 可选预处理、神经网络翻译、量化实现、训练程序、真实论文质量测试、Windows 验证、GUI 或 GPU/NPU 加速。未对 A/B/C 线路作修改，未合并 main。没有模型体积、翻译质量和本地性能实测。

## D1.1 阶段要求
实现安全、边界明确的 Safetensors 解析。先确认格式规范，定义头长度和张量数/维度/大小上限，解析元数据及 dtype、shape、offset，支持小 fixture 的 FP32/FP16/BF16，校验数据区域、字节序、越界、重叠、溢出和异常输入。与成熟实现对照；通过 CPU CI 后更新本文件。不要提前宣称可翻译。

## D1.1 实现记录
- 新增 rpmodel::safetensors：有界 JSON 权重索引、FP32/FP16/BF16、按需切片读取、范围检查。
- 官方 Python safetensors 0.7.0 生成合成参考文件；测试含损坏头、重复键、Unicode、空张量、溢出、半精度边界。
- [算法与限制](D1.1_SAFETENSORS.md)。CPU CI 已通过。
- 首轮 [格式检查失败](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45416657427)，已按 rustfmt 修正。第二轮 [Clippy 失败](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45416762689)，已将固定大小 chunks_exact 改为 as_chunks（需 Rust 1.88+），保留全部质量门槛。
- 另用 Python 重述 FP16 转换公式，与标准库 struct 对照全部 65536 种位模式，非 NaN 值逐位一致、NaN 分类一致；此检查不是 Rust 运行结果。

- 验收代码提交：[51bf64cf3fcb52dc0b8124e814d6c8c428cb3606](https://github.com/trade3403/ai-research-pdf-reader/commit/51bf64cf3fcb52dc0b8124e814d6c8c428cb3606)。[第三轮 GitHub Actions 45416917593](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45416917593) 已完成，结论 success；格式、Clippy、单元测试、Release 编译、自检全部通过。尚未验证真实大模型、其他 dtype、分片及性能。

## 当前阶段 D1.2a
新增自主 Unigram Viterbi 分词内核、TSV 词表、identity 空白归一化、特殊 token、11 组由 SentencePiece 0.2.1 实际生成的合成参考结果及重建脚本。[技术文档](D1.2_TOKENIZER.md)。实际 Rust CI 已通过：[代码提交 3e36fc4](https://github.com/trade3403/ai-research-pdf-reader/commit/3e36fc4547d4ffcf445cba772fecfdc0443d8820)，[Actions run 45419019144](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45419019144) 结论 success；格式、Clippy、单元测试、Release 编译与自检通过。D1.2b 尚未完成，不增加整阶段完成数。

## 当前阶段 D1.2b1
新增有界 SPM protobuf 解析、词表与特殊 ID 校验、identity profile 执行适配、独立 Marian ID 映射、实际 SentencePiece 0.2.1 生成的二进制 fixture。真实候选模型文件在当前环境下载失败，新增独立 Runner 探测（不下载权重）。CPU CI 和真实模型文件探测均已通过：[Actions 45427170950](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45427170950)，[代码提交](https://github.com/trade3403/ai-research-pdf-reader/commit/550fa0208d1f6ed8969e03f983c00e94301c1070)。实际 source/target 各 32000 pieces、Unigram、nmt_nfkc、237538 bytes charsmap。文件 SHA-256 见技术文档。完整 D1.2 仍未验收。

## 当前阶段 D1.2b2a
实现有界 compiled charsmap 解析、最长字节前缀匹配及默认空格归一化；新增真实 SPM 官方参考归一化对照测试。[代码提交 e0d7620](https://github.com/trade3403/ai-research-pdf-reader/commit/e0d76202ba43851d2432371a348dac9813ac071c)，[Actions 45508816763](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45508816763) 结论 success；CPU CI 全通过，固定版本三文件哈希一致，源模型与目标模型分别 19/19 归一化完全匹配 Python SentencePiece 0.2.1（合计 38/38）。仅验证默认归一化样本，尚未执行真实 token ID 或模型推理，D1.2 整体仍未完成。

## 当前阶段 D1.2b2b1
接入已验证的 compiled charsmap 与 Unigram，真实 source/target SPM 的 30 组样本各自对照 Python SentencePiece 0.2.1 ID 及 decode。新增离线示例 CLI 和独立参考脚本，保留 19 组归一化回归。[代码提交 0d876b6](https://github.com/trade3403/ai-research-pdf-reader/commit/0d876b64e9ab1428496bdd58e5cb0fbc109cdecd)，[Actions 45511290596](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45511290596) 已完成 success；CPU CI 通过，固定文件哈希一致；source/target 各 ID 30/30、decode 30/30，归一化各 19/19。SPM ID 不等于 Marian 网络 ID，D1.2 整体仍未完成。

## 当前阶段 D1.2b2b2
新增自主共享 Marian JSON 词表、双向 ID 映射、保留未知片段、显式语言标记和 EOS/PAD/UNK 处理；以真实固定模型和 Transformers 4.48.3 的底层 Marian tokenization、特殊 token 构造、skip-special 解码作对照。[代码 ffea623](https://github.com/trade3403/ai-research-pdf-reader/commit/ffea623488c865f28f3386488be1a0bf7250d3c4)，[Actions 45511680357](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45511680357) success。真实共享词表 65001，UNK=1、EOS=0、PAD=65000；网络 ID 与解码分别61/61，SPM ID 与解码分别60/60，归一化38/38。D1.2 选定模型底层样本门槛通过；可选 Moses、added-token splitting、padding/truncation 尚未实现，无神经翻译。

## 当前阶段 D1.3
新增 dense FP32 Tensor、shape/stride、索引、原地元数据 reshape、materialized permute、NumPy-style broadcast、加减乘除、标量与空张量、内存上限及非有限检查。NumPy 2.3.5 在研发环境生成参考数组，首轮 [45515473847](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45515473847) 格式检查失败，按真实日志修正两处格式；[修复提交 243e36c](https://github.com/trade3403/ai-research-pdf-reader/commit/243e36cc6d858006cca96ee4a4423ce06a3e8011)，[Actions 45515567318](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45515567318) completed/success，格式、Clippy、单元测试、Release、自检全部通过。真实 tokenizer 探测按条件跳过（本轮无 tokenizer 变更）；此前 D1.2 真实对照证据有效。[技术文档](D1.3_TENSOR.md)。

## 当前阶段 D1.4a
新增 rank-2 FP32 MatMul、32×32 分块、标量基准、矩形尾块/空张量/异常测试及独立 f64 逐元素参考。新增有限 release CPU 基准（128/256 方阵，3 次中位数）。实际验收：[代码 9515ca5](https://github.com/trade3403/ai-research-pdf-reader/commit/9515ca57909aa9cc4ff64ca3dc8411a065a36af5)、[Actions 45515952092](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45515952092) completed/success。4 项 MatMul 新测试及全部 workspace 35 项单元测试通过（3+11+19+2），格式、Clippy、Release、自检、矩阵基准通过。Runner Intel Xeon Platinum 8573C，4 vCPU，release 3 次中位数：128 方阵 scalar 1.620ms / blocked 0.328ms，4.944x；256 方阵 20.304ms / 2.724ms，7.453x。两路径结果完全相等（exact=true）；仅该次共享 Runner、该合成输入，不代表实际模型或本机性能。未实施显式 SIMD 或多线程，完整 D1.4 待验收。 不增加完整阶段计数。[技术文档](D1.4_MATMUL.md)。

## 当前阶段 D1.4b
新增 x86_64 AVX2 运行时派发、8 lane FP32 内核及尾部、portable-only 编译回退、按输出行分配的有界 scoped threads（1–16），小任务自动串行。保留标量/分块和 FP64 参考；增加线程数/尾块测试，CI 追加强制 portable-only 测试及 128/256/512 方阵四路径实际性能记录。待验收。

## 下一阶段 D1.5
Attention、mask、稳定 softmax 与归一化，先逐算子数值参考测试。

## 文档变更记录
2026-11-24：建立开发规范、路线、目标验收、进度四份独立文档和 README 索引；本次仅文档，不增加算法功能，不替代 D1.0 测试证据。

## 后续每阶段记录模板
- 阶段/日期/状态（未开始、开发中、待验证、已通过、阻塞）：
- 实现与文件：
- commit SHA / Actions run URL：
- 测试环境、命令、实际结果：
- 性能/质量数据及测试样本（未测则标注）：
- 已知限制、回归、未完成项：
- 下一阶段与进入条件：

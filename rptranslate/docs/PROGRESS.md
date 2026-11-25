# D 线路实际开发进度

更新：2026-11-25。当前工程版本 0.1.0；D1.0–D1.7 已通过阶段 CI。此文件只记录可核验进度，不将路线目标计为实现。

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
| D1.4b | 已通过 | AVX2 自动检测、向量化、标量回退、多线程 |
| D1.5a | 已通过 | 稳定 Softmax、单头缩放点积 Attention、组合 mask |
| D1.5b | 已通过 | 多头注意力、LayerNorm、QKV 投影 |
| D1.6a | 已通过 | Embedding、Marian 位置编码、激活与 FFN |
| D1.6b | 已通过 | post-norm Encoder 单层、多层与源 mask |
| D1.6c | 已通过 | post-norm Decoder、causal/self 与 cross attention |
| D1.6d | 已通过 | Encoder-Decoder 联合前向、共享词嵌入、词表 logits |
| D1.6e | 已通过 | 逐层精度与框架参考对照 |
| D1.7a | 已通过 | Greedy/Beam、稳定 log-softmax、EOS 与长度控制 |
| D1.7b1 | 已通过 | self/cross KV cache、增量 logits 与 Beam 分支 |
| D1.7b2 | 已通过 | 独立搜索参考与实际 generation 配置适配 |
| D1.8a | 已通过 | 固定真实权重与元数据核验 |
| D1.8b | 已通过 | 大词表分块读取、共享权重、投影 |
| D1.8c1 | 已通过 | 固定官方权重安全转换与 RPModel v1 |
| D1.8c2a | 已通过 | SHA256、严格 manifest、固定配置 profile、权重索引 |
| D1.8c2b | 已通过 | 类型化配置和 search 参数绑定 |
| D1.8c3 | 已通过 | 真实共享词表、位置权重、bias 和 logits |
| D1.8c4 | 已通过 | 真实六层 Encoder 加载与逐层前向 |
| D1.8c5 | 已通过 | 真实六层 Decoder 与联合 logits |
| D1.8c6 | 已通过 | 真实 KV Cache 与 beam generation |
| D1.8c7 | 已通过 | 完整模型构建与加载验收 |
| D1.8d | 待验证 | 真实跨内核与位置边界 |
| D1.8e | 待验证 | 真实文本翻译 |
| D1.8f | 未开始 | 性能与质量 |
| D1.9–D1.11 | 未开始 | 量化、科研保护和评估 |
| D1.12–D1.13 | 未开始 | 小规模训练和 Windows 工具 |
| D2 | 未开始 | 尚未使用本地电脑训练或测试 GPU/NPU |
| D3 | 未开始 | 未集成 A/B/C、无发布包及实际 GUI |

D1 里程碑完成 8/14（按阶段数统计，不是工作量百分比）。

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
新增 x86_64 AVX2 运行时派发、8 lane FP32 内核及尾部、portable-only 编译回退、按输出行分配的有界 scoped threads（1–16），小任务自动串行。保留标量/分块和 FP64 参考；增加线程数/尾块测试，CI 追加强制 portable-only 测试及 128/256/512 方阵四路径实际性能记录。实际验收：[代码 5bd2c31](https://github.com/trade3403/ai-research-pdf-reader/commit/5bd2c315e418fc82f99e672b37dca70719f54630)、[Actions 45516342377](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45516342377) completed/success。workspace 36/36 测试（3+12+19+2），强制 portable-only 12/12；格式、Clippy、Release、自检、基准均成功。AMD EPYC 9V45，4 vCPU，avx2_available=true。128 方阵 scalar/blocked/auto/parallel4 = 0.724/0.233/0.190/0.158 ms；256 = 12.447/1.811/1.214/0.628 ms；512 = 203.857/15.442/10.731/6.125 ms；三种尺寸四路径 exact=true。512 方阵 parallel4 比标量约33.28x，比同轮 blocked 约2.52x。仅共享 Runner 单次合成基准，未测试用户机器、真实翻译或物理无 AVX2 CPU；编译强制回退通过。D1.4 当前阶段门槛通过；后续按真实模型需要补 batched matmul、线程池和更大张量。

## 当前阶段 D1.5a
新增最后轴稳定 Softmax、预投影 Q/K/V 的 rank-2 缩放点积 Attention、padding/pair/causal mask 组合、增量 causal offset、全遮罩零输出；复用 D1.4 四路径 MatMul，独立 NumPy 2.3.5 FP64 参考。实际验收：[代码 23c9f2a](https://github.com/trade3403/ai-research-pdf-reader/commit/23c9f2af225706c421be6c629b1bc72696027838)，[Actions 45516689570](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45516689570) completed/success；格式、Clippy、workspace 单元测试、Release、自检、强制 portable-only 均通过。五项新增 Attention 测试通过，含独立 NumPy 数值对照、四种矩阵路径、极端 softmax、组合掩码、全遮罩、增量偏移及错误输入。本轮未重跑矩阵基准和真实 tokenizer 探测（条件跳过）；未测真实模型或 Attention 性能。D1.5a 通过，D1.5b 待完成，不增加完整阶段计数。[技术文档](D1.5_ATTENTION.md)。

## 当前阶段 D1.5b
新增 Linear [out,in] 权重布局、Q/K/V 与输出投影、单序列多头 self/cross attention、按头切分与合并、FP64 统计 LayerNorm 及 post-norm 残差。独立 NumPy 2.3.5 FP64 参考实际生成，测试四种矩阵路径、异常形状和全 mask 输出 bias。实际验收：首轮 [45517208297](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45517208297) rustfmt 失败，按日志修正11组格式；[修复代码 9365456](https://github.com/trade3403/ai-research-pdf-reader/commit/93654563fa3193ec06200e1e727286edfecc3995)、[复测 45517274719](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45517274719) completed/success。新增3项测试通过，含 Linear 权重布局、双头 cross attention 与 residual LayerNorm 的 NumPy 独立参考、全 mask bias、形状与极端值检查；格式、Clippy、workspace 测试、Release、自检及强制 portable-only 通过。未重跑真实 tokenizer 或矩阵性能基准；未加载真实权重、未测 Attention 速度。D1.5 阶段门槛通过，D1.6 待研发。

## 当前阶段 D1.6a
新增自主 embedding lookup、显式 scale、Marian grouped sin/cos 位置编码（含奇数宽度、offset、上限）、ReLU/GELU tanh/SiLU、两层 FFN。独立 NumPy 2.3.5 生成位置与 FFN 参考数组；实际验收：首轮 [45517949056](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45517949056) rustfmt 失败，按日志修正两组格式；[修复提交 68b3ae7](https://github.com/trade3403/ai-research-pdf-reader/commit/68b3ae78f04b2e2db76a1eb836d1a0ae2a744ed8)，[复测 45518013150](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45518013150) completed/success。新增四项测试、格式、Clippy、workspace 单元测试、Release、自检、强制 portable-only 全部通过。D1.6a 已通过；D1.6b 待研发，完整 D1.6 尚未验收，不增加整阶段计数。未加载真实模型或测试推理性能。[技术文档](D1.6_TRANSFORMER.md)。

## 当前阶段 D1.6b
新增 EncoderLayer、Norm、1–64 层顺序执行、token embedding 入口、显式源 padding keep mask。实际 NumPy 2.3.5 FP64 合成参考对照首层 attention/post-norm、首层及双层输出，验证改变被 mask 的 token 不影响有效位置。实际验收：首轮 [45518807677](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45518807677) rustfmt 失败，修正13组格式；第二轮 [45518859608](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45518859608) Clippy 指出 FP32 参考常量 excessive_precision，按 FP32 表示修正，不改变测试容差；[最终代码 513bfa1](https://github.com/trade3403/ai-research-pdf-reader/commit/513bfa15639d3a1b96cb8f35b49c3204c3eb38ef)，[复测 45518927770](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45518927770) completed/success。两层 Encoder 参考对照、mask 隔离、embedding 入口和边界测试通过，workspace 格式、Clippy、测试、Release、自检与 portable-only 通过。未加载真实模型、未测推理性能，D1.6b 通过，D1.6 整体尚未验收。

## 当前阶段 D1.6c
新增 DecoderLayer、目标因果 self attention、源 cross attention、各自残差 post-norm、FFN、1–64 层与显式源/目标 keep masks。完整 prefix 重算，未实现 KV cache；不自动 mask PAD（Marian 起始 token 可与 PAD 同 ID）。实际 NumPy 2.3.5 FP64 生成双层参考，比较首层 self/cross/最终输出与第二层输出，验证 prefix、未来 token、源和目标 padding mask 隔离。实际验收：首轮 [45526494539](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526494539) rustfmt 失败，按日志修正9组格式；[最终代码 6d55404](https://github.com/trade3403/ai-research-pdf-reader/commit/6d55404513ed7d0ad5cc9ca050b6dc9f50cd97d0)，[复测 45526552465](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526552465) completed/success。双层 Decoder 数值对照、因果隔离、prefix 一致性、源与目标 mask 隔离、embedding 入口及边界测试通过，workspace 格式、Clippy、单元测试、Release、自检和 portable-only 均通过。未加载真实权重、无 KV cache、未测推理速度。D1.6c 通过，完整 D1.6 尚未验收。

## 当前阶段 D1.6d
新增 Seq2Seq 与 Encoded 源状态、复用源 memory 和独立 mask、共享 embedding 借用、tied/untied LM Head、全序列 logits、末行 logits 和稳定概率。输出 head 连续行点积、不转置整个词表，仍受 Tensor 与单次 MAC 上限。实际 NumPy 2.3.5 FP64 合成模型参考已生成；测试四种矩阵路径、源/目标不同长度、源 mask、prefix 一致性、异常与 untied 权重。实际验收：首轮 [45526919951](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526919951) rustfmt 失败，按日志修正15组格式；[最终代码 0406dc6](https://github.com/trade3403/ai-research-pdf-reader/commit/0406dc6166ae459970ae7db18296864be92299c2)，[复测 45526983338](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45526983338) completed/success。联合前向的 NumPy FP64 参考 memory/hidden/logits/probability、末行投影、源 mask、prefix 一致性、untied 输出权重及边界测试通过；workspace 格式、Clippy、单元测试、Release、自检及 portable-only 通过。D1.6d 通过，完整 D1.6 尚未验收。真实模型加载、KV cache、搜索及推理速度均未验证。

## 当前阶段 D1.6e
新增 CI 专用 tiny Marian 权重桥接及 PyTorch 2.6.0 CPU / Transformers 4.48.3 独立参考，固定随机种子，2 encoder + 2 decoder，hidden=8、heads=2、vocab=17、ReLU、共享权重、非零 bias；三组序列与 mask、四种计算路径。对照 embedding、每层 attention post-norm、FFN、最终 norm、logits、末行 logits、概率及 argmax；记录逐项最大绝对误差和相对误差（分母下限1e-6），容差 atol/rtol=1e-5。实际验收：[代码 dcb854e](https://github.com/trade3403/ai-research-pdf-reader/commit/dcb854e9cdf52141755577a619fd70e55eb03cff)、[Actions 45527441758](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45527441758) completed/success。首轮三组 rustfmt 修正后，基础 CPU CI 与独立 Marian 对照均通过。PyTorch 2.6.0 CPU / Transformers 4.48.3，三组输入×四种 kernel×19个张量=228项，4336元素，最大绝对误差4.76837158e-7，最大相对误差（分母下限1e-6）0.000130250733，logits 最大绝对误差1.78813934e-7；全部满足 atol+rtol*abs(reference) 联合容差，12组 argmax 一致。小尺寸 Parallel 路径可能自动串行，非真实模型、非速度或翻译质量结论。D1.6 合成模型阶段门槛通过。

## 当前阶段 D1.7a
新增自主 search.rs 和 Encoded::generate 入口：beam 1–16、生成1–512 token、词表<=200000、稳定 FP64 log-softmax、partial top-k、确定性 tie-break、EOS、最小/最大生成长度、可选末步强制 EOS、显式 suppress IDs、长度惩罚。采用每步统一保留完成与活动候选的有界策略，非 HF BeamSearchScorer 等价；强制 EOS 保留模型原概率。源 memory 复用，目标前缀重算。[修复提交 40d6083](https://github.com/trade3403/ai-research-pdf-reader/commit/40d6083170ea8065abc0a07ecf41efa161ab26f9)、[复测 45528221709](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45528221709) completed/success。D1.7a 固定概率和合成 Seq2Seq 入口通过；首轮 rustfmt 修正15组格式。完整 D1.7 尚未验收。

## 当前阶段 D1.7b1
新增内部 Cache：每层 self K/V 追加单行、cross K/V 只投影一次并 Rc 共享、不可变父快照、显式目标 mask 与位置 offset、两代 beam 缓存及 256MiB KV payload 预算；新增 cached_logits、generate_cached，保留全 prefix 基准。测试缓存全序列/带 mask logits、1/2/4 beam token 与分数一致，扩展 PyTorch Marian 独立对照 cached_logits。实际验收：[修复代码 08be738](https://github.com/trade3403/ai-research-pdf-reader/commit/08be738a82ef91a8b278fe84546ae0e5712de81e)，[复测 45528484279](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45528484279) completed/success；首轮9组 rustfmt 修正；基础 CPU CI 与独立 PyTorch/Marian 对照通过，240张量、4744元素，整体 max_abs=4.76837158e-7，cached_logits max_abs=1.78813934e-7，1/2/4 beam 缓存与完整前缀一致。未测速度及真实模型。

## 当前阶段 D1.7b2
新增 MarianGeneration 受限生成元数据映射（total max_length-1，min_length 对应 EOS 允许步，start/pad 独立、forced EOS 校验、显式 suppress）。独立 PyTorch 小模型全量候选参考对照 3组源输入×2设置×3 beam×2缓存路径；检查所有返回序列、EOS、累计 logp、长度归一分数，另直接 HF greedy generate 三组对照。独立 Runner 下载固定 OPUS-MT config.json/可选 generation_config.json，记录 SHA256、有效 generation 参数、支持字段映射与未支持 processor 检查；未下载权重。真实 HF beam scorer 不是等价目标，明确 early_stopping/完成池/强制 EOS 评分差异。首轮 Actions 45528748920 基础 CI 通过，独立生成36/36、84/84候选、HF greedy 3/3，max_score_error=2.50345785e-7；独立张量240/240、4744元素、max_abs=4.76837158e-7。但旧 revision 仅含 tokenizer，config.json 404。改用完整固定版本 a4193836671069f1f80ce341f9227a850ffb52d4（2023-01-24，早于 renormalize_logits=true 更新），真实配置复测待验收。 实际验收：[a771f34](https://github.com/trade3403/ai-research-pdf-reader/commit/a771f34bde7e41843e7aa5ba3f8ee35e698b182f) / [Actions 45529283392](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45529283392) completed/success；独立生成36/36、84/84候选、HF greedy 3/3、max_score_error=3.14599827e-7。真实配置 start/pad=65000、EOS=0、beam=4、max_new=511、penalty=1、forced EOS、suppress=[65000]；HF beam scorer 差异保留。D1.7 通过。

## D1.8a 准备（待 D1.7b2 门槛）
新增固定版本真实权重清单探测。前置 generation 核验通过后执行；重新验证旧 tokenizer SHA256，单次权重下载400MiB上限、官方 SHA256、CPU weights_only=True、逐张量检查。D1.8a 实际验收同一 run 45529283392 成功：config.json 1403 bytes SHA256 bcae8ed74fed77fb51c58462b62397fee6b1a1a34aece79183a0dd02ad329e71；generation_config.json 293 bytes SHA256 524dc91570823feb51fbd33a35e5aee3050a4441bdf65e66f6f5e945e5838e40；3 tokenizer SHA256 均匹配。pytorch_model.bin 312087009 bytes SHA256 69a1d6ec829cee349360b3b677ac0aa99a7d88822d1a6578370029efffdca3f5；258 tensors、144569321 logical elements（含共享引用）、312033188 unique storage bytes、133122048 shared embedding bytes；FP32 finite、配置及主要 shape 检查通过。真实权重尚未由 Rust 运行。

## 当前阶段 D1.8b
新增 RowTable/ChunkedTable，有界分段读取 Safetensors（复用内部 rpmodel），共享 Encoder/Decoder/LM Head，单块<=1M元素，单表<=512MiB且调用者预算、保持 Tensor 64MiB。合成65001×512全词表 FP32、跨块查找与全词表投影、异常与小 Safetensors fixture，独立 PyTorch 合成模型改用分块表对照。实际验收：[f6891e2](https://github.com/trade3403/ai-research-pdf-reader/commit/f6891e22a8c3d12f796b840d3d792b9cf0db23bd)、[Actions 45529834798](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45529834798) completed/success。9组 rustfmt 修正后 workspace 61/61、portable 37/37、真实尺寸合成词表65001 logits、独立框架240张量4744元素、生成36组84候选3 HF greedy全部通过。尚非真实模型推理。

## 当前阶段 D1.8c1
离线 converter 固定官方 checkpoint 与5个元数据/分词 SHA256、严格258名称及 shape/FP32/finite，weights_only=True，2个共享词表别名逐位检查后去重；256 canonical tensors、原始位置表及 [1,65001] bias 保留，标准 Safetensors + RPModel v1 manifest，临时目录转换成功才发布到新本地目录。专用 CI 下载一次并验证官方 Safetensors 读取和自主 Rust 全量分段 FP32 位对照。D1.8c1 首轮 [45530926188](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45530926188) CPU CI 与真实权重校验通过，但 safetensors.torch 缺 packaging，转换失败；固定 packaging==24.2 后保留原 roundtrip 门槛复测。 待验收。

## 当前阶段 D1.8c2a
自主 SHA256 流式校验、受限 JSON manifest 解析、严格来源/版本/文件白名单/别名/shape/dtype/metadata，固定官方5个配置和分词文件原始字节哈希，256个权重索引校验；真实转换通过后同一 Runner 执行完整包验证、同大小配置损坏/版本错误/权重哈希错误反例。仅支持固定 revision，非通用 Marian JSON 配置解析，manifest 自述权重哈希不代表签名或真实性；实际 CI 待验收。保留 D1.8c1 逐位验证，未执行真实推理。

实际验收：[eb0ee44](https://github.com/trade3403/ai-research-pdf-reader/commit/eb0ee44471a6ca3bdc6dfa65dda3b07836c19859) / [Actions 45531555536](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45531555536) completed/success；workspace 64/64、portable 37/37，补 packaging==24.2、23组 rustfmt 后通过。真实转换256 tensors、2 aliases、312033188 payload bytes、312062636 file bytes；78008297 FP32 元素逐位一致，canonical stream SHA256 ca02d9887eb51fa34135fed6093fcee0660f0836e4ad6959b30c01ecb519da82；自主包校验与3个损坏反例通过。D1.8c1/c2a 已验收。

## 当前阶段 D1.8c2b
新增有界 JSON config 解析（64KiB、8层、单对象/数组128项、重复键/错误数字/非有限拒绝），类型化固定 Marian architecture + Generation，严格结构和特殊 token 参数、单 token bad_words、未知 generation 字段拒绝，显式默认 min_length=0、penalty=1，禁用 sampling/renormalize/early_stopping。package::open_profile 在固定文件哈希及权重索引验证后读取真实配置，MarianGeneration::from_profile 绑定自主 SearchConfig。仍仅支持固定审计 profile，不是通用 Marian 加载器，未装载真实神经网络权重执行推理。真实转换、逐位、包损坏反例和类型化配置专项 CI 待验收。

D1.8c2b 首轮 [45531897761](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45531897761) 整体失败（基础格式检查），真实模型专项成功：MARIAN_PROFILE PASS vocab=65001 hidden=512 6+6层 heads8 FFN2048 positions512 Swish scale=22.627417 tensors=256；MARIAN_SEARCH_PROFILE PASS start/pad=65000 EOS=0 beams4 max_new511 penalty1 forced_eos=true suppress=[65000]；真实逐位 roundtrip 78008297元素仍通过。本次修复14组格式后复测，完整 c2b 待验收。

## 当前阶段 D1.8c3
新增 MarianEmbeddings 自主实际权重加载：32块共享词表、两个真实位置表、bias [1,65001] 无复制 reshape 为 [65001]，预算135479204 bytes，预算不足预先拒绝，保留 Tensor 上限。实际 checkpoint 位置表直接 gather（不重新计算），共享 table 做 token embedding 和 tied LM Head；提供后续 Encoder/Decoder adapter 的借用接口，未改变现有合成模型结构。独立 PyTorch 原始权重参考，Rust 逐行输出全部加载词表及位置/bias 位对照，8个跨块/特殊 ID，两个方向3种位置 offset（含504），3行全65001 logits和末行投影（4行 argmax），atol/rtol=1e-5，额外预算与ID/位置异常。CI 待验收，尚无真实 Encoder/Decoder 层前向、译文、速度或总 RSS 测量。

D1.8c3 复测 [45532454659](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45532454659) 失败：格式检查通过，但自动格式补丁误删 verify_marian_profile.rs / real_embedding_reference.rs 的 rpmodel::package 导入，编译 E0433；真实文件转换、78008297 元素逐位一致与包损坏3反例仍通过，配置与 embedding 对照未执行。本轮恢复两个导入，原门槛不变，待复测。

## 当前阶段 D1.8c4
新增 MarianEncoder 真实6层96张量拥有型加载与借用视图，Q/K/V/out、8头 self attention、post-norm、2048 FFN、SiLU/Swish、实际位置表、源 key mask；独立 encoder 权重预算75657216 bytes，预算不足预拒绝，借用共享词表不复制；复用 OwnedLinear/Norm/Attention/Ffn 供后续 Decoder。独立 Transformers4.48.3 / PyTorch2.6.0 CPU eager 真实模型，3组 token 序列（含跨块、PAD mask）每组 input、6层各 attention/final norm、最终 memory，42个张量，预设 atol/rtol=1e-4（真实512/2048宽 FP32 累加差异；原 embedding 1e-5 门槛不变），public forward 与 trace 精确一致；真实 Encoder 专项仅在 c2b/c3 验证通过后运行。尚无测试结果、译文或性能实测。

D1.8c2b/c3/c4 [45532734293](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45532734293) 真实专项全部成功：配置及生成绑定通过；真实32块33869801权重元素逐位一致、9组288676计算元素、max_abs=1.78813934e-6、4/4 argmax；真实6层 Encoder 96 tensors、75657216 bytes、42组164864元素、max_abs=1.52587891e-5、memory max_abs=1.9762665e-6。整体 CPU format 失败，未完成全门槛验收。本批采用完整上下文补丁（含纯新增行）修复，确保保留 package imports。

## 当前阶段 D1.8c5
新增 MarianDecoder：156层张量、100896768 bytes 权重预算，真实 causal self/cross attention、FFN/Swish、post-norm、实际位置表，复用共享 embedding，前缀重算、全/末行 tied logits。测试3组 source/target（含 source PAD 和 target key mask/start），真实 HF MarianMTModel eager 逐层3个 norm、memory、input、hidden、完整词表及末行 logits，共69张量、13行 argmax、预设 atol/rtol=1e-4，额外 causal 首行不受未来 token、预算和非法 mask 检查；原 c3/c4 门槛不变。总纯权重312033188 bytes，非实测 RSS；尚未真实缓存或生成译文。待 CI。

实际验收：[5fc2a97](https://github.com/trade3403/ai-research-pdf-reader/commit/5fc2a973ca96be8750f7893e2b210235f98624e5) / [45533198998](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45533198998) completed/success；workspace69/69、portable39/39，真实配置、包及逐位转换复测通过；embedding 33869801位一致、288676计算元素、max_abs=2.02655792e-6；encoder 42/42、164864元素、max_abs=1.90734863e-5、memory max_abs=2.28732824e-6；decoder 69/69、958165元素、max_abs=4.19616699e-5、logits max_abs=2.67028809e-5、argmax13/13。D1.8c2b/c3/c4/c5 全门槛通过。

## 当前阶段 D1.8c6
新增真实 MarianEncoded：借用同一真实权重/encoder memory、实际 decoder position row 增量输入、6层 self KV immutable snapshot、共享 cross KV，复用原 Cache 256MiB KV payload 门槛与原 search 策略（非 HF beam scorer 等价），支持 full/cached、显式 target mask 保留 start，真实 config 派生 beam 参数。真实 CI 2组 token prefix 含 source/target mask、full/cached 全词表 logits 对照原始 PyTorch（4组910014元素），2组 source ×beam1/4×full/cache=8生成设置，max_new4、原模型 EOS/suppress/penalty、独立全候选扩展与精确序列/EOS/排序、分数 atol/rtol=1e-4、2组 HF greedy 对照；原门槛不变。仅短 token 序列测试，未测长生成速度、真实译文、RSS，cache 仍复制 self KV 前缀而非分页。待 CI。

实际验收：[ab6c838](https://github.com/trade3403/ai-research-pdf-reader/commit/ab6c838e387b09c70c9e627b781e0001fb4876d9) / [45533581675](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45533581675) completed/success；workspace69/69、portable39/39；真实生成2组、4 logits comparisons/910014元素、8生成设置/20候选、HF greedy2/2，max_abs=2.7179718e-5、cache_diff=0（测试样本）、max_score_error=1.01827798e-5；所有前序真实门槛通过。D1.8c6 已验收。

## 当前阶段 D1.8c7
新增拥有型 MarianModel：一次验证包、加载256 canonical tensors（唯一共享表）、总权重预算312033188 bytes、提前拒绝低预算，关闭 reader 后纯内存模型；TokenSession 拥有 source memory 与复制/自动 PAD mask，借用不可变模型、每次生成 request-local cache；统一 encode/logits/cached_logits/generate_tokens，默认官方 beam4/max_new511/cache，显式选项 beam1..16、max_new1..511、Full/Cached，保留官方特殊 IDs/EOS/suppress/penalty。真实独立生成参考改用统一接口（原门槛不变），新增真实生命周期 probe：预算、非法 source/mask/length/options、2 session 独立、自动 PAD 与显式 mask 一致、full/cache logits、重复/直接生成及 drop/reload 重现。9个负例，4次生成，260004 logits元素。权重预算不含 KV/临时/RSS；drop/reload 仅功能验证，不宣称 RSS 释放量或速度。暂不接 tokenizer 文本入口；固定审计 profile，不是通用模型 loader。待 CI。

实际验收：[fa9c9e4](https://github.com/trade3403/ai-research-pdf-reader/commit/fa9c9e46d4c7ef3c37b42341750986bc062207ae) / [45533989873](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45533989873) completed/success，workspace70/70、portable40/40；REAL_GENERATION_REFERENCE PASS cases=2 logits_comparisons=4 elements=910014 generation_checks=8 hypotheses=20 hf_greedy=2 max_abs=2.7179718e-05 cache_diff=0 max_score_error=1.01827798e-05 atol=0.0001 rtol=0.0001；FULL_MODEL PASS tensors=256 payload_bytes=312033188 sessions=2 generation_calls=4 invalid_cases=9 reload=true logits_elements=260004。D1.8c7 已验收。

## 当前阶段 D1.8d
D1.8d 新增实际模型跨内核验收程序：真实权重、2组短源/目标及PAD mask，Scalar/Blocked/Auto/Parallel4 四内核，native 明确要求 AVX2 available、portable-only 明确 false；两模式各24个 memory/full/cached logits 张量对照独立 HF eager，16组生成设置（beam1 full/beam4 cached）及40候选序列、排序、EOS、分数对照穷举参考。native 另加64源/16目标 Auto/Parallel4 6张量；512源与512目标 Auto 完整 hidden forward、仅末行 logits 两张量；4项长度/全词表 tensor/KV 预算拒绝。合计 native32、portable24张量；统一 atol/rtol1e-4 不改变前序门槛。512边界不代表511步完整生成，不做512行全词表 logits（超过 Tensor 上限），不测速度或RSS，尚未文本翻译。所有测试待CI。

D1.8d [45534413835](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45534413835) 基础 CI 已通过，真实跨内核专项尚待验收；本批保留全量前序门槛，不宣称已验收。

## 当前阶段 D1.8e
新增 OfflineTranslator/借用式 TextSession：固定来源的 source.spm/vocab 原字节独立 SHA256 校验，真实词表65001和特殊ID校验，自主 NmtUnigram + 固定 >>cmn_Hans<< + EOS → MarianModel cached beam → Marian 共享网络词表反分词（target.spm 已在包校验，运行不需重新解析 target SPM）；明确拒绝空、调用者语言标记、超长，不截断，返回 source/target IDs、score、reached_limit。新增 rptranslate translate MODEL_DIR TEXT [BEAM_WIDTH] [MAX_NEW]；保留无模型历史入口显式报错。CI 三句真实英文、beam1/4 六次生成、精确源/目标 token、解码对照、3 HF greedy、独立自定义 beam 参考、中文字符及自然 EOS、CLI、3负例。参考工具 Python 仅CI；Rust runtime 无网络请求，未测速度、真实语料质量；仅固定官方 FP32 profile，非自主训练权重。待CI，不预写译文。

## 下一阶段 D1.8f
真实英中翻译性能、内存及质量验收。

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

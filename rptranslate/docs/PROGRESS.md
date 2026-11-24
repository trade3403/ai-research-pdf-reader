# D 线路实际开发进度

更新：2026-11-24。当前工程版本 0.1.0；D1.0、D1.1 已通过 CI。此文件只记录可核验进度，不将路线目标计为实现。

## 状态总览
| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| D1.0 | 已通过 | 独立工程、基础数值/语料/元数据及 CPU CI |
| D1.1 | 已通过 | 自主 Safetensors 子集解析、参考样本及 CPU CI |
| D1.2a | 已通过 | Unigram Viterbi 内核、identity profile、参考样本 |
| D1.2b | 未开始 | 真实 SPM、归一化及 Marian ID 兼容性 |
| D1.3–D1.7 | 未开始 | Tensor、SIMD、Transformer 和解码 |
| D1.8–D1.11 | 未开始 | 真实模型、量化、科研保护和评估 |
| D1.12–D1.13 | 未开始 | 小规模训练和 Windows 工具 |
| D2 | 未开始 | 尚未使用本地电脑训练或测试 GPU/NPU |
| D3 | 未开始 | 未集成 A/B/C、无发布包及实际 GUI |

D1 里程碑完成 2/14（按阶段数统计，不是工作量百分比）。

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
尚未加载真实预训练模型（已有 FP32/FP16/BF16 Safetensors 子集读取器），已有受限 Unigram 分词子集（CPU CI 已通过），没有完整 Marian 分词兼容性、张量引擎、神经网络翻译、量化实现、训练程序、真实论文质量测试、Windows 验证、GUI 或 GPU/NPU 加速。未对 A/B/C 线路作修改，未合并 main。没有模型体积、翻译质量和本地性能实测。

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

## 下一阶段 D1.2b
真实 SPM protobuf、目标模型 normalizer、Marian vocab.json 与 token ID 对照。

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

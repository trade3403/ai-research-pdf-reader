# D 线路实际开发进度

更新：2026-11-25。当前工程版本 0.1.0；D1.0–D1.7 已通过阶段 CI。此文件只记录可核验进度，不将路线目标计为实现。

## 最新验收记录（优先于历史状态表）

2026-11-26 D1.11 [#112 / 45664250887](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45664250887) CPU Format 失败，[实际日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45664250887/job/139381965753) 七组差异：document_reference.rs:13、document.rs:64/153/176/186/192/218。严格按 rustfmt 日志修复，不改变算法、覆盖审计及验收门槛，不删除历史测试。真实专项查询时仍运行（术语词典验证），未取消；尚未取得全文审计数据。待修复版全量验收，不提前进入后续阶段。

2026-11-26 D1.9l 基线 36b5dec95f5eaeaa67a0ab720448eb96071fecb7 / [#111 / 45657831972](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45657831972) 全量 success；134 workspace + 56 portable=190/190。FP32六会话重建+编码167.803611ms→0.681986ms，INT8 167.690960ms→0.659993ms，token及跨会话A/B/A译文完全一致；只是会话/分词非整个推理加速。真实论文原普通INT8 269片段6036词、156.340300s、数字字符串287/317、capped0，8段严格审计4通过4拒绝，仅词面。用户授权D1.11首批：新增 Rust document API，严格递增稳定ID、2MiB输入/1024块/64KiB每块、8MiB接受文本总量上限，预检全部输入；同一resident session逐块执行已有paragraph审计，保留每块明确Result和UTF-8局部范围；验证接受结果无非空白缺口/重叠/越界且译文拼接一致，绝不以原文或部分译文冒充成功。4新增测试（顺序拒绝、预检、UTF8覆盖、无效成功）。真实固定SHA论文沿用既有269提取片段（标题/摘要/关键词/正文/图注；排除表格单元、参考文献、图形、附录；不是整个PDF），INT8全量、FP32固定8样本，Python独立检查ID/范围/拼接/数字多重集、记录拒译类别与词覆盖率、策略/耗时/RSS、同样本结果。结果留Runner临时目录，不新增全文译文日志或上传；保留历史CI，D1.11条件job上限40分钟，新增INT8/FP32探针各15/4分钟硬超时，标准免费CPU Runner；无本地Rust编译，待CI。工程验收要求每块明确结果及至少一块接受，不是质量合格门槛；没有人工参考译文，严禁声称语义准确率，低覆盖/事实错误须继续修正或标记发布阻塞。D2独立，D1.12训练验证是否保留待用户明确；不提前启动。

2026-11-25 D1.9l [#110 / 45654959744](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45654959744) CPU Format 失败，[实际日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45654959744/job/139349025884) benchmark_translation.rs:41 两处 tokenizer_asset 语句换行；按 rustfmt 日志逐字修复，不改变常驻模型、分词器复用、性能测试或所有验收门槛。真实专项查询时仍运行，未取消。待新提交全量验收，不提前进入D1.11。

2026-11-25 D1.9k 基线 7ea84ccf4be6e02c1a8706b7a2cea3265c72c88d / [#109 / 45652036320](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45652036320) 全量 success；133 workspace + 56 portable=189/189。真实映射97矩阵159辅助全部位级一致，普通564.402ms、映射566.768ms（未加速，不默认启用）；FP32/INT8加载1209.561/639.918ms，跨runner仅观察。用户授权D1.9l：既有OfflineTranslator已经持有模型及Marian词表，但每次session重新构建SPM。CharsMap改为Cow，保留原借用解析API，新增into_owned；NmtUnigram拥有型转换，无自引用或unsafe。OfflineTranslator加载一次编译分词器并持有，释放原SPM，TextSession只借用模型、词表、分词器，原session Result API不变。每次生成仍新建source-specific encoder/KV，禁止跨请求复用状态；不增加全局模型缓存、不开发后台服务。新增拥有型map释放源后回归；扩展两个真实benchmark解析器，FP32/INT8六组旧重建与新session的token精确对照、跨session A/B/A译文完全一致、非法空输入拒绝，分别测重建+编码与借用+编码耗时；旧参考临时词表与分词器使此次benchmark RSS与旧测量口径不同，不宣称模型常驻内存降低；session耗时提高输出精度。保留所有历史验收，待CI，不提前进入D1.11。D2独立。

2026-11-25 D1.9k [#108 / 45650243619](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45650243619) CPU Format 失败，[实际日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45650243619/job/139332476838) 四组差异：mixed_model.rs 两行、lib.rs 模块排序、quantized.rs 迭代器换行、sha256.rs 错误表达式。按 rustfmt 日志逐字修复，不改变算法、默认加载方式、内存安全约束及所有测试。真实专项查询时仍运行，未取消。待新提交全量验收，不提前进入D1.9l。

2026-11-25 D1.9j 基线 c59091649e8deffb20f04c0eb395f742f9be25ea / [#107 / 45644239731](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45644239731) 全量 success；131 workspace + 56 portable=187/187，真实三文件 Rust旧/新/hashlib 摘要一致；FP32 1403.670→1372.914ms、RPQ8 372.722→365.572ms、辅助12.604→12.391ms，加载FP32/INT8 1816.819/989.615ms，跨runner仅观察。用户授权D1.9k首批：自主Linux 64位 mmap RAII只读映射，长度/文件类型/预算校验，显式 unsafe 调用者必须保证文件全程不可变且不截断；默认安全文件读取不改，不向正式CLI暴露。直接slice SHA256与RPQ8映射解包入最终数组，统一manifest、索引、语义、配置和预算校验；内部RPQ8校验仍走原Reader，辅助F32仍解码复制。现有RPQ8交错布局不能直接作为分离的推理数组，非全模型零拷贝或常驻mmap，Windows/其他系统保留原路径，后续D1.13验收；需设计安全不可变安装策略才可默认启用。2新增单测（映射预算/字节、slice尾块），扩展SHA边界；真实97矩阵/159辅助全量位级对照、4篡改分别拒绝和同runner顺序耗时，仅测试私有不可变fixture，不设速度门槛。未本地编译，待CI；不提前进入D1.9l；D2独立。

2026-11-25 D1.9j [#106 / 45641446001](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45641446001) CPU Format 失败，[实际日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45641446001/job/139300867217) 仅 sha256.rs:191 assert_eq! 需换行；按 rustfmt 日志逐字修复，不改变算法、SHA校验或测试。真实专项查询时仍运行（科学保护翻译步骤），未取消，尚未取得真实SHA测速完整日志。待新提交全量验收，不提前进入D1.9k。

2026-11-25 D1.9i 基线 766128c38e98cec3c4da3f213e99bb856a9d67b1 / [#105 / 45635122116](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45635122116) 全量 success；129 workspace + 56 portable=185/185。真实97张量77320704 codes/1208136 scales 位级一致，旧2010.920ms、新67.815ms（同runner顺序、非冷缓存、不含SHA），FP32/INT8完整加载1843.237/985.301ms，RSS331844/107440KiB；论文8样本词面4通过4拒绝。#104 旧1045.689ms、新61.177ms，加载1733.189/925.477ms，跨runner比较仅观察。用户授权 D1.9j：默认自主SHA256改用64KiB连续读取，直接按64字节压缩，跨短读仅保留0–63字节尾部，保留原FIPS压缩/补位/长度上限、Interrupted/IO错误及完整哈希门槛；旧逐块BufReader作为回归与计时参考。新增2组Rust测试：11个边界长度、7种短读步长及Interrupted、限制/Take/IO错误；FP32权重、RPQ8和辅助FP32真实文件对比旧Rust及Python hashlib摘要，报告两路径读取+哈希耗时（先旧后新，不是冷缓存，不设未证实速度门槛）。不涉及SHA-NI、mmap或减少校验次数；没有本地Rust工具链，待真实CI，不提前开始D1.9k。D2独立。

2026-11-25 D1.9i [#104 / 45632499613](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45632499613) CPU Format 失败，[实际日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45632499613/job/139269142368) 仅 mixed_model_reference.rs:78 命令行 usage 字符串需换行；按 rustfmt 日志逐字修复，算法、预算、SHA校验和所有测试不变。查询时真实模型专项仍运行，已进入科学保护翻译步骤；不据此虚报批读性能，待完整日志及新提交全量CI，禁止提前启动D1.9j。

2026-11-25 D1.10h 基线 62e27a980b248e7cb0e8d01b124b37dbf02a7ef6 / [#103 / 45626514273](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45626514273) 全量 success；127 workspace + 56 portable=183/183，双精度固定段落、数值审计及预算负例通过。真实论文8样本通过词面4、拒绝4，较前版6/2更严格，不代表语义质量提高；后续D1.11仍须人工评估。用户授权 D1.9i：RPQ8 Reader::read_tensor_into 单张量一次seek、64KiB固定scratch按完整记录批量读取（含末尾短块），直接填入预分配codes/scales，每块沿用原语义校验，错误丢弃部分写入的数组。QuantizedTable::load 接入默认混合模型；保留原 read_block 和所有 SHA/格式/预算门槛。新增2组Rust测试（跨batch/尾块位对照、精确长度/名称和重签哈希后语义坏块拒绝）；真实97张量77320704 codes/1208136 scales逐位新旧对照及分路径时间观测。计时同runner先旧后新，非冷缓存、不含SHA，不设未证实的速度门槛；端到端加载仍由既有配对基准观测。未本地编译，待CI。D2训练/蒸馏独立项目不计入主线；不改其他线路，不启动D1.9j。

2026-11-25 D1.10g 基线 2fa70c175de7a25415489ecb12ffe3d09485f777 / [#102 / 45620641204](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45620641204) 全量 success；124 workspace + 56 portable = 180/180，数学12组/格式17拒绝，FP32/INT8各2段4片段、1数值审计、1预算负例。论文8样本成功6拒绝2，Whole1/Constrained0/ProtectedGaps5，仅词面。实际日志仍有缩略语、联合国日刊等无原文依据的疑似噪声，不能称语义准确。用户授权 D1.10h 首批：在现有段落候选审计加入窄范围原文证据检查（缩略语前缀、空白归一化联合国日刊）及百分号前附着正负号绑定，保留原审核/回退/拒绝，不改写译文、不改默认翻译。新增3组 Rust 回归，增强真实双精度数值专项的无缩略语门槛，保留接受或明确拒绝协议和所有历史CI。启发式可能误拒绝合法译文或漏检其他幻觉，未证明覆盖率提升，后续仍需人工语义评价。新提交待全量CI，未本地编译、不启动加载优化。

2026-11-25 D1.10g 最新：[#100 / 45615115595](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45615115595) 真实模型专项 success，CPU Clippy 失败已等价修复；[#101 / 45616691138](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45616691138) CPU [真实日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45616691138/job/139221264635) 显示 paragraph.rs:254 的三行布尔表达式因上次缩短后须合并为一行，按 rustfmt 日志逐字修复；不改变算法、不降低 Clippy 标准、不删除测试。#101 真实模型专项仍运行，未取消。新提交全量待验收，禁止提前推进后续阶段。

2026-11-25 D1.10g 最新：[#98 / 45613637482](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45613637482) 真实模型专项 success（FP32/INT8 各2段4片段、1数值审计、1负例；论文8样本成功6拒绝2，Whole1/Constrained0/ProtectedGaps5，仅词面，不代表语义准确率），CPU fmt 失败已修复。修复版 [#100 / 45615115595](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45615115595) fmt 通过，CPU Clippy [实际日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45615115595/job/139211191916) 在 paragraph.rs:258 报 needless_character_iteration；按日志将 suffix.chars().any(|c| !c.is_ascii()) 等价改为 !suffix.is_ascii()，不改变算法、不加 allow、不删测试。#100 真实专项仍运行，未主动取消。#99 已取消。保留所有历史专项触发标记，新提交全量待验收，禁止提前推进后续阶段。

2026-11-25 最新：D1.10f [#97 / 45582922442](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45582922442) 已直接核验 success；CPU 日志 122 workspace + 56 portable = 178/178，INT8 数学 12 组、RPQ8 格式 17 个拒绝，FP32/INT8 各2段4片段、1数值审计、1负例通过。D1.10g [#98 / 45613637482](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45613637482) 的 [CPU 日志](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45613637482/job/139203295512) 确认 paragraph.rs:526 两处 assert rustfmt 换行失败；Clippy/测试未执行，真实专项查询时仍运行。按日志仅修正两处断言格式，不改变算法和测试。最新 [#99 / 45614479395](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45614479395) 查询时 pending，尚无 jobs 或 commit status，未证明 Token 错误。现可使用 GitHub fetch GET https://api.github.com/repos/trade3403/ai-research-pdf-reader/actions/runs?branch=feature%2Frptranslate&event=push&per_page=5 直接获取 push Run ID；再用 /actions/runs/{id}/jobs 及 job logs 查询。不要使用仅返回 PR 的 commit workflow runs 包装器。状态回传已无必要，本次移除两个新增状态任务，工作流与 af64f5e 版本完全一致，保留原验收命令、条件、权限和全部提交触发标记。未取消已有 CI；新提交全量待验收，不开始下一阶段。

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
| D1.8f | 待验证 | 实际性能/RSS/质量基线 |
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

D1.8e [45534854851](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45534854851) 首轮基础 format 3组失败，真实专项尚未完成；D1.8d 独立运行被后继提交取消，仍需跨内核全量验收。此次修复完整上下文格式，不删除已有检查。

## 当前阶段 D1.8f
新增独立 Rust release 翻译基准：模型加载/分词会话初始化计时，2短句 beam1、Full/Cached 各3次交替计时（2 warmups 排除），目标序列严格一致；端到端 latency/输出Token/s（含EOS，非Decoder纯速度），median 与 full/cache latency ratio；Linux /usr/bin/time 单独 Rust 进程峰值RSS（含权重、warmup、质量请求，非Python）。6条自编科研短句 beam4、参考中文、数字多重集合匹配5项、术语/单位字符串备选12组、字符1..6 gram F2有效阶宏平均（非SacreBLEU），实际译文与上限标志逐条日志。质量观察不设虚构通过率或速度阈值，数字匹配不证明数量关系正确；非真实论文质量结论。先保留D1.8d/e全部门槛，之后执行基准。待CI。

D1.8f [45535270286](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45535270286) 基础 format 2组失败，真实专项仍运行，未取得完整跨内核/译文/性能数据；本次修复2组差异，并将本分支CI concurrency cancel-in-progress 改false，避免反复中断旧验收。新提交含D1.8f标记，保留原全部门槛。D1.9a 数学模块先行独立预研，正式进入量化模型研发须等D1.8d/e/f全部通过。

## D1.9a（独立预研，待验收）
D1.9a 独立数学预研：BlockInt8 对称零点0、[-127,127]，按连续块 max_abs/127 FP32 scale，f64 中间、ties-to-even、尾块、全零 scale=1、极小 subnormal 与最大有限值安全处理；拒绝 -128、非有限/非正 scale、异常 shape/长度/预算/重建溢出，支持无复制 from_parts、有界局部反量化。块长1..4096、单实例最多16Mi elements，payload=len+4*ceil(len/block)。5组Rust单测、独立Python stdlib 12组位级对照 codes/scales/restored，误差界包含最终FP32舍入。尚不接真实模型、不改FP32计算路径、不代表量化推理或实际模型体积。

D1.8f 45535270286：原生/便携跨内核、512位置、真实文本翻译步骤均 success；性能基准仍运行，基础格式此前失败。D1.9a 45535708639 尚待执行。保留运行中任务；新提交复跑 D1.8f/D1.9a 全量门槛。

## D1.9b（独立预研，待验收）
D1.9b 独立格式预研：RPQ8BIN v1 小端，32字节头(magic/version/count/index_bytes/payload_bytes)，排序名称、rank/dims/block size/offset/bytes，块记录FP32 scale+signed codes，尾部64 ASCII SHA256覆盖全部头/索引/载荷。文件<=512MiB、索引<=1MiB、<=4096 tensors、rank1..8、每张量<=128Mi元素、block1..4096；严格连续偏移/长度，无重叠/空洞/重复/路径名/尾随字节。Reader验证哈希和索引后按块读取，逐块检查 -128、scale有限正值、重建溢出和预算；未读取块仅做完整性哈希，不提前验证语义。Rust writer 接收借用已量化 codes/scales，先验证再写空目标，流式写入、回读计算SHA；IO失败需丢弃不完整输出，非原子替换。3 Rust 单测、独立Python字节精确writer对照/reader三块/17损坏反例。SHA非签名，假设稳定文件。仅INT8容器，未支持混合FP32完整模型、未转换真实权重、未接推理。待CI。

前置D1.9a 45535708639 基础格式失败已在c6520e4修复，真实专项仍运行；D1.9b复测45536217953排队。新提交保留D1.9b/D1.9a/D1.8f所有门槛，CI允许旧运行完成；待最新全量验收。

## D1.9c（待验收）
D1.9c 独立真实量化词表：QuantizedTable 只保留紧凑 codes/scales，行对齐 block 整除 width；FP32 reader 每次最多1Mi元素按行转换，RPQ8 writer借用原数组，释放后从RPQ8逐块检查并加载；row_into 一行反量化、按ID gather/scale、共享词表 tied projection FP32 累加，最多一行scratch，未变更原RowTable借用API与FP32翻译路径。实际65001x512 block64目标33280512 codes、520008 scales、35360544 bytes，2组单测。真实专项全权重代码/scale 独立NumPy位对照、8 token gather4096元素、2行全logits和末行195003元素，1e-5对照量化参考、FP32差异按绝对误差传播界检查并记录argmax，不声称量化无损。真实权重文件临时生成不提交、不发布。量化计算仍FP32，不等于INT8矩阵乘法或完整量化翻译。待CI。

D1.9c run45536668981 CPU format 9组失败，真实专项仍运行，前置a/b/c未完整验收；本批修复9组格式，保留D1.8f及a/b/c全部门槛，独立预研d，不改正式FP32路径。

## D1.9d（独立预研，待验收）
D1.9d 独立 QuantizedLinear：使用原 QuantizedTable 存储和行借用，W[out,in]；按权重块动态量化每输入行一次，signed INT8×INT8、INT32精确块内点积（4096*127²=66064384），FP32比例因子、f64乘积与跨块累加、FP32 bias后单次输出舍入（拒绝非有限），全/末行、维度与1<<30 MAC限额、仅一输入行量化scratch。不重新展开权重、不声称SIMD或模型速度。3组Rust测试（最大正/负点积、非法输入、跨块尺度/偏置、末行/溢出），真实原始Q投影512x512、FC1 2048x512、FC2 512x2048，3合成激活行/矩阵，独立NumPy整数点积及f64尺度参考12288元素，atol/rtol1e-6，FP32偏差按误差传播界检验并记录，不等同于真实模型激活或量化翻译。待CI。

D1.9c [45536668981/job/138945570654](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45536668981/job/138945570654) 真实专项 success：33280512 codes/520008 scales bitwise，RPQ8 file35360699 bytes、payload35360544，gather4096、logits195003元素，max_weight_error0.00306057185、quantized_reference_max_abs2.02655792e-6、fp32_logits_max_abs0.0460805893、fp32_argmax2/2。D1.9d 45537112727 基础format10组失败已修正，真实专项仍运行；a/b/d全量未验收。

## D1.9e（独立预研，待验收）
D1.9e AVX2 signed INT8 dot：每16个i8用128位非对齐加载、符号扩展为16个i16（256位），madd_epi16精确成8个i32、累加后横向求和，标量尾部；不使用饱和maddubs、不改变scale和FP64累加顺序。复用avx2_available运行时CPU/OS检测，portable-only编译去除intrinsics，forward默认Auto，forward_with明确Scalar/Auto，原dot_i8仍Scalar。新增非对齐/尾块/极值单测、真实d参考额外逐位Scalar对照，3个真实权重矩阵8合成激活行，native/portable各24576输出位级对照，预热、每内核5次交替计时取中位数，包含动态激活量化和FP64恢复，不含加载/转换，不声称端到端翻译加速。保留D1.8f和a–d门槛；待CI。

D1.9e run45538079043 尚排队，D1.9d 45537112727 真实专项仍运行；D1.9c真实专项已通过，基础CI未全验收。此次f独立预研，不取消旧运行，保留D1.8f及D1.9a–e门槛。

## D1.9f（独立预研，待验收）
D1.9f RPModel-Mixed v1：固定审计profile的97个矩阵（共享词表+72注意力投影+24 FFN）block64量化为RPQ8，159个Bias/LayerNorm/位置权重保留原FP32 Safetensors。77320704 codes、1208136 scales、687593 FP32元素，权重预算84903620 bytes（不含文件头、元数据、临时/RSS）。Rust convert 验证源RPModel，分批读FP32，保留紧凑量化矩阵，写新目录及固定manifest最后；失败需丢弃不完整目录。固定manifest绑定revision/source SHA、两个文件长度/摘要；五个元数据资产使用已固定SHA验证，严格索引及shape/block/dtype/count，所有block语义校验、所有F32有限；拥有型MixedModel提供matrix/auxiliary/config、共享别名指向同一对象、统一预算和资源释放。独立Python全量codes/scales/FP32位对照、元数据、完整目录大小、4篡改拒绝及恢复复验。无权重提交，无网络运行时，无完整量化神经网络适配或翻译速度承诺。SHA不是签名，稳定本地文件假设。待CI。

D1.9e [45538079043/job/138950535258](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45538079043/job/138950535258) 真实专项success：native/portable各24576元素bitwise，AVX2 Q/FC1/FC2 ratio=2.670087/2.624613/2.718530，portable约1倍；CPU format失败。D1.9f run45538582134仍执行，完整包尚未验收。

## D1.9g（独立预研，待验收）
D1.9g 独立混合推理：MixedSession 直接借用拥有型MixedModel 97个packed矩阵/159 FP32辅助，实际6+6层、8 heads、动态INT8线性Q/K/V/out与FFN、INT8共享LM head、单行反量化embedding、实际位置、FP32 attention/softmax/residual/norm/SiLU/KV，复用既有beam policy；source PAD mask、target显式mask、完整前缀和immutable KV snapshots共享cross，256MiB KV payload budget，Scalar/Auto独立选择。测试2组（显式target mask及真实英文token）、3种logits路径/encoder对照独立NumPy整数block及FP32参考，1e-4容差、cache/scalar位级要求、beam1/4各4步full/cache与独立排名/EOS/score；真实短句beam1混合及FP32 Rust实际译文/独立分词反分词、自然EOS。仅短样本不代表全论文质量；不改变正式FP32入口，性能与RSS待D1.9h。保留前序门槛，真实job timeout提高到15min。待CI。

D1.9f [45538582134/job/138953177087](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45538582134/job/138953177087) 专项success，MIXED_MODEL_REFERENCE PASS q_tensors=97 f_tensors=159 q_elements=77320704 q_blocks=1208136 f_elements=687593 bitwise=true payload_bytes=84903620 package_bytes=88160260 negatives=4；CPU format失败。D1.9g 45539539778 CPU format 23组失败，本次修正，真实专项仍运行。

## D1.9h（待验收）
D1.9h OfflineTranslator保留原load FP32，新增load_mixed/load_with_precision、Precision枚举、统一TextSession/分词/解码/预算/结果及默认生成选项，显式translate-int8命令（不自动切换或回退FP32）。原benchmark默认FP32，新增int8，同runner分开Rust进程GNU time测RSS、加载与session、两句Full/Cached各3次/每模型2预热、6句beam4质量、24正式计时/12真实译文、CLI两次及4负例、各模式确定性、同质量fixture。输出长度可能不同，分别报告吞吐比和延迟比；单次加载顺序运行非严格冷缓存，短样本质量观察非论文准确率，未设未经验证的速度/质量阈值。所有前序门槛保留，待CI。

D1.9g run45539539778 真实专项仍在运行，CPU format失败已于5023beb修复；D1.9h run45540199254排队。未取得全量验收，独立开发保护核心，保留D1.8f及D1.9a–h回归门槛。

## D1.10a（独立预研，待验收）
D1.10a 独立保护核心 Plan：UTF-8 原文范围、数学显式 $/$/\\(/\\[ 分隔符与转义、ASCII 有符号小数/e/E指数及×10^指数、有限单位表、8种常见化学式、调用者提供128项以内区分大小写词典（最长匹配、单词边界）。保留原字节，不将数值换算；术语仅按显式目标替换。提供 gaps/assemble（模型不见保护内容）及 masked/restore（严格检查预期占位符顺序、缺失、重复及未知标记），拒绝超限、未闭合数学和坏指数；64KiB输入/256KiB恢复/1024 spans。6组Rust测试，未运行，不代表实际译文质量。有限语法，不是通用LaTeX/化学/单位解析器，不处理所有范围、千分位、Unicode上下标和科学语义；分段可能损失上下文，占位符可能被模型改写。暂不改正式翻译入口，下一步进行实际模型保护接入与评估。

## 下一阶段 D1.10b
前序验收通过后，科研保护计划接入FP32/INT8文本翻译，真实样本完整性回归。

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

实际 FP32 专项验收：[45535270286/job/138940316695](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45535270286/job/138940316695) success，native32/8211233元素 max_abs=4.69684601e-5；portable24/3658488元素 max_abs=2.14576721e-5，max_cross=0（样本），512位置与4负例通过；REAL_TRANSLATION_REFERENCE PASS sentences=3 generations=6 hf_greedy=3 cli=1 invalid=3 max_score_error=7.17160084e-7；TRANSLATION_BASELINE PASS 12计时/2预热/6句，model_ms1444.469 session_ms26.833，RSS324.066MiB，cached3.116106/3.419395 output tokens/s，cache ratio1.075631/1.108269，number4/5（三次是中文数字），term12/12，mean_char_f2=72.914，capped0。质量观察而非全面质量通过；Hello world beam4 存在英文残留。该 run CPU format 失败，完整门槛仍需最新复测。D1.9a 首轮 CPU format 5组失败，本次修正；D1.9b 代码已提交，专项尚未执行。

D1.9d 实际专项：[45537112727/job/138948023467](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45537112727/job/138948023467) success，REAL_INT8_LINEAR_REFERENCE PASS matrices=3 elements=12288 reference_max_abs=0 fp32_max_abs=0.0910242527 atol=1e-6 rtol=1e-6。仅真实权重/合成激活；整体CPU格式失败。D1.9e 45538079043 CPU format 4组失败，已修复，真实专项仍在运行。D1.9f 尚待CI。

D1.9f run45538582134 CPU format 5组失败，本次按完整上下文修正；真实转换专项仍运行。D1.9g 已提交但未编译/验收，不宣称全模型量化翻译成功。

D1.9g/D1.9h 真实专项 [45540564400/job/138958019476](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540564400/job/138958019476) success。MIXED_INFERENCE_REFERENCE PASS 1371165元素、10候选、max_abs=0、cache/scalar位级一致。PAIRED_TRANSLATION_REFERENCE PASS 24正式计时/12质量输出/2 CLI/4负例；FP32/INT8 package315294472/88160260 bytes、RSS331828/107268KiB、load1752.359/1799.760ms、cached2.830836/64.540152及3.132952/73.752100 tokens/s，延迟比22.798972/23.540767；6/6译文相同、char_f2均72.914、数字4/5、术语12/12、capped0。仅短句与该Runner观察。CPU format 12组失败导致后续基础测试跳过，本提交按日志修正；全量CI尚待复验。冻结新功能开发，D1.9全量成功后等待用户指示；加载加速D1.9i-l为拟议任务，未授权执行。

D1.9 全量复验修复：run45542273543/job138963393404 format通过，Clippy `chunks_exact_to_as_chunks` 两处失败（mixed_inference.rs 80/81），改为固定数组切片并显式验证尾部与行数；另预防性修正 protection.rs 的奇偶判断为 is_multiple_of（不变更语义）。保留既有全部CI门槛和正在运行的真实专项，定时自动任务保持关闭，待新提交复验。不进入新阶段。

D1.9复验 run45542753928/job138965410757：上次修复新增 if 条件触发rustfmt单组换行差异，按日志原样修正，仅格式变化；Clippy和单测因format失败尚未执行。保留全部门槛，定时任务关闭，不开始新功能。

D1.9复验 run45543085996/job138967024836：Format通过，Lint在 int8_simd_bench.rs:70/79 报 needless_range_loop。将样本矩阵转为[repeat][kernel]，迭代器enumerate写入每轮，最后转置为两组中位数；5轮交替顺序及bitwise门槛不变。与HANDOFF.md同提交，待CI验证。定时任务仍关闭。

## D1.9h-paper / D1.10b
基线 1173be0 / [45549181329](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45549181329) 全量 success。真实论文 6036 词/269片段，INT8 110.690017秒、73.132159 token/s、114900 KiB，数字287/317，0截断；FP32/INT8同8片段 55.936918/3.206017秒、17.447482倍，4/8相同。D1.10a基础通过。
D1.10b 接入显式保护 API/CLI，识别角度、重力加速度、科学指数、特定物理符号与已有公式等；按原文间隔调用同一模型会话、拒绝错误与截断；保护仅限匹配跨度，可能损伤上下文，默认路径不变。六组真实双精度样本及同论文完整提取文本观察报告，全文拒绝不隐瞒，未设未经验证的语义准确率。待CI，不启动下一阶段。

D1.10b [45554897289](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45554897289) CPU/真实专项失败：Rust 2021 let-chain 编译错误及 rustfmt 元组换行。已按日志修复并待全量复验；未执行科学保护专项及新论文对照，不能宣称成功。

D1.10b run [45555576461](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45555576461) 真实专项 success（job139006695845），FP32/INT8 各6组、8个保护字面量、1负例通过；全文269片段，成功266、拒绝3（9 NoTranslation；45/252 Truncated），302识别跨度、493模型调用。同成功集合数字字符串280/310→306/310，时间106.642471→105.359267秒，保护峰值RSS120768KiB。未证明语义质量提升；样本有漏译、额外数字、分段幻觉及术语错误，保护路径仍仅显式选择。CPU因offline.rs与protection.rs两处rustfmt换行失败，按真实日志修复，全部门槛待重新验收。

## D1.10c
D1.10b 基线 HEAD 257a156 / [45559761420](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45559761420) 全量 success，97 workspace + 52 portable =149/149；真实 FP32/INT8 各6组8字面量1负例；论文成功266/269、拒绝3，数字280/310→306/310，同成功集合106.321602→105.831631秒、120704 KiB。保护仍显式选择，非语义质量通过。用户授权 D1.10c：本提交新增 Rust trie、69条项目自编候选别名（三学科配置，通用/岩土地震/生物），ASCII case、横向空格、三种Unicode连字符规范化、显式复数别名、最长匹配、UTF-8边界、跨通用/学科冲突拒绝、公式/数值优先、显式 translate-glossary[-int8] API/CLI、双精度各8组真实句与同论文8片段观察。无自动语义消歧或权重训练；保留 D1.10b 全量专项，待CI，不启动 D1.10d。

D1.10c run [45568226746](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45568226746) CPU Format 失败：glossary_reference.rs 8组元组换行及 offline.rs 闭包换行，已按实际 rustfmt 日志修复。原真实模型专项仍运行，保留结果；新提交全量复验前不启动 D1.10d。

## D1.10d
D1.10c 基线 HEAD 14d641b / [45569156783](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45569156783) 全量 success，104 workspace+52 portable=156/156；真实FP32/INT8各8组通过，论文8/8片段，指定术语3/17→17/17，2.632185秒；仅词面恢复，非语义准确率。用户授权 D1.10d。本提交新增自主 Rust distinct-cue 学科评分、强局部覆盖文档先验、局部无证据才用文档先验、混合或弱证据回退通用词典、显式决策报告、auto CLI 和双精度7组（4选择3弃权）与同论文8片段观察。文档上下文由调用方每篇创建，分类为片段级启发式，不是训练分类器或逐词语义模型；默认路径不变。保留 D1.10c/b 全量门槛，待新CI；不启动 D1.10e。

D1.10d run [45572696557](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45572696557) CPU Format 失败：context_reference.rs、context.rs、offline.rs 三处换行，已按实际 rustfmt 日志逐处修复。原真实模型专项仍运行，保留结果；新提交全量复验前不启动 D1.10e。

## D1.10e
D1.10d 基线 HEAD 2c6450f / [45573020932](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45573020932) 全量 success，109 workspace+52 portable=161/161；双精度7组各4选择3弃权，真实论文8/8、岩土7/8、术语14。D1.10e 用户授权，本提交实现有序非重叠token约束、EOS偏好或预算强制、词组连续完成、保持原始logp、FP32/INT8全输入缓存/完整前缀共用搜索、无损目标词表DP与文本复验、显式CLI、独立合成和双精度真实测试。约束顺序遵守原文，可能出现末尾补入、重复及语义错误，非全局最优或质量保证；词表不可表示/预算不足/截断显式拒绝。保留旧默认及保护模式和所有前置CI。待最新全量CI，不启动D1.10f。

D1.10e run [45577712411](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45577712411) 真实模型专项 success：FP32/INT8各4组+1预算负例；论文8/8成功、17术语跨度、0拒绝。严重语义缺陷仍存在：末尾强制补入术语、保留原错误词、重复数值及整句，如峰值加速度为0.1克。0.1 g.；不构成翻译质量通过。CPU Format 失败：search.rs两处、constrained.rs三处换行，已按真实日志修复。保留所有门槛，显式模式不作为默认，待最新全量复验，不启动D1.10f。

## D1.10f
D1.10e 基线 HEAD 6dadc58 / [45579659701](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45579659701) 全量 success，116 workspace+56 portable=172/172；双精度4组及1预算负例各通过，论文8/8、17术语，仍有强制末尾补入及数字重复。用户授权 D1.10f，本提交实现段落级先验、保守句边界/超192源token安全子句分割、64片段上限、完整句→约束→保护间隔候选顺序、数字串多重集合/最长非重叠术语计数/已观察错误词/重复长子句校验、拒绝原因报告及整段失败关闭、双精度2固定段落4片段、数字安全门和1负例、论文8样本覆盖率观察。校验仅词面启发式，可能误拒绝，不判断语义或数字位置；不进行神经跨句attention，不自动删改错误译文，旧默认不变。待最新全量CI，不启动 D1.10g。

D1.10f run [45582711679](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45582711679) CPU Format 失败：paragraph_reference.rs 一处、paragraph.rs 五处换行。已按实际 rustfmt 日志逐处修复；原真实模型专项仍运行，尚未取得段落专项结果。保留全部门槛，新提交全量复验前不启动 D1.10g。

## D1.10g（首批，待验收）
- D1.10f 基线 HEAD 7f1fdb282bbdc947dc4a5d953768efaed429959d：用户提供 GitHub Actions #97 截图，工作流绿色成功，耗时30m10s；当前接口仅支持 PR 工作流列表，未获得 #97 Run ID、最新任务详细日志或测试计数，不虚报。此前 #96 真实专项 [139096491848](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45582711679/job/139096491848) 实际通过，FP32/INT8各2固定段落4片段、1数值审计、1预算负例；论文8样本成功7拒绝1，Whole1/Constrained1/ProtectedGaps5，仅词面。已观察百分号脱离数值和重复后缀；用户持续授权全量绿灯后进入D1.10g。以#97截图为阶段绿灯依据，本提交D1.10g首批实现百分号邻接数值多重集合及连续长中文后缀重复检测，接入现有候选拒绝/回退，新增2组Rust回归；不自动删除或改写译文。旧默认不变，保留所有CI门槛。仅启发式，可能误拒绝合法重复或中文百分数，不是语义质量保证。未在本地编译（无Rust工具链），待新CI实际验收；不启动下一阶段。

## D 线路 CI 状态回传（待实测）
D 线路 CI 可发现性修复（D1.10g 验收中）：当前 GitHub 连接器按 commit 列工作流仅返回 PR 触发，push CI Run ID 无法自动取得。新增仅限 feature/rptranslate push 的 ci-status-start/ci-status-final 两个免费 CPU 状态任务，使用 job 级 statuses:write 和内置 GITHUB_TOKEN，将 pending/最终状态及真实 Actions URL 写入 rptranslate/ci commit status。最终任务等待原四个验收任务和初始回传任务，必需任务跳过/失败/取消不标绿；可选任务按现有触发条件判断。原验收命令、权限及触发规则不变，无第三方服务/定时任务。D1.10g 源码基线 af64f5e；此次只改 CI 与文档，保留所有阶段触发标记以完整复验。回传是否成功须由新 CI 实际证明，若整个工作流未启动或被强制取消，可能无最终状态。

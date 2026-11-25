# D 线路 RPTranslate 研发交接（2026-11-25）

## 首要任务与用户约束
- 仓库 trade3403/ai-research-pdf-reader，独立分支 feature/rptranslate；只修改 rptranslate/ 及 .github/workflows/rptranslate-ci.yml。禁止改 main/A/B/C、其他项目，不自动合并或发布。
- **当前只做 D1.10c 科研术语词典与精确匹配**。D1.10b 全量验收通过，用户已授权 D1.10c。未全量验收前不启动 D1.10d 或 D1.9i-l。
- 用户明确取消每小时自动任务；D线路自动研发验收 ID 6b05aef0467c81918bd85a7cc779030d 已核对为 disabled。不要重新启用、创建定时任务，也不要承诺后台修复。当前窗口/新窗口直接实际修复并提交CI。
- GitHub 免费 ubuntu CPU Runner；不使用收费、自托管、用户电脑或GPU。自主Rust推理、无Python/外部推理框架运行时。Python/PyTorch/NumPy仅CI独立参考或未来训练。
- 每次先读取最新 HEAD（交接文档不能替代实时状态）、Actions runs/jobs/logs 及 docs/PROGRESS.md、docs/D1.9_INT8.md、docs/D1.9H_PAIRED_BASELINE.md、ROADMAP/TARGETS/DEVELOPMENT。失败依据真实日志最小修复，保留 fmt/clippy -D warnings/workspace测试/portable-only/独立数学及格式/真实模型所有门槛。不要反复推测式提交；若现有真实专项执行中，尽量保留结果，避免重复触发或取消。每次提交前核对 HEAD，非强制快进 expected_sha，避免并发覆盖。提交标题保留 D1.9h D1.10a D1.8f 标记，以保证条件验收运行。
- 每阶段报告真实 commit、Actions链接、实际输出和未完成项。没有CI成功不能声称全量验收通过。完成 D1.10c 全量后通知用户，等待下一阶段指示。

## 当前状态（最新优先）
- D1.10b 基线 HEAD 257a156 / [45559761420](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45559761420) 全量 success，97 workspace + 52 portable =149/149；真实 FP32/INT8 各6组8字面量1负例；论文成功266/269、拒绝3，数字280/310→306/310，同成功集合106.321602→105.831631秒、120704 KiB。保护仍显式选择，非语义质量通过。用户授权 D1.10c：本提交新增 Rust trie、69条项目自编候选别名（三学科配置，通用/岩土地震/生物），ASCII case、横向空格、三种Unicode连字符规范化、显式复数别名、最长匹配、UTF-8边界、跨通用/学科冲突拒绝、公式/数值优先、显式 translate-glossary[-int8] API/CLI、双精度各8组真实句与同论文8片段观察。无自动语义消歧或权重训练；保留 D1.10b 全量专项，待CI，不启动 D1.10d。
- D1.10b run [45555576461](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45555576461) 真实专项 success（job139006695845），FP32/INT8 各6组、8个保护字面量、1负例通过；全文269片段，成功266、拒绝3（9 NoTranslation；45/252 Truncated），302识别跨度、493模型调用。同成功集合数字字符串280/310→306/310，时间106.642471→105.359267秒，保护峰值RSS120768KiB。未证明语义质量提升；样本有漏译、额外数字、分段幻觉及术语错误，保护路径仍仅显式选择。CPU因offline.rs与protection.rs两处rustfmt换行失败，按真实日志修复，全部门槛待重新验收。
- D1.10b run [45554897289](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45554897289) 失败：Rust 2021 不支持 let chains，protection.rs:269；scientific_reference.rs:30 rustfmt 元组换行。按真实日志改为 Option::filter 及格式修复，保持 Rust 2021 与原验收门槛。新提交待CI；上一轮科学专项未执行。
- 基线 HEAD 1173be0b79784ebbd4a5eff20025725db37fbc7d，run [45549181329](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45549181329) 全量 success；CPU、真实模型实际运行均成功。D1.10a 六组基础测试通过，尚未接入默认翻译。
- 真实论文 INT8 269/269 片段、6036英文词、110.690017秒、73.132159 tokens/s、114900 KiB、287/317数字字符串、0截断。同8片段 FP32/INT8 55.936918/3.206017秒、17.447482x，4/8译文一致；两个模型都将 strain 译为菌株、0.1 g 译为0.1克，非量化独有错误。仅提取文本，不含完整排版及独立人工评分。
- D1.10b 本提交显式 translate-scientific[-int8]、TextSession::translate_scientific、分段保护恢复、拒绝截断/错误/预算、六组真实双精度固定样本、同论文完整提取文本对照。保留普通入口。标记 D1.10b D1.10a D1.9h-paper D1.8f 触发所有已有门槛。待本提交CI；先查真实HEAD及run。保护仅保证识别跨度字面恢复，分割可能损伤语义，全文失败计入拒绝，不能宣传100%科学准确率。

## 历史状态（旧记录，非当前）
父提交 a3c1f49c3d0a3dd457cb9833b71a7c21ae75a854；run [45543085996](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543085996)，CPU [138967024836](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45543085996/job/138967024836) Format通过，Lint失败：int8_simd_bench.rs:70/79 needless_range_loop，循环 rep 索引 times[slot][rep]。本交接提交修复为 samples[repeat][kernel] + iter_mut().enumerate()，最后转置为两组中位数；原5次交替、位级一致要求不变。此修复待新CI，须通过HEAD查找新 run，不要把父提交当最新。真实专项138967025227仍执行。45542753928与45542273543被取消，不是成功；历史修复a3c1f49为确切rustfmt换行，d84d318修复两处固定chunk clippy与奇偶判断，51bf387修复12组format。

## D1.9 已经取得的真实证据（非整体绿灯）
- D1.9a/b：数学核心及RPQ8实现已提交，独立基础CI一直被格式/lint阻塞，待实际通过。
- D1.9c：[45536668981/job/138945570654](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45536668981/job/138945570654) 共享表33280512元素、520008块，codes/scales bitwise，FP32 logits max_abs .04608，argmax 2/2。
- D1.9d：[45537112727/job/138948023467](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45537112727/job/138948023467) 三矩阵12288元素独立INT8参考max_abs=0，FP32 max_abs=.0910242527。
- D1.9e：[45538079043/job/138950535258](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45538079043/job/138950535258) native/portable各24576元素bitwise；native AVX2 三矩阵完整Linear 2.62–2.72x。
- D1.9f/g/h：[45540564400/job/138958019476](https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45540564400/job/138958019476) **真实专项success**。97量化矩阵+159 FP32张量，77320704 codes、1208136 scales、687593 FP32元素，payload84903620 bytes、package88160260 bytes，独立全量bitwise，4篡改拒绝。完整6+6层前向1371165元素 max_abs=0、10候选、score误差4.08562073e-14，cache/scalar位级一致，一句FP32/INT8真实Token一致。
- D1.9h 同Runner真实24计时/12译文/CLI2/负例4；FP32/INT8 package315294472/88160260 bytes，RSS331828/107268 KiB，load1752.359/1799.760ms，session34.151/34.653ms，cached 2.830836/64.540152及3.132952/73.752100 token/s，延迟比22.798972/23.540767，6/6译文相同，char_f2均72.914、数字4/5（中文“三次”不是数字错误）、术语12/12、capped0。仅6短句/该机器，非所有论文的无损保证。模型加载未加速，不能捏造。
- 最终全量条件：最新CPU job fmt、clippy、workspace单元测试、release、smoke、portable-only、独立INT8数学、RPQ8格式均success，且最新真实job所有要求执行且success；检查实际输出，不能只看run绿色或skipped。D1.10a保护测试同workspace。

## 代码地图
- rptranslate/Cargo.toml workspace: rpdata、rpmodel、rpinfer、rptranslate，无外部运行时crate。workflow .github/workflows/rptranslate-ci.yml，push分支/路径条件，concurrency cancel-in-progress:false。
- rpinfer/src/quantization.rs（BlockInt8 动态量化、ties-even、reject -128）、rpmodel/src/quantized.rs（RPQ8BIN v1固定索引+SHA256、read_block、write_container）、rpinfer/src/quantized_table.rs（紧凑codes/scales、共享词表）、rpinfer/src/quantized_linear.rs（Scalar/Auto AVX2 i32 dot, f64恢复/累加）、rpinfer/src/mixed_model.rs（convert/load完整量化包及固定资产SHA、严格shape/budget）、rpinfer/src/mixed_inference.rs（MixedSession，6+6层、FP32 attention/norm/SiLU/KV、INT8矩阵及LM head，full/cached beam）。
- rptranslate/src/offline.rs：OfflineTranslator::load FP32, load_mixed INT8, load_with_precision；共用TextSession、分词与decode；main.rs CLI translate / translate-int8。
- rpinfer/tests/check_int8_reference.py、rpmodel/tests/check_quantized_format.py、rpinfer/tests/check_real_quantized_table.py、check_real_int8_linear.py、check_mixed_model.py、check_mixed_inference.py；rptranslate/tests/check_paired_translation.py 和 translation_quality_fixture.py。实际模型固定 OPUS-MT en-zh revision a4193836671069f1f80ce341f9227a850ffb52d4。
- D1.10a rptranslate/src/protection.rs Plan::prepare/spans/gaps/assemble/masked/restore，有限显式公式/数值/单位/化学式/词典；严格marker顺序，64KiB输入/256KiB输出/1024片段/128术语，6组单测待验收，不接默认翻译路径。注意可能有后续 lint/测试失败，以日志为准。

## 用户下一阶段意向（仅规划，先全量验收）
用户问为何 INT8 1.8秒加载与FP32相近。实际源码：MixedModel::load先全文件hash_file，再quantized::Reader::open内SHA，QuantizedTable::load对1208136块逐一read_block，每块seek/read/临时Vec/validate；OfflineTranslator加载验证分词器、session再次初始化。候选优化 D1.9i 连续批量读取无每块分配；D1.9j 双重SHA单次扫描/加速；D1.9k mmap/零拷贝；D1.9l 模型常驻及分词器复用。0.3–0.6秒仅目标，保持安全完整性、真实位级对照及内存验收。待用户明确同意才做。后续D1.10b真实保护接入；D1.11真实论文、D1.12小规模训练与断点、D1.13 Windows工具；D2训练，D3集成。

## 新窗口提示
继续 D 线路 RPTranslate 自主研发。读取 feature/rptranslate 的 rptranslate/HANDOFF.md，核对实际HEAD及CI日志。当前 D1.10c，先完成全量验收，按真实错误修复，不启用自动任务，不开发后续阶段或修改其他线路。

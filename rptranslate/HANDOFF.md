# D 线路 RPTranslate 研发交接（2026-11-25）

## 首要任务与用户约束
- 仓库 trade3403/ai-research-pdf-reader，独立分支 feature/rptranslate；只修改 rptranslate/ 及 .github/workflows/rptranslate-ci.yml。禁止改 main/A/B/C、其他项目，不自动合并或发布。
- **当前只做 D1.9 全量验收修复**。D1.10a 独立科研保护模块已提交但未验收；先不要开发 D1.10b 或 D1.9i-l。
- 用户明确取消每小时自动任务；D线路自动研发验收 ID 6b05aef0467c81918bd85a7cc779030d 已核对为 disabled。不要重新启用、创建定时任务，也不要承诺后台修复。当前窗口/新窗口直接实际修复并提交CI。
- GitHub 免费 ubuntu CPU Runner；不使用收费、自托管、用户电脑或GPU。自主Rust推理、无Python/外部推理框架运行时。Python/PyTorch/NumPy仅CI独立参考或未来训练。
- 每次先读取最新 HEAD（交接文档不能替代实时状态）、Actions runs/jobs/logs 及 docs/PROGRESS.md、docs/D1.9_INT8.md、docs/D1.9H_PAIRED_BASELINE.md、ROADMAP/TARGETS/DEVELOPMENT。失败依据真实日志最小修复，保留 fmt/clippy -D warnings/workspace测试/portable-only/独立数学及格式/真实模型所有门槛。不要反复推测式提交；若现有真实专项执行中，尽量保留结果，避免重复触发或取消。每次提交前核对 HEAD，非强制快进 expected_sha，避免并发覆盖。提交标题保留 D1.9h D1.10a D1.8f 标记，以保证条件验收运行。
- 每阶段报告真实 commit、Actions链接、实际输出和未完成项。没有CI成功不能声称全量验收通过。完成 D1.9 全量后通知用户，等待用户指示加载加速或D1.10b。

## 当前状态（写入本交接前）
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
继续 D 线路 RPTranslate 自主研发。读取 GitHub 仓库 trade3403/ai-research-pdf-reader 的 feature/rptranslate 分支下 rptranslate/HANDOFF.md，核对最新HEAD、研发进度与CI实际日志。先修复D1.9剩余错误，实际提交GitHub、重新验收；不启用定时任务，不开始新功能，直到D1.9全量CI成功。不要改其他线路。报告真实链接与数据。

# RPCore — PDFium 性能对照与验收目标
日期：2026-11-26。用户授权写入性能目标。本文是 B 线路新增专项研发约束，不代表已经实现或测得目标值。
## 启动顺序
- B-P0：当前 B11.3e3e 批次及其修复的实际 HEAD CI 全绿后，下一可用独立批次先建立基础性能对照（普通文本、矢量、已支持图像、解析、文本提取）。若算法批次仍在运行不得抢占，不要求等待全部 JBIG2 完成。记录不能支持的文件，不把失败/缺失渲染计入速度优势。
- B-P1：高级解码、字体、ICC、复杂页面集成逐步完成后扩展 corpus，定位 CPU 热点、内存分配、缓存、I/O。
- B-P2：依据剖析结果优化 SIMD（AVX2、按硬件评估 AVX-512/NEON），保持标量回退与质量回归。
- B-P3：独立页面、分块与图像并行；测单线程及相同线程预算的吞吐。
- B-P4：评估 Vulkan/wgpu GPU 路径，CPU/GPU 分组，不把不同硬件或后端混为同一对照。
- B-P5：零拷贝、映射、缓存、预加载及内存预算优化，以实际瓶颈决定优先级。
- B-P6：完成真实科研论文、扫描件、大文档、复杂图表及安全/兼容性验收后，全面对照 PDFium；最终 Windows x64 实机复测。
## 性能目标（非实测、非发布承诺）
以同机同质量 PDFium 吞吐为 100%，基础目标 50%，接近目标 80%，最终普通科研页面渲染目标 >=100%；多页并行吞吐、全文搜索目标 >=150%（仅对等功能、线程预算及索引构建成本下）；峰值 RSS 目标 <= PDFium，首屏/翻页/缩放延迟目标 <= PDFium。科研结构提取建立独立质量/速度目标；PDFium 不提供等价功能时不虚构倍数。目标未达须如实报告，不降质量门槛。
## 公平可复现协议
固定 RPCore/PDFium commit 或版本、构建配置和后端、CPU/OS/RAM、线程数、输入文件 SHA256、页面、DPI（96/150/200/300）、像素格式、背景、裁剪、抗锯齿和色彩设置。两者 release 构建；区分冷/热缓存、打开/首屏/单页/批量/文本提取/搜索/索引构建，计时包含相同工作边界，交替执行，预热后至少 10 次有效重复，记录 median/p95、页/秒、峰值 RSS、失败/超时及覆盖率；同时输出原始 JSON/CSV、环境和可复现命令。比较同一 runner 的配对结果，不跨 GitHub runner 直接比较绝对耗时。实际像素尺寸与质量检查（参考图、像素误差、文本完整性、字符坐标）必须先通过；抗锯齿差异采用预先记录的容差和人工抽查。未支持项及拒绝项列出，不作零耗时处理。使用合法合成/公开许可论文，保存来源及哈希。CI 性能只作观察和回归证据，硬门槛须有稳定基线；不引入收费 runner。
## 依赖与并发边界
PDFium 仅限隔离 benchmark/test 工具，固定来源/版本并审计许可，不链接、不打包进正式 RPCore 或桌面应用；继续保留现有测试和 PDF.js 过渡门槛，禁止以外部引擎替代自主算法。不得干扰 B 线当前 CI、并发覆盖或修改 A/C/D/main。后续 B 线每批先读 HANDOFF.md、本文件及实际 HEAD/CI；报告已测数据、目标差距、质量门槛与未完成事项。

## B-P0a 启动记录（待 CI 验收）
- B11.3e3e 已在 51e34cbcf69a774b754a51823b5a28cb981ad170 / CI #456 (45747789882) 正式验收，10/10 jobs、560/平台，共 1680 项。按启动顺序先进入 B-P0a。
- 新增原生 release `core/examples/perf_probe.rs`、独立 `corpus/pdfium_performance.py` 和 Linux `perf-pdfium` job。固定 pypdfium2 5.8.0（本地参考 PDFium 149.0.7825.0）及 Pillow 12.3.0，仅测试 venv；记录版本、库 SHA256 和许可声明/notice 哈希，不进入生产依赖。来源 https://pypi.org/project/pypdfium2/5.8.0/ 和 https://pypi.org/project/pillow/12.3.0/ 。
- 原始合成矢量、TrueType、JPEG 三份 PDF；29 个场景：3 个打开及页信息、24 个四 DPI 冷/热文档渲染、2 个文本/字符原点。每场景 1 对预热 + 10 对测量，交替引擎；输出 report.json、trials.csv、report.md、代表性 PNG、构建/硬件/输入和库哈希。暂用 RGB MAE <=12、最大通道误差 >32 的像素比例 <=8%、同尺寸非全白，文本 ODo 与字符原点 <=1pt；任何失败、不稳定输出或未通过质量门槛的场景不产生比率。
- 口径：输入已在内存，fresh document 非冷 OS 缓存；计时不含进程启动、磁盘 I/O、RGB 归一化；PDFium 含 Python/ctypes 包装开销，RSS 含整个进程，内部线程未单独测量。是探索基线而非原生同接口最终性能结论。暂未测真实论文、多页吞吐、索引搜索、Windows 实机；无已测 RPCore/PDFium 性能比率。需新 HEAD CI 全绿并阅读 perf-pdfium 结果后如实记录，必要时修复测量协议，之后回到 B11.3e3f。现有生产核心和 560/平台历史测试不变。

## B-P0a 实测记录（CI #472，正式完成限定基准）
HEAD 56946d71c9a1f6ebeabc7b60dbc246cdeeddc512，Run 45750135349，perf-pdfium job 139675648079；11/11 jobs、560/平台、1680 项通过，0 失败/忽略。29 场景中 25 合格，4 排除。原始报告/CSV/PNG artifact 15085626678：https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45750135349/artifacts/15085626678 ，165108 字节，SHA256 8db9338ab1adaa13331b6163a6d282bcea898a1d4fd6399f8a2ade2a2c078cfc。

相对吞吐（PDFium=100%，实际 CI 日志四舍五入；每格 fresh/warm）：

| DPI | 矢量 | TrueType | JPEG |
|---|---:|---:|---:|
| 96 | 5.47 / 3.95 | 14.30 / 5.98 | 排除 / 排除 |
| 150 | 4.86 / 3.87 | 10.13 / 5.93 | 排除 / 排除 |
| 200 | 4.89 / 3.80 | 7.98 / 4.99 | 26.86 / 12.66 |
| 300 | 4.74 / 3.64 | 6.46 / 4.48 | 15.61 / 8.25 |

打开及页信息：矢量 484.29%、文本 583.27%、图像 435.97%；文本及原点 fresh/warm 214.17/217.34%。渲染有效场景全未达 50% 基础目标。日志确认四个排除场景但未读取 ZIP 内报告，故不推断具体质量失败项，不声称核验绝对延迟/RSS。微型合成样本、Python 包装、不同实现成本等限制仍适用；这些数字不构成整体引擎领先结论。后续继续以同机配对、质量优先原则，先回归 B11.3e3f；B-P1 定位光栅热点、修复 JPEG 低 DPI 差异，之后再做 SIMD/并行/GPU 优化。保留原始基准及门槛。


## CI #623 paired observation
HEAD 056a0fec9a69fcce485c00a4fa524353597cf7b5, run 45914075351, perf job 140229006059. 25/29 eligible; four low-DPI JPEG cases excluded. Eligible render throughput 2.97–21.13% of PDFium; vector max 4.32%, TrueType max 11.26%, JPEG max 21.13%. Inspect vector/text/image 439.76/494.67/355.08%; text extraction fresh/warm 224.30/263.40%. Shared-runner observations, no regression attribution or RSS verification. All render cases below 50%; targets unchanged.


CI #625 run 45955913846: 25/29 eligible; render 4.18–33.85% of PDFium, vector max 5.64%, text max 16.39%, JPEG max 33.85%; four low-DPI JPEG exclusions. Runner variation not attributed to algorithm change; goals unchanged.


CI #627 run 45960007364: 25/29 eligible, rendering 3.70–28.72% of PDFium, vector max 5.10%, text max 13.07%, JPEG max 28.72%; four low-DPI JPEG exclusions. No speedup attribution; goals unchanged.


CI #629 run 45961925370: 25/29 eligible, rendering 3.35–26.65% of PDFium, vector max 4.95%, text max 12.46%, JPEG max 26.65%; four low-DPI JPEG exclusions. Targets unchanged.


CI #631 run 45964000554: 25/29 eligible, rendering 3.28–23.22% of PDFium; vector max 4.65%, text max 11.34%, JPEG max 23.22%. Four low-DPI JPEG exclusions. Goals unchanged.


CI #633 run 45968376763: 25/29 eligible; render 4.20–32.96% of PDFium, vector max 5.90%, text max 16.71%, JPEG max 32.96%; four low-DPI JPEG exclusions. Goals unchanged.


CI #635 run 45972718482: eligible render 3.67–26.50% of PDFium; vector max 5.35%, text max 13.45%, JPEG max 26.50%; four low-DPI JPEG exclusions. Goals unchanged.


CI #637 run 45975112373: eligible render 4.08–30.53% of PDFium, vector max 5.63%, text max 15.24%, JPEG max 30.53%; four low-DPI JPEG exclusions. Goals unchanged.


CI #639 run 45979660894: eligible rendering 3.98–29.09% of PDFium, vector max 5.59%, text max 14.30%, JPEG max 29.09%; four low-DPI JPEG exclusions. Goals unchanged.


CI #641 run 45982112470: eligible rendering 3.25–25.81% of PDFium, vector max 4.86%, text max 12.37%, JPEG max 25.81%; four low-DPI JPEG exclusions. Goals unchanged.


CI #643 run 45984485514: eligible rendering 3.50–27.11% of PDFium, vector max 5.01%, text max 13.89%, JPEG max 27.11%; four low-DPI JPEG exclusions. Goals unchanged.


CI #645 run 45986827499: eligible rendering 3.24–23.60% of PDFium; vector max 4.52%, text max 11.79%, JPEG max 23.60%; four low-DPI JPEG exclusions. Goals unchanged.


CI #647 run 45991420303: eligible rendering 3.63–27.25% of PDFium, vector max 5.47%, text max 13.52%, JPEG max 27.25%; four low-DPI JPEG exclusions. Goals unchanged.


CI #649 run 45996010067: eligible rendering 3.10–22.01% of PDFium; vector max 4.37%, text max 11.16%, JPEG max 22.01%; four low-DPI JPEG exclusions. Goals unchanged.


CI #653 run 46001668134 attempt 2: eligible rendering 3.39–23.65% of PDFium; vector max 4.52%, text max 12.14%, JPEG max 23.65%; four low-DPI JPEG exclusions. Goals unchanged.


CI #655 run 46005412759: eligible rendering 3.32–24.55% of PDFium; vector max 4.79%, text max 11.51%, JPEG max 24.55%; four low-DPI JPEG exclusions. Goals unchanged.


CI #657 run 46010272066, perf job 140543701723: 25/29 eligible, rendering 4.05–32.84% of PDFium; vector max 5.74%, text max 16.79%, JPEG max 32.84%. Four low-DPI JPEG exclusions. Inspect vector/text/image 393.87/471.28/358.15%; text extraction fresh/warm 189.08/203.09%. Artifact 15203851711 (164819 bytes). Shared-runner observation; no algorithm speedup attribution, absolute latency/RSS or archive verification. Goals unchanged.


CI #659 run 46020370444, perf job 140578140159: 25/29 eligible; render 2.95–21.31% of PDFium; vector max 4.22%, text max 10.89%, JPEG max 21.31%. Four low-DPI JPEG exclusions. Inspect vector/text/image 440.07/540.89/418.44%; text extraction fresh/warm 159.10/261.11%. Shared runner variation, no causal speed attribution; no RSS/absolute latency verification. Targets unchanged.


CI #661 run 46025198759, perf job 140594689703: 25/29 eligible; render 3.16–23.32% of PDFium; vector max 4.46%, text max 12.37%, JPEG max 23.32%. Four low-DPI JPEG exclusions. Inspect vector/text/image 571.24/632.18/479.45%; text extraction fresh/warm 174.32/260.87%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #663 run 46028515601, perf job 140606018801: 25/29 eligible; render 3.25–25.56% of PDFium; vector max 4.73%, text max 11.65%, JPEG max 25.56%. Four low-DPI JPEG exclusions. Inspect vector/text/image 438.48/473.32/360.90%; text extraction fresh/warm 157.76/222.30%. Shared runner observations, no causal attribution or RSS verification; targets unchanged.


CI #665 run 46032003668, perf job 140617876674: 25/29 eligible, render 3.68–30.68% of PDFium; vector max 5.35%, text max 14.86%, JPEG max 30.68%. Four low-DPI JPEG exclusions. Inspect vector/text/image 536.10/650.52/398.88%; text extraction fresh/warm 225.13/221.42%. Shared-runner variation, no causal attribution or RSS verification. Targets unchanged.


CI #667 run 46035148027 perf job 140628547610 succeeded but test suite failed; diagnostic measurements only, not accepted baseline. 25/29 eligible; render 3.07–21.64% of PDFium; vector max 4.32%, text max 10.84%, JPEG max 21.64%. Four low-DPI JPEG exclusions. Inspect vector/text/image 499.00/556.29/459.90%; text extraction fresh/warm 231.77/278.62%. Shared-runner variation, no causal attribution or RSS verification. Targets unchanged.


CI #669 run 46038170713, perf job 140638822062: 25/29 eligible, render 3.29–24.82% of PDFium; vector max 4.79%, text max 11.84%, JPEG max 24.82%. Four low-DPI JPEG exclusions. Inspect vector/text/image 365.50/480.43/381.30%; text extraction fresh/warm 162.13/224.24%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #671 run 46041358088, perf job 140649608292: 25/29 eligible, render 3.09–23.21% of PDFium; vector max 4.91%, text max 12.03%, JPEG max 23.21%. Four low-DPI JPEG exclusions. Inspect vector/text/image 545.97/634.40/455.43%; text extraction fresh/warm 248.93/248.36%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #673 run 46044350648, perf job 140659737736: 25/29 eligible; render 4.05–31.70% of PDFium; vector max 5.80%, text max 15.88%, JPEG max 31.70%. Four low-DPI JPEG exclusions. Inspect vector/text/image 403.84/528.16/398.46%; text extraction fresh/warm 177.16/204.73%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #675 run 46047214958, perf job 140669356549: 25/29 eligible; render 3.09–22.22% of PDFium; vector max 4.36%, text max 10.70%, JPEG max 22.22%. Four low-DPI JPEG exclusions. Inspect vector/text/image 439.04/514.66/441.12%; text extraction fresh/warm 153.84/247.85%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #677 run 46050228357, perf job 140679417146: 25/29 eligible; render 3.27–26.02% of PDFium; vector max 4.90%, text max 12.01%, JPEG max 26.02%. Four low-DPI JPEG exclusions. Inspect vector/text/image 444.71/558.36/449.40%; text extraction fresh/warm 240.83/235.00%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #679 run 46052926044, perf job 140688430032: 25/29 eligible; render 3.27–26.21% of PDFium; vector max 4.93%, text max 12.44%, JPEG max 26.21%. Four low-DPI JPEG exclusions. Inspect vector/text/image 468.11/489.38/418.88%; text extraction fresh/warm 182.78/214.88%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #681 run 46055730225, perf job 140697631306: 25/29 eligible; render 3.58–30.34% of PDFium; vector max 5.03%, text max 15.75%, JPEG max 30.34%. Four low-DPI JPEG exclusions. Inspect vector/text/image 392.51/576.31/390.44%; text extraction fresh/warm 187.46/186.79%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #683 run 46058438089, perf job 140706544913: 25/29 eligible; render 2.98–21.97% of PDFium; vector max 4.33%, text max 10.81%, JPEG max 21.97%. Four low-DPI JPEG exclusions. Inspect vector/text/image 453.46/500.01/386.52%; text extraction fresh/warm 231.44/258.18%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #685 run 46061071712, perf job 140715196524: 25/29 eligible; render 3.58–29.16% of PDFium; vector max 5.25%, text max 14.32%, JPEG max 29.16%. Four low-DPI JPEG exclusions. Inspect vector/text/image 527.71/601.45/508.06%; text extraction fresh/warm 180.18/198.00%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #687 run 46063725243, perf job 140723897934: 25/29 eligible; render 3.11–21.60% of PDFium; vector max 4.57%, text max 10.65%, JPEG max 21.60%. Four low-DPI JPEG exclusions. Inspect vector/text/image 459.60/581.24/416.81%; text extraction fresh/warm 169.35/250.32%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #689 run 46066258546, perf job 140732181560: 25/29 eligible; render 3.32–26.44% of PDFium; vector max 4.88%, text max 12.48%, JPEG max 26.44%. Four low-DPI JPEG exclusions. Inspect vector/text/image 457.94/498.68/439.41%; text extraction fresh/warm 189.21/221.14%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #691 run 46068863028, perf job 140740686471: 25/29 eligible; render 3.97–28.89% of PDFium; vector max 5.50%, text max 14.54%, JPEG max 28.89%. Four low-DPI JPEG exclusions. Inspect vector/text/image 394.13/425.42/368.44%; text extraction fresh/warm 163.06/207.01%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #693 run 46071380636, perf job 140748860686: 25/29 eligible; render 2.96–22.53% of PDFium; vector max 4.72%, text max 11.96%, JPEG max 22.53%. Four low-DPI JPEG exclusions. Inspect vector/text/image 562.86/655.48/524.87%; text extraction fresh/warm 240.72/233.90%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #695 run 46073790713, perf job 140756686532: 25/29 eligible; render 3.95–31.56% of PDFium; vector max 5.51%, text max 14.69%, JPEG max 31.56%. Four low-DPI JPEG exclusions. Inspect vector/text/image 380.44/422.17/356.99%; text extraction fresh/warm 201.28/210.10%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #697 run 46076261106, perf job 140764574446: 25/29 eligible; render 3.37–25.50% of PDFium; vector max 4.87%, text max 12.09%, JPEG max 25.50%. Four low-DPI JPEG exclusions. Inspect vector/text/image 424.31/478.02/389.24%; text extraction fresh/warm 186.10/217.51%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #699 run 46078505448, perf job 140771762684: 25/29 eligible; render 3.00–21.91% of PDFium; vector max 4.41%, text max 11.04%, JPEG max 21.91%. Four low-DPI JPEG exclusions. Inspect vector/text/image 481.21/520.53/401.99%; text extraction fresh/warm 242.28/280.00%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #701 run 46080737856, perf job 140778862938: 25/29 eligible; render 3.25–27.62% of PDFium; vector max 4.87%, text max 12.41%, JPEG max 27.62%. Four low-DPI JPEG exclusions. Inspect vector/text/image 428.36/548.82/431.49%; text extraction fresh/warm 189.06/223.33%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #703 run 46082442156, perf job 140784253173: 25/29 eligible; render 3.66–25.72% of PDFium; vector max 5.46%, text max 12.91%, JPEG max 25.72%. Four low-DPI JPEG exclusions. Inspect vector/text/image 446.53/536.47/405.41%; text extraction fresh/warm 158.42/201.18%. Shared-runner observations, no causal attribution or RSS verification. Targets unchanged.


CI #705 run 46082894108, perf job 140785730258: 25/29 eligible; render 3.03–21.00% of PDFium; vector max 4.47%, text max 11.36%, JPEG max 21.00%. Four low-DPI JPEG exclusions. Inspect vector/text/image 437.13/449.34/411.29%; text extraction fresh/warm 167.72/248.30%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #707 run 46085320853, perf job 140793430606: 25/29 eligible; render 3.28–24.56% of PDFium; vector max 4.98%, text max 11.67%, JPEG max 24.56%. Four low-DPI JPEG exclusions. Inspect vector/text/image 393.28/454.81/330.80%; text extraction fresh/warm 222.67/229.73%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #709 run 46087765391, perf job 140801275156: 25/29 eligible; render 4.05–31.69% of PDFium; vector max 5.56%, text max 15.07%, JPEG max 31.69%. Four low-DPI JPEG exclusions. Inspect vector/text/image 340.36/346.01/313.96%; text extraction fresh/warm 209.30/218.06%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #711 run 46090123349, perf job 140808843724: 25/29 eligible; render 3.20–23.41% of PDFium; vector max 4.43%, text max 11.44%, JPEG max 23.41%. Four low-DPI JPEG exclusions. Inspect vector/text/image 448.67/524.05/410.20%; text extraction fresh/warm 181.72/252.19%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #713 run 46092451972, perf job 140816248707: 25/29 eligible; render 3.25–25.83% of PDFium; vector max 4.77%, text max 12.59%, JPEG max 25.83%. Four low-DPI JPEG exclusions. Inspect vector/text/image 411.24/461.38/366.97%; text extraction fresh/warm 203.87/226.08%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #715 run 46094704726, perf job 140823297936: 25/29 eligible; render 3.76–28.23% of PDFium; vector max 5.22%, text max 14.05%, JPEG max 28.23%. Four low-DPI JPEG exclusions. Inspect vector/text/image 436.72/563.45/413.67%; text extraction fresh/warm 228.86/223.42%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #717 run 46096923716, perf job 140830299577: 25/29 eligible; render 3.01–21.08% of PDFium; vector max 4.39%, text max 10.67%, JPEG max 21.08%. Four low-DPI JPEG exclusions. Inspect vector/text/image 470.08/510.64/377.76%; text extraction fresh/warm 237.63/259.43%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #719 run 46099144679, perf job 140837368591: 25/29 eligible; render 3.68–29.68% of PDFium; vector max 5.34%, text max 15.40%, JPEG max 29.68%. Four low-DPI JPEG exclusions. Inspect vector/text/image 419.13/504.11/402.14%; text extraction fresh/warm 222.41/205.19%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #721 run 46101385048, perf job 140844379540: 25/29 eligible; render 3.03–21.19% of PDFium; vector max 4.29%, text max 11.10%, JPEG max 21.19%. Four low-DPI JPEG exclusions. Inspect vector/text/image 448.38/545.39/406.10%; text extraction fresh/warm 174.90/258.29%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #723 run 46103443667, perf job 140850813750: 25/29 eligible; render 4.04–31.88% of PDFium; vector max 5.75%, text max 15.85%, JPEG max 31.88%. Four low-DPI JPEG exclusions. Inspect vector/text/image 470.26/479.94/375.20%; text extraction fresh/warm 225.97/212.62%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #725 run 46105522080, perf job 140857280902: 25/29 eligible; render 3.06–22.81% of PDFium; vector max 4.50%, text max 11.41%, JPEG max 22.81%. Four low-DPI JPEG exclusions. Inspect vector/text/image 425.25/564.15/416.84%; text extraction fresh/warm 288.26/264.64%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #727 run 46107646558, perf job 140863878653: 25/29 eligible; render 2.98–20.19% of PDFium; vector max 4.22%, text max 10.33%, JPEG max 20.19%. Four low-DPI JPEG exclusions. Inspect vector/text/image 454.97/515.58/409.66%; text extraction fresh/warm 237.78/260.03%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #729 run 46109707871, perf job 140870288420: 25/29 eligible; render 3.06–31.40% of PDFium; vector max 4.78%, text max 14.42%, JPEG max 31.40%. Four low-DPI JPEG exclusions. Inspect vector/text/image 330.20/381.23/269.47%; text extraction fresh/warm 191.36/204.23%. Shared-runner observation, no causal attribution or RSS verification; targets unchanged.


CI #733 run 46113753685, perf job 140882828270: 25/29 eligible; render 3.71–25.27% of PDFium; vector max 4.89%, text max 12.66%, JPEG max 25.27%. Four low-DPI JPEG exclusions. Inspect vector/text/image 419.99/544.25/405.79%; text extraction fresh/warm 182.17/200.78%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #735 run 46115835659, perf job 140889341541: 25/29 eligible; render 4.00–33.76% of PDFium; vector max 5.81%, text max 15.62%, JPEG max 33.76%. Four low-DPI JPEG exclusions. Inspect vector/text/image 373.93/512.06/357.66%; text extraction fresh/warm 209.57/200.99%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #737 run 46117933397, perf job 140895814409: 25/29 eligible; render 3.80–30.29% of PDFium; vector max 5.30%, text max 15.13%, JPEG max 30.29%. Four low-DPI JPEG exclusions. Inspect vector/text/image 336.33/428.98/319.08%; text extraction fresh/warm 203.58/223.38%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #739 run 46119963775, perf job 140902093876: 25/29 eligible; render 3.57–26.31% of PDFium; vector max 5.35%, text max 13.46%, JPEG max 26.31%. Four low-DPI JPEG exclusions. Inspect vector/text/image 495.90/573.51/446.13%; text extraction fresh/warm 201.47/223.52%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #741 run 46122039350, perf job 140908442318: 25/29 eligible; render 3.58–26.91% of PDFium; vector max 5.49%, text max 13.04%, JPEG max 26.91%. Four low-DPI JPEG exclusions. Inspect vector/text/image 446.50/502.87/429.43%; text extraction fresh/warm 164.90/191.03%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #743 run 46124051069, perf job 140914566456: 25/29 eligible; render 4.02–28.37% of PDFium; vector max 5.73%, text max 14.57%, JPEG max 28.37%. Four low-DPI JPEG exclusions. Inspect vector/text/image 376.02/476.09/297.22%; text extraction fresh/warm 192.85/223.48%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #745 run 46126164489, perf job 140921123924: 25/29 eligible; render 3.74–27.32% of PDFium; vector max 4.86%, text max 13.74%, JPEG max 27.32%. Four low-DPI JPEG exclusions. Inspect vector/text/image 451.63/485.39/461.76%; text extraction fresh/warm 210.25/211.21%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #747 run 46128266615, perf job 140927617426: 25/29 eligible; render 3.66–31.63% of PDFium; vector max 5.67%, text max 15.84%, JPEG max 31.63%. Four low-DPI JPEG exclusions. Inspect vector/text/image 482.24/572.23/430.51%; text extraction fresh/warm 188.15/193.04%. Shared-runner observation, no causal attribution or production RSS verification; targets unchanged.


CI #749 run 46130377272, perf job 140933970652: 25/29 eligible; render 3.49–24.04% of PDFium; vector max 4.79%, text max 13.15%, JPEG max 24.04%. Four low-DPI JPEG exclusions. Inspect vector/text/image 559.54/598.80/498.55%; text extraction fresh/warm 271.54/276.82%. Shared-runner observation; not a JBIG2-specific benchmark, no causal attribution or production RSS verification. Targets unchanged; 12i-u to establish a targeted baseline.


CI #751 run 46132467310, perf job 140940432201: 25/29 eligible; render 3.05–20.52% of PDFium; vector max 4.38%, text max 10.70%, JPEG max 20.52%. Four low-DPI JPEG exclusions. Inspect vector/text/image 438.99/492.36/381.57%; text extraction fresh/warm 212.29/268.59%. Shared-runner observation, not JBIG2-specific. Targets unchanged.

B11.3e4a4b 12i-u1 candidate: add standalone JBIG2 observational performance job (14 jobs/run). Native raw benchmark uses the existing feature-gated PDF image compatible entry and three Annex H seed pairs, 20 same-process repeated decodes checked against strict reclaimed bitmap and deterministic compatible result. Report median/p95, Mpix/s, dimensions, packed bytes, work, decisions, dictionary work, composition, whole-process peak RSS. Six pinned synthetic PDF cases (generic, MMR, stripe, halftone, refine, custom Huffman dictionary), 72dpi cold/warm, five measured isolated process pairs plus one discarded pair; exact reference RGB every trial required for eligibility. Pin test-only PDFium/Pillow wheels, record versions and hashes, retain raw trials, open time, median/p95 and RSS, report only qualified ratios. Existing perf probe permits 72dpi; ordinary default builds skip feature-gated raw example; production decoder unchanged. Require 3 raw and 12 page-mode results, native exact and at least one qualified pair, no speed target yet. Target 797/platform (2391), frozen 107/107, Fuzz 150000. No local compile/benchmark, actual results pending CI. RSS includes wrapper, process and output, not engine heap; raw fixtures are small, not full real scans. Next 12i-u2 inspect measured baseline and optimize native hot paths with pixel gates intact; larger scan workload and real scan tests remain pending. No other branch changes.

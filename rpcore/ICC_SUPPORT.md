# RPCore ICC source-color core — frozen scope candidate
Stage B10.5c5c3d4b5d4m. Closure verified at commit 7fe48de1000b21424b844784b285a411579ecb04, CI #107 run 45566224005 (10/10, mandatory external RGB native 1024/1024). Baseline verified at commit 3d79f1027c4e1c5c48597f47f7a55c01f22f8019, CI #105 run 45560773547. Closure commit passed all ten jobs before the B-line proceeds. This is a source-color subsystem milestone, not RPCore 1.0 or an ICC conformance certificate.

## Supported scope
| Capability | Native path / limits |
|---|---|
| ICC v2/v4 bounded profile inspection | RGB, GRAY, CMYK source paths; tag bounds, shared exact ranges, duplicate and overlap checks |
| RGB matrix/TRC | XYZ D50 input/display classes; sampled and supported parametric curves 0–4; LUT-bearing RGB not implicitly treated as matrix |
| Gray TRC | XYZ D50 input/display/output classes; supported curve and white-point subset |
| Legacy CMYK LUT | mft1/mft2 with supported PCS encodings; output class; multilinear CLUT |
| v4 CMYK mAB | Supported A/CLUT/B and A/CLUT/M/matrix/B subsets |
| MPE | Supported float matrix, CLUT and segmented curves; bounded program; PDF exact D2B intent path; B2D program inspection/evaluation API is not a display destination workflow |
| Rendering intent | Exact supported D2B tag, legacy CMYK A2B0/1/2 with documented fallback; legacy absolute media-white scaling; native D2B3 not double-scaled; matrix/TRC perceptual/saturation are approximations |
| CHAD | signed fixed-point 3x3, bounded inverse, full adaptation default; explicit Bradford cone-gain partial adaptation for absolute RGB/Gray matrix/TRC only |
| PDF integration | ICCBased vector fill/stroke and supported image samples, 1/3/4 components, default Range, compatible device Alternate; page-local intent cache and fixed page adaptation degree; alpha not interpreted as display color |
| Output | sRGB RGB8 rendering; not an arbitrary output-device conversion engine |

## CI #105 measured baseline (2026-11-25)
| Gate | Observations | Result |
|---|---:|---|
| Synthetic ICC rendering | 288 | all pass |
| Intent switching | 16 | all pass |
| Synthetic legacy absolute | 192 | all pass; RGB8 max 0 |
| RGB/Gray absolute | 384 | all pass; max 1 |
| Partial Bradford RGB | 160 | all pass; max 0 |
| Gray partial / missing CHAD | 320 | all pass; max 0 |
| Non-Bradford diagnostic | 2 | all pass; diagnostic cases, not color comparisons |
| SWOP, FOGRA40, CRPC6 relative | 1536 | all native; max 2 |
| SWOP, FOGRA40, CRPC6 absolute | 1536 | all native; max 2 |
| Installed sRGB, AdobeRGB1998 | 1024 | all native; max 1 |
| Total render/diagnostic observations above | 5458 | all pass (5456 render, 2 diagnostic) |
| Independent raw MPE matrix/CLUT/mixed | 1875 | max raw channel error 0.000000093 |
| Rust test executions | 609 | 203 each Windows/Linux/macOS |

RGB8 maximum channel error is not Delta E. Samples are not a random or comprehensive PDF/profile population. CI #105 installed profile SHA256: sRGB.icc 1bdc23fb14463058a52ac513bf3bff4e3c52fd75c919cfb0da35dd7e71fbded0; AdobeRGB1998.icc 59b0418391ee14b5edc84ece59a51bedbfc10ef912cf038490c2bd0ff48fd1da. These are packaged working-space profiles, not measured monitor calibrations. Each new report records actual hashes; package changes require review rather than assuming identical bytes.

## Closure gate
Keep all existing CI gates. Both installed RGB profiles now MUST use native vector/image paths for all 512 observations each (RGB8 error <=3, parity/alpha <=1, uniform interiors); a fallback fails this fixture gate. CMYK dense gates retain max <=12, mean <=3, P95 <=6; synthetic RGB comparisons retain their existing thresholds. LittleCMS and Poppler remain test-only. No dependency or rendering algorithm change in closure commit.

## Resource and performance review
- 16 MiB maximum encoded or decoded profile; 32 MiB charged profile bytes and 16 profile/intent entries per vector or image cache, independently scoped; not an aggregate process-memory limit.
- Profile tag count <=1024; curve samples <=65536. MPE <=64 elements, <=16 channels, <=256 segments per curve, bounded per-evaluation work; PDF rejects MPE work >4096. Owned MPE bytes charged conservatively; image work charged separately.
- Invalid/unsupported profiles use explicit compatible device alternate in supported contexts; recognized N mismatch and resource-limit errors fail. Diagnostics record source/path/reason. No silent claim of native accuracy for an alternate.
- Historical CRPC6 transform-only benchmark (CI 45531992235) reported about 7.34 million XYZ evaluations/s on a shared runner; not a current page-render latency result, nor an installed-device benchmark. No new performance speedup claim. Global cumulative process limits, fuzzing, peak RSS and desktop page performance remain release tasks.

## Stable use and exclusions
Existing default render_page / render_page_with_text_quality remain fully adapted. Opt-in AdaptationDegree and render_page_with_adaptation use the autonomous source-CHAD policy, not LittleCMS partial-adaptation equivalence. page_display_list_with_adaptation is for the same page policy. Existing APIs are additive at this milestone; no stable binary ABI is promised.
Not certified: all ICC v2/v4 variants, ICCmax, arbitrary channel counts, arbitrary destination/output profiles, full reverse conversion, black-point compensation, arbitrary LUT types/classes, non-Bradford partial adaptation, measured hardware calibration, complete PDF OutputIntent/ICC blending-space workflow, or production sandboxing. Missing CHAD keeps the full-adaptation source behavior; unsupported non-Bradford partial CHAD has explicit alternate diagnostics.

Freeze this validated source-color scope when closure CI succeeds; reopen only for actual compatibility, correctness or security regressions. Next B11: inventory PDF file compatibility and prioritize encryption and unsupported image/stream formats using real corpus evidence. Do not merge or remove PDF.js until overall integration gates pass.

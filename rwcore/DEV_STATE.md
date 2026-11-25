# C 线路当前研发检查点

- Last verified baseline: b6501d281249eec45a40320f6be30b5371f8868d
- CI #75: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45587882429
- Actual logs: 504 passed per platform (1512 total), zero failed/ignored; release profiles finished. Workflow uses locked test/build.
- Current: C4.3h-2a real ABeeZee Regular OFL fixture, independently extracted nominal metrics and rendering smoke/metamorphic checks. Five added tests (509 per platform / 1527 expected, pending CI).
- Next: verify exact HEAD and actual logs, fix evidenced failures; then C4.3h-2b independent unhinted reference grayscale/geometry comparison. No independent raster acceptance yet.

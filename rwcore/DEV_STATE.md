# C 线路当前研发检查点

- Last verified baseline: 766c0e5af9dc3468fdbc48cd016d3949e773b96a
- CI #76: https://github.com/trade3403/ai-research-pdf-reader/actions/runs/45595093198
- Logs: 509 passed per platform (1527 total), zero failed/ignored, exact SHA and locked release builds verified.
- Current: C4.3h-2b test-only dense quadratic sampling and point-in-polygon grayscale oracle for ABeeZee, six additional tests (515/platform, 1545 expected, pending CI).
- Oracle shares production parsed paths; independent flattening and rasterization only. Explicit error tolerances; no external engine, full outline independence, analytic coverage or broad font acceptance.
- Next: verify exact HEAD and all logs, including new oracle tests; repair evidenced failures. Then extend independent geometry validation and real-font corpus before C4.3h closure.

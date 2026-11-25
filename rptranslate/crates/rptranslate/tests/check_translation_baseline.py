"""Measure the Rust process, not PyTorch; record quality observations."""
import collections
import json
import math
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

package, exe = map(Path, sys.argv[1:3])
from translation_quality_fixture import CASES, norm, char_f

with tempfile.TemporaryDirectory() as temp:
    rss_file = Path(temp) / "rss.txt"
    result = subprocess.run(
        ["/usr/bin/time", "-f", "%M", "-o", str(rss_file),
         str(exe.resolve()), str(package.resolve())],
        text=True, encoding="utf-8", capture_output=True, timeout=240, check=True,
    )
    rss_kib = int(rss_file.read_text().strip())
    assert rss_kib > 0
    print(f"BENCH_PROCESS peak_rss_kib={rss_kib} peak_rss_mib={rss_kib/1024:.3f}", flush=True)

runs = collections.defaultdict(list)
quality = {}
loads = 0
reuse = None
for line in result.stdout.splitlines():
    fields = line.split("\t")
    if fields[0] == "BENCH_LOAD":
        _, load, session, payload, avx = fields
        assert int(payload) == 312033188 and avx in ("true", "false")
        assert all(math.isfinite(float(v)) and float(v) > 0 for v in (load, session))
        print(f"BENCH_LOAD model_ms={load} session_ms={session} weight_bytes={payload} avx2={avx}", flush=True)
        loads += 1
    elif fields[0] == "BENCH_REUSE":
        _, count, rebuild, reused, equal = fields
        assert reuse is None and int(count) == 6 and equal == "true"
        reuse = (float(rebuild), float(reused))
        assert all(math.isfinite(v) and v > 0 for v in reuse)
        print(f"RESIDENT_REFERENCE PASS precision=fp32 sessions=6 tokens_equal=true "
              f"repeat_translation_equal=true rebuild_ms={reuse[0]:.6f} reuse_ms={reuse[1]:.6f}", flush=True)
    elif fields[0] == "BENCH_RUN":
        _, case, repeat, mode, source_tokens, generated, seconds = fields
        case, repeat, source_tokens, generated = map(int, (case, repeat, source_tokens, generated))
        seconds = float(seconds)
        assert case in (0, 1) and repeat in (0, 1, 2) and mode in ("Full", "Cached")
        assert source_tokens > 0 and generated > 0 and math.isfinite(seconds) and seconds > 0
        runs[case, mode].append((repeat, generated, seconds))
        print(f"BENCH_RUN case={case} repeat={repeat} mode={mode} source_tokens={source_tokens} "
              f"generated_tokens={generated} seconds={seconds:.6f} output_tokens_per_second={generated/seconds:.6f}", flush=True)
    elif fields[0] == "BENCH_QUALITY":
        _, case, generated, limit, hex_text = fields
        case = int(case)
        assert 0 <= case < len(CASES) and case not in quality
        text = bytes.fromhex(hex_text).decode("utf-8")
        assert text and int(generated) > 0 and limit in ("0", "1")
        quality[case] = (text, limit == "1", int(generated))
    else:
        raise AssertionError(line)
assert loads == 1 and reuse is not None and len(runs) == 4 and len(quality) == len(CASES)
for case in range(2):
    stats = {}
    for mode in ("Full", "Cached"):
        rows = runs[case, mode]
        assert sorted(x[0] for x in rows) == [0, 1, 2]
        assert len({x[1] for x in rows}) == 1
        med = statistics.median(x[2] for x in rows)
        stats[mode] = med
        print(f"BENCH_MEDIAN case={case} mode={mode} repeats=3 seconds={med:.6f} "
              f"output_tokens_per_second={rows[0][1]/med:.6f}", flush=True)
    assert runs[case, "Full"][0][1] == runs[case, "Cached"][0][1]
    print(f"BENCH_CACHE case={case} latency_ratio_full_over_cached={stats['Full']/stats['Cached']:.6f}", flush=True)

number_hit = number_total = term_hit = term_total = capped = 0
scores = []
for case, (source, reference, numbers, terms) in enumerate(CASES):
    actual, limit, tokens = quality[case]
    observed = collections.Counter(re.findall(r"[0-9]+(?:\.[0-9]+)?", norm(actual)))
    hits = sum((collections.Counter(numbers) & observed).values())
    matches = sum(any(norm(term) in norm(actual) for term in alternatives) for alternatives in terms)
    score = char_f(actual, reference)
    number_hit += hits
    number_total += len(numbers)
    term_hit += matches
    term_total += len(terms)
    capped += limit
    scores.append(score)
    print("QUALITY_CASE " + json.dumps({
        "case": case, "source": source, "reference": reference, "actual": actual,
        "generated_tokens": tokens, "reached_limit": limit, "numbers": [hits, len(numbers)],
        "term_groups": [matches, len(terms)], "char_f2": round(score, 3),
    }, ensure_ascii=False), flush=True)
print(f"TRANSLATION_BASELINE PASS timing_runs=12 warmups=2 quality_cases={len(CASES)} "
      f"number_matches={number_hit}/{number_total} term_matches={term_hit}/{term_total} "
      f"mean_char_f2={statistics.mean(scores):.3f} capped={capped} "
      f"peak_rss_kib={rss_kib} quality_status=observational", flush=True)

"""Same-runner FP32/INT8 benchmark with separately measured Rust peak RSS."""
import collections
import json
import math
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path
from translation_quality_fixture import CASES, norm, char_f

fp32, mixed, exe, cli = [Path(v).resolve() for v in sys.argv[1:5]]
def run(mode, root, temp):
    rss = temp / (mode + ".rss")
    proc = subprocess.run(
        ["/usr/bin/time", "-f", "%M", "-o", str(rss), str(exe), str(root), mode],
        check=True, capture_output=True, text=True, encoding="utf-8", timeout=240)
    peak = int(rss.read_text().strip())
    assert peak > 0
    runs, quality = collections.defaultdict(list), {}
    load = None
    reuse = None
    for line in proc.stdout.splitlines():
        fields = line.split("\t")
        if fields[0] == "BENCH_LOAD":
            _, ms, session, payload, avx = fields
            assert load is None and int(payload) == {"fp32":312033188,"int8":84903620}[mode]
            assert avx in ("true","false")
            assert all(math.isfinite(float(v)) and float(v)>0 for v in (ms,session))
            load = (float(ms),float(session),avx,int(payload))
        elif fields[0] == "BENCH_REUSE":
            _, count, rebuild, reused, equal = fields
            assert reuse is None and int(count) == 6 and equal == "true"
            reuse = (float(rebuild), float(reused))
            assert all(math.isfinite(v) and v > 0 for v in reuse)
        elif fields[0] == "BENCH_RUN":
            _, case, rep, kernel, src, count, seconds = fields
            case, rep, src, count = map(int,(case,rep,src,count))
            seconds = float(seconds)
            assert case in (0,1) and rep in (0,1,2) and kernel in ("Full","Cached")
            assert src>0 and count>0 and math.isfinite(seconds) and seconds>0
            runs[case,kernel].append((rep,src,count,seconds))
        elif fields[0] == "BENCH_QUALITY":
            _, case, count, cap, hex_text = fields
            case, count = int(case), int(count)
            actual = bytes.fromhex(hex_text).decode("utf-8")
            assert case not in quality and 0<=case<len(CASES) and count>0 and cap in ("0","1")
            assert actual and any("\u4e00"<=c<="\u9fff" for c in actual)
            quality[case] = (actual,count,int(cap))
        else:
            raise AssertionError(line)
    assert load is not None and reuse is not None and len(runs)==4 and len(quality)==len(CASES)
    print(f"RESIDENT_REFERENCE PASS precision={mode} sessions=6 tokens_equal=true "
          f"repeat_translation_equal=true rebuild_ms={reuse[0]:.6f} reuse_ms={reuse[1]:.6f}",flush=True)
    medians = {}
    for (case,kernel), rows in runs.items():
        assert sorted(row[0] for row in rows)==[0,1,2]
        assert len({row[1:3] for row in rows})==1
        sec = statistics.median(row[3] for row in rows)
        medians[case,kernel] = (rows[0][1],rows[0][2],sec)
        print(f"PAIRED_MEDIAN precision={mode} case={case} mode={kernel} seconds={sec:.6f} "
              f"source_tokens={rows[0][1]} generated_tokens={rows[0][2]} "
              f"tokens_per_second={rows[0][2]/sec:.6f}",flush=True)
    for case in (0,1):
        assert medians[case,"Full"][:2] == medians[case,"Cached"][:2]
    package_bytes = sum(p.stat().st_size for p in root.iterdir() if p.is_file())
    print(f"PAIRED_LOAD precision={mode} model_ms={load[0]:.3f} session_ms={load[1]:.9f} "
          f"avx2={load[2]} payload_bytes={load[3]} package_bytes={package_bytes} peak_rss_kib={peak}",flush=True)
    scores, numbers, terms, capped = [], 0, 0, 0
    for case,(source,reference,expected,groups) in enumerate(CASES):
        actual,count,cap = quality[case]
        observed = collections.Counter(re.findall(r"[0-9]+(?:\.[0-9]+)?",norm(actual)))
        hits = sum((collections.Counter(expected)&observed).values())
        matched = sum(any(norm(t) in norm(actual) for t in group) for group in groups)
        score = char_f(actual,reference)
        scores.append(score)
        numbers += hits
        terms += matched
        capped += cap
        print("PAIRED_QUALITY "+json.dumps({
            "precision":mode,"case":case,"source":source,"reference":reference,
            "actual":actual,"generated_tokens":count,"reached_limit":bool(cap),
            "numbers":[hits,len(expected)],"terms":[matched,len(groups)],
            "char_f2":round(score,3)},ensure_ascii=False),flush=True)
    command = "translate" if mode=="fp32" else "translate-int8"
    check = subprocess.run([str(cli),command,str(root),CASES[0][0],"4","64"],
                           check=True,capture_output=True,text=True,encoding="utf-8",timeout=90)
    assert check.stdout.strip()==quality[0][0]
    assert bool(check.stderr.strip())==bool(quality[0][2])
    for invalid in (" ",">>cmn_Hant<< hello"):
        rejected = subprocess.run([str(cli),command,str(root),invalid,"1","16"],
                                  capture_output=True,text=True,encoding="utf-8",timeout=90)
        assert rejected.returncode!=0 and not rejected.stdout
    return dict(load=load,peak=peak,medians=medians,quality=quality,score=statistics.mean(scores),
                numbers=numbers,terms=terms,capped=capped)

with tempfile.TemporaryDirectory() as directory:
    a = run("fp32",fp32,Path(directory))
    b = run("int8",mixed,Path(directory))
assert a["load"][2] == b["load"][2]
for case in (0,1):
    x,y = a["medians"][case,"Cached"],b["medians"][case,"Cached"]
    assert x[0]==y[0]
    print(f"PAIRED_RATIO case={case} cached_latency_fp32_over_int8={x[2]/y[2]:.6f} "
          f"throughput_int8_over_fp32={(y[1]/y[2])/(x[1]/x[2]):.6f} output_length_equal={x[1]==y[1]}",flush=True)
same = sum(a["quality"][case][0]==b["quality"][case][0] for case in range(len(CASES)))
print(f"PAIRED_TRANSLATION_REFERENCE PASS timing_runs=24 quality_outputs=12 cli=2 invalid=4 "
      f"fp32_rss_kib={a['peak']} int8_rss_kib={b['peak']} rss_ratio={a['peak']/b['peak']:.6f} "
      f"fp32_char_f2={a['score']:.3f} int8_char_f2={b['score']:.3f} "
      f"fp32_numbers={a['numbers']}/5 int8_numbers={b['numbers']}/5 "
      f"fp32_terms={a['terms']}/12 int8_terms={b['terms']}/12 "
      f"fp32_capped={a['capped']} int8_capped={b['capped']} same_text={same}/6 "
      "quality_status=observational",flush=True)

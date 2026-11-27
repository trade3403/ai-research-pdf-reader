#!/usr/bin/env python3
"""External isolated Latin MarkToBase feature reference, no product dependency.

Candidate selection is based only on HB glyph identity and baseline scope,
never on agreement with RWCore. Every admitted candidate must match exactly.
The JSON retains all candidate exclusions, including Unicode composition.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from harfbuzz_gpos_reference import HB

# Non-composing and potentially composing candidates deliberately included.
# Actual HB normalization determines admission, not a precomputed glyph list.
BASES = "xqvwjfAOT"
MARKS = ("\u0301", "\u0300", "\u0302", "\u0308", "\u0304", "\u0323", "\u0307")
DEFAULT_FONT = Path("core/tests/fixtures/alegreyasans/AlegreyaSans-Regular.ttf")

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--probe", required=True, type=Path)
    p.add_argument("--report", required=True, type=Path)
    p.add_argument("--font", type=Path, default=DEFAULT_FONT)
    args = p.parse_args()
    hb = HB()
    data = args.font.read_bytes()
    report = {"harfbuzz": hb.version_string().decode(),
              "font": args.font.name, "sha256": hashlib.sha256(data).hexdigest(),
              "scope": "Latin mark toggle; HB glyph IDs and mark-disabled widths; LTR UPEM",
              "candidates": len(BASES)*len(MARKS), "cases": 0, "nonzero_cases": 0,
              "mismatches": 0, "max_absolute_error": 0, "admitted": [], "excluded": []}
    try:
        groups = []
        for base in BASES:
            for mark in MARKS:
                text = base + mark
                ga, on, upem = hb.shape(data, text, False, mark=True)
                gb, off, upem_off = hb.shape(data, text, False, mark=False)
                row = {"text": text, "codepoints": [ord(c) for c in text],
                       "glyphs_on": ga, "glyphs_off": gb, "upem": upem}
                reason = None
                if ga != gb or upem != upem_off:
                    reason = "feature changes glyph identity or scale"
                elif len(ga) != 2:
                    reason = "normalized glyph count is not two"
                elif any(g == 0 for g, _ in ga):
                    reason = "missing glyph"
                elif any(v[1:] != [0, 0, 0] for v in off):
                    reason = "baseline has vertical advance or placement"
                if reason:
                    row["reason"] = reason
                    report["excluded"].append(row)
                    continue
                row["baseline"] = off
                row["reference_delta"] = [[x-y for x,y in zip(a,b)] for a,b in zip(on,off)]
                report["admitted"].append(row)
                groups.append(",".join(f"{g}:{v[0]}" for (g,_),v in zip(ga,off)))
        if not groups:
            raise RuntimeError("no admissible mark reference samples")
        proc = subprocess.run([str(args.probe.resolve()), str(args.font), "--mark", *groups],
                              capture_output=True, text=True, timeout=60)
        if proc.returncode:
            raise RuntimeError(f"probe failed ({proc.returncode}): {proc.stderr}")
        lines = proc.stdout.splitlines()
        if len(lines) != len(groups):
            raise RuntimeError("probe row count")
        for row,line in zip(report["admitted"],lines):
            actual = [[int(x) for x in part.split(",")] for part in line.split(";")]
            expected = row["reference_delta"]
            if len(actual) != 2 or any(len(v) != 4 for v in actual):
                raise RuntimeError("probe field count")
            errors = [abs(x-y) for a,b in zip(actual,expected) for x,y in zip(a,b)]
            row["rwcore_delta"] = actual
            row["max_absolute_error"] = max(errors)
            report["cases"] += 1
            # Require actual reference positioning, not only advance changes.
            report["nonzero_cases"] += int(any(v[2] or v[3] for v in expected))
            report["mismatches"] += int(actual != expected)
            report["max_absolute_error"] = max(report["max_absolute_error"], *errors)
            print("mark", row["codepoints"], "max_error", max(errors), flush=True)
        if report["cases"] != len(groups) or report["nonzero_cases"] < 3:
            raise RuntimeError("insufficient positive mark positioning evidence")
        if report["mismatches"]:
            raise RuntimeError(f"{report['mismatches']} mark reference mismatches")
    except Exception as e:
        report["error"] = str(e)
        raise
    finally:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print("HB_MARK_REFERENCE", json.dumps({k:v for k,v in report.items() if k not in ("admitted","excluded")}), flush=True)

if __name__ == "__main__":
    main()

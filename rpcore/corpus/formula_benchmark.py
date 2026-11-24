#!/usr/bin/env python3
"""Real-paper formula benchmark. Pinned pages/regions and human transcription.
No PDFs or full page text in artifacts; only selected formulas and diagnostics.
Strict string matching is a limited transcription metric, not math equivalence.
"""
import argparse
import collections
import hashlib
import json
import math
import os
import pathlib
import re
import subprocess
import tempfile
import time
from run import download, KNOWN

ROOT = pathlib.Path(__file__).parent

def invoke(probe, path, page, rect, font_dir):
    env = os.environ.copy()
    env.pop("RPCORE_STANDARD_FONT_DIR", None)
    if font_dir:
        env["RPCORE_STANDARD_FONT_DIR"] = str(font_dir)
    start = time.monotonic()
    try:
        p = subprocess.run([str(probe), str(path), "math", str(page), ",".join(map(str,rect))],
                           env=env, capture_output=True, text=True, encoding="utf-8", errors="strict", timeout=20)
    except subprocess.TimeoutExpired:
        return {"status":"timeout"}
    except (OSError, UnicodeError):
        return {"status":"protocol_error"}
    if p.returncode:
        return {"status":"process_error","exit_code":p.returncode}
    fields = dict(line.split("=",1) for line in p.stdout.splitlines() if "=" in line)
    if fields.get("status") not in KNOWN:
        return {"status":"protocol_error"}
    for key in ("selected","mapped","scripts","fractions","radicals","engine_ms"):
        if key in fields:
            try:
                fields[key] = int(fields[key])
            except ValueError:
                return {"status":"protocol_error"}
    if fields["status"] == "ok" and (not fields.get("latex") or fields.get("math_tree")!="ok" or not fields.get("selected")):
        return {"status":"protocol_error"}
    fields["wall_ms"] = round((time.monotonic()-start)*1000)
    return fields

def normalize(s):
    # Whitespace only. Do not erase braces, signs or structure.
    return "".join(s.split())

def outcome(result, expected):
    if result["status"] == "ok":
        return "exact_match" if normalize(result["latex"]) in {normalize(x) for x in expected} else "mismatch"
    if result["status"] in ("invalid","unsupported","limit"):
        if result.get("math_tree") == "ok":
            return "latex_rejected"
        if result.get("math_geometry") == "ok":
            return "tree_rejected"
        return "geometry_rejected"
    return result["status"]

def load_cases(path, papers):
    data = json.loads(path.read_text(encoding="utf-8"))
    cases = data["cases"]
    if data["schema"] != 1 or not 1 <= len(cases) <= 64:
        raise ValueError("invalid formula manifest")
    ids = set()
    for c in cases:
        if not re.fullmatch(r"[a-z0-9-]{1,64}",c["id"]) or c["id"] in ids or c["paper"] not in papers:
            raise ValueError("invalid formula case ID")
        ids.add(c["id"])
        if type(c["page"]) is not int or not 1<=c["page"]<=1000:
            raise ValueError("invalid page")
        r=c["rect"]
        if len(r)!=4 or not all(type(x) in (int,float) and math.isfinite(x) and 0<=x<=1 for x in r) or not(r[0]<r[2] and r[1]<r[3]):
            raise ValueError("invalid formula rectangle")
        if not 1<=len(c["expected"])<=8 or not all(isinstance(x,str) and 0<len(x)<=4096 for x in c["expected"]):
            raise ValueError("missing reference transcription")
    return cases

def report_md(report):
    lines=["# RPCore real-paper formula benchmark","",
           "Manually selected normalized CropBox regions. Exact transcription comparison ignores whitespace only; not mathematical equivalence or automatic formula detection.",
           "Strict and explicit URW runs are scored separately. All cases, including rejected and failed cases, remain in the denominator.","",
           "| Case | Font mode | Page | Result | Selected | Mapped | Scripts | Fractions | Roots | Reason |",
           "|---|---|---:|---|---:|---:|---:|---:|---:|---|"]
    for row in report["rows"]:
        v=row["result"]
        fields=[row["id"],row["font_mode"],row["page"],row["outcome"],v.get("selected","—"),v.get("mapped","—"),v.get("scripts","—"),v.get("fractions","—"),v.get("radicals","—"),v.get("reason","—")]
        lines.append("| "+" | ".join(str(x).replace("|",r"\|").replace("\n"," ") for x in fields)+" |")
    lines+=["","## Summary",""]
    for mode,counts in report["counts"].items():
        lines.append(f"- {mode}: {counts.get('exact_match',0)}/{sum(counts.values())} exact matches; {json.dumps(counts,sort_keys=True)}")
    lines+=["","Reference and candidate LaTeX, source hashes and timings are in report.json. No threshold for fidelity is enforced."]
    return "\n".join(lines)+"\n"

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--probe",required=True,type=pathlib.Path)
    p.add_argument("--out",required=True,type=pathlib.Path)
    p.add_argument("--font-dir",type=pathlib.Path)
    p.add_argument("--cases",type=pathlib.Path,default=ROOT/"formulas.json")
    a=p.parse_args()
    if not a.probe.is_file() or (a.font_dir and not a.font_dir.is_dir()):
        p.error("missing probe or font directory")
    papers={x["id"]:x for x in json.loads((ROOT/"papers.json").read_text(encoding="utf-8"))}
    cases=load_cases(a.cases,papers)
    modes=[("strict",None)]+([("explicit_urw",a.font_dir.resolve())] if a.font_dir else [])
    report={"schema":1,"sources":{},"rows":[],"counts":{}}
    deadline=time.monotonic()+240
    with tempfile.TemporaryDirectory(prefix="rpcore-formula-") as tmp:
        files={}
        for pid in sorted({c["paper"] for c in cases}):
            path=pathlib.Path(tmp)/(pid+".pdf")
            try:
                if time.monotonic()>=deadline:
                    raise TimeoutError("benchmark budget")
                size=download(papers[pid]["url"],path)
                report["sources"][pid]={"status":"ok","url":papers[pid]["url"],"sha256":hashlib.sha256(path.read_bytes()).hexdigest(),"bytes":size}
                files[pid]=path
            except Exception as e:
                report["sources"][pid]={"status":"download_error","reason":str(e)[:300]}
        for mode,fonts in modes:
            counts=collections.Counter()
            for c in cases:
                if c["paper"] not in files:
                    v={"status":"download_error"}
                elif time.monotonic()>=deadline:
                    v={"status":"budget_skipped"}
                else:
                    v=invoke(a.probe.resolve(),files[c["paper"]],c["page"],c["rect"],fonts)
                result=outcome(v,c["expected"])
                counts[result]+=1
                report["rows"].append({**c,"font_mode":mode,"result":v,"outcome":result})
            report["counts"][mode]=dict(counts)
    a.out.mkdir(parents=True,exist_ok=True)
    (a.out/"report.json").write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
    md=report_md(report)
    (a.out/"report.md").write_text(md,encoding="utf-8")
    print(md)
    # Recognition failures are observations. Infrastructure errors fail CI.
    return int(any(x["outcome"] in {"timeout","process_error","protocol_error","download_error","budget_skipped"} for x in report["rows"]))

if __name__=="__main__":
    raise SystemExit(main())

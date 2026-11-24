#!/usr/bin/env python3
"""Bounded research PDF corpus runner; standard-library only.

No paper PDFs, text or rendered pages are uploaded by default. Results measure
execution coverage, not visual correctness. Unknown/unsupported is not success.
"""
import argparse
import collections
import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
import time
import urllib.request

MAX_DOWNLOAD = 25 * 1024 * 1024
STAGES = ("content", "text", "render")
KNOWN = {"ok", "invalid", "unsupported", "limit"}

def download(url, path):
    req = urllib.request.Request(url, headers={"User-Agent": "RPCore-research-corpus/0.1 (PDF compatibility testing)"})
    start = time.monotonic()
    with urllib.request.urlopen(req, timeout=15) as response, path.open("wb") as out:
        if not response.geturl().startswith("https://arxiv.org/"):
            raise ValueError("unexpected download redirect")
        size = 0
        while True:
            if time.monotonic() - start > 30:
                raise TimeoutError("download budget")
            b = response.read(65536)
            if not b:
                break
            size += len(b)
            if size > MAX_DOWNLOAD:
                raise ValueError("download size limit")
            out.write(b)
    with path.open("rb") as f:
        if not f.read(1024).startswith(b"%PDF-"):
            raise ValueError("not a PDF")
    return size

def invoke(exe, path, stage, page=None, png=None, font_dir=None, dpi=36, text_aa=2, text_grid=False):
    cmd = [str(exe), str(path), stage]
    if page is not None:
        cmd.append(str(page))
    if png is not None:
        cmd.append(str(png))
    env = os.environ.copy()
    env.pop("RPCORE_STANDARD_FONT_DIR", None)
    env.pop("RPCORE_RENDER_DPI", None)
    env.pop("RPCORE_TEXT_AA", None)
    env.pop("RPCORE_TEXT_GRID", None)
    if stage == "render":
        if dpi not in (36, 72, 144):
            raise ValueError("unsupported render DPI")
        env["RPCORE_RENDER_DPI"] = str(dpi)
        if text_aa not in (2,4):
            raise ValueError("unsupported text AA")
        env["RPCORE_TEXT_AA"] = str(text_aa)
        if text_grid and text_aa != 4:
            raise ValueError("grid fitting requires text AA 4")
        env["RPCORE_TEXT_GRID"] = "1" if text_grid else "0"
    if font_dir is not None and stage == "render":
        env["RPCORE_STANDARD_FONT_DIR"] = str(font_dir)
    start = time.monotonic()
    try:
        p = subprocess.run(cmd, env=env, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=15)
    except subprocess.TimeoutExpired:
        return {"status": "timeout", "wall_ms": round((time.monotonic()-start)*1000)}
    elapsed = round((time.monotonic()-start)*1000)
    if p.returncode != 0:
        return {"status": "process_error", "exit_code": p.returncode, "wall_ms": elapsed, "diagnostic": p.stderr[-1200:]}
    fields = dict(line.split("=", 1) for line in p.stdout.splitlines() if "=" in line)
    if fields.get("status") not in KNOWN:
        return {"status": "protocol_error", "wall_ms": elapsed}
    for key in ("pages", "bytes", "engine_ms", "operations", "glyphs", "mapped", "warnings", "width", "height", "nonwhite_pixels", "png_bytes"):
        if key in fields:
            try:
                fields[key] = int(fields[key])
            except ValueError:
                return {"status": "protocol_error", "wall_ms": elapsed}
    fields["wall_ms"] = elapsed
    return fields

def sample_pages(n):
    return sorted({1, (n+1)//2, n}) if n > 0 else []

def process(exe, item, path, report_dir, save_png, deadline, all_pages, font_dir):
    result = dict(item)
    result.pop("local", None)
    try:
        if "local" in item:
            path = pathlib.Path(item["local"]).resolve()
            size = path.stat().st_size
            if size > MAX_DOWNLOAD:
                raise ValueError("local size limit")
        else:
            size = download(item["url"], path)
        result["download"] = {"status": "ok", "bytes": size, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    except Exception as e:
        result["download"] = {"status": "download_error", "reason": str(e)[:300]}
        return result
    result["inspect"] = invoke(exe, path, "inspect")
    inspect = result["inspect"]
    result["pages"] = []
    if inspect["status"] != "ok":
        return result
    n = inspect.get("pages", 0)
    if n <= 0 or n > 100000:
        result["inspect"]["status"] = "protocol_error"
        return result
    selected = list(range(1, min(n, 30)+1)) if all_pages else sample_pages(n)
    result["selected_pages"] = selected
    for page in selected:
        row = {"page": page}
        for stage in STAGES:
            if time.monotonic() >= deadline:
                row[stage] = {"status": "budget_skipped"}
                continue
            png = report_dir / f"{item['id']}-{page}.png" if save_png and stage == "render" else None
            row[stage] = invoke(exe, path, stage, page, png, font_dir)
        result["pages"].append(row)
    return result

def summarize(results):
    counts = {s: collections.Counter() for s in ("download", "inspect", *STAGES)}
    reasons = collections.Counter()
    for doc in results:
        for s in ("download", "inspect"):
            if s in doc:
                counts[s][doc[s]["status"]] += 1
                if doc[s].get("reason"):
                    reasons[(s, doc[s]["status"], doc[s]["reason"])] += 1
        for page in doc.get("pages", []):
            for s in STAGES:
                v = page[s]
                counts[s][v["status"]] += 1
                if v.get("reason"):
                    reasons[(s, v["status"], v["reason"])] += 1
    return {"counts": {k: dict(v) for k,v in counts.items()},
            "failure_reasons": [{"stage": k[0], "status": k[1], "reason": k[2], "count": v} for k,v in reasons.most_common()]}

def markdown(results, summary, font_dir):
    lines = ["# RPCore real-paper corpus report", "", "Strict render at 0.5 px/PDF unit; no missing-text fallback. Execution success is NOT a visual-fidelity score.", "", "| Stage | OK | Other | Total |", "|---|---:|---:|---:|"]
    for stage, c in summary["counts"].items():
        total = sum(c.values())
        lines.append(f"| {stage} | {c.get('ok',0)} | {total-c.get('ok',0)} | {total} |")
    lines[2] = ("Explicit URW substitute fonts enabled; not strict original-font coverage. " if font_dir else "Strict original-font render. ") + "0.5 px/PDF unit; no missing-text fallback. Execution success is NOT a visual-fidelity score."
    lines += ["", "| Paper | Download | Parse | Page | Content | Text | Render | Render ms | Substituted faces |", "|---|---|---|---:|---|---|---|---:|---|"]
    for d in results:
        prefix = f"| {d['id']} | {d['download']['status']} | {d.get('inspect',{}).get('status','not_run')} |"
        if not d.get("pages"):
            lines.append(prefix+" — | — | — | — | — | — |")
        for p in d.get("pages", []):
            lines.append(prefix+f" {p['page']} | {p['content']['status']} | {p['text']['status']} | {p['render']['status']} | {p['render'].get('wall_ms','—')} | {p['render'].get('substituted_fonts') or '—'} |")
    lines += ["", "## Failure reasons", ""]
    for x in summary["failure_reasons"]:
        reason = x["reason"].replace("|", r"\|").replace("\n", " ")
        lines.append(f"- {x['stage']} / {x['status']} / {reason}: {x['count']}")
    return "\n".join(lines)+"\n"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", required=True, type=pathlib.Path)
    parser.add_argument("--out", required=True, type=pathlib.Path)
    parser.add_argument("--manifest", type=pathlib.Path, default=pathlib.Path(__file__).with_name("papers.json"))
    parser.add_argument("--font-dir", type=pathlib.Path, help="Explicit URW Base35 OTF directory; separate substitute coverage")
    parser.add_argument("--local", type=pathlib.Path, help="Run one local PDF instead of downloading papers")
    parser.add_argument("--all-pages", action="store_true", help="Up to 30 pages per paper instead of first/middle/last")
    parser.add_argument("--save-png", action="store_true", help="Local opt-in only; respect source permissions")
    parser.add_argument("--require-success", action="store_true", help="Fail if any stage is not OK (synthetic smoke test)")
    a = parser.parse_args()
    if a.save_png and a.local is None:
        parser.error("--save-png requires --local")
    exe = a.probe.resolve()
    if not exe.is_file():
        parser.error("probe executable missing")
    items = [{"id": "local", "title": "Local PDF", "local": str(a.local)}] if a.local else json.loads(a.manifest.read_text(encoding="utf-8"))
    if a.font_dir is not None:
        a.font_dir = a.font_dir.resolve()
        if not a.font_dir.is_dir():
            parser.error("font directory missing")
    a.out.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic()+360
    with tempfile.TemporaryDirectory(prefix="rpcore-corpus-") as tmp:
        results = []
        for item in items:
            if time.monotonic() >= deadline:
                results.append({**item, "download":{"status":"budget_skipped"}})
                continue
            d = process(exe, item, pathlib.Path(tmp)/f"{item['id']}.pdf", a.out, a.save_png, deadline, a.all_pages, a.font_dir)
            results.append(d)
            print(item["id"], d["download"]["status"], d.get("inspect",{}).get("status","not_run"), flush=True)
    summary = summarize(results)
    (a.out/"report.json").write_text(json.dumps({"schema":1,"font_mode":"explicit_urw" if a.font_dir else "strict","scale":0.5,"selection":"up_to_30" if a.all_pages else "first_middle_last","documents":results,"summary":summary}, indent=2, ensure_ascii=False)+"\n",encoding="utf-8")
    md = markdown(results,summary,a.font_dir)
    (a.out/"report.md").write_text(md,encoding="utf-8")
    print(md)
    # Unsupported documents are expected during development. Crashes/timeouts and
    # infrastructure failure must not be mistaken for a completed compatibility run.
    counts = summary["counts"]
    if a.require_success:
        return int(any(k!="ok" and v for c in counts.values() for k,v in c.items()) or not counts["render"].get("ok"))
    if not counts["download"].get("ok"):
        return 2
    if any(c.get(k,0) for c in counts.values() for k in ("process_error","protocol_error","timeout","budget_skipped")):
        return 3
    return 0

if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Observational JBIG2 differential corpus. No reference code in RPCore.
Pinned public conformance PDFs are synthetic, not physical scanner output.
Optional user-supplied scans are never uploaded by this script.
"""
import argparse
import base64
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import tempfile
import urllib.request
from PIL import Image, ImageChops

REV = "e64acd24f00d365c3f7c60e9c4eff16ce2d71f5d"
BASE = f"https://pdfium.googlesource.com/pdfium_tests/+/{REV}/third_party/jbig2/"
CASES = ("bitmap-halftone-global", "bitmap-symbol-global",
         "bitmap-symbol-symbolrefine-textrefine-export",
         "bitmap-symbol-symhuff-texthuff-runcodes32-34")
CAP = 16 * 1024 * 1024
PIXELS = 4_000_000
Image.MAX_IMAGE_PIXELS = PIXELS

def sha(data):
    return hashlib.sha256(data).hexdigest()

def fetch(name):
    url = BASE + name + "?format=TEXT"
    req = urllib.request.Request(url, headers={"User-Agent": "RPCore-test-corpus/1"})
    with urllib.request.urlopen(req, timeout=30) as response:
        data = response.read(CAP * 2 + 1)
    if len(data) > CAP * 2:
        raise ValueError("encoded download limit")
    raw = base64.b64decode(b"".join(data.split()), validate=True)
    if len(raw) > CAP:
        raise ValueError("download limit")
    return raw

def expanded_cases(entries, limit=4):
    # Only simple filenames with a matching page-1 reference at the same
    # immutable revision. Never follow nested paths or directory entries.
    names = {e["name"] for e in entries
             if isinstance(e, dict) and e.get("type") == "blob"
             and isinstance(e.get("name"), str)
             and re.fullmatch(r"[A-Za-z0-9_.-]{1,160}", e["name"])}
    candidates = sorted(n[:-4] for n in names if n.endswith(".pdf")
                        and n[:-4] not in CASES
                        and n[:-4] + "_expected.pdf.0.png" in names)
    return candidates[:limit]

def discover():
    req = urllib.request.Request(BASE + "?format=JSON",
                                 headers={"User-Agent": "RPCore-test-corpus/1"})
    with urllib.request.urlopen(req, timeout=30) as response:
        raw = response.read(1024 * 1024 + 1)
    if len(raw) > 1024 * 1024 or not raw.startswith(b")]}'\n"):
        raise ValueError("invalid gitiles directory response")
    listing = json.loads(raw[5:])
    if not isinstance(listing, dict) or not isinstance(listing.get("entries"), list):
        raise ValueError("invalid gitiles directory entries")
    return expanded_cases(listing["entries"])

def frozen_gate(rows):
    frozen = [r for r in rows if r.get("suite") == "frozen"]
    return (len(frozen) == len(CASES)
            and {r["id"] for r in frozen} == set(CASES)
            and all(r["status"] == "exact" and r.get("different_pixels") == 0
                    and r.get("pixels", 0) > 0 for r in frozen))

def bounded_read(path):
    with path.open("rb") as f:
        data = f.read(CAP + 1)
    if len(data) > CAP:
        raise ValueError("local file limit")
    return data

def pixels(path):
    with Image.open(path) as im:
        if im.width * im.height > PIXELS:
            raise ValueError("image pixel limit")
        return im.convert("RGB")

def compare(native, ref):
    if native.size != ref.size:
        return {"status": "dimension_mismatch", "native_size": native.size,
                "reference_size": ref.size}
    diff = ImageChops.difference(native, ref)
    hist = diff.histogram()
    channels = sum(i * hist[c * 256 + i] for c in range(3) for i in range(256))
    mismatch = sum(p != (0, 0, 0) for p in diff.getdata())
    total = native.width * native.height
    return {"status": "exact" if mismatch == 0 else "pixel_mismatch",
            "pixels": total, "different_pixels": mismatch,
            "rgb_mae": channels / (total * 3),
            "reference_nonwhite": sum(p != (255,255,255) for p in ref.getdata()),
            "native_nonwhite": sum(p != (255,255,255) for p in native.getdata())}

def render(probe, pdf, page, png):
    env = dict(os.environ, RPCORE_RENDER_DPI="72", RPCORE_TEXT_AA="2",
               RPCORE_TEXT_GRID="0", RPCORE_TEXT_AREA="0")
    result = subprocess.run([str(probe), str(pdf), "render", str(page), str(png)],
                            capture_output=True, text=True, timeout=60, env=env)
    fields = dict(line.split("=", 1) for line in result.stdout.splitlines() if "=" in line)
    if result.returncode or fields.get("status") != "ok" or not png.is_file():
        return {"status": "native_rejected", "exit": result.returncode,
                "diagnostic": result.stdout[-2000:] + result.stderr[-1000:]}
    return {"status": "rendered", "engine_ms": fields.get("engine_ms")}

def classic_generation_diagnostics(raw):
    # Public fixture only: diagnostic, not a PDF parser or repair.
    matches = list(re.finditer(rb"startxref\s+(\d{1,12})\s+%%EOF", raw[-65536:]))
    if not matches:
        return []
    offset = int(matches[-1].group(1))
    if offset >= len(raw):
        return []
    table = raw[offset:offset+65536]
    if not table.startswith(b"xref"):
        return []
    rows = []
    for m in re.finditer(rb"(?m)^[ \t]*(\d{1,12})[ \t]+(\d{1,12})[ \t]+([nf])[ \t]*\r?$", table.split(b"trailer",1)[0]):
        if int(m.group(2)) > 65535:
            rows.append({"offset":int(m.group(1)), "generation":int(m.group(2)),
                         "kind":m.group(3).decode("ascii")})
        if len(rows) >= 16:
            break
    return rows

def case(probe, work, row, raw, expected):
    pdf, ref, out = (work / (row["id"] + suffix) for suffix in (".pdf", "-ref.png", "-native.png"))
    row.update(pdf_sha256=sha(raw), reference_sha256=sha(expected))
    if not raw.startswith(b"%PDF-"):
        raise ValueError("not PDF")
    if row["kind"] == "public_synthetic":
        row["classic_generation_diagnostics"] = classic_generation_diagnostics(raw)
    pdf.write_bytes(raw)
    ref.write_bytes(expected)
    row.update(render(probe, pdf, row["page"], out))
    if row["status"] == "rendered":
        row.update(compare(pixels(out), pixels(ref)))
    return row

def self_test():
    a = Image.new("RGB", (2, 1), "white")
    b = a.copy()
    assert compare(a,b)["status"] == "exact"
    b.putpixel((1,0),(0,0,0))
    m = compare(a,b)
    assert m["different_pixels"] == 1 and m["rgb_mae"] == 127.5
    assert compare(a,Image.new("RGB",(1,1)))["status"] == "dimension_mismatch"
    entries = [{"name": n, "type": "blob"} for n in
               ("z.pdf","z_expected.pdf.0.png","a.pdf","a_expected.pdf.0.png",
                "unpaired.pdf","../bad.pdf","../bad_expected.pdf.0.png",
                CASES[0]+".pdf",CASES[0]+"_expected.pdf.0.png")]
    assert expanded_cases(entries) == ["a","z"]
    assert expanded_cases(entries,1) == ["a"]
    rows = [{"suite":"frozen","id":n,"status":"exact","different_pixels":0,"pixels":1}
            for n in CASES]
    assert frozen_gate(rows)
    assert not frozen_gate(rows[:-1])
    assert not frozen_gate(rows + rows[:1])
    assert not frozen_gate([dict(r,status="native_rejected") for r in rows])
    assert frozen_gate(rows + [{"suite":"exploratory","status":"native_rejected"}])

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--probe", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--local-manifest", type=Path, help="JSON list of local PDF/reference PNG pairs with SHA256 and page")
    p.add_argument("--no-public", action="store_true")
    p.add_argument("--require-exact", action="store_true", help="require all four frozen public cases")
    p.add_argument("--expand-public", action="store_true", help="discover up to four further pinned paired samples")
    a = p.parse_args()
    self_test()
    a.out.mkdir(parents=True, exist_ok=True)
    rows = []
    discovery = {"status":"not_requested","cases":[]}
    if a.no_public and (a.require_exact or a.expand_public):
        p.error("public options require public corpus")
    if a.expand_public:
        try:
            extras = discover()
            discovery = {"status":"ok","cases":extras}
        except (OSError, ValueError) as e:
            discovery = {"status":"failed","cases":[],"diagnostic":str(e)}
    with tempfile.TemporaryDirectory(prefix="rpcore-jbig2-") as tmp:
        work = Path(tmp)
        if not a.no_public:
            for name in (*CASES, *discovery["cases"]):
                row = {"id": name, "source": BASE + name + ".pdf",
                       "kind": "public_synthetic", "page": 1,
                       "suite": "frozen" if name in CASES else "exploratory"}
                try:
                    raw = fetch(name + ".pdf")
                    expected = fetch(name + "_expected.pdf.0.png")
                    case(a.probe.resolve(), work, row, raw, expected)
                except (OSError, ValueError, subprocess.SubprocessError) as e:
                    row.update(status="infrastructure_or_timeout", diagnostic=str(e))
                rows.append(row)
        if a.local_manifest:
            root = a.local_manifest.resolve().parent
            items = json.loads(a.local_manifest.read_text())
            if not isinstance(items,list) or len(items)>32:
                raise ValueError("manifest must contain at most 32 cases")
            for i, item in enumerate(items):
                row = {"id": f"local-{i}", "kind": "user_supplied_scan",
                       "page": item["page"]}
                try:
                    if type(row["page"]) is not int or not 1 <= row["page"] <= 10000:
                        raise ValueError("invalid page")
                    raw = bounded_read(root / item["pdf"])
                    expected = bounded_read(root / item["reference_png"])
                    if sha(raw)!=item["pdf_sha256"] or sha(expected)!=item["reference_sha256"]:
                        raise ValueError("manifest SHA256 mismatch")
                    case(a.probe.resolve(), work, row, raw, expected)
                except (OSError, ValueError, subprocess.SubprocessError) as e:
                    row.update(status="infrastructure_or_timeout", diagnostic=str(e))
                rows.append(row)
    if not rows:
        raise ValueError("no corpus cases")
    gate = frozen_gate(rows)
    report = {"schema":2,"reference_revision":REV,"dpi":72,"rows":rows,
              "frozen_exact":gate,"discovery":discovery}
    (a.out/"report.json").write_text(json.dumps(report,indent=2)+"\n")
    lines = ["# JBIG2 differential corpus", "",
             f"Frozen exact gate: {'PASS' if gate else 'FAIL'}",
             "Exploratory discovery: " + str(discovery), "",
             "Public tests are synthetic SerenityOS conformance images, not scans.",
             "Exact matches, mismatches, native rejection and infrastructure failures are distinct.",
             "", "| Case | Suite | Kind | Status | Different pixels |", "|---|---|---|---|---:|"]
    for row in rows:
        lines.append(f"| {row['id']} | {row.get('suite','local')} | {row['kind']} | {row['status']} | {row.get('different_pixels','-')} |")
    for row in rows:
        if row["kind"] == "public_synthetic":
            lines.append("\n" + row["id"] + ": " + row.get("diagnostic","").strip()
                         + "; oversized classic xref generations: " + str(row.get("classic_generation_diagnostics",[])))
    (a.out/"report.md").write_text("\n".join(lines)+"\n")
    print("\n".join(lines))
    # An unavailable corpus is not successful validation. Unsupported or
    # mismatched inputs are measured coverage gaps, not exact passes.
    if a.require_exact and not gate:
        raise SystemExit(1)
    if all(r["status"]=="infrastructure_or_timeout" for r in rows):
        raise SystemExit(1)

if __name__ == "__main__":
    main()

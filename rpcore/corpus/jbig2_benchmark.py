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
import urllib.error
import time
from PIL import Image, ImageChops

REV = "e64acd24f00d365c3f7c60e9c4eff16ce2d71f5d"
BASE = f"https://pdfium.googlesource.com/pdfium_tests/+/{REV}/third_party/jbig2/"
CASES = ("bitmap-halftone-global", "bitmap-symbol-global",
         "bitmap-symbol-symbolrefine-textrefine-export",
         "bitmap-symbol-symhuff-texthuff-runcodes32-34",
         "bitmap", "bitmap-composite-and-xnor",
         "bitmap-composite-and-xnor-halftone",
         "bitmap-composite-and-xnor-refine",
         "bitmap-composite-and-xnor-text",
         "bitmap-composite-or-xor-replace",
         "bitmap-composite-or-xor-replace-halftone",
         "bitmap-composite-or-xor-replace-refine",
         "bitmap-composite-or-xor-replace-text", "bitmap-customat",
         "bitmap-customat-tpgdon", "bitmap-halftone",
         "bitmap-halftone-10bpp", "bitmap-halftone-10bpp-mmr",
         "bitmap-halftone-composite", "bitmap-halftone-grid",
         "bitmap-halftone-refine", "bitmap-halftone-skip-dummy",
         "bitmap-halftone-skip-grid", "bitmap-halftone-skip-grid-template1",
         "bitmap-halftone-skip-grid-template2", "bitmap-halftone-skip-grid-template3",
         "bitmap-halftone-template1", "bitmap-halftone-template2",
         "bitmap-halftone-template3", "bitmap-mmr",
         "bitmap-randomaccess", "bitmap-refine", "bitmap-refine-customat",
         "bitmap-refine-customat-tpgron", "bitmap-initially-unknown-size",
         "bitmap-refine-lossless", "bitmap-refine-page", "bitmap-refine-page-subrect",
         "bitmap-refine-refine", "bitmap-p32-eof",
         "bitmap-refine-template1", "bitmap-refine-template1-tpgron", "bitmap-refine-tpgron",
         "bitmap-stripe", "bitmap-stripe-last-implicit", "bitmap-stripe-single-no-end-of-stripe",
         "bitmap-stripe-initially-unknown-height", "bitmap-symbol", "bitmap-symbol-32bit-arithint",
         "bitmap-symbol-big-segmentid", "bitmap-symbol-context-reuse",
         "bitmap-stripe-single", "bitmap-symbol-empty", "bitmap-symbol-manyrefs",
         "bitmap-symbol-negative-sbdsoffset", "bitmap-symbol-refine")
KNOWN_GAPS = ()
EXPLORATORY_BATCH = 16
EXPECTED_PAIRS = 107
CAP = 16 * 1024 * 1024
PIXELS = 4_000_000
Image.MAX_IMAGE_PIXELS = PIXELS

def sha(data):
    return hashlib.sha256(data).hexdigest()

# Retry only transient transport errors; never retry malformed content,
# permanent HTTP failures, or relax the exact frozen gate.
RETRY_HTTP = {408, 429, 500, 502, 503, 504}
def rate_limit_delay(header, attempt):
    # Respect delta seconds and HTTP dates, with bounded waits. An invalid
    # header falls back to exponential rate-limit backoff.
    from email.utils import parsedate_to_datetime
    fallback = min(60, 8 * 2 ** attempt)
    if header is None:
        return fallback
    try:
        seconds = int(header)
    except (TypeError, ValueError):
        try:
            seconds = int(parsedate_to_datetime(header).timestamp() - time.time()) + 1
        except (TypeError, ValueError, OverflowError, OSError, AttributeError):
            return fallback
    return min(60, max(fallback, seconds))

def fetch_bounded(url, limit, opener=None, sleeper=None):
    real_request = opener is None
    opener = urllib.request.urlopen if opener is None else opener
    sleeper = time.sleep if sleeper is None else sleeper
    req = urllib.request.Request(url, headers={"User-Agent": "RPCore-test-corpus/1"})
    # Pace even successful requests to avoid bursts against the public server.
    if real_request:
        sleeper(0.75)
    for attempt in range(5):
        try:
            with opener(req, timeout=20) as response:
                data = response.read(limit + 1)
            if len(data) > limit:
                raise ValueError("bounded response limit")
            return data
        except urllib.error.HTTPError as exc:
            last = 4 if exc.code == 429 else 2
            if exc.code not in RETRY_HTTP or attempt >= last:
                raise
            delay = rate_limit_delay(exc.headers.get("Retry-After") if exc.headers else None, attempt) if exc.code == 429 else 2 ** attempt
            exc.close()
        except (urllib.error.URLError, TimeoutError, ConnectionError):
            if attempt >= 2:
                raise
            delay = 2 ** attempt
        print(f"Transient corpus download failure; retry {attempt+1} after {delay}s: {url}", flush=True)
        sleeper(delay)

def fetch(name):
    url = BASE + name + "?format=TEXT"
    data = fetch_bounded(url, CAP * 2)
    raw = base64.b64decode(b"".join(data.split()), validate=True)
    if len(raw) > CAP:
        raise ValueError("download limit")
    return raw

def paired_cases(entries):
    # Only simple filenames with a matching page-1 reference at the same
    # immutable revision. Never follow nested paths or directory entries.
    names = {e["name"] for e in entries
             if isinstance(e, dict) and e.get("type") == "blob"
             and isinstance(e.get("name"), str)
             and re.fullmatch(r"[A-Za-z0-9_.-]{1,160}", e["name"])}
    return sorted(n[:-4] for n in names if n.endswith(".pdf")
                  and n[:-4] + "_expected.pdf.0.png" in names)

def expanded_cases(entries, limit=EXPLORATORY_BATCH):
    return [n for n in paired_cases(entries) if n not in CASES and n not in KNOWN_GAPS][:limit]

def inventory(entries):
    pairs = paired_cases(entries)
    missing = sorted(set(CASES) - set(pairs))
    remaining = [n for n in pairs if n not in CASES]
    gaps = [n for n in KNOWN_GAPS if n in pairs]
    new = expanded_cases(entries)
    return {"status":"ok", "cases":gaps+new, "paired_total":len(pairs),
            "frozen_missing":missing, "remaining_total":len(remaining),
            "known_gaps":gaps, "new_cases":new,
            "remaining_after_batch":len(remaining)-len(gaps)-len(new)}

def discover():
    raw = fetch_bounded(BASE + "?format=JSON", 1024 * 1024)
    if len(raw) > 1024 * 1024 or not raw.startswith(b")]}'\n"):
        raise ValueError("invalid gitiles directory response")
    listing = json.loads(raw[5:])
    if not isinstance(listing, dict) or not isinstance(listing.get("entries"), list):
        raise ValueError("invalid gitiles directory entries")
    return inventory(listing["entries"])

def inventory_gate(discovery, rows):
    # A successful exploratory audit requires actual evidence for every
    # scheduled name. Native rejections and pixel differences are legitimate
    # observed outcomes, but missing downloads or a changed listing are not.
    if discovery.get("status") != "ok" or discovery.get("paired_total") != EXPECTED_PAIRS:
        return False
    if discovery.get("frozen_missing") or discovery.get("remaining_total") != EXPECTED_PAIRS-len(CASES):
        return False
    names = discovery.get("cases", [])
    gaps = discovery.get("known_gaps", [])
    fresh = discovery.get("new_cases", [])
    if (names != gaps+fresh or len(names) != len(set(names)) or set(names)&set(CASES)
            or set(gaps) != set(KNOWN_GAPS) or set(fresh)&set(KNOWN_GAPS)
            or len(fresh) != min(EXPLORATORY_BATCH, EXPECTED_PAIRS-len(CASES)-len(gaps))
            or discovery.get("remaining_after_batch") != EXPECTED_PAIRS-len(CASES)-len(names)):
        return False
    observed = [r for r in rows if r.get("suite") == "exploratory"]
    return (len(observed) == len(names) and {r["id"] for r in observed} == set(names)
            and all(r.get("status") in {"exact","pixel_mismatch","dimension_mismatch","native_rejected"}
                    and r.get("pdf_sha256") and r.get("reference_sha256") for r in observed))

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

# Diagnostics only for simple public fixtures; never used for native decoding.
# Stop rather than guessing an indeterminate length or a wrapped stream.
def segment_headers(data):
    pos = 0
    rows = []
    seen = {}
    def take(n):
        nonlocal pos
        if n > len(data)-pos:
            raise ValueError("truncated segment field")
        out = data[pos:pos+n]
        pos += n
        return out
    def uint(n):
        return int.from_bytes(take(n),"big")
    try:
        while pos < len(data) and len(rows) < 64:
            start = pos
            number, flags, first = uint(4), uint(1), uint(1)
            count = first >> 5
            if count == 7:
                count = ((first & 31)<<24) | uint(3)
                if count > 4096:
                    raise ValueError("reference count cap")
                take((count+8)//8)
            elif count > 4:
                raise ValueError("reserved reference count")
            width = 1 if number <= 256 else 2 if number <= 65536 else 4
            refs = [uint(width) for _ in range(count)]
            page = uint(4 if flags & 64 else 1)
            length = uint(4)
            row = dict(offset=start, number=number, kind=flags&63, page=page,
                       length=length, refs=refs[:32], ref_count=count,
                       local_ref_kinds=[seen.get(n) for n in refs[:32]])
            rows.append(row)
            seen[number] = flags&63
            if length == 0xffffffff:
                row["prefix_hex"] = data[pos:pos+40].hex()
                row["tail_hex"] = data[-32:].hex()
                return {"status":"indeterminate_length","segments":rows}
            if length > len(data)-pos:
                raise ValueError("segment payload truncated")
            if flags&63 in (48,50):
                row["control_payload_hex"] = data[pos:pos+19].hex()[:min(length,19)*2]
            if flags&63 in (0,4,6,7,16,20,22,23,36,38,39,40,42,43,53,62):
                # Fixed, bounded prefix only; this does not interpret entropy.
                row["body_prefix_hex"] = data[pos:pos+min(length,40)].hex()
            pos += length
        return {"status":"complete" if pos == len(data) else "segment_cap","segments":rows}
    except ValueError as e:
        return {"status":"stopped","reason":str(e),"segments":rows}

def jbig2_stream_diagnostics(raw):
    # Deliberately limited to direct single-filter JBIG2 streams and raw
    # streams in public synthetic PDFs. Use direct Length; no marker search
    # through entropy data. Global candidates are labeled, not assumed.
    result = []
    pattern = rb"(?m)^[ \t]*(\d{1,10})[ \t]+(\d{1,5})[ \t]+obj\b"
    for m in re.finditer(pattern,raw):
        if len(result) >= 16:
            break
        header = raw[m.end():m.end()+8192]
        end = re.search(rb"\bstream(?:\r\n|\n|\r)",header)
        if not end or b"endobj" in header[:end.start()]:
            continue
        dictionary = header[:end.start()]
        if b"<<" not in dictionary:
            continue
        length = re.search(rb"/Length\s+(\d{1,9})(?![\d])",dictionary)
        if not length or re.match(rb"\s+\d+\s+R\b",dictionary[length.end():]):
            continue
        size = int(length.group(1))
        pos = m.end()+end.end()
        if size > CAP or pos+size > len(raw):
            continue
        if not re.match(rb"\s*endstream\b",raw[pos+size:pos+size+32]):
            continue
        jb = bool(re.search(rb"/Filter\s*(?:/JBIG2Decode\b|\[\s*/JBIG2Decode\s*\])",dictionary))
        if not jb and b"/Filter" in dictionary:
            continue
        data = raw[pos:pos+size]
        # Raw stream candidates require a plausible first segment header.
        if not jb and (len(data)<11 or data[4]&63 not in (0,16,48,53)):
            continue
        result.append(dict(object=int(m.group(1)),role="jbig2_image" if jb else "raw_candidate",
                           bytes=size,**segment_headers(data)))
    return result

def diagnostic_self_test():
    def seg(n,k,refs,body,length=None):
        return (n.to_bytes(4,"big")+bytes([k,len(refs)<<5])+bytes(refs)+b"\x01"
                +(len(body) if length is None else length).to_bytes(4,"big")+body)
    a=seg(1,48,[],bytes(19))
    b=seg(2,39,[1],b"x")
    c=seg(3,39,[],b"abc",0xffffffff)
    rows=segment_headers(a+b+c)
    assert rows["status"]=="indeterminate_length"
    assert rows["segments"][1]["local_ref_kinds"]==[48]
    assert rows["segments"][2]["prefix_hex"]=="616263"
    assert segment_headers(a+b[:-1])["status"]=="stopped"
    raw=(b"%PDF-1.4\n7 0 obj\n<< /Filter /JBIG2Decode /Length "+str(len(a+b+c)).encode()
         +b" >>\nstream\n"+a+b+c+b"\nendstream\nendobj\n")
    assert jbig2_stream_diagnostics(raw)[0]["segments"]==rows["segments"]
    assert not jbig2_stream_diagnostics(raw.replace(b"/Length ",b"/Length 9 0 R /Other "))

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
    if row["kind"] == "public_synthetic" and row["status"] in ("native_rejected","pixel_mismatch","dimension_mismatch"):
        row["jbig2_stream_diagnostics"] = jbig2_stream_diagnostics(raw)
    return row

def retry_self_test():
    import io
    from unittest.mock import Mock
    def http(code, headers=None):
        return urllib.error.HTTPError("https://example.invalid", code, "test", headers or {}, io.BytesIO())
    url = "https://example.invalid"
    pause = Mock()
    opener = Mock(side_effect=[http(503), urllib.error.URLError("test"), io.BytesIO(b"ok")])
    assert fetch_bounded(url, 2, opener, pause) == b"ok"
    assert opener.call_count == 3
    assert [call.args for call in pause.call_args_list] == [(1,), (2,)]
    pause = Mock()
    opener = Mock(side_effect=[http(429, {"Retry-After":"12"}), http(429),
                               http(429, {"Retry-After":"99999"}), http(429), io.BytesIO(b"ok")])
    assert fetch_bounded(url, 2, opener, pause) == b"ok"
    assert opener.call_count == 5
    assert [call.args for call in pause.call_args_list] == [(12,), (16,), (60,), (60,)]
    assert rate_limit_delay("bad", 0) == 8
    assert rate_limit_delay("-5", 0) == 8
    assert rate_limit_delay("Thu, 01 Jan 1970 00:00:00 GMT", 0) == 8
    assert rate_limit_delay("Fri, 01 Jan 2100 00:00:00 GMT", 0) == 60
    for errors, count, expected in [
        ([http(404)], 1, urllib.error.HTTPError),
        ([http(429) for _ in range(5)], 5, urllib.error.HTTPError),
        ([http(503), http(503), http(503)], 3, urllib.error.HTTPError),
        ([io.BytesIO(b"too long")], 1, ValueError),
    ]:
        opener = Mock(side_effect=errors)
        pause = Mock()
        try:
            fetch_bounded(url, 2, opener, pause)
        except expected:
            pass
        else:
            raise AssertionError("expected bounded fetch failure")
        assert opener.call_count == count and pause.call_count == count-1

def self_test():
    retry_self_test()
    diagnostic_self_test()
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
    assert len(CASES) == 56 and len(set(CASES)) == 56
    # Every frozen case, including the promoted refinement case, is excluded
    # from discovery, even if the listing contains its matching reference.
    entries.extend({"name": n+s, "type": "blob"} for n in CASES
                   for s in (".pdf","_expected.pdf.0.png"))
    assert expanded_cases(entries) == ["a","z"]
    assert expanded_cases(entries,1) == ["a"]
    inv = inventory(entries)
    assert inv["paired_total"] == len(CASES)+2 and inv["frozen_missing"] == []
    assert inv["remaining_total"] == 2 and inv["remaining_after_batch"] == 0
    assert inventory([])["frozen_missing"] == sorted(CASES)
    gap_entries = entries + [{"name":n+s,"type":"blob"} for n in KNOWN_GAPS for s in (".pdf","_expected.pdf.0.png")]
    inv = inventory(gap_entries)
    assert inv["cases"] == list(KNOWN_GAPS)+["a","z"] and inv["remaining_after_batch"] == 0
    rows = [{"suite":"frozen","id":n,"status":"exact","different_pixels":0,"pixels":1}
            for n in CASES]
    assert frozen_gate(rows)
    assert not frozen_gate(rows[:-1])
    assert not frozen_gate(rows + rows[:1])
    assert not frozen_gate([dict(r,status="native_rejected") for r in rows])
    for i in range(len(rows)):
        for change in ({"status":"pixel_mismatch","different_pixels":1},
                       {"status":"infrastructure_or_timeout"},
                       {"pixels":0}, {"different_pixels":1}):
            bad = list(rows)
            bad[i] = dict(bad[i], **change)
            assert not frozen_gate(bad)
    assert frozen_gate(rows + [{"suite":"exploratory","status":"native_rejected"}])
    assert not set(CASES)&set(KNOWN_GAPS)
    all_entries = entries + [{"name":f"extra-{i:03d}"+s,"type":"blob"}
                             for i in range(EXPECTED_PAIRS-len(CASES)-2)
                             for s in (".pdf","_expected.pdf.0.png")]
    full = inventory(all_entries)
    assert full["paired_total"] == EXPECTED_PAIRS
    assert len(full["new_cases"]) == EXPLORATORY_BATCH
    assert full["remaining_after_batch"] == EXPECTED_PAIRS-len(CASES)-EXPLORATORY_BATCH
    observed = [dict(id=n,suite="exploratory",status="native_rejected",
                     pdf_sha256="a"*64,reference_sha256="b"*64) for n in full["cases"]]
    assert inventory_gate(full, rows+observed)
    assert not inventory_gate(full, rows+observed[:-1])
    assert not inventory_gate(full, rows+observed+observed[:1])
    assert not inventory_gate(dict(full,status="failed"), rows+observed)
    assert not inventory_gate(dict(full,paired_total=EXPECTED_PAIRS-1), rows+observed)
    assert not inventory_gate(dict(full,frozen_missing=[CASES[0]]), rows+observed)
    assert not inventory_gate(full, rows+[dict(observed[0],status="infrastructure_or_timeout")]+observed[1:])
    assert not inventory_gate(full, rows+[dict(observed[0],pdf_sha256=None)]+observed[1:])

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--probe", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--local-manifest", type=Path, help="JSON list of local PDF/reference PNG pairs with SHA256 and page")
    p.add_argument("--no-public", action="store_true")
    p.add_argument("--require-exact", action="store_true", help="require all frozen public cases")
    p.add_argument("--expand-public", action="store_true", help="discover up to sixteen further pinned paired samples plus known gaps")
    a = p.parse_args()
    self_test()
    a.out.mkdir(parents=True, exist_ok=True)
    rows = []
    discovery = {"status":"not_requested","cases":[]}
    if a.no_public and (a.require_exact or a.expand_public):
        p.error("public options require public corpus")
    if a.expand_public:
        try:
            discovery = discover()
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
    counts = {}
    for suite in ("frozen","exploratory","local"):
        group = [r for r in rows if r.get("suite","local") == suite]
        counts[suite] = {"total":len(group),"exact":sum(r["status"] == "exact" for r in group),
                         "statuses":{status:sum(r["status"] == status for r in group)
                                     for status in sorted({r["status"] for r in group})}}
    inventory_ok = inventory_gate(discovery,rows) if a.expand_public else None
    report = {"schema":5,"reference_revision":REV,"dpi":72,"rows":rows,
              "frozen_exact":gate,"inventory_gate":inventory_ok,"discovery":discovery,"counts":counts}
    (a.out/"report.json").write_text(json.dumps(report,indent=2)+"\n")
    lines = ["# JBIG2 differential corpus", "",
             f"Frozen exact gate: {'PASS' if gate else 'FAIL'}",
             "Exploratory discovery: " + str(discovery),
             "Inventory evidence gate: " + ("NOT REQUESTED" if inventory_ok is None else "PASS" if inventory_ok else "FAIL"),
             "Frozen exact: {}/{}".format(counts["frozen"]["exact"],len(CASES)),
             "Exploratory exact: {}/{}".format(counts["exploratory"]["exact"],counts["exploratory"]["total"]), "",
             "Public tests are synthetic SerenityOS conformance images, not scans.",
             "Exact matches, mismatches, native rejection and infrastructure failures are distinct.",
             "", "| Case | Suite | Kind | Status | Different pixels |", "|---|---|---|---|---:|"]
    for row in rows:
        lines.append(f"| {row['id']} | {row.get('suite','local')} | {row['kind']} | {row['status']} | {row.get('different_pixels','-')} |")
    for row in rows:
        if row["kind"] == "public_synthetic":
            lines.append("\n" + row["id"] + ": " + row.get("diagnostic","").strip()
                         + "; oversized classic xref generations: " + str(row.get("classic_generation_diagnostics",[])))
            if "jbig2_stream_diagnostics" in row:
                lines.append("  JBIG2 stream diagnostics: " + json.dumps(row["jbig2_stream_diagnostics"]))
    (a.out/"report.md").write_text("\n".join(lines)+"\n")
    print("\n".join(lines))
    # An unavailable corpus is not successful validation. Unsupported or
    # mismatched inputs are measured coverage gaps, not exact passes.
    if a.require_exact and not gate:
        raise SystemExit(1)
    if a.expand_public and not inventory_ok:
        raise SystemExit(1)
    if all(r["status"]=="infrastructure_or_timeout" for r in rows):
        raise SystemExit(1)

if __name__ == "__main__":
    main()

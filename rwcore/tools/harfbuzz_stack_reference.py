#!/usr/bin/env python3
"""Independent HB mark + mkmk differential over vendored Latin fonts.
Admit by native feature presence and stable glyph identity / baseline only.
No comparison-dependent exclusions. Nonzero mkmk contribution is required.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
from harfbuzz_gpos_reference import HB

FONTS = ("alegreyasans/AlegreyaSans-Regular.ttf",
         "abeezee/ABeeZee-Regular.ttf", "abeezee/ABeeZee-Italic.ttf",
         "abel/Abel-Regular.ttf", "archivo/Archivo[wdth,wght].ttf")
BASES = "xqvwjfAOT"
MARKS = ("\u0301", "\u0300", "\u0302", "\u0308", "\u0304", "\u0323", "\u0307")
def u16(b, p):
    return struct.unpack_from(">H", b, p)[0]
def u32(b, p):
    return struct.unpack_from(">I", b, p)[0]
def table(data, tag):
    for i in range(u16(data, 4)):
        p = 12 + 16*i
        if data[p:p+4] == tag:
            start, length = u32(data,p+8), u32(data,p+12)
            return data[start:start+length]
    return None
def native_script_tags(data, script):
    b = table(data, b"GPOS")
    if b is None:
        return set()
    sl, fl = u16(b, 4), u16(b, 6)
    scripts = {b[p:p+4]: sl+u16(b,p+4)
               for p in range(sl+2,sl+2+6*u16(b,sl),6)}
    s = scripts.get(script, scripts.get(b"DFLT"))
    if s is None or not u16(b,s):
        return set()
    lang = s+u16(b,s)
    ids = [u16(b,lang+6+2*i) for i in range(u16(b,lang+4))]
    req = u16(b,lang+2)
    if req != 0xffff:
        ids.append(req)
    return {b[fl+2+6*i:fl+6+6*i] for i in ids}

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--probe", required=True, type=Path)
    p.add_argument("--report", required=True, type=Path)
    p.add_argument("--font-dir", type=Path, default=Path("core/tests/fixtures"))
    args = p.parse_args()
    hb = HB()
    report = {"harfbuzz": hb.version_string().decode(),
              "scope": "Latin mark+mkmk toggle; HB glyph IDs and baseline widths; LTR UPEM",
              "candidates": 0, "cases": 0, "nonzero_cases": 0, "mkmk_effect_cases": 0,
              "mismatches": 0, "max_absolute_error": 0, "fonts": []}
    try:
        for name in FONTS:
            path = args.font_dir / name
            data = path.read_bytes()
            tags = native_script_tags(data, b"latn")
            font = {"name": name, "sha256": hashlib.sha256(data).hexdigest(),
                    "native_tags": sorted(t.decode("ascii") for t in tags),
                    "admitted": [], "excluded": []}
            report["fonts"].append(font)
            if not {b"mark", b"mkmk"} <= tags:
                font["reason"] = "no native Latin mark and mkmk feature pair"
                continue
            groups = []
            for base in BASES:
                for a in MARKS:
                    for z in MARKS:
                        text = base+a+z
                        report["candidates"] += 1
                        ids, on, upem = hb.shape(data,text,False,mark=True,mkmk=True)
                        mid_ids, mid, mid_upem = hb.shape(data,text,False,mark=True)
                        off_ids, off, off_upem = hb.shape(data,text,False)
                        row = {"codepoints": [ord(c) for c in text], "glyphs": ids,
                               "glyphs_mark_only": mid_ids, "glyphs_off": off_ids, "upem": upem}
                        reason = None
                        if ids != mid_ids or ids != off_ids or upem != mid_upem or upem != off_upem:
                            reason = "feature changes glyph identity or scale"
                        elif len(ids) != 3:
                            reason = "normalized glyph count is not three"
                        elif any(g == 0 for g,_ in ids):
                            reason = "missing glyph"
                        elif any(v[1:] != [0,0,0] for v in off):
                            reason = "baseline has vertical advance or placement"
                        if reason:
                            row["reason"] = reason
                            font["excluded"].append(row)
                            continue
                        expected = [[x-y for x,y in zip(p,q)] for p,q in zip(on,off)]
                        effect = [[x-y for x,y in zip(p,q)] for p,q in zip(on,mid)]
                        row.update(baseline=off, reference_delta=expected, mkmk_delta=effect)
                        font["admitted"].append(row)
                        groups.append(",".join(f"{g}:{v[0]}" for (g,_),v in zip(ids,off)))
            if not groups:
                continue
            proc = subprocess.run([str(args.probe.resolve()),str(path),"--stack",*groups],
                                  capture_output=True,text=True,timeout=90)
            if proc.returncode:
                raise RuntimeError(f"{name}: probe failed: {proc.stderr}")
            lines = proc.stdout.splitlines()
            if len(lines) != len(groups):
                raise RuntimeError("probe row count")
            for row,line in zip(font["admitted"],lines):
                actual = [[int(x) for x in part.split(",")] for part in line.split(";")]
                expected = row["reference_delta"]
                if len(actual) != 3 or any(len(v) != 4 for v in actual):
                    raise RuntimeError("probe field count")
                err = max(abs(x-y) for a,b in zip(actual,expected) for x,y in zip(a,b))
                row.update(rwcore_delta=actual,max_absolute_error=err)
                report["cases"] += 1
                report["nonzero_cases"] += int(any(v[2] or v[3] for v in expected))
                report["mkmk_effect_cases"] += int(any(v[2] or v[3] for v in row["mkmk_delta"]))
                report["mismatches"] += int(actual != expected)
                report["max_absolute_error"] = max(report["max_absolute_error"],err)
            print("stack",name,"admitted",len(groups),"excluded",len(font["excluded"]),flush=True)
        if not report["cases"] or not report["mkmk_effect_cases"]:
            raise RuntimeError("no positive native mkmk positioning evidence")
        if report["mismatches"]:
            raise RuntimeError(f"{report['mismatches']} stack reference mismatches")
    except Exception as e:
        report["error"] = str(e)
        raise
    finally:
        args.report.parent.mkdir(parents=True,exist_ok=True)
        args.report.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
        print("HB_STACK_REFERENCE",json.dumps({k:v for k,v in report.items() if k!="fonts"}),flush=True)

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Strict external RTL reference gate for the admitted isolated feature scope.

HB provides shaped glyph IDs and feature-off advances. Its RTL visual
sequence is reversed into logical order before calling RWCore. Positive
physical widths are retained; positions use the same x/y axes, with origin
translation removed by comparing right-edge-relative glyph origins.
Mismatches and unsupported cases are retained, not silently dropped.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from harfbuzz_gpos_reference import HB
from harfbuzz_stack_reference import native_script_tags

FONTS = ("arefruqaa/ArefRuqaa-Regular.ttf",
         "notonastaliqurdu/NotoNastaliqUrdu[wght].ttf")
BASES = ("ب", "ت", "س", "م", "ن", "ي", "ك", "ه")
MARKS = ("\u064e", "\u064f", "\u0650", "\u0651", "\u0652")
CURSIVE = ("بب", "بت", "تب", "سس", "مم", "نن", "بي", "يم", "سم",
           "بسم", "كتب", "بيت", "نبت", "تمت", "تبت", "بنت", "سمسم")
def logical(glyphs, positions):
    # HB's horizontal RTL output order is visual left to right. Reverse
    # occurrence order, but do not negate physical advances or x offsets.
    return list(reversed(glyphs)), list(reversed(positions))
def origins(metrics):
    x = y = 0
    out = []
    for ax,ay,px,py in metrics:
        x -= ax
        out.append([x+px,y+py])
        y += ay
    return out
def subtract(a,b):
    return [[x-y for x,y in zip(p,q)] for p,q in zip(a,b)]
def max_error(a,b):
    return max((abs(x-y) for p,q in zip(a,b) for x,y in zip(p,q)),default=0)

# Verified CI #221 floor, not an assertion about all possible Arabic text.
MIN_EVIDENCE = {
    FONTS[0]: {"mark": 25, "curs": 4},
    FONTS[1]: {"mark": 20, "curs": 4},
}
def validate_gate(report):
    """Fail closed on parity, missing evidence or inconsistent accounting."""
    if report["candidate_count"] != 114 or report["candidate_count"] != report["admitted"] + report["excluded"]:
        raise RuntimeError("RTL candidate coverage/accounting regression")
    if report["admitted"] != report["compared"] + report["probe_errors"]:
        raise RuntimeError("RTL admitted row accounting")
    if report["compared"] != report["exact"] + report["mismatches"]:
        raise RuntimeError("RTL compared row accounting")
    if (report["mismatches"] or report["probe_errors"] or
            report["max_absolute_error"] or report["max_world_error"]):
        raise RuntimeError("RTL external precision regression")
    fonts = {f["name"]: f for f in report["fonts"]}
    if len(fonts) != len(report["fonts"]) or set(fonts) != set(MIN_EVIDENCE):
        raise RuntimeError("RTL font inventory regression")
    total = {"admitted":0, "compared":0, "exact":0, "excluded":0}
    for name, floors in MIN_EVIDENCE.items():
        features = {f["tag"]:f for f in fonts[name]["features"]}
        if len(features) != len(fonts[name]["features"]) or set(features) != set(floors):
            raise RuntimeError(f"{name}: RTL feature inventory regression")
        for tag, floor in floors.items():
            feature = features[tag]
            rows = feature["rows"]
            if len(rows) != (len(BASES)*len(MARKS) if tag=="mark" else len(CURSIVE)):
                raise RuntimeError(f"{name} {tag}: candidate coverage regression")
            nonzero = exact = 0
            for row in rows:
                if row["status"]=="excluded":
                    if not row.get("reason"):
                        raise RuntimeError("unexplained RTL exclusion")
                    total["excluded"] += 1
                    continue
                if row["status"]!="exact" or row["max_absolute_error"] or row["max_world_error"]:
                    raise RuntimeError(f"{name} {tag}: RTL row precision regression")
                if row["rwcore_delta"]!=row["reference_delta"] or row["rwcore_world_delta"]!=row["reference_world_delta"]:
                    raise RuntimeError("RTL row marked exact but values differ")
                exact += 1
                nonzero += int(any(x for m in row["reference_delta"] for x in m))
            if exact < floor or nonzero < floor:
                raise RuntimeError(f"{name} {tag}: evidence {exact}/{nonzero} below {floor}")
            for k in ("admitted","compared","exact"):
                total[k] += exact
    if any(report[k]!=v for k,v in total.items()):
        raise RuntimeError("RTL report totals differ from row evidence")

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--probe",required=True,type=Path)
    p.add_argument("--report",required=True,type=Path)
    p.add_argument("--font-dir",type=Path,default=Path("core/tests/fixtures"))
    args=p.parse_args()
    hb=HB()
    report={"harfbuzz":hb.version_string().decode(),
            "scope":"Arabic RTL isolated mark/curs feature toggle; strict exact gate for admitted rows",
            "gate_pass":False,
            "candidate_count":0,"admitted":0,"compared":0,"exact":0,"mismatches":0,
            "probe_errors":0,"excluded":0,"max_absolute_error":0,"max_world_error":0,
            "by_feature":{tag:{"admitted":0,"compared":0,"nonzero_reference":0,"exact":0,
                              "mismatches":0,"probe_errors":0} for tag in ("mark","curs")},
            "fonts":[]}
    try:
        for name in FONTS:
            path=args.font_dir/name
            data=path.read_bytes()
            tags=native_script_tags(data,b"arab")
            font={"name":name,"sha256":hashlib.sha256(data).hexdigest(),
                  "native_tags":sorted(t.decode("ascii") for t in tags),"features":[]}
            report["fonts"].append(font)
            for tag,texts in (("mark",[b+m for b in BASES for m in MARKS]),("curs",CURSIVE)):
                feature={"tag":tag,"rows":[]}
                font["features"].append(feature)
                if tag.encode("ascii") not in tags:
                    feature["reason"]="native feature unavailable"
                    continue
                stat=report["by_feature"][tag]
                for text in texts:
                    report["candidate_count"]+=1
                    g,on,u=hb.shape(data,text,False,mark=tag=="mark",rtl=True,curs=tag=="curs")
                    z,off,v=hb.shape(data,text,False,rtl=True,curs=False)
                    g,on=logical(g,on);z,off=logical(z,off)
                    row={"codepoints":[ord(c) for c in text],"glyphs":g,"glyphs_off":z,"upem":u}
                    feature["rows"].append(row)
                    reason=None
                    if g!=z or u!=v:
                        reason="feature changed glyph identity or scale"
                    elif len(g)!=len(text):
                        reason="glyph count differs from input count"
                    elif any(i==0 for i,_ in g):
                        reason="missing glyph"
                    elif any(m[1:]!=[0,0,0] for m in off):
                        reason="nonzero baseline placement or vertical advance"
                    if reason:
                        row.update(status="excluded",reason=reason)
                        report["excluded"]+=1
                        continue
                    expected=subtract(on,off)
                    row.update(baseline=off,reference_delta=expected,
                               reference_world_delta=subtract(origins(on),origins(off)))
                    report["admitted"]+=1;stat["admitted"]+=1
                    stat["nonzero_reference"]+=int(any(x for m in expected for x in m))
                    group=",".join(f"{i}:{m[0]}" for (i,_),m in zip(g,off))
                    mode="--rtl-mark" if tag=="mark" else "--rtl-cursive"
                    proc=subprocess.run([str(args.probe.resolve()),str(path),mode,group],
                                        capture_output=True,text=True,timeout=30)
                    if proc.returncode:
                        row.update(status="probe_error",error=proc.stderr.strip(),exit_code=proc.returncode)
                        report["probe_errors"]+=1;stat["probe_errors"]+=1
                        continue
                    actual=[[int(x) for x in part.split(",")] for part in proc.stdout.strip().split(";")]
                    if len(actual)!=len(expected) or any(len(m)!=4 for m in actual):
                        raise RuntimeError("probe output dimensions")
                    metrics=[[x+y for x,y in zip(m,d)] for m,d in zip(off,actual)]
                    world=subtract(origins(metrics),origins(off))
                    err=max_error(actual,expected)
                    werr=max_error(world,row["reference_world_delta"])
                    row.update(status="exact" if not (err or werr) else "mismatch",rwcore_delta=actual,
                               rwcore_world_delta=world,max_absolute_error=err,max_world_error=werr)
                    report["compared"]+=1;stat["compared"]+=1
                    key="exact" if not (err or werr) else "mismatches"
                    report[key]+=1;stat[key]+=1
                    report["max_absolute_error"]=max(report["max_absolute_error"],err)
                    report["max_world_error"]=max(report["max_world_error"],werr)
                print("RTL feature",name,tag,stat,flush=True)
        if report["candidate_count"]!=report["admitted"]+report["excluded"]:
            raise RuntimeError("unaccounted candidates")
        if report["admitted"]!=report["compared"]+report["probe_errors"]:
            raise RuntimeError("unaccounted admitted rows")
        for tag,stat in report["by_feature"].items():
            if not stat["nonzero_reference"] or not stat["compared"]:
                raise RuntimeError(f"insufficient external {tag} evidence: {stat}")
        report["compatibility_pass"]=(report["mismatches"]==0 and report["probe_errors"]==0
                                       and report["max_absolute_error"]==0 and report["max_world_error"]==0)
        validate_gate(report)
        report["gate_pass"]=True
    except Exception as e:
        report["error"]=str(e)
        raise
    finally:
        args.report.parent.mkdir(parents=True,exist_ok=True)
        args.report.write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
        print("HB_RTL_REFERENCE",json.dumps({k:v for k,v in report.items() if k!="fonts"}),flush=True)

if __name__=="__main__":
    main()

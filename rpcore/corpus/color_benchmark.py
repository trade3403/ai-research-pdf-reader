#!/usr/bin/env python3
"""Original calibrated/CMYK patches; independent analytic checks and reference observations."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile
from PIL import Image, ImageStat
from chart_benchmark import render_pair

def srgb8(x):
    return round(255*(12.92*x if x<=0.0031308 else 1.055*x**(1/2.4)-0.055))

def neutral_lab(l):
    y=l/903.2962962963 if l<=8 else ((l+16)/116)**3
    return [srgb8(y)]*3

def patches():
    rows=[]
    def add(id,cmd,expected=None):
        rows.append(dict(id=id,cmd=cmd,expected=expected))
    for a in (0,.1,.25,.5,.75,1):
        add(f"gray-{a:g}",f"/CG cs {a:g} sc",[srgb8(a*a)]*3)
    for id,values,expected in [
        ("red","1 0 0",[255,0,0]),("green","0 1 0",[0,255,0]),("blue","0 0 1",[0,0,255]),
        ("white","1 1 1",[255]*3),("black","0 0 0",[0]*3),("mid",".5 .5 .5",[srgb8(.25)]*3),
    ]:
        add("calrgb-"+id,f"/CR cs {values} sc",expected)
    for l in (0,8,25,50,75,100):
        add(f"lab-neutral-{l}",f"/L cs {l} 0 0 sc",neutral_lab(l))
    for id,v in [("red","50 60 40"),("green","65 -50 40"),("blue","40 30 -60"),("range-clamp","50 120 -120")]:
        add("lab-"+id,f"/L cs {v} sc")
    for id,v in [("white","0 0 0 0"),("cyan","1 0 0 0"),("magenta","0 1 0 0"),("yellow","0 0 1 0"),
                 ("red","0 1 1 0"),("green","1 0 1 0"),("blue","1 1 0 0"),("black","0 0 0 1"),
                 ("gray25","0 0 0 .25"),("gray50","0 0 0 .5"),("gray75","0 0 0 .75"),
                 ("mixed1",".2 .4 .6 .1"),("mixed2",".6 .2 .1 .4"),("mixed3",".5 .5 .5 .5")]:
        add("cmyk-"+id,v+" k")
    return rows

def fixture(path,rows):
    # 6 columns, fixed integer point coordinates, each patch 40x40 with
    # 8pt gutters. Measure a central 16x16pt region, away from all edges.
    width=288;height=((len(rows)+5)//6)*48
    cmds=[]
    for i,p in enumerate(rows):
        x=(i%6)*48+4;y=height-(i//6+1)*48+4
        p["sample"]=[x+12,height-y-28,x+28,height-y-12]
        cmds.append(f"{p['cmd']} {x} {y} 40 40 re f")
    data=("\n".join(cmds)+"\n").encode()
    spaces=b"/CG [/CalGray << /WhitePoint [.96422 1 .82521] /Gamma 2 >>] /CR [/CalRGB << /WhitePoint [.95047 1 1.08883] /Gamma [2 2 2] /Matrix [.4124564 .2126729 .0193339 .3575761 .7151522 .1191920 .1804375 .0721750 .9503041] >>] /L [/Lab << /WhitePoint [.96422 1 .82521] /Range [-100 100 -100 100] >>]"
    objects=[b"<< /Type /Catalog /Pages 2 0 R >>",b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << /ColorSpace << ".encode()+spaces+b" >> >> /Contents 4 0 R >>",
        f"<< /Length {len(data)} >>\nstream\n".encode()+data+b"endstream"]
    buf=bytearray(b"%PDF-1.4\n");offsets=[]
    for i,obj in enumerate(objects,1):
        offsets.append(len(buf));buf.extend(f"{i} 0 obj\n".encode()+obj+b"\nendobj\n")
    x=len(buf);buf.extend(f"xref\n0 {len(objects)+1}\n0000000000 65535 f \n".encode())
    for off in offsets:buf.extend(f"{off:010d} 00000 n \n".encode())
    buf.extend(f"trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n".encode())
    path.write_bytes(buf)
    return width,height

def measure(im,box):
    patch=im.crop(box)
    stats=ImageStat.Stat(patch)
    return [round(x,6) for x in stats.mean], max(hi-lo for lo,hi in patch.getextrema())

def main():
    p=argparse.ArgumentParser()
    for arg in ("probe","reference","out"):
        p.add_argument("--"+arg,type=pathlib.Path,required=True)
    a=p.parse_args()
    a.probe=a.probe.resolve();a.reference=a.reference.resolve()
    a.out.mkdir(parents=True,exist_ok=True)
    version=subprocess.run([str(a.reference),"-v"],capture_output=True,text=True,timeout=5,check=True)
    report={"schema":1,"reference_version":(version.stderr or version.stdout).splitlines()[0],"rows":[],"renders":[]}
    rows=patches()
    with tempfile.TemporaryDirectory(prefix="rpcore-color-") as tmp:
        work=pathlib.Path(tmp);pdf=work/"patches.pdf"
        size=fixture(pdf,rows);report["fixture_sha256"]=hashlib.sha256(pdf.read_bytes()).hexdigest()
        for dpi in (72,144):
            n,r,im,ref=render_pair(a,pdf,1,dpi,work,f"patches-{dpi}",None)
            render={"dpi":dpi,"native":n,"reference":r}
            report["renders"].append(render)
            scale=dpi//72;expected_size=tuple(x*scale for x in size)
            if im is None or ref is None or im.size!=expected_size or ref.size!=expected_size:
                render["status"]="render_failed"
                continue
            render["status"]="compared"
            im.save(a.out/f"patches-{dpi}-native.png");ref.save(a.out/f"patches-{dpi}-reference.png")
            for p in rows:
                box=tuple(x*scale for x in p["sample"])
                nv,ns=measure(im,box);rv,rs=measure(ref,box)
                err=[abs(x-y) for x,y in zip(nv,rv)]
                analytic=None if p["expected"] is None else max(abs(x-y) for x,y in zip(nv,p["expected"]))
                report["rows"].append({**p,"dpi":dpi,"native_rgb":nv,"reference_rgb":rv,
                    "native_spread":ns,"reference_spread":rs,"reference_mae":round(sum(err)/3,6),
                    "analytic_max_error":analytic})
    (a.out/"report.json").write_text(json.dumps(report,indent=2)+"\n")
    lines=["# RPCore calibrated color patch benchmark","",
        "Central 16x16 PDF-point samples, at 72/144 DPI. RGB differences versus Poppler are diagnostic, not colorimetric accuracy. Analytic checks use independent scalar sRGB/neutral Lab values and primary endpoints; no CMYK profile is assumed.","",
        f"Reference: {report['reference_version']}","","| Patch | DPI | RPCore RGB | Reference RGB | Reference MAE | Analytic max error |",
        "|---|---:|---|---|---:|---:|"]
    for r in report["rows"]:
        lines.append(f"| {r['id']} | {r['dpi']} | {r['native_rgb']} | {r['reference_rgb']} | {r['reference_mae']} | {r['analytic_max_error']} |")
    checked=[r for r in report["rows"] if r["analytic_max_error"] is not None]
    passed=sum(r["analytic_max_error"]<=1 and r["native_spread"]==0 for r in checked)
    lines += ["",f"Analytic checks: {passed}/{len(checked)} (max channel error <=1, uniform patch).",
        f"Compared patches: {len(report['rows'])}/{len(rows)*2}.",
        "Reference differences are reported without thresholds. Original synthetic images only."]
    md="\n".join(lines)+"\n";(a.out/"report.md").write_text(md);print(md)
    return int(len(report["rows"])!=len(rows)*2 or len(checked)!=36 or passed!=len(checked)
               or any(r["native_spread"]!=0 for r in report["rows"]))

if __name__=="__main__":
    raise SystemExit(main())

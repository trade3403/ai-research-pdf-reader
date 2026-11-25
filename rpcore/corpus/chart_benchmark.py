#!/usr/bin/env python3
"""Chart-region fidelity baseline; Poppler/Pillow are test-only."""
import argparse
import hashlib
import json
import math
import pathlib
import re
import subprocess
import tempfile
import time
from PIL import Image, ImageChops, ImageStat
from run import download, invoke
from visual import open_rgb, compare, reference_render

ROOT=pathlib.Path(__file__).parent

def load_cases():
    m=json.loads((ROOT/"charts.json").read_text(encoding="utf-8"))
    if m["schema"]!=1 or not 1<=len(m["cases"])<=16:
        raise ValueError("chart manifest")
    ids=set()
    for c in m["cases"]:
        if not re.fullmatch(r"[a-z0-9-]{1,64}",c["id"]) or c["id"] in ids:
            raise ValueError("chart ID")
        ids.add(c["id"])
        if type(c["page"]) is not int or not 1<=c["page"]<=1000:
            raise ValueError("chart page")
        r=c["rect"]
        if len(r)!=4 or not all(type(v) in (int,float) and math.isfinite(v) and 0<=v<=1 for v in r) or not(r[0]<r[2] and r[1]<r[3]):
            raise ValueError("chart rectangle")
    return m

def pixel_box(size,rect):
    w,h=size
    return (math.floor(w*rect[0]),math.floor(h*rect[1]),math.ceil(w*rect[2]),math.ceil(h*rect[3]))

def chroma_mask(im):
    r,g,b=im.split()
    hi=ImageChops.lighter(ImageChops.lighter(r,g),b)
    lo=ImageChops.darker(ImageChops.darker(r,g),b)
    return ImageChops.subtract(hi,lo).point(lambda x:255 if x>=24 else 0)

def chart_metrics(a,b):
    result,heat=compare(a,b)
    if heat is None:
        return result,heat
    am,bm=chroma_mask(a),chroma_mask(b)
    union=ImageChops.lighter(am,bm)
    both=ImageChops.darker(am,bm)
    count=union.histogram()[255]
    result["chroma_union_pixels"]=count
    result["chroma_iou"]=round(both.histogram()[255]/count,6) if count else None
    result["chroma_union_mae"]=round(sum(ImageStat.Stat(ImageChops.difference(a,b),mask=union).mean)/3,6) if count else None
    return result,heat

def synthetic_pdf(path):
    # Original, deterministic three-panel graphic: RGB lines/grid, CMYK
    # swatches, alpha overlap. No fonts or borrowed chart data.
    data=b"""q
0.8 G 0.5 w [4 3] 0 d
20 35 m 220 35 l 20 60 m 220 60 l 20 85 m 220 85 l 20 110 m 220 110 l S
[] 0 d 0 G 1 w 20 20 m 20 135 l 220 135 l S
1 0 0 RG 1.5 w 20 30 m 65 48 l 110 45 l 155 90 l 220 115 l S
0 0.65 0.9 RG 2 w [7 3] 2 d 20 40 m 65 65 l 110 60 l 155 103 l 220 120 l S
Q
q
0 1 1 0 k 240 95 35 35 re f
1 0 1 0 k 280 95 35 35 re f
1 1 0 0 k 320 95 35 35 re f
0 0 0 0.5 k 240 55 35 35 re f
0.2 0.4 0.6 0.1 k 280 55 35 35 re f
0.2 0.4 0.6 rg 320 55 35 35 re f
Q
q 1 0 0 rg 375 50 55 65 re f /Half gs 0 0 1 rg 405 25 55 65 re f Q
"""
    objects=[b"<< /Type /Catalog /Pages 2 0 R >>",b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
             b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 480 150] /Resources << /ExtGState << /Half 5 0 R >> >> /Contents 4 0 R >>",
             f"<< /Length {len(data)} >>\nstream\n".encode()+data+b"endstream",
             b"<< /Type /ExtGState /ca 0.5 /CA 0.5 >>"]
    out=bytearray(b"%PDF-1.4\n")
    offsets=[]
    for i,obj in enumerate(objects,1):
        offsets.append(len(out))
        out.extend(f"{i} 0 obj\n".encode()+obj+b"\nendobj\n")
    x=len(out)
    out.extend(f"xref\n0 {len(objects)+1}\n0000000000 65535 f \n".encode())
    for off in offsets:
        out.extend(f"{off:010d} 00000 n \n".encode())
    out.extend(f"trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n".encode())
    path.write_bytes(out)

def render_pair(a,pdf,page,dpi,work,stem,fonts):
    native=work/(stem+"-native.png")
    reference=work/(stem+"-reference")
    n=invoke(a.probe,pdf,"render",page,native,fonts,dpi,4,False,True)
    r=reference_render(a.reference,pdf,page,reference,dpi)
    if n["status"]!="ok" or r["status"]!="ok":
        return n,r,None,None
    im,ref=open_rgb(native),open_rgb(reference.with_suffix(".png"))
    if list(im.size)!=[n.get("width"),n.get("height")]:
        raise ValueError("native PNG size mismatch")
    return n,r,im,ref

def report_md(report):
    lines=["# RPCore chart-region visual baseline","",
           "Identical page rendering DPI, no rescaling, then the same normalized crop on both images. RGB MAE is 0–255; chroma mask means channel spread >=24. Chroma MAE is calculated over the union of both masks, including missing/extra colored ink. These are diagnostic metrics, not colorimetric Delta E.",
           "",f"Reference: {report['reference_version']}","",
           "| Chart | DPI | Native | Reference | Result | RGB MAE | Ink IoU | Chroma IoU | Chroma MAE |",
           "|---|---:|---|---|---|---:|---:|---:|---:|"]
    for row in report["rows"]:
        c=row["comparison"]
        lines.append(f"| {row['id']} | {row['dpi']} | {row['native']['status']} | {row['reference']['status']} | {c['status']} | {c.get('mae_rgb_0_255','—')} | {c.get('ink_iou','—')} | {c.get('chroma_iou','—')} | {c.get('chroma_union_mae','—')} |")
    lines+=["",f"Compared {sum(r['comparison']['status']=='compared' for r in report['rows'])}/{len(report['rows'])} selected regions.",
            "Only original synthetic fixture PNGs are uploaded. Real-paper crops remain temporary. No fidelity threshold is asserted."]
    return "\n".join(lines)+"\n"

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--probe",required=True,type=pathlib.Path)
    p.add_argument("--reference",required=True,type=pathlib.Path)
    p.add_argument("--font-dir",required=True,type=pathlib.Path)
    p.add_argument("--out",required=True,type=pathlib.Path)
    a=p.parse_args()
    a.probe=a.probe.resolve();a.reference=a.reference.resolve();a.font_dir=a.font_dir.resolve()
    if not a.probe.is_file() or not a.reference.is_file() or not a.font_dir.is_dir():
        p.error("missing executables or font directory")
    a.out.mkdir(parents=True,exist_ok=True)
    version=subprocess.run([str(a.reference),"-v"],capture_output=True,text=True,timeout=5,check=True)
    m=load_cases()
    report={"schema":1,"reference_version":(version.stderr or version.stdout).splitlines()[0][:200],"source":m["source"],"rows":[]}
    deadline=time.monotonic()+240
    with tempfile.TemporaryDirectory(prefix="rpcore-charts-") as tmp:
        work=pathlib.Path(tmp)
        real=work/"resnet.pdf";synthetic=work/"original.pdf"
        synthetic_pdf(synthetic)
        report["synthetic_sha256"]=hashlib.sha256(synthetic.read_bytes()).hexdigest()
        try:
            download(m["source"],real)
            report["source_sha256"]=hashlib.sha256(real.read_bytes()).hexdigest()
        except Exception as e:
            report["download_error"]=str(e)[:300]
        synthetic_cases=[
            {"id":"synthetic-lines","page":1,"rect":[0,0,.48,1]},
            {"id":"synthetic-swatches","page":1,"rect":[.48,0,.77,1]},
            {"id":"synthetic-alpha","page":1,"rect":[.77,0,1,1]}]
        for dpi in (72,144):
            for pdf,cases,fonts,label in [(synthetic,synthetic_cases,None,"synthetic"),(real,m["cases"],a.font_dir,"paper")]:
                for page in sorted({c["page"] for c in cases}):
                    n=r={"status":"not_run"};im=ref=None
                    try:
                        if label=="paper" and "download_error" in report:
                            n=r={"status":"download_error"}
                        elif time.monotonic()>=deadline:
                            n=r={"status":"budget_skipped"}
                        else:
                            n,r,im,ref=render_pair(a,pdf,page,dpi,work,f"{label}-{page}-{dpi}",fonts)
                    except (OSError,ValueError,Image.DecompressionBombError) as e:
                        n={"status":"image_error","reason":str(e)[:300]}
                    for c in [x for x in cases if x["page"]==page]:
                        row={**c,"dpi":dpi,"native":n,"reference":r,"comparison":{"status":"render_failed"}}
                        if im is not None and ref is not None:
                            if im.size!=ref.size:
                                row["comparison"]={"status":"dimension_mismatch","native_size":list(im.size),"reference_size":list(ref.size)}
                            else:
                                box=pixel_box(im.size,c["rect"])
                                row["pixel_box"]=list(box)
                                ac,bc=im.crop(box),ref.crop(box)
                                row["comparison"],heat=chart_metrics(ac,bc)
                                if label=="synthetic" and heat is not None:
                                    panel=Image.new("RGB",(ac.width*3,ac.height),"white")
                                    panel.paste(ac,(0,0));panel.paste(bc,(ac.width,0));panel.paste(heat,(ac.width*2,0))
                                    panel.save(a.out/f"{c['id']}-{dpi}-native-reference-diff.png")
                        report["rows"].append(row)
    (a.out/"report.json").write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
    md=report_md(report);(a.out/"report.md").write_text(md,encoding="utf-8");print(md)
    return int(len(report["rows"])!=14 or any(r["comparison"]["status"]!="compared" for r in report["rows"]))

if __name__=="__main__":
    raise SystemExit(main())

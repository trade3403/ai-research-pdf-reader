#!/usr/bin/env python3
"""Independent observational image comparison. Test tooling only; no runtime renderer dependency."""
import argparse
import hashlib
import json
import os
import pathlib
import re
import subprocess
import tempfile
import time

from PIL import Image, ImageChops, ImageStat, ImageFilter
from run import download, invoke, sample_pages, MAX_DOWNLOAD

MAX_PIXELS = 4_000_000
MAX_PNG_BYTES = 32 * 1024 * 1024
Image.MAX_IMAGE_PIXELS = MAX_PIXELS
DPI = 36  # default; both engines receive the same requested DPI
INK_THRESHOLD = 245
DIFF_THRESHOLD = 16
TILE = 64

def open_rgb(path):
    if path.stat().st_size > MAX_PNG_BYTES:
        raise ValueError("PNG byte limit")
    with Image.open(path) as im:
        if im.format != "PNG" or im.width * im.height > MAX_PIXELS or min(im.size) <= 0:
            raise ValueError("PNG dimensions or format")
        rgba = im.convert("RGBA")
        return Image.alpha_composite(Image.new("RGBA", rgba.size, "white"), rgba).convert("RGB")

def ink_mask(im):
    # Any non-white-enough RGB channel counts as ink, including bright yellow.
    r, g, b = im.split()
    low = ImageChops.darker(ImageChops.darker(r, g), b)
    return low.point(lambda v: 255 if v < INK_THRESHOLD else 0)

def compare(native, reference):
    if native.size != reference.size:
        return {"status": "dimension_mismatch", "native_size": list(native.size),
                "reference_size": list(reference.size)}, None
    w, h = native.size
    n = w*h
    diff = ImageChops.difference(native, reference)
    r, g, b = diff.split()
    maximum = ImageChops.lighter(ImageChops.lighter(r, g), b)
    hist = maximum.histogram()
    na, nb = ink_mask(native), ink_mask(reference)
    both = ImageChops.darker(na, nb).histogram()[255]
    union = ImageChops.lighter(na, nb).histogram()[255]
    native_ink, ref_ink = na.histogram()[255], nb.histogram()[255]
    # Spatial tolerance is diagnostic only: dilate the opposite ink mask
    # by one pixel (Chebyshev distance). Raw exact and IoU metrics remain.
    # This distinguishes small edge shifts from unmatched ink far from any
    # opposite ink, without moving, resizing or aligning either image.
    near_native = na.filter(ImageFilter.MaxFilter(3))
    near_reference = nb.filter(ImageFilter.MaxFilter(3))
    ref_matched = ImageChops.darker(nb, near_native).histogram()[255]
    native_matched = ImageChops.darker(na, near_reference).histogram()[255]
    # Low-pass MAE is supplementary, not a replacement for raw MAE.
    blurred = ImageChops.difference(native.filter(ImageFilter.GaussianBlur(1)),
                                   reference.filter(ImageFilter.GaussianBlur(1)))
    tiles = []
    for y in range(0,h,TILE):
        for x in range(0,w,TILE):
            box = (x,y,min(w,x+TILE),min(h,y+TILE))
            crop = maximum.crop(box)
            th = crop.histogram()
            count = sum(th[DIFF_THRESHOLD+1:])
            if count:
                tiles.append({"box": list(box), "fraction_gt16": round(count/crop.width/crop.height,6)})
    tiles.sort(key=lambda t:t["fraction_gt16"], reverse=True)
    # White = identical; deeper red = larger per-pixel maximum channel error.
    inverse = maximum.point(lambda v: 255-v)
    heat = Image.merge("RGB", (Image.new("L", maximum.size,255), inverse, inverse))
    return {
        "status": "compared", "width": w, "height": h,
        "mae_rgb_0_255": round(sum(ImageStat.Stat(diff).mean)/3,6),
        "exact_pixel_fraction": round(hist[0]/n,6),
        "fraction_gt16": round(sum(hist[DIFF_THRESHOLD+1:])/n,6),
        "native_ink_fraction": round(native_ink/n,6),
        "reference_ink_fraction": round(ref_ink/n,6),
        "ink_iou": round(both/union,6) if union else 1.0,
        "reference_ink_missing_fraction": round((ref_ink-both)/ref_ink,6) if ref_ink else 0.0,
        "native_extra_ink_fraction": round((native_ink-both)/native_ink,6) if native_ink else 0.0,
        "reference_ink_unmatched_1px_fraction": round((ref_ink-ref_matched)/ref_ink,6) if ref_ink else 0.0,
        "native_ink_unmatched_1px_fraction": round((native_ink-native_matched)/native_ink,6) if native_ink else 0.0,
        "blurred_mae_rgb_0_255": round(sum(ImageStat.Stat(blurred).mean)/3,6),
        "worst_tiles": tiles[:10],
    }, heat

def reference_render(exe, pdf, page, stem, dpi=DPI):
    # Single page and crop box at requested DPI. No resize or alignment.
    start = time.monotonic()
    try:
        p = subprocess.run([str(exe), "-f",str(page),"-l",str(page),"-singlefile",
                            "-cropbox","-r",str(dpi),"-png",str(pdf),str(stem)],
                           capture_output=True, timeout=20)
    except subprocess.TimeoutExpired:
        return {"status":"timeout"}
    result = {"wall_ms":round((time.monotonic()-start)*1000)}
    path = stem.with_suffix(".png")
    if p.returncode or not path.is_file():
        result.update(status="reference_error", exit_code=p.returncode,
                      diagnostic=p.stderr[-1000:].decode("utf-8","replace"))
    else:
        result["status"]="ok"
    return result

def compare_page(args, pdf, page, work, name):
    stem=work/(name+"-reference")
    native=work/(name+"-native.png")
    ref=stem.with_suffix(".png")
    row={"page":page}
    row["native"]=invoke(args.probe,pdf,"render",page,native,args.font_dir,args.dpi)
    row["reference"]=reference_render(args.reference,pdf,page,stem,args.dpi)
    if row["native"]["status"]!="ok" or row["reference"]["status"]!="ok":
        row["comparison"]={"status":"render_failed"}
        return row
    try:
        a,b=open_rgb(native),open_rgb(ref)
        if list(a.size)!=[row["native"].get("width"),row["native"].get("height")]:
            raise ValueError("probe PNG size does not match reported size")
        row["comparison"],heat=compare(a,b)
        if args.save_images:
            images=args.out/"images"
            images.mkdir(exist_ok=True)
            a.save(images/(name+"-native.png"))
            b.save(images/(name+"-reference.png"))
            if heat is not None:
                heat.save(images/(name+"-difference.png"))
                panel=Image.new("RGB",(a.width*3,a.height),"white")
                panel.paste(a,(0,0));panel.paste(b,(a.width,0));panel.paste(heat,(a.width*2,0))
                panel.save(images/(name+"-comparison.png"))
    except (OSError,ValueError,Image.DecompressionBombError) as e:
        row["comparison"]={"status":"image_error","reason":str(e)[:300]}
    return row

def markdown(report):
    lines=["# RPCore visual comparison","",
           f"Independent Poppler reference, {report['dpi']} DPI / {report['dpi']/72:g} px per PDF point. No image rescaling.",
           "Comparison completion is not a visual-fidelity pass. Font replacement, hinting and anti-aliasing can affect pixel metrics.",
           "",f"Font mode: {report['font_mode']}. Reference: {report['reference_version']}","",
           "| Paper | Page | Native | Reference | Comparison | RGB MAE (0–255) | Pixels Δ>16 | Ink IoU | Missing reference ink | Unmatched ref. ink (1 px) | Unmatched native ink (1 px) | Blurred MAE | Substituted faces |",
           "|---|---:|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|"]
    def pct(d,k):
        return f"{d[k]*100:.2f}%" if k in d else "—"
    for doc in report["documents"]:
        if not doc.get("pages"):
            lines.append(f"| {doc['id']} | — | — | — | {doc['status']} | — | — | — | — | — | — | — | — |")
        for row in doc.get("pages",[]):
            c=row["comparison"];n=row["native"]
            lines.append(f"| {doc['id']} | {row['page']} | {n['status']} | {row['reference']['status']} | {c['status']} | {c.get('mae_rgb_0_255','—')} | {pct(c,'fraction_gt16')} | {pct(c,'ink_iou')} | {pct(c,'reference_ink_missing_fraction')} | {pct(c,'reference_ink_unmatched_1px_fraction')} | {pct(c,'native_ink_unmatched_1px_fraction')} | {c.get('blurred_mae_rgb_0_255','—')} | {n.get('substituted_fonts') or '—'} |")
    lines += ["",f"Compared pages: {report['compared']}/{report['selected']}.",
              "Worst 64×64 pixel tiles and source SHA-256 are in report.json.",
              "One-pixel tolerance and Gaussian blur radius 1 are supplementary diagnostics, not accuracy thresholds; physical tolerance differs with DPI.",
              "No automatic accuracy threshold is asserted; inspect discrepancies before release."]
    return "\n".join(lines)+"\n"

def main():
    p=argparse.ArgumentParser()
    p.add_argument("--probe",required=True,type=pathlib.Path)
    p.add_argument("--reference",required=True,type=pathlib.Path,help="pdftoppm executable")
    p.add_argument("--out",required=True,type=pathlib.Path)
    p.add_argument("--manifest",type=pathlib.Path,default=pathlib.Path(__file__).with_name("papers.json"))
    p.add_argument("--font-dir",type=pathlib.Path)
    p.add_argument("--dpi",type=int,choices=(36,72,144),default=DPI)
    p.add_argument("--local",type=pathlib.Path)
    p.add_argument("--save-images",action="store_true",help="Local PDF only; confirm source permissions")
    a=p.parse_args()
    if a.save_images and not a.local:
        p.error("--save-images requires --local (source permissions)")
    a.probe=a.probe.resolve();a.reference=a.reference.resolve()
    if not a.probe.is_file() or not a.reference.is_file():
        p.error("missing executable")
    if a.font_dir:
        a.font_dir=a.font_dir.resolve()
        if not a.font_dir.is_dir():
            p.error("font directory missing")
    a.out.mkdir(parents=True,exist_ok=True)
    version=subprocess.run([str(a.reference),"-v"],capture_output=True,text=True,timeout=5)
    if version.returncode:
        p.error("reference version check failed")
    report={"schema":2,"dpi":a.dpi,"ink_tolerance_pixels":1,"blur_radius_pixels":1,"font_mode":"explicit_urw" if a.font_dir else "strict",
            "reference_version":(version.stderr or version.stdout).splitlines()[0][:200],
            "ink_threshold":INK_THRESHOLD,"diff_threshold":DIFF_THRESHOLD,
            "documents":[],"selected":0,"compared":0}
    items=[{"id":"local","local":str(a.local)}] if a.local else json.loads(a.manifest.read_text(encoding="utf-8"))
    if not isinstance(items,list) or not 1<=len(items)<=10:
        p.error("manifest count")
    ids=[i["id"] for i in items]
    if len(set(ids))!=len(ids) or any(not re.fullmatch(r"[a-zA-Z0-9_-]{1,64}",i) for i in ids):
        p.error("manifest IDs")
    deadline=time.monotonic()+360
    with tempfile.TemporaryDirectory(prefix="rpcore-visual-") as tmp:
        work=pathlib.Path(tmp)
        for item in items:
            d={"id":item["id"],"source":item.get("source"),"pages":[]}
            report["documents"].append(d)
            if time.monotonic()>=deadline:
                d["status"]="budget_skipped";continue
            pdf=pathlib.Path(item["local"]).resolve() if "local" in item else work/(item["id"]+".pdf")
            try:
                if "local" not in item:
                    download(item["url"],pdf)
                if pdf.stat().st_size>MAX_DOWNLOAD:
                    raise ValueError("PDF byte limit")
                d["sha256"]=hashlib.sha256(pdf.read_bytes()).hexdigest()
            except Exception as e:
                d.update(status="download_error",reason=str(e)[:300]);continue
            d["inspect"]=invoke(a.probe,pdf,"inspect")
            n=d["inspect"].get("pages",0)
            if d["inspect"]["status"]!="ok" or not 1<=n<=100000:
                d["status"]="inspect_error";continue
            d["status"]="ok"
            for page in sample_pages(n):
                report["selected"]+=1
                if time.monotonic()>=deadline:
                    row={"page":page,"native":{"status":"budget_skipped"},"reference":{"status":"budget_skipped"},"comparison":{"status":"budget_skipped"}}
                else:
                    row=compare_page(a,pdf,page,work,f"{item['id']}-{page}")
                d["pages"].append(row)
                if row["comparison"]["status"]=="compared":
                    report["compared"]+=1
    (a.out/"report.json").write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
    md=markdown(report)
    (a.out/"report.md").write_text(md,encoding="utf-8")
    print(md)
    return int(not report["selected"] or report["compared"]!=report["selected"] or any(d["status"]!="ok" for d in report["documents"]))

if __name__=="__main__":
    raise SystemExit(main())

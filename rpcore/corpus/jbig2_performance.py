#!/usr/bin/env python3
"""JBIG2-only observational baseline; PDFium is used only by test workers."""
import argparse
import importlib.metadata as metadata
import json
import os
from pathlib import Path
import platform
import sys
import subprocess
import tempfile
from PIL import Image
import jbig2_benchmark as corpus
import pdfium_performance as paired

CASES=("bitmap","bitmap-mmr","bitmap-stripe","bitmap-halftone",
       "bitmap-refine","bitmap-symbol-symhuffcustom-texthuffcustom")
REPEATS=5
SEEDS={"annex-h-1":(64,56),"annex-h-2":(64,56),"annex-h-3":(37,8)}

def exact_rgb(fields,data,expected):
    return (fields.get("status")=="ok"
            and (int(fields.get("width",0)),int(fields.get("height",0)))==expected.size
            and data==expected.tobytes())

def self_test():
    im=Image.new("RGB",(2,1),"white")
    f={"status":"ok","width":"2","height":"1"}
    assert exact_rgb(f,im.tobytes(),im)
    assert not exact_rgb(f,bytes(6),im)
    assert not exact_rgb(dict(f,width="1"),im.tobytes(),im)
    assert not exact_rgb(dict(f,status="failed"),im.tobytes(),im)

def main():
    p=argparse.ArgumentParser()
    for k in ("probe","raw-probe","seeds","out","commit"):
        p.add_argument("--"+k,required=True)
    a=p.parse_args()
    self_test()
    if metadata.version("pypdfium2")!=paired.PDFIUM_VERSION or metadata.version("Pillow")!=paired.PILLOW_VERSION:
        raise ValueError("reference version mismatch")
    import pypdfium2 as pdfium
    import pypdfium2_raw
    out=Path(a.out);out.mkdir(parents=True,exist_ok=True)
    report={"schema":3,"commit":a.commit,"reference_revision":corpus.REV,
        "rpcore_probe_sha256":corpus.sha(Path(a.probe).read_bytes()),
        "raw_probe_sha256":corpus.sha(Path(a.raw_probe).read_bytes()),
        "pdfium_binding":metadata.version("pypdfium2"),"pdfium_version":str(pdfium.PDFIUM_INFO),
        "pdfium_binary_sha256":{p.name:corpus.sha(p.read_bytes()) for p in Path(pypdfium2_raw.__file__).parent.iterdir() if p.is_file() and ".so" in p.name},
        "pillow":metadata.version("Pillow"),"os":platform.platform(),"python":sys.version,
        "cpu":Path("/proc/cpuinfo").read_text().split("\n\n")[0],"cpu_count":os.cpu_count(),
        "scope":"raw native decode only, 20 same-process repetitions; PDF page render at 72dpi, cold/warm, 5 isolated measured pairs plus one discarded pair. Render excludes document open, RGB conversion and disk I/O. RSS is whole process including wrapper and output, not engine-only. Native raw stats are logical work/retained bytes, not heap peak.",
        "raw":[],"pages":[]}
    if not report["pdfium_binary_sha256"]:
        raise ValueError("reference binary missing")
    with tempfile.TemporaryDirectory(prefix="rpcore-jbig2-perf-") as td:
        tmp=Path(td);dest=tmp/"output";rss=tmp/"rss"
        for name,size in SEEDS.items():
            seed=Path(a.seeds)/name
            fields,data=paired.invoke([a.raw_probe,str(seed),str(dest)],dest,rss)
            if fields.get("status")!="ok":
                raise ValueError("raw benchmark failed "+name+": "+str(fields))
            w,h=int(fields["width"]),int(fields["height"])
            times=[int(t) for t in fields["samples_ns"].split(",")]
            if (w,h)!=size or len(data)!=((w+7)//8)*h or len(times)!=20 or min(times)<=0 or int(fields["packed_bytes"])!=len(data):
                raise ValueError("invalid raw output "+name)
            if fields["peak_process_rss_kib"]<=0:
                raise ValueError("invalid raw RSS")
            row=dict(id=name,seed_sha256=corpus.sha(seed.read_bytes()),packed_sha256=corpus.sha(data),
                     throughput_mpix_s=w*h*1000/fields["elapsed_ns"],**fields)
            report["raw"].append(row)
            print("JBIG2 raw",name,"median_ns",fields["elapsed_ns"],"rss_kib",fields["peak_process_rss_kib"],flush=True)
        proc=subprocess.run([a.raw_probe,"--compose"],text=True,capture_output=True,timeout=60,check=True)
        fields=dict(line.split("=",1) for line in proc.stdout.splitlines() if "=" in line)
        kernels=[]
        for x in (0,3):
            f=int(fields[f"compose_x{x}_packed_ns"]);s=int(fields[f"compose_x{x}_scalar_ns"])
            if min(f,s)<=0:
                raise ValueError("invalid composition kernel timing")
            kernels.append({"x":x,"packed_ns":f,"scalar_ns":s,"speedup":s/f})
            print("JBIG2 compose",x,"packed_ns",f,"scalar_ns",s,"speedup",s/f,flush=True)
        report["compose_kernels"]=kernels
        for name in CASES:
            raw=corpus.fetch(name+".pdf");ref=corpus.fetch(name+"_expected.pdf.0.png")
            file=tmp/"input.pdf";file.write_bytes(raw)
            ref_file=tmp/"reference.png";ref_file.write_bytes(ref);expected=corpus.pixels(ref_file)
            for mode in ("cold","warm"):
                row={"id":name,"mode":mode,"pdf_sha256":corpus.sha(raw),"reference_sha256":corpus.sha(ref),"trials":[]}
                for i in range(REPEATS+1):
                    order=("rpcore","pdfium") if i%2==0 else ("pdfium","rpcore")
                    trial={"index":i,"discarded":i==0,"order":order}
                    for engine in order:
                        base=[str(file),"render","72",mode,str(dest)]
                        cmd=[a.probe,*base] if engine=="rpcore" else [sys.executable,paired.__file__,"--worker",*base]
                        f,data=paired.invoke(cmd,dest,rss)
                        f["exact_reference"]=exact_rgb(f,data,expected)
                        f["output_sha256"]=corpus.sha(data)
                        trial[engine]=f
                    row["trials"].append(trial)
                measured=row["trials"][1:]
                row["native_exact"]=all(t["rpcore"]["exact_reference"] for t in row["trials"])
                row["eligible"]=all(t[e]["exact_reference"] and t[e].get("peak_process_rss_kib",0)>0
                                    for t in row["trials"] for e in ("rpcore","pdfium"))
                for e in ("rpcore","pdfium"):
                    if all(t[e].get("status")=="ok" for t in measured):
                        row[e]={**paired.summarize([t[e]["elapsed_ns"] for t in measured]),
                                "open":paired.summarize([t[e]["open_ns"] for t in measured]),
                                "peak_process_rss_kib":max(t[e]["peak_process_rss_kib"] for t in measured)}
                if row["native_exact"]:
                    stage_names=("viewport_ns","display_list_ns","raster_ns")
                    for t in row["trials"]:
                        f=t["rpcore"]
                        stages=[int(f[k]) for k in stage_names]
                        if min(stages)<0 or sum(stages)>int(f["elapsed_ns"]):
                            raise ValueError("invalid stage timings "+name)
                    inner_names=("image_decode_ns","image_sampling_ns","resolve_ns")
                    for t in row["trials"]:
                        f=t["rpcore"]
                        inner=[int(f[k]) for k in inner_names]
                        if min(inner)<0 or sum(inner)>int(f["raster_ns"]):
                            raise ValueError("invalid raster substage timings "+name)
                    row["raster_parts"]={k:paired.summarize([int(t["rpcore"][k]) for t in measured]) for k in inner_names}
                    print("JBIG2 raster parts",name,mode,*(f"{k}={row['raster_parts'][k]['median_ns']}" for k in inner_names),flush=True)
                    row["native_stages"]={k:paired.summarize([int(t["rpcore"][k]) for t in measured]) for k in stage_names}
                    print("JBIG2 stages",name,mode,*(f"{k}={row['native_stages'][k]['median_ns']}" for k in stage_names),flush=True)
                if row["eligible"]:
                    row["relative_throughput_pct"]=100*row["pdfium"]["median_ns"]/row["rpcore"]["median_ns"]
                report["pages"].append(row)
                print("JBIG2 page",name,mode,"eligible" if row["eligible"] else "excluded",row.get("relative_throughput_pct"),flush=True)
    report["coverage"]={"raw":len(report["raw"]),"page_pairs":len(report["pages"]),
                        "eligible":sum(r["eligible"] for r in report["pages"])}
    (out/"report.json").write_text(json.dumps(report,indent=2,allow_nan=False)+"\n")
    lines=["# JBIG2专项性能 baseline",str(report["coverage"]),
           "Native raw decode; RSS is whole process. Ratios below require exact RGB against the pinned reference on every trial.",
           "| Raw seed | median µs | Mpix/s | RSS KiB |","|---|---:|---:|---:|"]
    for r in report["raw"]:
        lines.append(f"| {r['id']} | {r['elapsed_ns']/1000:.2f} | {r['throughput_mpix_s']:.3f} | {r['peak_process_rss_kib']} |")
    lines+=["","| PDF case / mode | RPCore median ms | PDFium median ms | ratio |","|---|---:|---:|---:|"]
    for r in report["pages"]:
        if r["eligible"]:
            lines.append(f"| {r['id']} / {r['mode']} | {r['rpcore']['median_ns']/1e6:.3f} | {r['pdfium']['median_ns']/1e6:.3f} | {r['relative_throughput_pct']:.2f}% |")
        else:
            lines.append(f"| {r['id']} / {r['mode']} | — | — | excluded |")
    (out/"report.md").write_text("\n".join(lines)+"\n")
    print("\n".join(lines))
    if len(report["raw"])!=3 or len(report["pages"])!=12 or not all(r["native_exact"] for r in report["pages"]) or not report["coverage"]["eligible"]:
        raise SystemExit(1)

if __name__=="__main__":
    main()

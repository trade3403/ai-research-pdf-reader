#!/usr/bin/env python3
"""Isolated B-P0a paired process benchmark. PDFium and Pillow are test-only."""
import argparse
import csv
import ctypes
import hashlib
import importlib.metadata as metadata
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import time

PDFIUM_VERSION = "5.8.0"
PILLOW_VERSION = "12.3.0"
DPIS = (96, 150, 200, 300)
REPEATS = 10
# Predeclared exploratory quality thresholds, not release quality gates.
MAE_MAX, DIFF_FRACTION_MAX, ORIGIN_TOL = 12.0, 0.08, 1.0

def pdfium_worker(path, op, dpi, mode, output):
    import pypdfium2 as pdfium
    data = Path(path).read_bytes()
    t = time.perf_counter_ns()
    doc = pdfium.PdfDocument(data)
    count = len(doc)
    if count != 1:
        raise ValueError("one-page fixtures only")
    page = doc[0]
    width_pt, height_pt = page.get_size()
    open_ns = time.perf_counter_ns() - t
    print(f"pages={count}\nopen_ns={open_ns}")
    if op == "inspect":
        print(f"elapsed_ns={open_ns}\nwidth_pt={width_pt}\nheight_pt={height_pt}")
    elif op == "render":
        def render():
            return page.render(scale=dpi / 72, fill_color=(255,255,255,255),
                               draw_annots=False, may_draw_forms=False,
                               force_bitmap_format=pdfium.raw.FPDFBitmap_BGRA,
                               rev_byteorder=True)
        if mode == "warm":
            render().close()
        t = time.perf_counter_ns()
        bitmap = render()
        ns = time.perf_counter_ns() - t
        im = bitmap.to_pil().convert("RGB")
        Path(output).write_bytes(im.tobytes())
        print(f"elapsed_ns={ns}\nwidth={im.width}\nheight={im.height}")
        bitmap.close()
    elif op == "text":
        def extract():
            text = page.get_textpage()
            rows = []
            for i in range(text.count_chars()):
                cp = pdfium.raw.FPDFText_GetUnicode(text.raw, i)
                if not cp:
                    raise ValueError("unmapped character")
                if chr(cp).isspace():
                    continue
                x, y = ctypes.c_double(), ctypes.c_double()
                if not pdfium.raw.FPDFText_GetCharOrigin(text.raw, i, ctypes.byref(x), ctypes.byref(y)):
                    raise ValueError("missing character origin")
                rows.append(f"{cp}\t{x.value}\t{y.value}\n")
            text.close()
            return "".join(rows)
        if mode == "warm":
            extract()
        t = time.perf_counter_ns()
        value = extract()
        ns = time.perf_counter_ns() - t
        Path(output).write_text(value, encoding="utf-8")
        print(f"elapsed_ns={ns}")
    else:
        raise ValueError("unknown operation")
    page.close()
    doc.close()
    print("status=ok")

def sha(data):
    return hashlib.sha256(data).hexdigest()

def quality(op, a, b, aa, bb):
    if op == "inspect":
        ok = (a["pages"] == b["pages"] == "1" and
              all(abs(float(a[k])-float(b[k])) <= 1e-6 for k in ("width_pt","height_pt")))
        return {"pass":ok, "reason":"page count and size"}
    if op == "text":
        def decode(data):
            rows = [line.split("\t") for line in data.decode("utf-8").splitlines()]
            return "".join(chr(int(r[0])) for r in rows), [(float(r[1]),float(r[2])) for r in rows]
        sa, xa = decode(aa)
        sb, xb = decode(bb)
        if sa != "ODo" or sb != "ODo" or len(xa) != len(xb):
            return {"pass":False,"reason":"expected complete ODo text","rpcore_text":sa,"pdfium_text":sb}
        diffs = [abs(x-y) for a,b in zip(xa,xb) for x,y in zip(a,b)]
        if not all(math.isfinite(d) for d in diffs):
            return {"pass":False,"reason":"nonfinite character origin"}
        delta = max(diffs)
        return {"pass":delta<=ORIGIN_TOL,
                "reason":"character origins in PDF points","max_origin_error_pt":delta}
    from PIL import Image, ImageChops
    w,h = int(a["width"]),int(a["height"])
    if (w,h)!=(int(b["width"]),int(b["height"])) or w<=0 or h<=0:
        return {"pass":False,"reason":"dimension mismatch"}
    if len(aa)!=w*h*3 or len(bb)!=w*h*3:
        return {"pass":False,"reason":"wrong byte count"}
    ia,ib=Image.frombytes("RGB",(w,h),aa),Image.frombytes("RGB",(w,h),bb)
    d=ImageChops.difference(ia,ib)
    hist=d.histogram()
    mae=sum((i%256)*n for i,n in enumerate(hist))/(w*h*3)
    channels=d.split()
    worst=ImageChops.lighter(ImageChops.lighter(channels[0],channels[1]),channels[2])
    frac=sum(worst.histogram()[33:])/(w*h)
    # Fixtures are not blank; reject an all-white result even if small ink coverage.
    nonwhite=all(any(lo<250 for lo,hi in im.getextrema()) for im in (ia,ib))
    return {"pass":nonwhite and mae<=MAE_MAX and frac<=DIFF_FRACTION_MAX,
            "reason":"RGB pixel gate","mae":mae,"fraction_channel_error_gt_32":frac,"nonwhite":nonwhite}

def summarize(xs):
    v=sorted(xs)
    return {"median_ns":statistics.median(v),"p95_ns":v[math.ceil(.95*len(v))-1]}

def invoke(cmd, output, rss, timeout=60):
    output.unlink(missing_ok=True)
    rss.unlink(missing_ok=True)
    try:
        proc=subprocess.run(["/usr/bin/time","-f","%M","-o",str(rss),*cmd],
                            text=True,capture_output=True,timeout=timeout,check=False)
    except subprocess.TimeoutExpired:
        return {"status":"timeout"},b""
    fields=dict(line.split("=",1) for line in proc.stdout.splitlines() if "=" in line)
    if proc.returncode or fields.get("status")!="ok":
        return {"status":"failed","exit":proc.returncode,"reason":proc.stderr[-1000:]},b""
    try:
        fields["elapsed_ns"]=int(fields["elapsed_ns"])
        fields["open_ns"]=int(fields["open_ns"])
        fields["peak_process_rss_kib"]=int(rss.read_text().strip())
        if fields["elapsed_ns"]<=0 or fields["open_ns"]<=0:
            raise ValueError("invalid timing")
    except (ValueError, KeyError, OSError) as e:
        return {"status":"invalid_measurement","reason":str(e)},b""
    data=output.read_bytes() if output.exists() else b""
    return fields,data

def benchmark(args):
    import pypdfium2 as pdfium
    if metadata.version("pypdfium2")!=PDFIUM_VERSION or metadata.version("Pillow")!=PILLOW_VERSION:
        raise ValueError("reference package version mismatch")
    out=Path(args.out);out.mkdir(parents=True,exist_ok=True)
    fixtures=Path(args.fixtures)
    cases=[]
    for kind,name in [("vector","rpcore-vector-demo.pdf"),("text","rpcore-truetype-demo.pdf"),("image","rpcore-image-demo.pdf")]:
        file=fixtures/name
        cases.append((kind,file,"inspect",96,"cold"))
        for dpi in DPIS:
            for mode in ("cold","warm"):
                cases.append((kind,file,"render",dpi,mode))
        if kind=="text":
            for mode in ("cold","warm"):
                cases.append((kind,file,"text",96,mode))
    # Record installed reference binary and packaged license notice fingerprints.
    import pypdfium2_raw
    rawdir=Path(pypdfium2_raw.__file__).parent
    binaries={p.name:sha(p.read_bytes()) for p in rawdir.iterdir() if p.is_file() and (".so" in p.name or p.suffix in (".dll",".dylib"))}
    if not binaries:
        raise ValueError("PDFium shared library not found in pinned wheel")
    notices={}
    for name in ("pypdfium2","Pillow"):
        dist=metadata.distribution(name)
        notices[name]={"license":dist.metadata.get("License-Expression") or dist.metadata.get("License"),
                       "files":{str(p):sha(dist.locate_file(p).read_bytes()) for p in dist.files or []
                                if "license" in str(p).lower() and dist.locate_file(p).is_file()}}
    report={"protocol":"B-P0a process probes; observational only",
            "reference_binary_sha256":binaries,"reference_license_metadata":notices,
            "command":sys.argv,
            "rpcore_commit":args.commit,"rpcore_probe_sha256":sha(Path(args.probe).read_bytes()),
            "pdfium_binding":metadata.version("pypdfium2"),"pdfium_version":str(pdfium.PDFIUM_INFO),
            "pillow":metadata.version("Pillow"),"python":sys.version,"os":platform.platform(),
            "cpu":platform.processor(),"cpu_count":os.cpu_count(),"thread_budget":"one process, serial requests; internal engine threads not instrumented",
            "cpu_info":Path("/proc/cpuinfo").read_text().split("\n\n")[0],
            "memory_info":Path("/proc/meminfo").read_text().splitlines()[0],
            "thresholds":{"mae":MAE_MAX,"fraction_error_gt32":DIFF_FRACTION_MAX,"origin_pt":ORIGIN_TOL},
            "timing_scope":"in-memory open+page enumeration; render API only or nonwhitespace Unicode+origin extraction; cold=fresh document, warm=one untimed same-document operation; excludes process startup, disk I/O and RGB conversion; Python/ctypes overhead included on PDFium side; RSS is whole process, not engine-only",
            "cases":[]}
    flat=[]
    with tempfile.TemporaryDirectory() as td:
        tmp=Path(td)
        for kind,file,op,dpi,mode in cases:
            label=f"{kind}-{op}-{dpi}-{mode}"
            case={"case":label,"input":file.name,"sha256":sha(file.read_bytes()),
                  "op":op,"dpi":dpi,"mode":mode,"trials":[]}
            hashes={"rpcore":set(),"pdfium":set()}
            for i in range(REPEATS+1):
                results={};buffers={}
                order=("rpcore","pdfium") if i%2==0 else ("pdfium","rpcore")
                for engine in order:
                    dest=tmp/f"{engine}.out";rss=tmp/f"{engine}.rss"
                    base=[str(file),op,str(dpi),mode,str(dest)]
                    cmd=[args.probe,*base] if engine=="rpcore" else [sys.executable,__file__,"--worker",*base]
                    results[engine],buffers[engine]=invoke(cmd,dest,rss)
                both=all(r["status"]=="ok" for r in results.values())
                try:
                    q=quality(op,results["rpcore"],results["pdfium"],buffers["rpcore"],buffers["pdfium"]) if both else {"pass":False,"reason":"failed operation"}
                except (ValueError, KeyError, OSError, OverflowError, IndexError) as e:
                    q={"pass":False,"reason":"invalid quality output: "+str(e)}
                trial={"trial":i,"warmup":i==0,"order":order,"results":results,"quality":q}
                case["trials"].append(trial)
                if i==0:
                    if op=="render" and both:
                        from PIL import Image
                        for e in order:
                            rr=results[e]
                            Image.frombytes("RGB",(int(rr["width"]),int(rr["height"])),buffers[e]).save(out/f"{label}-{e}.png")
                    continue
                for e in order:
                    if results[e]["status"]=="ok":
                        hashes[e].add(sha(buffers[e]))
                    flat.append({"case":label,"trial":i,"engine":e,"quality_pass":q["pass"],**results[e]})
            valid=all(t["quality"]["pass"] for t in case["trials"]) and all(len(v)==1 for v in hashes.values())
            case["eligible"]=valid
            case["valid_pairs"]=sum(t["quality"]["pass"] for t in case["trials"][1:])
            case["repeatable_output"]=all(len(v)==1 for v in hashes.values())
            if valid:
                for e in hashes:
                    v=[t["results"][e] for t in case["trials"][1:]]
                    case[e]={**summarize([x["elapsed_ns"] for x in v]),
                             "open":summarize([x["open_ns"] for x in v]),
                             "peak_process_rss_kib":max(x["peak_process_rss_kib"] for x in v)}
                case["rpcore_relative_throughput_pct"]=100*case["pdfium"]["median_ns"]/case["rpcore"]["median_ns"]
            report["cases"].append(case)
            print(label, "eligible" if valid else "excluded", case.get("rpcore_relative_throughput_pct"), flush=True)
    report["coverage"]={"eligible":sum(c["eligible"] for c in report["cases"]),"total":len(report["cases"])}
    (out/"report.json").write_text(json.dumps(report,indent=2,allow_nan=False),encoding="utf-8")
    fields=sorted({k for x in flat for k in x})
    with (out/"trials.csv").open("w",newline="",encoding="utf-8") as file:
        writer=csv.DictWriter(file,fieldnames=fields);writer.writeheader();writer.writerows(flat)
    lines=["# B-P0a PDFium paired benchmark (observational)",f"Coverage: {report['coverage']}",
           "Ratios only for quality-passing, repeatable cases. Whole-process RSS includes wrapper overhead.",
           "| case | valid pairs | RPCore median ms | PDFium median ms | relative throughput |",
           "|---|---:|---:|---:|---:|"]
    for c in report["cases"]:
        if c["eligible"]:
            lines.append(f"| {c['case']} | {c['valid_pairs']} | {c['rpcore']['median_ns']/1e6:.3f} | {c['pdfium']['median_ns']/1e6:.3f} | {c['rpcore_relative_throughput_pct']:.2f}% |")
        else:
            lines.append(f"| {c['case']} | {c['valid_pairs']} | — | — | excluded |")
    (out/"report.md").write_text("\n".join(lines)+"\n",encoding="utf-8")
    # Require successful measurement machinery, not a speed/quality threshold.
    if not report["coverage"]["eligible"]:
        raise RuntimeError("no quality-qualified benchmark cases")

def self_test():
    a={"pages":"1","width_pt":"20","height_pt":"10"}
    assert quality("inspect",a,a,b"",b"")["pass"]
    assert not quality("inspect",a,{**a,"pages":"2"},b"",b"")["pass"]
    v=b"79\t12\t24\n68\t44.85\t24\n111\t77.7\t24\n"
    assert quality("text",{}, {},v,v)["pass"]
    assert not quality("text",{}, {},v,v.replace(b"77.7",b"80.0"))["pass"]
    a={"width":"2","height":"1"}
    black=bytes([0]*6);white=bytes([255]*6)
    assert quality("render",a,a,black,black)["pass"]
    assert not quality("render",a,a,black,white)["pass"]
    assert not quality("render",a,a,white,white)["pass"]
    assert summarize(list(range(1,11)))=={"median_ns":5.5,"p95_ns":10}
    print("B-P0a benchmark protocol self-test passed")

if __name__=="__main__":
    if len(sys.argv)>1 and sys.argv[1]=="--worker":
        path,op,dpi,mode,output=sys.argv[2:]
        pdfium_worker(path,op,int(dpi),mode,output)
    elif sys.argv[1:]==["--self-test"]:
        self_test()
    else:
        p=argparse.ArgumentParser()
        for k in ("probe","fixtures","out","commit"):p.add_argument("--"+k,required=True)
        benchmark(p.parse_args())

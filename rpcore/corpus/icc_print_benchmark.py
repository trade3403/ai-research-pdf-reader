#!/usr/bin/env python3
"""Installed print profiles, numeric-only compatibility baseline. No profile assets exported."""
import argparse
import ctypes as C
import hashlib
import itertools
import random
import statistics
import json
import pathlib
import struct as S
import tempfile
from icc_render_benchmark import lcms,make_pdf,error
from color_benchmark import measure
from chart_benchmark import render_pair

CMYK_DOUBLE=(1<<22)|(6<<16)|(4<<3)
RGB8=(4<<16)|(3<<3)|1
SAMPLES=[(0,0,0,0),(255,0,0,0),(0,255,0,0),(0,0,255,0),(0,0,0,255),
         (255,255,0,0),(0,255,255,0),(255,0,255,0),(64,128,192,32),
         (128,128,128,128),(32,0,128,64),(255,255,255,255)]

# Retain original 12 swatches. Add an interior 3^4 grid, seeded irregular
# samples and near-boundary values. Deduplicate with stable order.
DENSE=list(dict.fromkeys(SAMPLES+list(itertools.product((17,113,239),repeat=4))))
rng=random.Random(3403)
DENSE=list(dict.fromkeys(DENSE+[tuple(rng.randrange(256) for _ in range(4)) for _ in range(35)]))
assert len(DENSE)==128
GATE_MAX=12
GATE_MEAN=3.0
GATE_P95=6
# These are engineering RGB8 thresholds, not perceptual Delta E thresholds.
def reference(lib,data,rows,intent=1):
    buf=C.create_string_buffer(data)
    src=lib.cmsOpenProfileFromMem(buf,len(data));dst=lib.cmsCreate_sRGBProfile()
    t=None
    try:
        if not src or not dst:raise RuntimeError('reference profile open')
        t=lib.cmsCreateTransform(src,CMYK_DOUBLE,dst,RGB8,intent,0)
        if not t:raise RuntimeError('reference transform')
        out=[]
        for row in rows:
            x=(C.c_double*4)(*(100*v for v in row['source']))
            y=(C.c_uint8*3)()
            lib.cmsDoTransform(t,x,y,1)
            out.append(list(y))
        return out
    finally:
        if t:lib.cmsDeleteTransform(t)
        if src:lib.cmsCloseProfile(src)
        if dst:lib.cmsCloseProfile(dst)

def metadata(data):
    if not 132<=len(data)<=16*1024*1024 or data[36:40]!=b'acsp' or data[16:20]!=b'CMYK':
        raise ValueError('expected bounded CMYK ICC')
    n=S.unpack_from('>I',data,128)[0]
    if n>1024 or 132+12*n>len(data):raise ValueError('ICC directory')
    tags={};media_white=None
    for i in range(n):
        key,off,size=S.unpack_from('>4sII',data,132+12*i)
        if off+size>len(data) or size<8:raise ValueError('ICC bounds')
        if key==b'wtpt':
            if size!=20 or data[off:off+8]!=b'XYZ '+bytes(4):raise ValueError('media white tag')
            media_white=[v/65536 for v in S.unpack_from('>iii',data,off+8)]
            if any(not 0<v<=2 for v in media_white):raise ValueError('media white range')
        tags[key.decode('ascii',errors='replace')]=data[off:off+4].decode('ascii',errors='replace')
    return {'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'version':data[8],
            'class':data[12:16].decode('ascii'),'pcs':data[20:24].decode('ascii'),'tag_types':tags,'media_white_xyz':media_white}

def absolute_print_gate(a,lib,work,data,meta,name):
    """Independent absolute LCMS oracle, mandatory native paths, numeric output only."""
    if meta['media_white_xyz'] is None:raise ValueError('missing media white')
    tag=meta['tag_types'].get('A2B1')
    if tag not in ('mft1','mft2','mAB '):raise ValueError('unsupported absolute A2B1')
    want='mab_A2B1' if tag=='mAB ' else 'cmyk_A2B1'
    result={'media_white_xyz':meta['media_white_xyz'],'rows':[],'pages':0}
    for bits in (8,16):
        samples=DENSE if bits==8 else [tuple(v*257+(1 if 0<v<255 else 0) for v in row) for row in DENSE]
        for offset in range(0,len(samples),32):
            pdf=work/'absolute.pdf'
            size,rows=make_pdf(pdf,data,4,samples[offset:offset+32],bits,intent='AbsoluteColorimetric')
            expected=reference(lib,data,rows,intent=3)
            for dpi in (72,144):
                label=f'{name}-absolute-{bits}-{offset}-{dpi}'
                n,r,im,ri=render_pair(a,pdf,1,dpi,work,label,None)
                diag=[v for k,v in n.items() if k.startswith('icc_diag_')]
                if im is None or im.size!=tuple(v*dpi//72 for v in size):raise ValueError('absolute render size')
                if (n.get('icc_vector_native'),n.get('icc_image_native'),n.get('icc_vector_alternate'),n.get('icc_image_alternate'))!=('1','1','0','0') or len(diag)!=2 or any(';'+want+';' not in d for d in diag):
                    raise ValueError('absolute native A2B1 required')
                result['pages']+=1
                for row,ex in zip(rows,expected):
                    nv,ns=zip(*(measure(im,tuple(v*dpi//72 for v in b)) for b in row['boxes']))
                    parity=max(error(nv[0],v) for v in nv[:3])
                    alpha=error(nv[3],[round((v*128+255*127)/255) for v in nv[2]])
                    result['rows'].append({'case':label,'raw':row['raw'],'lcms_rgb':ex,'native_rgb':nv,
                        'error':max(error(v,ex) for v in nv[:3]),'parity':parity,'alpha':alpha,'uniform':max(ns)==0})
    errors=sorted(r['error'] for r in result['rows'])
    result['max']=max(errors);result['mean']=statistics.mean(errors);result['p95']=errors[(95*len(errors)+99)//100-1]
    result['pass']=len(errors)==512 and result['pages']==16 and result['max']<=GATE_MAX and result['mean']<=GATE_MEAN and result['p95']<=GATE_P95 and all(r['parity']<=1 and r['alpha']<=1 and r['uniform'] for r in result['rows'])
    return result

def display_profile_gate(a,lib,work,directory):
    """Unmodified installed RGB profiles; native accuracy and explicit alternates.
    Standard working-space profiles, not measured monitor calibration.
    """
    import os
    import subprocess
    from PIL import Image
    from icc_render_benchmark import RGB_DOUBLE,RGB8
    samples=[(0,0,0),(255,255,255),(255,0,0),(0,255,0),(0,0,255),(128,128,128),(2,64,197),(17,127,239)]
    samples=list(dict.fromkeys(samples+list(itertools.product((17,113,239),repeat=3))))
    rng=random.Random(3403)
    while len(samples)<64:
        x=tuple(rng.randrange(256) for _ in range(3))
        if x not in samples:samples.append(x)
    def oracle(data,rows,intent):
        buf=C.create_string_buffer(data);src=lib.cmsOpenProfileFromMem(buf,len(data));dst=lib.cmsCreate_sRGBProfile();t=None
        try:
            if not src or not dst:raise ValueError('RGB reference open')
            t=lib.cmsCreateTransform(src,RGB_DOUBLE,dst,RGB8,intent,0)
            if not t:raise ValueError('RGB reference transform')
            result=[]
            for row in rows:
                out=(C.c_uint8*3)()
                lib.cmsDoTransform(t,(C.c_double*3)(*row['source']),out,1)
                result.append(list(out))
            return result
        finally:
            if t:lib.cmsDeleteTransform(t)
            if src:lib.cmsCloseProfile(src)
            if dst:lib.cmsCloseProfile(dst)
    report=[]
    for name in ('sRGB.icc','AdobeRGB1998.icc'):
        entry={'file':name,'status':'error','rows':[],'decisions':[]}
        report.append(entry)
        try:
            path=directory/name
            if not 132<=path.stat().st_size<=16*1024*1024:raise ValueError('RGB profile size')
            data=path.read_bytes()
            if S.unpack_from('>I',data)[0]!=len(data) or data[36:40]!=b'acsp' or data[16:20]!=b'RGB ':raise ValueError('RGB header')
            count=S.unpack_from('>I',data,128)[0]
            if count>1024 or 132+12*count>len(data):raise ValueError('RGB directory')
            tags={}
            for i in range(count):
                tag,off,n=S.unpack_from('>4sII',data,132+12*i)
                if off<132+12*count or n<8 or off+n>len(data):raise ValueError('RGB tag bounds')
                tags[tag.decode('ascii','replace')]=data[off:off+4].decode('ascii','replace')
            entry.update(sha256=hashlib.sha256(data).hexdigest(),version=data[8],profile_class=data[12:16].decode('ascii'),pcs=data[20:24].decode('ascii'),tag_types=tags)
            for intent,ri in ((1,'RelativeColorimetric'),(3,'AbsoluteColorimetric')):
                for bits in (8,16):
                    raw=samples if bits==8 else [tuple(v*257+(1 if 0<v<255 else 0) for v in x) for x in samples]
                    for offset in (0,32):
                        pdf=work/'display.pdf'
                        size,rows=make_pdf(pdf,data,3,raw[offset:offset+32],bits,intent=ri)
                        expected=oracle(data,rows,intent)
                        for dpi in (72,144):
                            label=f'{name}-{intent}-{bits}-{offset}-{dpi}'
                            png=work/(label+'.png')
                            q=subprocess.run([str(a.probe.resolve()),str(pdf),'render','1',str(png)],env=dict(os.environ,RPCORE_RENDER_DPI=str(dpi),RPCORE_ICC_ADAPTATION='1'),capture_output=True,text=True,timeout=30)
                            fields=dict(line.split('=',1) for line in q.stdout.splitlines() if '=' in line)
                            diag=[v for k,v in fields.items() if k.startswith('icc_diag_')]
                            if q.returncode or fields.get('status')!='ok' or len(diag)!=2 or not png.exists():raise ValueError('RGB render or diagnostic missing')
                            native=all(';rgb_matrix;' in d for d in diag)
                            alternate=all(';alternate;' in d and d.rsplit(';',1)[-1] for d in diag)
                            if not (native or alternate):raise ValueError('RGB inconsistent native/alternate decisions')
                            entry['decisions'].append({'case':label,'native':native,'diagnostics':diag})
                            with Image.open(png) as image:im=image.convert('RGB')
                            if im.size!=tuple(v*dpi//72 for v in size):raise ValueError('RGB raster size')
                            for row,ex in zip(rows,expected):
                                nv,ns=zip(*(measure(im,tuple(v*dpi//72 for v in b)) for b in row['boxes']))
                                delta=max(error(v,ex) for v in nv[:3])
                                parity=max(error(nv[0],v) for v in nv[:3])
                                alpha=error(nv[3],[round((v*128+255*127)/255) for v in nv[2]])
                                entry['rows'].append({'case':label,'native':native,'raw':row['raw'],'error':delta,'parity':parity,'alpha':alpha,'pass':parity<=1 and alpha<=1 and max(ns)==0 and (not native or delta<=3)})
            if len(entry['rows'])!=512 or len(entry['decisions'])!=16 or not all(r['pass'] for r in entry['rows']):
                entry['status']='accuracy_failed'
            elif all(r['native'] for r in entry['rows']):entry['status']='native_pass'
            else:entry['status']='explicit_alternate'
        except (OSError,ValueError,RuntimeError,subprocess.SubprocessError) as e:
            entry['reason']=str(e)[:300]
    return report

def main():
    p=argparse.ArgumentParser()
    for k in ('probe','reference','out','swop','fogra'):
        p.add_argument('--'+k,type=pathlib.Path,required=True)
    a=p.parse_args();a.probe=a.probe.resolve();a.reference=a.reference.resolve()
    a.out.mkdir(parents=True,exist_ok=True)
    lib=lcms()
    report={'schema':2,'profiles':[],'gate':{'max':GATE_MAX,'mean_max_channel':GATE_MEAN,'p95_max_channel':GATE_P95},
            'note':'LCMS relative colorimetric reference, no BPC. Require native A2B1 vector/image; RGB8 engineering thresholds, not Delta E.'}
    with tempfile.TemporaryDirectory(prefix='rpcore-print-') as tmp:
        work=pathlib.Path(tmp)
        for name,path in [('swop',a.swop),('fogra40',a.fogra)]:
            entry={'id':name,'file':path.name,'rows':[],'status':'error'}
            report['profiles'].append(entry)
            try:
                if not 132<=path.stat().st_size<=16*1024*1024:raise ValueError('profile size')
                data=path.read_bytes()
                entry.update(metadata(data))
                entry['renders']=[]
                for bits in (8,16):
                    samples=DENSE if bits==8 else [tuple(v*257+(1 if 0<v<255 else 0) for v in row) for row in DENSE]
                    # 32 rows/page keeps 144 DPI well within raster limits.
                    for offset in range(0,len(samples),32):
                        pdf=work/(name+'.pdf')
                        size,rows=make_pdf(pdf,data,4,samples[offset:offset+32],bits)
                        ex=reference(lib,data,rows)
                        for dpi in (72,144):
                            label=f'{name}-{bits}-{offset}-{dpi}'
                            n,r,im,ri=render_pair(a,pdf,1,dpi,work,label,None)
                            entry['renders'].append({'case':label,'native':n,'reference':r})
                            scale=dpi//72;expected_size=tuple(x*scale for x in size)
                            if im is None or ri is None or im.size!=expected_size or ri.size!=expected_size:
                                raise ValueError('render or size mismatch')
                            native=(n.get('icc_vector_native'),n.get('icc_image_native'))
                            alt=(n.get('icc_vector_alternate'),n.get('icc_image_alternate'))
                            diag=[v for k,v in sorted(n.items()) if k.startswith('icc_diag_')]
                            entry['diagnostics']=diag
                            if native!=('1','1') or alt!=('0','0') or len(diag)!=2 or any(';cmyk_A2B1;' not in d for d in diag):
                                entry['path']='alternate_or_wrong_intent'
                                raise ValueError('native A2B1 required in both paths')
                            entry['path']='native'
                            for row,want in zip(rows,ex):
                                boxes=[tuple(x*scale for x in box) for box in row['boxes']]
                                nv,ns=zip(*(measure(im,box) for box in boxes))
                                rv,rs=zip(*(measure(ri,box) for box in boxes))
                                parity=max(error(nv[0],nv[1]),error(nv[0],nv[2]))
                                alpha=error(nv[3],[round((v*128+255*127)/255) for v in nv[2]])
                                entry['rows'].append({'case':label,'bits':bits,'dpi':dpi,'raw':row['raw'],'lcms_rgb':want,'native_rgb':nv,'poppler_rgb':rv,
                                    'parity':parity,'alpha':alpha,'lcms_error':max(error(v,want) for v in nv[:3]),
                                    'poppler_error':max(error(x,y) for x,y in zip(nv,rv)),
                                    'pass':parity<=1 and alpha<=1 and max(ns)==0})
                errors=sorted(r['lcms_error'] for r in entry['rows'])
                metrics={'max':max(errors),'mean_max_channel':statistics.mean(errors),'p95_max_channel':errors[(95*len(errors)+99)//100-1]}
                entry['metrics']=metrics
                entry['absolute']=absolute_print_gate(a,lib,work,data,entry,name)
                entry['status']='compared' if entry['absolute']['pass'] and all(r['pass'] for r in entry['rows']) and metrics['max']<=GATE_MAX and metrics['mean_max_channel']<=GATE_MEAN and metrics['p95_max_channel']<=GATE_P95 else 'accuracy_failed'
            except (OSError,ValueError,RuntimeError) as e:
                entry['reason']=str(e)[:300]
        report['display_profiles']=display_profile_gate(a,lib,work,a.fogra.parent)
    (a.out/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    lines=['# Installed CMYK print profile compatibility','','| Profile | A2B0 / A2B1 | Path | Status | Samples | Max LCMS error | Mean max channel | P95 |','|---|---|---|---|---:|---:|---:|---:|']
    for e in report['profiles']:
        tags=e.get('tag_types',{})
        lines.append(f"| {e['id']} | {tags.get('A2B0','—')} / {tags.get('A2B1','—')} | {e.get('path','—')} | {e['status']} | {len(e['rows'])} | {max((r['lcms_error'] for r in e['rows']),default='—')} | {e.get('metrics',{}).get('mean_max_channel','—')} | {e.get('metrics',{}).get('p95_max_channel','—')} |")
        if 'absolute' in e:
            v=e['absolute'];lines.append(f"\n- {e['id']} absolute: {len(v['rows'])} checks; max {v['max']}; mean {v['mean']}; P95 {v['p95']}; media white {v['media_white_xyz']}; {'PASS' if v['pass'] else 'FAIL'}")
        for d in e.get('diagnostics',[]):lines.append(f"\n- {e['id']}: {d}")
        if e.get('reason'):lines.append(f"\n- {e['id']}: {e['reason']}")
    lines+=['',f"Native A2B1 mandatory. LCMS RGB8 max <= {GATE_MAX}, mean maximum channel <= {GATE_MEAN}, p95 <= {GATE_P95}; image/stroke parity <=1, alpha <=1 and uniform patches. Poppler diagnostic only. No ICC/PDF/images exported."]
    lines+=['','## Installed RGB working-space profiles (not measured monitors)']
    for e in report['display_profiles']:
        native=[r for r in e['rows'] if r['native']]
        lines.append(f"- {e['file']}: {e['status']}; {len(e['rows'])}/512 observations; native {len(native)}; max native RGB8 error {max((r['error'] for r in native),default='—')}; ICC v{e.get('version','?')}; SHA256 {e.get('sha256','—')}")
        if e.get('reason'):lines.append(f"  Reason: {e['reason']}")
    md='\n'.join(lines)+'\n'
    (a.out/'report.md').write_text(md);print(md)
    # Freeze both verified external RGB profiles as mandatory native paths.
    # An explicit alternate remains useful in diagnostics but is a regression
    # for these two fixtures; do not silently lower the closure gate.
    rgb=report['display_profiles']
    rgb_ok=({e['file'] for e in rgb}=={'sRGB.icc','AdobeRGB1998.icc'} and len(rgb)==2
            and all(e['status']=='native_pass' and len(e['rows'])==512
                    and len(e['decisions'])==16 and all(r['native'] and r['pass'] for r in e['rows'])
                    and all(d['native'] for d in e['decisions']) for e in rgb))
    print(f"ICC closure external RGB native gate: {'PASS' if rgb_ok else 'FAIL'}; {sum(r['native'] and r['pass'] for e in rgb for r in e['rows'])}/1024.")
    return int(not rgb_ok or any(e['status']!='compared' or len(e['rows'])!=len(DENSE)*4 or len(e.get('renders',[]))!=16 for e in report['profiles']))

if __name__=='__main__':
    raise SystemExit(main())

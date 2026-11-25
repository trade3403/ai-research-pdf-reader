#!/usr/bin/env python3
"""Installed print profiles, numeric-only compatibility baseline. No profile assets exported."""
import argparse
import ctypes as C
import hashlib
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

def reference(lib,data,rows):
    buf=C.create_string_buffer(data)
    src=lib.cmsOpenProfileFromMem(buf,len(data));dst=lib.cmsCreate_sRGBProfile()
    t=None
    try:
        if not src or not dst:raise RuntimeError('reference profile open')
        t=lib.cmsCreateTransform(src,CMYK_DOUBLE,dst,RGB8,1,0)
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
    tags={}
    for i in range(n):
        key,off,size=S.unpack_from('>4sII',data,132+12*i)
        if off+size>len(data) or size<8:raise ValueError('ICC bounds')
        tags[key.decode('ascii',errors='replace')]=data[off:off+4].decode('ascii',errors='replace')
    return {'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'version':data[8],
            'class':data[12:16].decode('ascii'),'pcs':data[20:24].decode('ascii'),'tag_types':tags}

def main():
    p=argparse.ArgumentParser()
    for k in ('probe','reference','out','swop','fogra'):
        p.add_argument('--'+k,type=pathlib.Path,required=True)
    a=p.parse_args();a.probe=a.probe.resolve();a.reference=a.reference.resolve()
    a.out.mkdir(parents=True,exist_ok=True)
    lib=lcms()
    report={'schema':1,'profiles':[],'note':'Diagnostic LCMS relative colorimetric reference, no BPC; no numerical accuracy gate on real profiles. Native path must be identified explicitly.'}
    with tempfile.TemporaryDirectory(prefix='rpcore-print-') as tmp:
        work=pathlib.Path(tmp)
        for name,path in [('swop',a.swop),('fogra40',a.fogra)]:
            entry={'id':name,'file':path.name,'rows':[],'status':'error'}
            report['profiles'].append(entry)
            try:
                if not 132<=path.stat().st_size<=16*1024*1024:raise ValueError('profile size')
                data=path.read_bytes()
                entry.update(metadata(data))
                pdf=work/(name+'.pdf')
                size,rows=make_pdf(pdf,data,4,SAMPLES,8)
                ex=reference(lib,data,rows)
                n,r,im,ri=render_pair(a,pdf,1,72,work,name,None)
                entry.update(native=n,reference=r)
                if im is None or ri is None or im.size!=size or ri.size!=size:
                    raise ValueError('render or size mismatch')
                native=(n.get('icc_vector_native'),n.get('icc_image_native'))
                alt=(n.get('icc_vector_alternate'),n.get('icc_image_alternate'))
                if native==('1','1') and alt==('0','0'):
                    entry['path']='native'
                elif native==('0','0') and alt==('1','1'):
                    entry['path']='alternate'
                else:raise ValueError('inconsistent ICC path diagnostics')
                entry['diagnostics']=[v for k,v in sorted(n.items()) if k.startswith('icc_diag_')]
                if len(entry['diagnostics'])!=2:raise ValueError('missing profile diagnostics')
                for row,want in zip(rows,ex):
                    nv,ns=zip(*(measure(im,tuple(box)) for box in row['boxes']))
                    rv,rs=zip(*(measure(ri,tuple(box)) for box in row['boxes']))
                    parity=max(error(nv[0],nv[1]),error(nv[0],nv[2]))
                    alpha=error(nv[3],[round((v*128+255*127)/255) for v in nv[2]])
                    entry['rows'].append({'raw':row['raw'],'lcms_rgb':want,'native_rgb':nv,'poppler_rgb':rv,
                        'parity':parity,'alpha':alpha,'lcms_error':max(error(v,want) for v in nv[:3]),
                        'poppler_error':max(error(x,y) for x,y in zip(nv,rv)),
                        'pass':parity<=1 and alpha<=1 and max(ns)==0})
                entry['status']='compared' if all(x['pass'] for x in entry['rows']) else 'parity_failed'
            except (OSError,ValueError,RuntimeError) as e:
                entry['reason']=str(e)[:300]
    (a.out/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    lines=['# Installed CMYK print profile compatibility','','| Profile | A2B0 / A2B1 | Path | Status | Samples | Max LCMS error |','|---|---|---|---|---:|---:|']
    for e in report['profiles']:
        tags=e.get('tag_types',{})
        lines.append(f"| {e['id']} | {tags.get('A2B0','—')} / {tags.get('A2B1','—')} | {e.get('path','—')} | {e['status']} | {len(e['rows'])} | {max((r['lcms_error'] for r in e['rows']),default='—')} |")
        for d in e.get('diagnostics',[]):lines.append(f"\n- {e['id']}: {d}")
        if e.get('reason'):lines.append(f"\n- {e['id']}: {e['reason']}")
    lines+=['','Alternate is not native compatibility. Color difference is diagnostic, not a printer accuracy score. No ICC files, embedded PDF or image artifacts exported.']
    md='\n'.join(lines)+'\n'
    (a.out/'report.md').write_text(md);print(md)
    return int(any(e['status']!='compared' or len(e['rows'])!=len(SAMPLES) for e in report['profiles']))

if __name__=='__main__':
    raise SystemExit(main())

#!/usr/bin/env python3
"""End-to-end ICC image/vector parity and test-only color-engine reference."""
import argparse
import ctypes as C
import hashlib
import itertools
import json
import pathlib
import struct as S
import subprocess
import tempfile
from PIL import Image, ImageChops
from chart_benchmark import render_pair
from color_benchmark import measure
from icc_reference import fixture as cmyk_fixture, reference as cmyk_reference, mab_fixture
from icc_mpe_reference import fixture as mpe_fixture, reference as mpe_reference

def lcms():
    lib=C.CDLL('liblcms2.so.2')
    P=C.c_void_p
    U=C.c_uint32
    for name,ret,args in [
        ('cmsCreate_sRGBProfile',P,[]),
        ('cmsCreateXYZProfile',P,[]),
        ('cmsCreateRGBProfile',P,[P,P,P]),
        ('cmsCreateGrayProfile',P,[P,P]),
        ('cmsBuildGamma',P,[P,C.c_double]),
        ('cmsFreeToneCurve',None,[P]),
        ('cmsReadTag',P,[P,U]),
        ('cmsSaveProfileToMem',C.c_int,[P,P,C.POINTER(U)]),
        ('cmsOpenProfileFromMem',P,[P,U]),
        ('cmsCloseProfile',C.c_int,[P]),
        ('cmsCreateTransform',P,[P,U,P,U,U,U]),
        ('cmsDoTransform',None,[P,P,P,U]),
        ('cmsDeleteTransform',None,[P]),
    ]:
        f=getattr(lib,name)
        f.restype=ret
        f.argtypes=args
    return lib

# LCMS pixel format macros: FLOAT_SH(1), COLORSPACE_SH, CHANNELS_SH, BYTES_SH.
RGB8=(4<<16)|(3<<3)|1
RGB_DOUBLE=(1<<22)|(4<<16)|(3<<3)
XYZ_DOUBLE=(1<<22)|(9<<16)|(3<<3)

def serialize_profile(lib,h):
    if not h:
        raise RuntimeError('RGB profile creation')
    try:
        n=C.c_uint32()
        if not lib.cmsSaveProfileToMem(h,None,C.byref(n)) or not 132<=n.value<=1024*1024:
            raise RuntimeError('profile size')
        b=C.create_string_buffer(n.value)
        if not lib.cmsSaveProfileToMem(h,b,C.byref(n)):
            raise RuntimeError('profile serialize')
        src=b.raw[:n.value]
    finally:
        lib.cmsCloseProfile(h)
    return src

def wide_rgb_profile(lib):
    # Test-only standard D65 wide-gamut primaries with the unmodified LCMS
    # sRGB transfer curve. This is a generated non-sRGB matrix profile,
    # not a measured monitor or commercial Adobe RGB profile.
    h=lib.cmsCreate_sRGBProfile()
    if not h:
        raise RuntimeError('sRGB creation')
    try:
        trc=lib.cmsReadTag(h,int.from_bytes(b'rTRC','big'))
        if not trc:
            raise RuntimeError('missing sRGB TRC')
        wp=(C.c_double*3)(.3127,.3290,1.)
        primaries=(C.c_double*9)(.64,.33,1.,.21,.71,1.,.15,.06,1.)
        curves=(C.c_void_p*3)(trc,trc,trc)
        return serialize_profile(lib,lib.cmsCreateRGBProfile(wp,primaries,curves))
    finally:
        lib.cmsCloseProfile(h)

def standard_rgb_profile(lib):
    # Preserve the frozen sampled baseline, derived from the LCMS profile.
    src=serialize_profile(lib,lib.cmsCreate_sRGBProfile())
    tags=[]
    for i in range(S.unpack_from('>I',src,128)[0]):
        name,off,size=S.unpack_from('>4sII',src,132+12*i)
        tag=src[off:off+size]
        if name in (b'rTRC',b'gTRC',b'bTRC'):
            def linear(v):
                return v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4
            tag=b'curv'+bytes(4)+S.pack('>I',4096)+b''.join(S.pack('>H',round(65535*linear(j/4095))) for j in range(4096))
        tags.append((name,tag))
    out=bytearray(src[:128])+bytearray(4+12*len(tags))
    S.pack_into('>I',out,128,len(tags))
    # Modified content invalidates the original profile ID.
    out[84:100]=bytes(16)
    for i,(name,tag) in enumerate(tags):
        S.pack_into('>4sII',out,132+12*i,name,len(out),len(tag))
        out+=tag
        out+=bytes(-len(out)%4)
    S.pack_into('>I',out,0,len(out))
    return bytes(out)

def gray_profile(lib):
    curve=lib.cmsBuildGamma(None,2.2)
    if not curve:raise RuntimeError('gray gamma creation')
    try:
        wp=(C.c_double*3)(.3457,.3585,1.)
        return serialize_profile(lib,lib.cmsCreateGrayProfile(wp,curve))
    finally:
        lib.cmsFreeToneCurve(curve)

def oracle_gray(lib,profile,samples):
    buf=C.create_string_buffer(profile)
    src=lib.cmsOpenProfileFromMem(buf,len(profile));dst=lib.cmsCreate_sRGBProfile()
    t=None
    try:
        if not src or not dst:raise RuntimeError('gray profile open')
        t=lib.cmsCreateTransform(src,(1<<22)|(3<<16)|(1<<3),dst,RGB8,1,0)
        if not t:raise RuntimeError('gray transform')
        out=[]
        for x in samples:
            v=(C.c_double*1)(*x);rgb=(C.c_uint8*3)()
            lib.cmsDoTransform(t,v,rgb,1);out.append(list(rgb))
        return out
    finally:
        if t:lib.cmsDeleteTransform(t)
        if src:lib.cmsCloseProfile(src)
        if dst:lib.cmsCloseProfile(dst)

def oracle_rgb(lib,profile,samples,xyz=False):
    srcbuf=C.create_string_buffer(profile) if not xyz else None
    src=lib.cmsCreateXYZProfile() if xyz else lib.cmsOpenProfileFromMem(srcbuf,len(profile))
    dst=lib.cmsCreate_sRGBProfile()
    t=None
    try:
        if not src or not dst:
            raise RuntimeError('oracle profile open')
        # Relative colorimetric, no BPC. For CMYK, raw A2B0 has already
        # been evaluated independently; this second step is PCS -> display.
        t=lib.cmsCreateTransform(src,XYZ_DOUBLE if xyz else RGB_DOUBLE,dst,RGB8,1,0)
        if not t:
            raise RuntimeError('oracle transform')
        return [convert(lib,t,x) for x in samples]
    finally:
        if t:lib.cmsDeleteTransform(t)
        if src:lib.cmsCloseProfile(src)
        if dst:lib.cmsCloseProfile(dst)

def convert(lib,t,x):
    a=(C.c_double*3)(*x)
    b=(C.c_uint8*3)()
    lib.cmsDoTransform(t,a,b,1)
    return list(b)

def stream(dictionary,data):
    return f'<< {dictionary} /Length {len(data)} >>\nstream\n'.encode()+data+b'\nendstream'

def make_pdf(path,profile,n,samples,bits):
    # Each row contains vector fill, wide stroke, image, and image on white
    # with a separate 128/255 soft mask. Interior-only sampling avoids edges.
    w,h=192,48*len(samples)
    objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'',b'',
             stream(f'/N {n}',profile),
             stream('/Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8',b'\x80')]
    cmds=[];names=[];rows=[]
    maxv=(1<<bits)-1
    for i,raw in enumerate(samples):
        y=h-(i+1)*48+4
        values=' '.join(f'{v/maxv:.12f}' for v in raw)
        data=S.pack('>'+('B' if bits==8 else 'H')*n,*raw)
        ids=[]
        for alpha in ('',' /SMask 6 0 R'):
            oid=len(objects)+1
            name=f'I{oid}'
            names.append(f'/{name} {oid} 0 R')
            ids.append(name)
            objects.append(stream(f'/Subtype /Image /Width 1 /Height 1 /ColorSpace [/ICCBased 5 0 R] /BitsPerComponent {bits}'+alpha,data))
        cmds += [f'/C cs {values} sc 4 {y} 40 40 re f',
                 f'/C CS {values} SC 40 w 68 {y} m 68 {y+40} l S',
                 f'q 40 0 0 40 100 {y} cm /{ids[0]} Do Q',
                 f'q 40 0 0 40 148 {y} cm /{ids[1]} Do Q']
        rows.append({'raw':list(raw),'source':[v/maxv for v in raw],
                     'boxes':[[x,h-y-28,x+16,h-y-12] for x in (16,60,112,160)]})
    objects[2]=f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents 4 0 R /Resources << /ColorSpace << /C [/ICCBased 5 0 R] >> /XObject << {" ".join(names)} >> >> >>'.encode()
    objects[3]=stream('',('\n'.join(cmds)+'\n').encode())
    buf=bytearray(b'%PDF-1.4\n');offs=[]
    for i,o in enumerate(objects,1):
        offs.append(len(buf));buf+=f'{i} 0 obj\n'.encode()+o+b'\nendobj\n'
    x=len(buf);buf+=f'xref\n0 {len(objects)+1}\n0000000000 65535 f \n'.encode()
    for off in offs:buf+=f'{off:010d} 00000 n \n'.encode()
    buf+=f'trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n'.encode()
    path.write_bytes(buf)
    return (w,h),rows

def error(a,b):
    return max(abs(x-y) for x,y in zip(a,b))

def main():
    p=argparse.ArgumentParser()
    for k in ('probe','reference','out'):
        p.add_argument('--'+k,required=True,type=pathlib.Path)
    p.add_argument('--icc-probe',required=True,type=pathlib.Path)
    a=p.parse_args()
    a.probe=a.probe.resolve();a.reference=a.reference.resolve();a.out.mkdir(parents=True,exist_ok=True)
    lib=lcms()
    rgb=standard_rgb_profile(lib)
    cmyk=cmyk_fixture(2,2)
    cases=[('srgb',rgb,3,[(0,0,0),(255,255,255),(255,0,0),(0,255,0),(0,0,255),(17,127,239),(128,128,128),(2,64,197)]),
           ('cmyk-affine',cmyk,4,[(0,0,0,0),(255,0,0,0),(0,255,0,0),(0,0,255,0),(0,0,0,255),(64,128,192,32),(128,128,128,128),(255,255,255,255)])]
    raw=serialize_profile(lib,lib.cmsCreate_sRGBProfile())
    wide=wide_rgb_profile(lib)
    cases.extend([('srgb-native',raw,3,cases[0][3]),('wide-native',wide,3,cases[0][3]),('mab-full',mab_fixture(2,True),4,cases[1][3]),('gray-native',gray_profile(lib),1,[(v,) for v in (0,2,17,64,128,197,239,255)])])
    cases.extend([('mpe-'+mode,mpe_fixture(mode),4,cases[1][3]) for mode in ('matrix','clut','mixed')])
    report={'schema':2,'profiles':{},'renders':[],'rows':[],'native_matrix_checks':[],
            'gate':{'native_image_vector_max':1,'native_lcms_max':2,'alpha_max':1,'native_spread':0},
            'note':'Sampled and unmodified LCMS sRGB, generated wide RGB and synthetic affine CMYK; Poppler diagnostic only.'}
    with tempfile.TemporaryDirectory(prefix='rpcore-icc-render-') as tmp:
        work=pathlib.Path(tmp)
        for name,profile,n,samples8 in cases:
            report['profiles'][name]=hashlib.sha256(profile).hexdigest()
            if n==3:
                # Direct matrix verification prevents a DeviceRGB fallback from
                # appearing to pass the standard sRGB identity transform.
                icc_path=work/(name+'.icc');icc_path.write_bytes(profile)
                q=subprocess.run([str(a.icc_probe.resolve()),str(icc_path),'rgb'],capture_output=True,text=True,timeout=15)
                entry={'profile':name,'pass':False,'status':'native_rejected'}
                if q.returncode==0:
                    xyz=[list(map(float,line.split(','))) for line in q.stdout.splitlines()]
                    samples=[[0.,0.,0.],[1.,1.,1.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.],[.02,.5,.8],[.5,.5,.5],[.8,.3,.1]]
                    import math
                    if len(xyz)==8 and all(len(v)==3 and all(map(math.isfinite,v)) for v in xyz):
                        actual=oracle_rgb(lib,profile,xyz,xyz=True)
                        expected=oracle_rgb(lib,profile,samples)
                        delta=max(error(x,y) for x,y in zip(actual,expected))
                        entry.update(status='compared',max_rgb_error=delta,**{'pass':delta<=2})
                report['native_matrix_checks'].append(entry)
            for bits in (8,16):
                # 16-bit non-replicated intermediate values exercise precision.
                samples=samples8 if bits==8 else [tuple(v*257+(1 if 0<v<255 else 0) for v in x) for x in samples8]
                stem=f'{name}-{bits}'
                pdf=work/(stem+'.pdf')
                size,rows=make_pdf(pdf,profile,n,samples,bits)
                src=[r['source'] for r in rows]
                if name.startswith('mpe-'):
                    expected=oracle_rgb(lib,profile,list(mpe_reference(profile,src)),xyz=True)
                else:
                    expected=oracle_gray(lib,profile,src) if n==1 else oracle_rgb(lib,profile,src) if n==3 else oracle_rgb(lib,profile,[v[6:9] for v in cmyk_reference(profile,1 if name.startswith('mab-') else 2,src)],xyz=True)
                for dpi in (72,144):
                    label=f'{stem}-{dpi}'
                    native,ref,im,ri=render_pair(a,pdf,1,dpi,work,label,None)
                    scale=dpi//72
                    dimensions=tuple(v*scale for v in size)
                    ok=im is not None and ri is not None and im.size==ri.size==dimensions
                    if name.startswith(('mab-','mpe-')) or n==1:
                        want='mpe_D2B1' if name.startswith('mpe-') else 'mab_A2B0' if n==4 else 'gray_trc'
                        diag=[v for k,v in native.items() if k.startswith('icc_diag_')]
                        ok=ok and native.get('icc_vector_native')=='1' and native.get('icc_image_native')=='1' and native.get('icc_vector_alternate')=='0' and native.get('icc_image_alternate')=='0' and len(diag)==2 and all(';'+want+';' in x for x in diag)
                    report['renders'].append({'case':label,'native':native,'reference':ref,'compared':ok})
                    if not ok:continue
                    panel=Image.new('RGB',(im.width*3,im.height),'white')
                    panel.paste(im,(0,0));panel.paste(ri,(im.width,0))
                    panel.paste(ImageChops.difference(im,ri).point(lambda v:min(255,v*4)),(im.width*2,0))
                    panel.save(a.out/(label+'-native-reference-diff.png'))
                    for row,ex in zip(rows,expected):
                        boxes=[tuple(v*scale for v in box) for box in row['boxes']]
                        nv,ns=zip(*(measure(im,box) for box in boxes))
                        rv,rs=zip(*(measure(ri,box) for box in boxes))
                        # Renderer quantizes the opaque source before compositing.
                        # Check the soft mask against the actual opaque source.
                        alpha=[round((v*128+255*127)/255) for v in nv[2]]
                        parity=max(error(nv[0],nv[1]),error(nv[0],nv[2]))
                        oracle=max(error(v,ex) for v in nv[:3])
                        ae=error(nv[3],alpha)
                        report['rows'].append({'case':label,'raw':row['raw'],'lcms_rgb':ex,
                            'native_rgb':nv,'poppler_rgb':rv,'native_spread':ns,'poppler_spread':rs,
                            'parity_error':parity,'lcms_error':oracle,'alpha_error':ae,
                            'poppler_error':max(error(x,y) for x,y in zip(nv,rv)),
                            'pass':parity<=1 and oracle<=2 and ae<=1 and max(ns)==0})
    total=288
    passed=sum(r['pass'] for r in report['rows'])
    report['expected_rows']=total
    report['passed']=passed
    (a.out/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    lines=['# Embedded ICC end-to-end rendering gate','',
           'Columns in images: RPCore, Poppler, amplified absolute difference. Swatches: fill, stroke, image, soft-masked image. Interior sample checks against LCMS; Poppler differences diagnostic only.','',
           '| Case | Rows | Pass | Parity max | LCMS max | Alpha max | Poppler max |',
           '|---|---:|---:|---:|---:|---:|---:|']
    for render in report['renders']:
        key=render['case'];rr=[r for r in report['rows'] if r['case']==key]
        mx=[max((r[k] for r in rr),default='—') for k in ('parity_error','lcms_error','alpha_error','poppler_error')]
        lines.append(f"| {key} | {len(rr)} | {sum(r['pass'] for r in rr)} | "+' | '.join(map(str,mx))+' |')
    lines+=['',f'Passed {passed}/{total} paired color observations; {len(report["renders"])} page comparisons.',
            '', 'Native RGB matrix checks:']
    for check in report['native_matrix_checks']:
        lines.append(f"- {check['profile']}: {check['status']}, max RGB error {check.get('max_rgb_error','—')}, {'PASS' if check['pass'] else 'FAIL'}")
    lines+=['','Unmodified LCMS sRGB and generated wide-gamut matrix profile, sampled baseline and original synthetic CMYK LUT. Includes three synthetic MPE D2B1 profiles (matrix, CLUT, mixed). No measured printer or monitor calibration claim.']
    md='\n'.join(lines)+'\n';(a.out/'report.md').write_text(md);print(md)
    return int(len(report['renders'])!=36 or len(report['rows'])!=total or passed!=total or len(report['native_matrix_checks'])!=3 or not all(x['pass'] for x in report['native_matrix_checks']))

if __name__=='__main__':
    raise SystemExit(main())

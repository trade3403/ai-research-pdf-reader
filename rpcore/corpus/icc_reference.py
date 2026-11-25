#!/usr/bin/env python3
"""Test-only LCMS raw A2B0 pipeline and PCS comparison on original affine fixtures."""
import argparse
import ctypes as C
import itertools
import json
import math
import pathlib
import struct as S
import subprocess
import tempfile

def fixture(word, version):
    n=256
    pack=lambda x: S.pack('>B' if word==1 else '>H',round(x*((1<<(word*8))-1)))
    tag=bytearray(b'mft1' if word==1 else b'mft2')+bytearray(44)
    tag[8:11]=bytes([4,3,2])
    for i in (0,4,8):
        S.pack_into('>i',tag,12+4*i,65536)
    if word==2:
        tag+=S.pack('>HH',n,n)
    tag+=b''.join(pack(i/(n-1)) for _ in range(4) for i in range(n))
    # Affine CLUT avoids confounding multilinear vs tetrahedral interpolation.
    for c,m,y,k in itertools.product(range(2),repeat=4):
        v=(80-20*c-10*k,-20+40*m,-30+50*y)
        raw=(round(v[0]*255/100),v[1]+128,v[2]+128) if word==1 else (round(v[0]*652.8),(v[1]+128)*256,(v[2]+128)*256)
        tag+=S.pack('>BBB' if word==1 else '>HHH',*raw)
    tag+=b''.join(pack(i/(n-1)) for _ in range(3) for i in range(n))
    wp=b'XYZ '+bytes(4)+S.pack('>iii',round(.9642*65536),65536,round(.8249*65536))
    b=bytearray(156)
    b[8]=version
    b[12:24]=b'prtrCMYKLab '
    b[36:40]=b'acsp'
    b[68:80]=wp[8:]
    S.pack_into('>I',b,128,2)
    for i,(name,data) in enumerate(((b'A2B0',tag),(b'wtpt',wp))):
        S.pack_into('>4sII',b,132+i*12,name,len(b),len(data))
        b+=data
        b+=bytes(-len(b)%4)
    S.pack_into('>I',b,0,len(b))
    return bytes(b)


def mab_fixture(word,full):
    # Nonuniform grid; affine code-space CLUT (integer increments per
    # grid axis), distinct A/B/M curves and nonidentity matrix with offset.
    grid=(2,3,2,2)
    identity=b'curv'+bytes(8)
    gamma=lambda g:b'para'+bytes(8)+S.pack('>i',round(g*65536))
    a=gamma(2)+identity+identity+identity
    b=identity+gamma(2)+identity
    m=identity+identity+gamma(2)
    mat=S.pack('>12i',*(round(x*65536) for x in (.75,0,0,0,.75,0,0,0,.5,.1,.05,.2)))
    clut=bytes(grid)+bytes(12)+bytes([word,0,0,0])
    for c,m0,y,k in itertools.product(*(range(g) for g in grid)):
        codes=(180-40*c-20*k,100+20*m0,105+40*y)
        clut+=S.pack('>BBB' if word==1 else '>HHH',*(v if word==1 else v*257 for v in codes))
    tag=bytearray(b'mAB '+bytes(4)+bytes([4,3,0,0])+bytes(20))
    for pos,payload in [(12,b),(16,mat if full else b''),(20,m if full else b''),(24,clut),(28,a)]:
        if not payload:continue
        S.pack_into('>I',tag,pos,len(tag))
        tag+=payload
        tag+=bytes(-len(tag)%4)
    wp=b'XYZ '+bytes(4)+S.pack('>iii',round(.9642*65536),65536,round(.8249*65536))
    p=bytearray(156);p[8]=4;p[12:24]=b'prtrCMYKLab ';p[36:40]=b'acsp';p[68:80]=wp[8:]
    S.pack_into('>I',p,128,2)
    for i,(name,data) in enumerate(((b'A2B0',tag),(b'wtpt',wp))):
        S.pack_into('>4sII',p,132+i*12,name,len(p),len(data));p+=data;p+=bytes(-len(p)%4)
    S.pack_into('>I',p,0,len(p))
    return bytes(p)

def reference(data,word,samples):
    lib=C.CDLL('liblcms2.so.2')
    P=C.c_void_p
    U=C.c_uint32
    for name,ret,args in [
        ('cmsOpenProfileFromMem',P,[P,U]),
        ('cmsReadTag',P,[P,U]),
        ('cmsPipelineEvalFloat',None,[P,P,P]),
        ('cmsLabEncoded2Float',None,[P,P]),
        ('cmsLabEncoded2FloatV2',None,[P,P]),
        ('cmsLab2XYZ',None,[P,P,P]),
        ('cmsCloseProfile',C.c_int,[P]),
    ]:
        f=getattr(lib,name)
        f.restype=ret
        f.argtypes=args
    buf=C.create_string_buffer(data)
    h=lib.cmsOpenProfileFromMem(buf,len(data))
    if not h:
        raise RuntimeError('LCMS rejected fixture profile')
    try:
        pipeline=lib.cmsReadTag(h,int.from_bytes(b'A2B0','big'))
        if not pipeline:
            raise RuntimeError('LCMS rejected fixture A2B0')
        for x in samples:
            a=(C.c_float*4)(*x)
            o=(C.c_float*3)()
            lib.cmsPipelineEvalFloat(a,o,pipeline)
            # LCMS decoder consumes integer codes, so allow half-code rounding.
            code=(C.c_uint16*3)(*(round(max(0,min(1,v))*65535) for v in o))
            lab=(C.c_double*3)()
            (lib.cmsLabEncoded2Float if word==1 else lib.cmsLabEncoded2FloatV2)(lab,code)
            xyz=(C.c_double*3)()
            wp=(C.c_double*3)(.9642,1.,.8249)
            lib.cmsLab2XYZ(wp,xyz,lab)
            yield list(o)+list(lab)+list(xyz)
    finally:
        lib.cmsCloseProfile(h)

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--probe',required=True,type=pathlib.Path)
    p.add_argument('--out',required=True,type=pathlib.Path)
    a=p.parse_args()
    a.out.mkdir(parents=True,exist_ok=True)
    samples=list(itertools.product((0.,.25,1.),repeat=4))
    # Per-channel bounds, not a perceptual Delta E or real-printer accuracy claim.
    thresholds=[.0001,.02,.0005]
    results=[]
    with tempfile.TemporaryDirectory(prefix='rpcore-icc-') as tmp:
        for word,version in itertools.product((1,2),(2,4)):
            data=fixture(word,version)
            path=pathlib.Path(tmp)/'fixture.icc'
            path.write_bytes(data)
            raw=subprocess.run([str(a.probe.resolve()),str(path)],check=True,capture_output=True,text=True,timeout=20).stdout
            actual=[list(map(float,line.split(','))) for line in raw.splitlines()]
            expected=list(reference(data,word,samples))
            if len(actual)!=len(samples) or any(len(v)!=9 or not all(map(math.isfinite,v)) for v in actual+expected):
                raise RuntimeError('invalid probe/reference output')
            maxima=[max(abs(x[i]-y[i]) for x,y in zip(actual,expected) for i in range(j,j+3)) for j in (0,3,6)]
            rows=[{'cmyk':x,'native':n,'reference':r} for x,n,r in zip(samples,actual,expected)]
            results.append({'bits':word*8,'version':version,'samples':len(samples),'max_errors':maxima,'pass':all(v<=t for v,t in zip(maxima,thresholds)),'rows':rows})
        for word,full in itertools.product((1,2),(False,True)):
            data=mab_fixture(word,full)
            path=pathlib.Path(tmp)/'mab.icc';path.write_bytes(data)
            raw=subprocess.run([str(a.probe.resolve()),str(path),'mab'],check=True,capture_output=True,text=True,timeout=20).stdout
            actual=[list(map(float,line.split(','))) for line in raw.splitlines()]
            # mAB uses full-range v4 Lab irrespective of CLUT word size.
            expected=list(reference(data,1,samples))
            if len(actual)!=len(samples) or any(len(v)!=9 or not all(map(math.isfinite,v)) for v in actual+expected):
                raise RuntimeError('invalid mAB probe/reference output')
            maxima=[max(abs(x[i]-y[i]) for x,y in zip(actual,expected) for i in range(j,j+3)) for j in (0,3,6)]
            gate=[.0002,.05,.001]
            results.append({'bits':word*8,'version':4,'type':'mAB-full' if full else 'mAB-basic','samples':len(samples),
                'max_errors':maxima,'thresholds':gate,'pass':all(v<=t for v,t in zip(maxima,gate)),
                'rows':[{'cmyk':x,'native':n,'reference':r} for x,n,r in zip(samples,actual,expected)]})
    report={'schema':1,'oracle':'LCMS raw A2B0 and PCS decoding, affine synthetic fixtures only','thresholds_encoded_lab_xyz':thresholds,'cases':results}
    (a.out/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    lines=['# ICC independent differential test','','| Type | LUT bits | ICC major | Samples | Encoded max | Lab max | XYZ max | Gate |','|---|---:|---:|---:|---:|---:|---:|---|']
    for r in results:
        errors=' | '.join(f'{v:.8f}' for v in r['max_errors'])
        lines.append(f"| {r.get('type','legacy')} | {r['bits']} | {r['version']} | {r['samples']} | {errors} | {'PASS' if r['pass'] else 'FAIL'} |")
    lines+=['','Synthetic affine fixtures only. No full color management, rendering-intent or real printer profile accuracy claim.']
    md='\n'.join(lines)+'\n'
    (a.out/'report.md').write_text(md)
    print(md)
    return 0 if all(r['pass'] for r in results) else 1

if __name__=='__main__':
    raise SystemExit(main())

#!/usr/bin/env python3
"""Raw ICC MPE floating pipeline comparison (LCMS test only, no PCS conversion)."""
import argparse
import ctypes as C
import itertools
import json
import math
import pathlib
import struct as S
import subprocess
import tempfile

def head(k,p,q):
    return k+bytes(4)+S.pack('>HH',p,q)
def floats(*v):
    return S.pack('>'+str(len(v))+'f',*v)
def directory(prefix,items):
    b=bytearray(prefix)+bytearray(8*len(items))
    for i,item in enumerate(items):
        S.pack_into('>II',b,len(prefix)+8*i,len(b),len(item))
        b+=item;b+=bytes(-len(b)%4)
    return bytes(b)
def formula(kind,*params):
    return b'parf'+bytes(4)+S.pack('>HH',kind,0)+floats(*params)
def curve():
    # Identity lower tail, sampled non-linear interior, linear upper tail.
    return b'curf'+bytes(4)+S.pack('>HH',3,0)+floats(0.,1.)+formula(0,1,1,0,0)+b'samf'+bytes(4)+S.pack('>I',2)+floats(.25,1.)+formula(0,1,1,0,0)
def matrix(p,q,coeff,offset):
    return head(b'matf',p,q)+floats(*coeff,*offset)
def fixture(mode):
    # Affine CLUT avoids different valid interpolation algorithms confounding
    # the reference. Nonuniform axes and distinct matrix rows test ordering.
    m=matrix(4,3,[.5,0,.25,0, 0,.5,0,.25, .25,0,0,.5],[.05,.1,.15])
    grid=(2,3,2,2)
    clut=head(b'clut',4,3)+bytes(grid)+bytes(12)
    for c,m0,y,k in itertools.product(*(range(g) for g in grid)):
        clut+=floats(.1+.2*c+.15*k,.1+.2*m0+.1*y,.2+.3*y+.1*k)
    curves=directory(head(b'cvst',4,4),[curve()]*4)
    tail=matrix(3,3,[1.,.25,0.,0.,1.,0.,0.,0.,2.],[-.5,0.,.1])
    stages={'matrix':[m],'clut':[clut],'mixed':[curves,clut,tail]}[mode]
    tag=directory(head(b'mpet',4,3)+S.pack('>I',len(stages)),stages)
    wp=b'XYZ '+bytes(4)+S.pack('>iii',round(.9642*65536),65536,round(.8249*65536))
    b=bytearray(156);b[8:10]=bytes([4,0x30]);b[12:24]=b'prtrCMYKXYZ ';b[36:40]=b'acsp';b[68:80]=wp[8:]
    S.pack_into('>I',b,128,2)
    for i,(name,payload) in enumerate(((b'D2B1',tag),(b'wtpt',wp))):
        S.pack_into('>4sII',b,132+12*i,name,len(b),len(payload))
        b+=payload;b+=bytes(-len(b)%4)
    S.pack_into('>I',b,0,len(b))
    return bytes(b)
def reference(data,samples):
    lib=C.CDLL('liblcms2.so.2');P=C.c_void_p;U=C.c_uint32
    for name,ret,args in [('cmsOpenProfileFromMem',P,[P,U]),('cmsReadTag',P,[P,U]),('cmsPipelineEvalFloat',None,[P,P,P]),('cmsCloseProfile',C.c_int,[P])]:
        f=getattr(lib,name);f.restype=ret;f.argtypes=args
    buf=C.create_string_buffer(data);h=lib.cmsOpenProfileFromMem(buf,len(data))
    if not h:raise RuntimeError('LCMS profile open')
    try:
        pipeline=lib.cmsReadTag(h,int.from_bytes(b'D2B1','big'))
        if not pipeline:raise RuntimeError('LCMS MPE tag open')
        for x in samples:
            a=(C.c_float*4)(*x);b=(C.c_float*3)()
            lib.cmsPipelineEvalFloat(a,b,pipeline);yield list(b)
    finally:
        lib.cmsCloseProfile(h)
def main():
    p=argparse.ArgumentParser()
    p.add_argument('--probe',type=pathlib.Path,required=True);p.add_argument('--out',type=pathlib.Path,required=True)
    a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
    samples=list(itertools.product((0.,.25,.5,.75,1.),repeat=4))
    results=[];threshold=.0002
    with tempfile.TemporaryDirectory(prefix='rpcore-mpe-') as tmp:
        for mode in ('matrix','clut','mixed'):
            data=fixture(mode);path=pathlib.Path(tmp)/'fixture.icc';path.write_bytes(data)
            raw=subprocess.run([str(a.probe.resolve()),str(path),'mpe-eval'],check=True,capture_output=True,text=True,timeout=20).stdout
            actual=[list(map(float,line.split(','))) for line in raw.splitlines()]
            expected=list(reference(data,samples))
            if len(actual)!=len(samples) or len(expected)!=len(samples) or any(len(v)!=3 or not all(map(math.isfinite,v)) for v in actual+expected):
                raise RuntimeError('invalid MPE probe/reference output')
            delta=max(abs(x-y) for n,r in zip(actual,expected) for x,y in zip(n,r))
            results.append({'mode':mode,'samples':len(samples),'max_error':delta,'pass':delta<=threshold,
                'rows':[{'input':x,'native':n,'reference':r} for x,n,r in zip(samples,actual,expected)]})
    (a.out/'report.json').write_text(json.dumps({'schema':1,'threshold':threshold,'cases':results},indent=2,allow_nan=False)+'\n')
    lines=['# ICC MPE raw pipeline comparison','','| Pipeline | Samples | Max absolute channel error | Gate |','|---|---:|---:|---|']
    for r in results:lines.append(f"| {r['mode']} | {r['samples']} | {r['max_error']:.9f} | {'PASS' if r['pass'] else 'FAIL'} |")
    lines+=['','Original synthetic float32 fixtures; raw D2B1 pipeline only, not display rendering or real-profile/intent acceptance.']
    md='\n'.join(lines)+'\n';(a.out/'report.md').write_text(md);print(md)
    return int(len(results)!=3 or not all(r['pass'] for r in results))
if __name__=='__main__':
    raise SystemExit(main())

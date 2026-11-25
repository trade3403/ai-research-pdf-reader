#!/usr/bin/env python3
"""Bounded, numeric-only official ICC Registry profile discovery/compatibility."""
import argparse
import json
import pathlib
import tempfile
import time
import urllib.request
import urllib.parse
from icc_print_benchmark import metadata,reference,DENSE
from icc_render_benchmark import lcms,make_pdf,error
from color_benchmark import measure
from chart_benchmark import render_pair

BASE='https://registry.color.org/profile-registry/profiles/'
CANDIDATES=[
    ('APTEC_Offset_Coated_LinearCTV_2025.icc','b435129ac50c246b372451ef59494ae1'),
    ('APTEC_Offset_Uncoated_LinearCTV_2025.icc','69143dbd94ac6a02b19c37cf4b0dbb1b'),
    ('CGATS21_CRPC6.icc','b3107b20a279592ff4f14a02b678a95b'),
]
LIMIT=16*1024*1024
class SameOrigin(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,req,fp,code,msg,headers,newurl):
        p=urllib.parse.urlsplit(newurl)
        if p.scheme!='https' or p.netloc!='registry.color.org':
            raise ValueError('registry redirect outside origin')
        return super().redirect_request(req,fp,code,msg,headers,newurl)

def download(name):
    req=urllib.request.Request(BASE+name,headers={'User-Agent':'RPCore CI profile compatibility test'})
    deadline=time.monotonic()+30
    with urllib.request.build_opener(SameOrigin()).open(req,timeout=10) as f:
        chunks=bytearray()
        while True:
            if time.monotonic()>deadline:raise TimeoutError('download elapsed limit')
            part=f.read(min(65536,LIMIT+1-len(chunks)))
            if not part:break
            chunks.extend(part)
            if len(chunks)>LIMIT:raise ValueError('profile byte limit')
        return bytes(chunks)

def main():
    p=argparse.ArgumentParser()
    for k in ('probe','reference','out'):
        p.add_argument('--'+k,type=pathlib.Path,required=True)
    p.add_argument('--require-mab',action='store_true',help='Require at least one native v4 mAB candidate with all gates passed')
    a=p.parse_args();a.probe=a.probe.resolve();a.reference=a.reference.resolve();a.out.mkdir(parents=True,exist_ok=True)
    lib=lcms()
    report={'schema':1,'profiles':[],'native_mab_pass':0,'note':'Official registry candidates, not assumed mAB; network/download and unsupported profiles explicitly reported. Numeric results only.'}
    with tempfile.TemporaryDirectory(prefix='rpcore-registry-') as tmp:
        work=pathlib.Path(tmp)
        for name,profile_id in CANDIDATES:
            e={'file':name,'url':BASE+name,'expected_profile_id':profile_id,'status':'download_error','rows':[]}
            report['profiles'].append(e)
            try:
                data=download(name)
                e['status']='metadata_error';e.update(metadata(data))
                e['profile_id']=data[84:100].hex()
                if e['profile_id']!=profile_id:raise ValueError('profile header ID differs from registry')
                tag=e['tag_types'].get('A2B1')
                if e['version']!=4 or tag!='mAB ':
                    e['status']='not_v4_mab';continue
                e['status']='reference_or_render_error'
                pdf=work/'candidate.pdf'
                # 32 reproducible interior/endpoint samples, four columns.
                size,rows=make_pdf(pdf,data,4,DENSE[:32],8)
                want=reference(lib,data,rows)
                n,r,im,ri=render_pair(a,pdf,1,72,work,'candidate',None)
                e.update(native=n,reference=r)
                if im is None or ri is None or im.size!=size or ri.size!=size:raise ValueError('render/size mismatch')
                diag=[v for k,v in sorted(n.items()) if k.startswith('icc_diag_')]
                e['diagnostics']=diag
                native=(n.get('icc_vector_native'),n.get('icc_image_native'))==('1','1') and (n.get('icc_vector_alternate'),n.get('icc_image_alternate'))==('0','0') and len(diag)==2 and all(';mab_A2B1;' in v for v in diag)
                for row,ex in zip(rows,want):
                    nv,ns=zip(*(measure(im,tuple(b)) for b in row['boxes']))
                    rv,_=zip(*(measure(ri,tuple(b)) for b in row['boxes']))
                    parity=max(error(nv[0],nv[1]),error(nv[0],nv[2]))
                    alpha=error(nv[3],[round((v*128+255*127)/255) for v in nv[2]])
                    e['rows'].append({'raw':row['raw'],'lcms_rgb':ex,'native_rgb':nv,'poppler_rgb':rv,
                        'lcms_error':max(error(v,ex) for v in nv[:3]),'parity':parity,'alpha':alpha,'uniform':max(ns)==0})
                errors=[x['lcms_error'] for x in e['rows']]
                e['max_error']=max(errors);e['mean_max_channel']=sum(errors)/len(errors)
                passed=native and len(errors)==32 and e['max_error']<=12 and e['mean_max_channel']<=3 and all(x['parity']<=1 and x['alpha']<=1 and x['uniform'] for x in e['rows'])
                e['status']='native_pass' if passed else 'native_accuracy_failed' if native else 'alternate_or_unsupported'
                report['native_mab_pass']+=int(passed)
            except (OSError,ValueError,RuntimeError) as exc:
                e['reason']=str(exc)[:300]
    (a.out/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    lines=['# ICC Registry v4 mAB compatibility discovery','','| Profile | ICC major | A2B1 type | Status | Samples | Max RGB error |','|---|---:|---|---|---:|---:|']
    for e in report['profiles']:
        lines.append(f"| {e['file']} | {e.get('version','—')} | {e.get('tag_types',{}).get('A2B1','—')} | {e['status']} | {len(e['rows'])} | {e.get('max_error','—')} |")
        for d in e.get('diagnostics',[]):lines.append(f"\n- {e['file']}: {d}")
        if e.get('reason'):lines.append(f"\n- {e['file']}: {e['reason']}")
    lines+=['',f"Verified native mAB profiles: {report['native_mab_pass']}/{len(CANDIDATES)} candidates.",
            'Discovery is diagnostic by default; only native_pass counts as acceptance. Missing downloads or other LUT types do not establish compatibility. Use --require-mab for a strict minimum-one gate. No profile/PDF/image assets exported.']
    md='\n'.join(lines)+'\n';(a.out/'report.md').write_text(md);print(md)
    return int(a.require_mab and not report['native_mab_pass'])

if __name__=='__main__':
    raise SystemExit(main())

#!/usr/bin/env python3
"""Bounded, numeric-only official ICC Registry profile discovery/compatibility."""
import argparse
import json
import hashlib
import subprocess
import statistics
import re
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
REQUIRED='CGATS21_CRPC6.icc'
MAX_ERROR=12
MEAN_ERROR=3
P95_ERROR=6

def computed_id(data):
    # ICC v4 profile ID: MD5 with flags, intent, and ID fields zeroed.
    # Diagnostic only; this is not authentication of publisher identity.
    b=bytearray(data)
    for start,end in ((44,48),(64,68),(84,100)):b[start:end]=bytes(end-start)
    return hashlib.md5(b,usedforsecurity=False).hexdigest()

def performance(exe,path):
    q=subprocess.run([str(exe),str(path),'perf-mab'],capture_output=True,text=True,timeout=25,check=True)
    fields=dict(line.split('=',1) for line in q.stdout.splitlines() if '=' in line)
    out={k:int(fields[k]) for k in ('samples','parse_us','min_ns','median_ns','max_ns')}
    if out['samples']!=100000 or not 0<out['min_ns']<=out['median_ns']<=out['max_ns'] or not re.fullmatch(r'[0-9a-f]{16}',fields['checksum']):
        raise ValueError('invalid benchmark output')
    out['checksum']=fields['checksum']
    out['median_samples_per_second']=round(out['samples']*1e9/out['median_ns'],2)
    return out
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
    p.add_argument('--require-mab',action='store_true',help='Require fixed CRPC6 native dense accuracy and performance checks')
    p.add_argument('--icc-probe',required=True,type=pathlib.Path)
    a=p.parse_args();a.probe=a.probe.resolve();a.reference=a.reference.resolve();a.out.mkdir(parents=True,exist_ok=True)
    lib=lcms()
    report={'schema':2,'profiles':[],'native_mab_pass':0,'required_profile':REQUIRED,
            'thresholds':{'max_rgb8':MAX_ERROR,'mean_max_channel':MEAN_ERROR,'p95':P95_ERROR},
            'note':'Pinned registry header ID for CRPC6; SHA256 and recomputed ICC ID recorded. Mismatched IDs diagnostic only. Numeric output only.'}
    with tempfile.TemporaryDirectory(prefix='rpcore-registry-') as tmp:
        work=pathlib.Path(tmp)
        for name,profile_id in CANDIDATES:
            e={'file':name,'url':BASE+name,'expected_profile_id':profile_id,'status':'download_error','rows':[]}
            report['profiles'].append(e)
            try:
                data=download(name)
                e['status']='metadata_error';e.update(metadata(data))
                e['profile_id']=data[84:100].hex()
                e['computed_profile_id']=computed_id(data)
                e['identity_match']=e['profile_id']==profile_id
                e['header_matches_computed']=e['profile_id']==e['computed_profile_id']
                tag=e['tag_types'].get('A2B1')
                if e['version']!=4 or tag!='mAB ':
                    e['status']='not_v4_mab';continue
                # Required profile retains the published ID pin. Changed
                # APTEC bytes may be evaluated, but never counted as verified.
                if name==REQUIRED and not e['identity_match']:
                    e['status']='identity_mismatch';continue
                e['status']='reference_or_render_error'
                e['renders']=[];native=True
                bits_list=(8,16) if name==REQUIRED else (8,)
                dpi_list=(72,144) if name==REQUIRED else (72,)
                source=DENSE if name==REQUIRED else DENSE[:32]
                for bits in bits_list:
                    samples=source if bits==8 else [tuple(v*257+(1 if 0<v<255 else 0) for v in row) for row in source]
                    for offset in range(0,len(samples),32):
                        pdf=work/'candidate.pdf'
                        size,rows=make_pdf(pdf,data,4,samples[offset:offset+32],bits)
                        want=reference(lib,data,rows)
                        for dpi in dpi_list:
                            label=f'{bits}-{offset}-{dpi}'
                            n,r,im,ri=render_pair(a,pdf,1,dpi,work,label,None)
                            e['renders'].append({'case':label,'native':n,'reference':r})
                            scale=dpi//72;expected_size=tuple(v*scale for v in size)
                            if im is None or ri is None or im.size!=expected_size or ri.size!=expected_size:raise ValueError('render/size mismatch')
                            diag=[v for k,v in sorted(n.items()) if k.startswith('icc_diag_')]
                            e['diagnostics']=diag
                            native=native and (n.get('icc_vector_native'),n.get('icc_image_native'))==('1','1') and (n.get('icc_vector_alternate'),n.get('icc_image_alternate'))==('0','0') and len(diag)==2 and all(';mab_A2B1;' in v for v in diag)
                            for row,ex in zip(rows,want):
                                boxes=[tuple(x*scale for x in b) for b in row['boxes']]
                                nv,ns=zip(*(measure(im,b) for b in boxes))
                                rv,_=zip(*(measure(ri,b) for b in boxes))
                                parity=max(error(nv[0],nv[1]),error(nv[0],nv[2]))
                                alpha=error(nv[3],[round((v*128+255*127)/255) for v in nv[2]])
                                e['rows'].append({'case':label,'raw':row['raw'],'lcms_rgb':ex,'native_rgb':nv,'poppler_rgb':rv,
                                    'lcms_error':max(error(v,ex) for v in nv[:3]),'parity':parity,'alpha':alpha,'uniform':max(ns)==0})
                errors=sorted(x['lcms_error'] for x in e['rows'])
                e['max_error']=max(errors);e['mean_max_channel']=statistics.mean(errors)
                e['p95']=errors[(95*len(errors)+99)//100-1]
                expected=len(source)*len(bits_list)*len(dpi_list)
                passed=native and len(errors)==expected and e['max_error']<=MAX_ERROR and e['mean_max_channel']<=MEAN_ERROR and e['p95']<=P95_ERROR and all(x['parity']<=1 and x['alpha']<=1 and x['uniform'] for x in e['rows'])
                if passed and name==REQUIRED:
                    e['status']='performance_error'
                    profile_path=work/'benchmark.icc';profile_path.write_bytes(data)
                    e['performance']=performance(a.icc_probe.resolve(),profile_path)
                e['status']=('native_pass' if e['identity_match'] else 'unverified_identity_native') if passed else 'native_accuracy_failed' if native else 'alternate_or_unsupported'
                report['native_mab_pass']+=int(e['status']=='native_pass')
            except (OSError,ValueError,RuntimeError,subprocess.SubprocessError,KeyError) as exc:
                e['reason']=str(exc)[:300]
    (a.out/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    lines=['# ICC Registry v4 mAB compatibility and dense gate','','| Profile | ICC major | A2B1 type | Status | Samples | Max RGB error | Mean | P95 |','|---|---:|---|---|---:|---:|---:|---:|']
    for e in report['profiles']:
        lines.append(f"| {e['file']} | {e.get('version','—')} | {e.get('tag_types',{}).get('A2B1','—')} | {e['status']} | {len(e['rows'])} | {e.get('max_error','—')} | {e.get('mean_max_channel','—')} | {e.get('p95','—')} |")
        if 'profile_id' in e:
            lines.append(f"\n- {e['file']}: expected ID {e['expected_profile_id']}; header ID {e['profile_id']}; recomputed ID {e['computed_profile_id']}; SHA256 {e['sha256']}")
        if 'performance' in e:
            v=e['performance']
            lines.append(f"\n- {e['file']}: parse {v['parse_us']} us; median {v['median_samples_per_second']} XYZ evaluations/s, {v['samples']} samples x 3 measured passes, checksum {v['checksum']}")
        for d in e.get('diagnostics',[]):lines.append(f"\n- {e['file']}: {d}")
        if e.get('reason'):lines.append(f"\n- {e['file']}: {e['reason']}")
    lines+=['',f"Verified native mAB profiles: {report['native_mab_pass']}/{len(CANDIDATES)} candidates.",
            'CRPC6: 512 paired checks, pinned header ID, native A2B1 both paths, max <=12/mean <=3/P95 <=6. Performance requires successful repeatable finite XYZ output, not a fixed speed on shared runners. APTEC identity mismatch is diagnostic only. No profile/PDF/image assets exported.']
    md='\n'.join(lines)+'\n';(a.out/'report.md').write_text(md);print(md)
    required=next(x for x in report['profiles'] if x['file']==REQUIRED)
    return int(a.require_mab and (required['status']!='native_pass' or len(required['rows'])!=512 or len(required.get('renders',[]))!=16 or 'performance' not in required))

if __name__=='__main__':
    raise SystemExit(main())

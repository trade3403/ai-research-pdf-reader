#!/usr/bin/env python3
"""Generate complete, original synthetic PDFs with an independent writer.
Only fixed public test credentials. No corpus/user documents or PDF exports.
pypdf/cryptography/Poppler are CI-only, not RPCore dependencies.
"""
import argparse
import json
import subprocess
import tempfile
from pathlib import Path
import pypdf
import cryptography
from pypdf import PdfWriter, PdfReader
from pypdf.generic import NameObject as N, DictionaryObject as D, DecodedStreamObject, NumberObject as I

CASES=[('RC4-40',2),('RC4-128',3),('AES-128',4),('AES-256-R5',5),('AES-256',6)]
EXPECTED=['RPCore page 1','RPCore page 2']

def generate(path,algorithm=None):
    w=PdfWriter()
    font=w._add_object(D({N('/Type'):N('/Font'),N('/Subtype'):N('/Type1'),N('/BaseFont'):N('/Helvetica'),N('/Encoding'):N('/WinAnsiEncoding')}))
    im=DecodedStreamObject()
    im.set_data(bytes([255,0,0,0,255,0,0,0,255,255,255,0]))
    im.update({N('/Type'):N('/XObject'),N('/Subtype'):N('/Image'),N('/Width'):I(2),N('/Height'):I(2),N('/ColorSpace'):N('/DeviceRGB'),N('/BitsPerComponent'):I(8)})
    image=w._add_object(im.flate_encode())
    for i in (1,2):
        p=w.add_blank_page(144,96)
        p[N('/Resources')]=D({N('/Font'):D({N('/F1'):font}),N('/XObject'):D({N('/Im'):image})})
        stream=DecodedStreamObject()
        stream.set_data(f'0 0 1 rg 5 5 40 20 re f\nq 20 0 0 20 100 5 cm /Im Do Q\n0 g BT /F1 12 Tf 10 60 Td (RPCore page {i}) Tj ET\n'.encode())
        p[N('/Contents')]=w._add_object(stream.flate_encode())
    w.add_metadata({'/Title':'RPCore synthetic encryption gate'})
    if algorithm:
        w.encrypt('reader','owner',algorithm=algorithm)
    with path.open('wb') as f:
        w.write(f)

def run():
    p=argparse.ArgumentParser()
    p.add_argument('--probe',required=True,type=Path)
    p.add_argument('--font-dir',required=True,type=Path)
    p.add_argument('--out',required=True,type=Path)
    a=p.parse_args()
    a.out.mkdir(parents=True,exist_ok=True)
    report={'pypdf':pypdf.__version__,'cryptography':cryptography.__version__,'cases':[]}
    with tempfile.TemporaryDirectory() as d:
        work=Path(d)
        plain=work/'plain.pdf'
        generate(plain)
        assert [page.extract_text() for page in PdfReader(plain).pages]==EXPECTED
        # Independent reference renderer: compare each encrypted page against
        # the same generator's clear document, not a different rendering engine.
        for dpi in (72,144):
            subprocess.run(['pdftoppm','-r',str(dpi),str(plain),str(work/f'plain-{dpi}')],check=True,timeout=30)
        for alg,revision in CASES:
            encrypted=work/f'r{revision}.pdf'
            generate(encrypted,alg)
            for password,role in [('reader',1),('owner',2)]:
                reader=PdfReader(encrypted)
                assert reader.decrypt(password)==role
                assert [page.extract_text() for page in reader.pages]==EXPECTED
                assert reader.metadata.title=='RPCore synthetic encryption gate'
            reader=PdfReader(encrypted)
            assert reader.decrypt('incorrect')==0
            for dpi in (72,144):
                prefix=work/f'r{revision}-{dpi}'
                subprocess.run(['pdftoppm','-upw','reader','-r',str(dpi),str(encrypted),str(prefix)],check=True,timeout=30)
                for page in (1,2):
                    assert (work/f'plain-{dpi}-{page}.ppm').read_bytes()==Path(f'{prefix}-{page}.ppm').read_bytes()
            result=subprocess.run([str(a.probe.resolve()),str(plain),str(encrypted),str(a.font_dir),str(revision)],check=True,capture_output=True,text=True,timeout=180)
            expected=f'R{revision}: 8/8 render parity; 4/4 text and glyph parity; wrong password rejected'
            if expected not in result.stdout.splitlines():
                raise RuntimeError(f'R{revision} missing native gate result')
            print(expected,flush=True)
            report['cases'].append({'revision':revision,'algorithm':alg,'render_parity':8,'text_geometry_parity':4,'poppler_parity':4,'wrong_password_rejected':True})
    report['totals']={'render_parity':40,'text_geometry_parity':20,'poppler_parity':20}
    (a.out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    (a.out/'report.md').write_text('# Encrypted PDF closure gate\n\n5/5 independent writer revisions; 40/40 native pixel comparisons; 20/20 native text/geometry comparisons; 20/20 Poppler comparisons. Synthetic fixtures only.\n')
    print('Encryption closure: 5/5 revisions; 40/40 native render; 20/20 text geometry; 20/20 reference render.',flush=True)

if __name__=='__main__':
    run()

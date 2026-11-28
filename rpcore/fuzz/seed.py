#!/usr/bin/env python3
"""Extract the already validated Annex H test vectors; no external downloads."""
from pathlib import Path
import re
import struct
ROOT = Path(__file__).resolve().parent
source = (ROOT.parent / "core/src/filters/jbig2/page.rs").read_text()
dest = ROOT / "corpus/jbig2_page"
dest.mkdir(parents=True, exist_ok=True)
def extract(name, expected):
    pattern = r"const " + name + r":&\[u8\]=&\[(.*?)\];"
    matches = re.findall(pattern, source, re.S)
    if len(matches) != 1:
        raise ValueError("missing or ambiguous seed: " + name)
    data = bytes(int(x,16) for x in re.findall(r"0x([0-9a-fA-F]{2})\b",matches[0]))
    if len(data) != expected:
        raise ValueError("changed seed length: " + name)
    return data
g1 = extract("GLOBAL1",35)
g3 = extract("GLOBAL3",33)
for name, g, p in [("annex-h-1",g1,extract("PAGE1",352)),
                   ("annex-h-2",g1,extract("PAGE2",282)),
                   ("annex-h-3",g3,extract("PAGE3",145))]:
    (dest / name).write_bytes(struct.pack("<H",len(g))+g+p)
print("JBIG2 seed corpus: three original Annex H page/global pairs")

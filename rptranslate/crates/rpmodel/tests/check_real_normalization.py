"""Reference comparison; only CI development, not shipped at runtime."""
import subprocess
import sys
from pathlib import Path

import sentencepiece as sp

cases = [
    "", "  ", "  a  b  ", "ＡＢＣ１２３", "学术论文。", "e\u0301 café",
    "ﬁ ﬃ Ⅲ ①", "1\u00a0mg\u3000L", "a\tb\nc", "α β μ",
    "10⁻³ ± 0.05", "Ｆｅ₂Ｏ₃", "中文 English 😀", "a\u200bb",
    "a\u200db", "▁ab", "x\u202fy", "a\r\nb", "  温度  25 ℃  ",
]
root = Path(sys.argv[1])
exe = Path("target/debug/examples/normalize_spm").resolve()
for filename in ("source.spm", "target.spm"):
    model = sp.SentencePieceProcessor(model_file=str(root / filename))
    failures = []
    for text in cases:
        expected = model.normalize(text)
        got = subprocess.check_output([str(exe), str(root / filename), text]).decode()
        if got != expected:
            failures.append((text, expected, got))
    print(f"{filename}: {len(cases)-len(failures)}/{len(cases)} exact normalization")
    for case in failures:
        print("MISMATCH", repr(case))
    if failures:
        raise SystemExit(1)

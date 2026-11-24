"""CI-only independent SentencePiece ID and decode comparison."""
import subprocess
import sys
from pathlib import Path

import sentencepiece as sp

CASES = [
    "", " ", "  a  b  ", "Hello world.", "Academic paper translation.",
    "The results are statistically significant (p < 0.05).",
    "A 95% confidence interval was calculated.",
    "The Transformer uses multi-head attention.",
    "The solution contained 25 mg/mL of NaCl.",
    "Equation (3) defines the objective function.",
    "学术论文。", "实验结果表明模型具有较高的准确率。",
    "计算机科学与人工智能", "参考文献 [12]", "温度为 25 ℃。",
    "ＡＢＣ１２３", "e\u0301 café", "ﬁ ﬃ Ⅲ ①", "1\u00a0mg\u3000L",
    "α β μ", "10⁻³ ± 0.05", "Ｆｅ₂Ｏ₃", "中文 English 😀",
    "a\u200bb", "a\u200db", "▁ab", "x\u202fy", "🧪🦄",
    "  温度  25 ℃  ", "a\tb",
]
root = Path(sys.argv[1])
exe = Path("target/debug/examples/tokenize_spm").resolve()
failed = False
for filename in ("source.spm", "target.spm"):
    model = sp.SentencePieceProcessor(model_file=str(root / filename))
    out = subprocess.run(
        [str(exe), str(root / filename)],
        input="\n".join(CASES) + "\n", text=True, capture_output=True, check=True,
    ).stdout.splitlines()
    assert len(out) == len(CASES), (filename, len(out))
    id_ok = decode_ok = 0
    for text, line in zip(CASES, out):
        ids, decoded = line.split("\t")
        ids = [int(i) for i in ids.split(",")] if ids else []
        decoded = bytes.fromhex(decoded).decode()
        expected = model.encode(text, out_type=int)
        expected_decode = model.decode(expected)
        id_ok += ids == expected
        decode_ok += decoded == expected_decode
        if ids != expected or decoded != expected_decode:
            print("MISMATCH", filename, repr(text), expected, ids,
                  repr(expected_decode), repr(decoded), flush=True)
    print(f"{filename}: IDs {id_ok}/{len(CASES)}, decode {decode_ok}/{len(CASES)}", flush=True)
    failed |= id_ok != len(CASES) or decode_ok != len(CASES)
if failed:
    raise SystemExit(1)

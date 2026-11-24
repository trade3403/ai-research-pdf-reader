"""CI-only comparison against pinned Hugging Face MarianTokenizer."""
import subprocess
import sys
from pathlib import Path

from transformers import MarianTokenizer

root = Path(sys.argv[1])
ref = MarianTokenizer(source_spm=str(root / "source.spm"),
                      target_spm=str(root / "target.spm"),
                      vocab=str(root / "vocab.json"),
                      source_lang="en", target_lang="zh")
print("network vocabulary", ref.vocab_size, "special IDs",
      ref.unk_token_id, ref.eos_token_id, ref.pad_token_id,
      "language codes", ref.supported_language_codes, flush=True)
exe = Path("target/debug/examples/marian_codec").resolve()
cases = [
    "", " ", "  a  b  ", "Academic paper translation.",
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
failed = False
for filename in ("source.spm", "target.spm"):
    texts = list(cases)
    if filename == "source.spm":
        ref._switch_to_input_mode()
        texts.extend(code + " Academic paper." for code in ref.supported_language_codes[:3])
    else:
        ref._switch_to_target_mode()
    result = subprocess.run(
        [str(exe), str(root / filename), str(root / "vocab.json")],
        input="\n".join(texts) + "\n", text=True, capture_output=True, check=True,
    ).stdout.splitlines()
    assert len(result) == len(texts)
    id_ok = decode_ok = 0
    for text, line in zip(texts, result):
        ids, decoded = line.split("\t")
        ids = [int(i) for i in ids.split(",")]
        decoded = bytes.fromhex(decoded).decode()
        # Compare Marian's low-level tokenization, then official special-token
        # construction. No optional Moses normalization/added-token splitting.
        expected = ref.build_inputs_with_special_tokens(
            ref.convert_tokens_to_ids(ref._tokenize(text)))
        expected_decode = ref.decode(expected, skip_special_tokens=True,
                                     clean_up_tokenization_spaces=False,
                                     use_source_tokenizer=filename == "source.spm")
        id_ok += ids == expected
        decode_ok += decoded == expected_decode
        if ids != expected or decoded != expected_decode:
            print("MISMATCH", filename, repr(text), expected, ids,
                  repr(expected_decode), repr(decoded), flush=True)
    print(f"{filename}: Marian IDs {id_ok}/{len(texts)}, decode {decode_ok}/{len(texts)}", flush=True)
    failed |= id_ok != len(texts) or decode_ok != len(texts)
if failed:
    raise SystemExit(1)

"""Actual offline translations: independent official tokenizer/model and search."""
import json
import os
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

os.environ["HF_HUB_OFFLINE"] = "1"
os.environ["TRANSFORMERS_OFFLINE"] = "1"
import numpy as np
import torch
from transformers import GenerationConfig, MarianConfig, MarianMTModel, MarianTokenizer

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe, cli = map(Path, sys.argv[1:5])
state = load_checked(source)
cfg = MarianConfig.from_dict(json.loads((source / "config.json").read_bytes()))
cfg._attn_implementation = "eager"
model = MarianMTModel(cfg).eval()
missing, unexpected = model.load_state_dict(state, strict=False)
assert not unexpected and set(missing) <= {"lm_head.weight"}
del state
tokenizer = MarianTokenizer(source_spm=str(source / "source.spm"),
                            target_spm=str(source / "target.spm"),
                            vocab=str(source / "vocab.json"),
                            source_lang="en", target_lang="zh")
cases = ["Hello world.", "The result is stable.", "The temperature is 25 degrees Celsius."]
output = subprocess.run([str(exe.resolve()), str(package)], input="\n".join(cases) + "\n",
                        text=True, encoding="utf-8", capture_output=True, check=True,
                        timeout=180).stdout.splitlines()
assert len(output) == len(cases) * 2
max_score_error = 0.
with torch.inference_mode():
    for case, text in enumerate(cases):
        tokenizer._switch_to_input_mode()
        ids = tokenizer.build_inputs_with_special_tokens(
            tokenizer.convert_tokens_to_ids(tokenizer._tokenize(">>cmn_Hans<< " + text)))
        src = torch.tensor([ids])
        mask = torch.ones_like(src)
        enc = model.model.encoder(input_ids=src, attention_mask=mask, return_dict=True)

        @lru_cache(maxsize=None)
        def log_probs(prefix):
            result = model(encoder_outputs=enc, attention_mask=mask,
                           decoder_input_ids=torch.tensor([prefix]), use_cache=False,
                           return_dict=True)
            return torch.log_softmax(result.logits[0, -1].double(), -1)

        def beam_reference(width):
            beam = [((65000,), 0., 0., False)]
            for step in range(1, 49):
                candidates = []
                for tokens, lp, score, ended in beam:
                    if ended:
                        candidates.append((tokens, lp, score, ended))
                        continue
                    probs = log_probs(tokens)
                    # Stable sort of all legal next IDs; at most width per parent
                    # can enter the global top width at the same depth.
                    if step == 48:
                        legal = [0]
                    else:
                        legal = [i for i in torch.argsort(probs, descending=True, stable=True).tolist()
                                 if i != 65000][:width]
                    for token in legal:
                        total = lp + float(probs[token])
                        candidates.append((tokens + (token,), total, total / step, token == 0))
                beam = sorted(candidates, key=lambda h: (-h[2], h[0]))[:width]
                if all(h[3] for h in beam):
                    break
            return beam[0]

        for index, width in enumerate((1, 4)):
            w, src_ids, tgt_ids, hex_text, limit, score = output[case * 2 + index].split("\t")
            assert int(w) == width and list(map(int, src_ids.split(","))) == ids
            tokens = tuple(map(int, tgt_ids.split(",")))
            actual = bytes.fromhex(hex_text).decode("utf-8")
            expected = beam_reference(width)
            assert tokens == expected[0] and expected[3], (case, width, tokens, expected)
            assert limit == "0" and len(tokens) < 49, (case, width, "truncated")
            np.testing.assert_allclose(float(score), expected[2], atol=1e-4, rtol=1e-4)
            max_score_error = max(max_score_error, abs(float(score) - expected[2]))
            decoded = tokenizer.decode(list(tokens), skip_special_tokens=True,
                                       clean_up_tokenization_spaces=False)
            assert actual == decoded and any("\u4e00" <= c <= "\u9fff" for c in actual), (actual, decoded)
            if width == 1:
                gc = GenerationConfig(decoder_start_token_id=65000, pad_token_id=65000,
                                      eos_token_id=0, forced_eos_token_id=0, max_length=49,
                                      num_beams=1, do_sample=False, bad_words_ids=[[65000]],
                                      use_cache=False)
                hf = tuple(model.generate(input_ids=src, attention_mask=mask,
                                          generation_config=gc)[0].tolist())
                assert hf == tokens
            print(f"REAL_TRANSLATION case={case} beam={width} source={text!r} target={actual!r} "
                  f"tokens={len(tokens)-1} score={score}", flush=True)
            if case == 0 and width == 4:
                result = subprocess.run([str(cli.resolve()), "translate", str(package), text, "4", "48"],
                                        capture_output=True, text=True, encoding="utf-8", check=True,
                                        timeout=60)
                assert result.stdout.strip() == actual and not result.stderr
        log_probs.cache_clear()
print(f"REAL_TRANSLATION_REFERENCE PASS sentences=3 generations=6 hf_greedy=3 "
      f"cli=1 invalid=3 max_score_error={max_score_error:.9g}", flush=True)

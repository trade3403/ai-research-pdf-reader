"""Real-model cache/logits and independent exhaustive bounded beam reference."""
import json
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

import numpy as np
import torch
from transformers import GenerationConfig, MarianConfig, MarianMTModel

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
state = load_checked(source)
cfg = MarianConfig.from_dict(json.loads((source / "config.json").read_bytes()))
cfg._attn_implementation = "eager"
model = MarianMTModel(cfg).eval()
missing, unexpected = model.load_state_dict(state, strict=False)
assert not unexpected and set(missing) <= {"lm_head.weight"}, (missing, unexpected)
cases = [
    ([1, 2048, 0], None, [65000, 4, 4096], None),
    ([2047, 2048, 4095, 4096, 0, 65000], [1, 1, 1, 1, 1, 0],
     [65000, 4, 65000, 4096], [1, 1, 0, 1]),
]
atol = rtol = 1e-4
comparisons = elements = hypotheses = checks = hf_checks = 0
max_abs = max_cache_diff = max_score_error = 0.
with torch.inference_mode(), subprocess.Popen(
        [str(exe.resolve()), str(package)], stdout=subprocess.PIPE) as proc:
    try:
        for case, (ids, sm, tgt, tm) in enumerate(cases):
            src = torch.tensor([ids])
            sm_t = None if sm is None else torch.tensor([sm])
            enc = model.model.encoder(input_ids=src, attention_mask=sm_t, return_dict=True)
            ref = model(encoder_outputs=enc, attention_mask=sm_t,
                        decoder_input_ids=torch.tensor([tgt]),
                        decoder_attention_mask=None if tm is None else torch.tensor([tm]),
                        use_cache=False, return_dict=True).logits[0].contiguous().numpy()
            actual = []
            for mode in ("full", "cached"):
                raw = proc.stdout.read(ref.nbytes)
                assert len(raw) == ref.nbytes, (case, mode, len(raw), ref.nbytes)
                arr = np.frombuffer(raw, dtype="<f4").reshape(ref.shape)
                assert np.isfinite(arr).all()
                np.testing.assert_allclose(arr, ref, atol=atol, rtol=rtol)
                assert np.array_equal(arr.argmax(axis=-1), ref.argmax(axis=-1))
                max_abs = max(max_abs, float(np.max(np.abs(arr.astype(np.float64) - ref))))
                comparisons += 1
                elements += arr.size
                actual.append(arr)
            np.testing.assert_allclose(actual[0], actual[1], atol=atol, rtol=rtol)
            max_cache_diff = max(max_cache_diff, float(np.max(np.abs(actual[0] - actual[1]))))
            rows = {}
            while True:
                line = proc.stdout.readline().decode("ascii").strip()
                assert line, "unexpected EOF"
                if line == "END":
                    break
                width, mode, rank, tokens, lp, score, ended = line.split("|")
                rows.setdefault((int(width), mode), []).append(
                    (int(rank), tuple(map(int, tokens.split(","))), float(lp), float(score), bool(int(ended))))

            @lru_cache(maxsize=None)
            def log_probs(prefix):
                result = model(encoder_outputs=enc, attention_mask=sm_t,
                               decoder_input_ids=torch.tensor([prefix]),
                               use_cache=False, return_dict=True)
                return torch.log_softmax(result.logits[0, -1].double(), -1).tolist()

            def reference(width):
                beam = [((65000,), 0., 0., False)]
                for step in range(1, 5):
                    candidates = []
                    for tokens, lp, score, ended in beam:
                        if ended:
                            candidates.append((tokens, lp, score, ended))
                            continue
                        for token, delta in enumerate(log_probs(tokens)):
                            if token == 65000 or (step == 4 and token != 0):
                                continue
                            total = lp + delta
                            candidates.append((tokens + (token,), total, total / step, token == 0))
                    beam = sorted(candidates, key=lambda h: (-h[2], h[0]))[:width]
                    if all(h[3] for h in beam):
                        break
                return beam

            assert set(rows) == {(w, m) for w in (1, 4) for m in ("full", "cached")}
            for width in (1, 4):
                expected = reference(width)
                for mode in ("full", "cached"):
                    result = rows[(width, mode)]
                    assert len(result) == len(expected)
                    for rank, (a, b) in enumerate(zip(result, expected)):
                        assert a[0] == rank and a[1] == b[0] and a[4] == b[3], (case, width, mode, a, b)
                        np.testing.assert_allclose(a[2:4], b[1:3], atol=atol, rtol=rtol)
                        max_score_error = max(max_score_error, abs(a[2]-b[1]), abs(a[3]-b[2]))
                        hypotheses += 1
                    checks += 1
                if width == 1:
                    gc = GenerationConfig(decoder_start_token_id=65000, pad_token_id=65000,
                                          eos_token_id=0, forced_eos_token_id=0, max_length=5,
                                          num_beams=1, do_sample=False, bad_words_ids=[[65000]],
                                          use_cache=False)
                    hf = tuple(model.generate(input_ids=src, attention_mask=sm_t,
                                              generation_config=gc)[0].tolist())
                    assert hf == expected[0][0], (case, hf, expected)
                    hf_checks += 1
            log_probs.cache_clear()
        assert proc.stdout.read(1) == b""
        assert proc.wait() == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
assert comparisons == 4 and checks == 8 and hf_checks == 2
print(f"REAL_GENERATION_REFERENCE PASS cases=2 logits_comparisons={comparisons} "
      f"elements={elements} generation_checks={checks} hypotheses={hypotheses} "
      f"hf_greedy={hf_checks} max_abs={max_abs:.9g} cache_diff={max_cache_diff:.9g} "
      f"max_score_error={max_score_error:.9g} atol={atol} rtol={rtol}", flush=True)

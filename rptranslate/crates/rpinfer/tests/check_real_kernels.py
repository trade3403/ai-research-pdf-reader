"""Real fixed-profile kernel comparison, with independent HF references."""
import json
import subprocess
import sys
from functools import lru_cache
from pathlib import Path

import numpy as np
import torch
from transformers import MarianConfig, MarianMTModel

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
mode = sys.argv[4]
assert mode in ("native", "portable")
state = load_checked(source)
cfg = MarianConfig.from_dict(json.loads((source / "config.json").read_bytes()))
cfg._attn_implementation = "eager"
model = MarianMTModel(cfg).eval()
missing, unexpected = model.load_state_dict(state, strict=False)
assert not unexpected and set(missing) <= {"lm_head.weight"}, (missing, unexpected)
assert model.lm_head.weight.data_ptr() == model.model.shared.weight.data_ptr()
del state
atol = rtol = 1e-4
comparisons = elements = argmax_rows = hypotheses = generation_checks = 0
max_abs = max_cross = max_score = 0.0

def ids(n, start, stride):
    return [2 + (start + i * stride) % 64997 for i in range(n)]

def reference(src, tgt, tm, last_only=False):
    src_t = torch.tensor([src])
    sm_t = (src_t != 65000).long()
    enc = model.model.encoder(input_ids=src_t, attention_mask=sm_t, return_dict=True)
    dec = model.model.decoder(
        input_ids=torch.tensor([tgt]),
        attention_mask=None if tm is None else torch.tensor([tm]),
        encoder_hidden_states=enc.last_hidden_state,
        encoder_attention_mask=sm_t,
        use_cache=False, return_dict=True)
    hidden = dec.last_hidden_state[:, -1:] if last_only else dec.last_hidden_state
    logits = model.lm_head(hidden) + model.final_logits_bias
    return enc, sm_t, logits[0].contiguous().numpy()

with torch.inference_mode(), subprocess.Popen(
        [str(exe.resolve()), str(package), mode], stdout=subprocess.PIPE) as proc:
    def compare(label, expected, baseline=None, logits=False):
        global comparisons, elements, argmax_rows, max_abs, max_cross
        ref = np.asarray(expected, dtype=np.float32)
        raw = proc.stdout.read(ref.nbytes)
        assert len(raw) == ref.nbytes, (label, len(raw), ref.nbytes)
        actual = np.frombuffer(raw, dtype="<f4").reshape(ref.shape)
        assert np.isfinite(actual).all() and np.isfinite(ref).all()
        err = float(np.max(np.abs(actual.astype(np.float64) - ref)))
        print(f"REAL_KERNEL_COMPARE {mode} {label} elements={ref.size} max_abs={err:.9g}", flush=True)
        np.testing.assert_allclose(actual, ref, atol=atol, rtol=rtol, err_msg=label)
        if baseline is not None:
            np.testing.assert_allclose(actual, baseline, atol=atol, rtol=rtol, err_msg=label)
            max_cross = max(max_cross, float(np.max(np.abs(actual - baseline))))
        if logits:
            assert np.array_equal(actual.argmax(axis=-1), ref.argmax(axis=-1)), label
            argmax_rows += ref.shape[0]
        max_abs = max(max_abs, err)
        comparisons += 1
        elements += ref.size
        return actual

    try:
        for case, (src, tgt, tm) in enumerate([
            ([1, 2048, 0], [65000, 4, 4096], None),
            ([2047, 2048, 4095, 4096, 0, 65000],
             [65000, 4, 65000, 4096], [1, 1, 0, 1]),
        ]):
            enc, sm, logits = reference(src, tgt, tm)
            memory = enc.last_hidden_state[0].numpy()

            @lru_cache(maxsize=None)
            def log_probs(prefix):
                output = model(encoder_outputs=enc, attention_mask=sm,
                               decoder_input_ids=torch.tensor([prefix]),
                               use_cache=False, return_dict=True)
                return torch.log_softmax(output.logits[0, -1].double(), -1).tolist()

            def beam_reference(width):
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

            expected = {w: beam_reference(w) for w in (1, 4)}
            baseline = None
            for kernel in ("scalar", "blocked", "auto", "parallel4"):
                arr = [
                    compare(f"{case}/{kernel}/memory", memory, None if baseline is None else baseline[0]),
                    compare(f"{case}/{kernel}/full", logits, None if baseline is None else baseline[1], True),
                    compare(f"{case}/{kernel}/cached", logits, None if baseline is None else baseline[2], True),
                ]
                np.testing.assert_allclose(arr[1], arr[2], atol=atol, rtol=rtol)
                if baseline is None:
                    baseline = arr
                actual = {}
                while True:
                    line = proc.stdout.readline().decode("ascii").strip()
                    assert line, "unexpected EOF"
                    if line == "END":
                        break
                    w, rank, tokens, lp, score, ended = line.split("|")
                    actual.setdefault(int(w), []).append(
                        (int(rank), tuple(map(int, tokens.split(","))), float(lp), float(score), bool(int(ended))))
                assert set(actual) == {1, 4}
                for width, rows in actual.items():
                    assert len(rows) == len(expected[width])
                    for rank, (a, b) in enumerate(zip(rows, expected[width])):
                        assert a[0] == rank and a[1] == b[0] and a[4] == b[3], (case, kernel, a, b)
                        np.testing.assert_allclose(a[2:4], b[1:3], atol=atol, rtol=rtol)
                        max_score = max(max_score, abs(a[2]-b[1]), abs(a[3]-b[2]))
                        hypotheses += 1
                    generation_checks += 1
            log_probs.cache_clear()

        if mode == "native":
            src = ids(64, 17, 7919)
            src[-2:] = [0, 65000]
            tgt = ids(16, 23, 3571)
            tgt[0] = 65000
            enc, _, logits = reference(src, tgt, None)
            memory = enc.last_hidden_state[0].numpy()
            baseline = None
            for kernel in ("auto", "parallel4"):
                arr = [
                    compare(f"64/{kernel}/memory", memory, None if baseline is None else baseline[0]),
                    compare(f"64/{kernel}/full", logits, None if baseline is None else baseline[1], True),
                    compare(f"64/{kernel}/cached", logits, None if baseline is None else baseline[2], True),
                ]
                np.testing.assert_allclose(arr[1], arr[2], atol=atol, rtol=rtol)
                if baseline is None:
                    baseline = arr
            src = ids(512, 17, 7919)
            src[507:] = [0] + [65000] * 4
            tgt = ids(512, 23, 3571)
            tgt[0] = 65000
            enc, _, logits = reference(src, tgt, None, last_only=True)
            compare("512/auto/memory", enc.last_hidden_state[0].numpy())
            compare("512/auto/last_logits", logits, logits=True)
        assert proc.stdout.read(1) == b""
        assert proc.wait() == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()

assert comparisons == (32 if mode == "native" else 24)
assert generation_checks == 16 and hypotheses == 40
print(f"REAL_KERNEL_REFERENCE PASS mode={mode} comparisons={comparisons} "
      f"elements={elements} argmax_rows={argmax_rows} generation_checks={generation_checks} "
      f"hypotheses={hypotheses} max_abs={max_abs:.9g} max_cross={max_cross:.9g} "
      f"max_score_error={max_score:.9g} atol={atol} rtol={rtol}", flush=True)

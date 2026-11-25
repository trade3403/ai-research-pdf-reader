"""Independent Transformers Marian encoder on the actual audited checkpoint."""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
import torch
from transformers import MarianConfig, MarianMTModel

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
state = load_checked(source)
cfg = MarianConfig.from_dict(json.loads((source / "config.json").read_bytes()))
cfg._attn_implementation = "eager"
model = MarianMTModel(cfg).eval()
missing, unexpected = model.load_state_dict(state, strict=False)
assert not unexpected and set(missing) <= {"lm_head.weight"}, (missing, unexpected)
encoder = model.model.encoder
cases = [
    ([1, 2048, 0], None),
    ([2047, 2048, 4095, 4096, 64999, 0, 65000, 65000], [1, 1, 1, 1, 1, 1, 0, 0]),
    ([4, 17, 28, 109, 2047, 2048, 4096, 12991, 32000, 50000, 64999, 0], None),
]
# Predeclared real-width tolerance: FP32 accumulation order and FP64 Rust
# normalization differ from PyTorch; retain earlier embedding/logit gate.
atol = rtol = 1e-4
comparisons = elements = 0
max_abs = max_final = 0.
with torch.no_grad(), subprocess.Popen([str(exe.resolve()), str(package)], stdout=subprocess.PIPE) as proc:
    try:
        for case, (ids, mask) in enumerate(cases):
            traces = {}
            hooks = []
            for index, layer in enumerate(encoder.layers):
                for label, module in (("attention", layer.self_attn_layer_norm), ("final", layer.final_layer_norm)):
                    def capture(_module, _inputs, output, key=(index, label)):
                        traces[key] = output.detach().clone()
                    hooks.append(module.register_forward_hook(capture))
            try:
                result = encoder(input_ids=torch.tensor([ids]),
                                 attention_mask=None if mask is None else torch.tensor([mask]),
                                 output_hidden_states=True, return_dict=True)
            finally:
                for hook in hooks:
                    hook.remove()
            refs = [("input", result.hidden_states[0])]
            for index in range(6):
                for label in ("attention", "final"):
                    refs.append((f"layer_{index}_{label}", traces[(index, label)]))
            refs.append(("memory", result.last_hidden_state))
            for label, expected in refs:
                ref = expected.squeeze(0).contiguous().numpy()
                raw = proc.stdout.read(ref.nbytes)
                assert len(raw) == ref.nbytes, (case, label, len(raw), ref.nbytes)
                actual = np.frombuffer(raw, dtype="<f4").reshape(ref.shape)
                assert np.isfinite(actual).all()
                error = float(np.max(np.abs(actual.astype(np.float64) - ref)))
                print(f"REAL_ENCODER_COMPARE case={case} {label} elements={ref.size} max_abs={error:.9g}", flush=True)
                np.testing.assert_allclose(actual, ref, atol=atol, rtol=rtol, err_msg=f"{case}:{label}")
                max_abs = max(max_abs, error)
                if label == "memory":
                    max_final = max(max_final, error)
                comparisons += 1
                elements += ref.size
        assert proc.stdout.read(1) == b""
        assert proc.wait() == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
assert comparisons == 42
print(f"REAL_ENCODER_REFERENCE PASS cases=3 layers=6 comparisons={comparisons} "
      f"elements={elements} max_abs={max_abs:.9g} max_memory_abs={max_final:.9g} "
      f"atol={atol} rtol={rtol}", flush=True)

"""Independent Transformers Marian full model on the actual audited checkpoint."""
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
assert model.lm_head.weight.data_ptr() == model.model.shared.weight.data_ptr()
cases = [
    ([1, 2048, 0], None, [65000], None),
    ([2047, 2048, 4095, 4096, 0, 65000], [1, 1, 1, 1, 1, 0],
     [65000, 4, 65000, 4096], [1, 1, 0, 1]),
    ([4, 17, 28, 109, 2047, 2048, 4096, 12991, 32000, 50000, 64999, 0], None,
     [65000, 32000, 4096, 2048, 1], None),
]
# Same predeclared real-width tolerance as encoder, including propagated logits.
atol = rtol = 1e-4
comparisons = elements = argmax_rows = 0
max_abs = max_logits = 0.
with torch.no_grad(), subprocess.Popen([str(exe.resolve()), str(package)], stdout=subprocess.PIPE) as proc:
    try:
        for case, (ids, source_mask, target, target_mask) in enumerate(cases):
            traces = {}
            hooks = []
            for index, layer in enumerate(model.model.decoder.layers):
                for label, module in (("self", layer.self_attn_layer_norm),
                                      ("cross", layer.encoder_attn_layer_norm),
                                      ("final", layer.final_layer_norm)):
                    def capture(_module, _inputs, output, key=(index, label)):
                        traces[key] = output.detach().clone()
                    hooks.append(module.register_forward_hook(capture))
            try:
                result = model(input_ids=torch.tensor([ids]),
                               attention_mask=None if source_mask is None else torch.tensor([source_mask]),
                               decoder_input_ids=torch.tensor([target]),
                               decoder_attention_mask=None if target_mask is None else torch.tensor([target_mask]),
                               use_cache=False, output_hidden_states=True, return_dict=True)
            finally:
                for hook in hooks:
                    hook.remove()
            refs = [("memory", result.encoder_last_hidden_state),
                    ("input", result.decoder_hidden_states[0])]
            for index in range(6):
                for label in ("self", "cross", "final"):
                    refs.append((f"layer_{index}_{label}", traces[(index, label)]))
            refs.extend([("hidden", result.decoder_hidden_states[-1]),
                         ("logits", result.logits), ("last_logits", result.logits[:, -1:])])
            for label, expected in refs:
                ref = expected.squeeze(0).contiguous().numpy()
                raw = proc.stdout.read(ref.nbytes)
                assert len(raw) == ref.nbytes, (case, label, len(raw), ref.nbytes)
                actual = np.frombuffer(raw, dtype="<f4").reshape(ref.shape)
                assert np.isfinite(actual).all()
                error = float(np.max(np.abs(actual.astype(np.float64) - ref)))
                print(f"REAL_DECODER_COMPARE case={case} {label} elements={ref.size} max_abs={error:.9g}", flush=True)
                np.testing.assert_allclose(actual, ref, atol=atol, rtol=rtol, err_msg=f"{case}:{label}")
                if label in ("logits", "last_logits"):
                    assert np.array_equal(actual.argmax(axis=-1), ref.argmax(axis=-1)), (case, label)
                    argmax_rows += ref.shape[0]
                    max_logits = max(max_logits, error)
                max_abs = max(max_abs, error)
                comparisons += 1
                elements += ref.size
        assert proc.stdout.read(1) == b""
        assert proc.wait() == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
assert comparisons == 69 and argmax_rows == 13
print(f"REAL_DECODER_REFERENCE PASS cases=3 layers=6 comparisons={comparisons} "
      f"elements={elements} max_abs={max_abs:.9g} max_logits_abs={max_logits:.9g} "
      f"argmax_rows={argmax_rows} atol={atol} rtol={rtol}", flush=True)

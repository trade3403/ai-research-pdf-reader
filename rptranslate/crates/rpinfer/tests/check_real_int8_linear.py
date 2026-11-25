"""Independent integer block matmul on three real weight matrices."""
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
state = load_checked(source)
raw = subprocess.check_output([str(exe.resolve()), str(package)], timeout=120)
actual = np.frombuffer(raw, dtype="<f4")
offset = elements = 0
max_error = max_fp32 = 0.

def quantize(x):
    blocks = x.reshape(x.shape[0], -1, 64).astype(np.float64)
    peak = np.max(np.abs(blocks), axis=-1)
    scale = np.where(peak == 0., 1., peak/127.).astype(np.float32)
    scale = np.maximum(scale, np.nextafter(np.float32(0), np.float32(1)))
    codes = np.clip(np.rint(blocks/scale[..., None]), -127, 127).astype(np.int32)
    return codes, scale.astype(np.float64)

for name in ("model.encoder.layers.0.self_attn.q_proj",
             "model.encoder.layers.0.fc1", "model.decoder.layers.5.fc2"):
    w = state[name+".weight"].numpy().astype(np.float64)
    bias = state[name+".bias"].numpy().astype(np.float64)
    outputs, width = w.shape
    x = ((np.arange(3*width) % 31).astype(np.float64)-15)/16
    x = x.reshape(3, width)
    qa, sa = quantize(x)
    qw, sw = quantize(w)
    ref = np.zeros((3, outputs), np.float64)
    for block in range(width//64):
        dot = qa[:, block, :] @ qw[:, block, :].T
        assert np.max(np.abs(dot)) <= 64*127*127
        ref += dot.astype(np.float64) * sa[:, block, None] * sw[None, :, block]
    ref += bias
    ref = ref.astype(np.float32)
    full = actual[offset:offset+3*outputs].reshape(3, outputs)
    offset += 3*outputs
    last = actual[offset:offset+outputs]
    offset += outputs
    np.testing.assert_allclose(full, ref, atol=1e-6, rtol=1e-6)
    assert np.array_equal(last, full[-1])
    restored_a = (qa*sa[..., None]).reshape(x.shape)
    restored_w = (qw*sw[..., None]).reshape(w.shape)
    fp = x @ w.T + bias
    bound = np.abs(restored_a-x) @ np.abs(w).T + np.abs(restored_a) @ np.abs(restored_w-w).T
    assert np.all(np.abs(full-fp) <= bound + 1e-5 + 1e-6*np.abs(fp))
    error = float(np.max(np.abs(full-ref)))
    fp_error = float(np.max(np.abs(full-fp)))
    max_error = max(max_error, error)
    max_fp32 = max(max_fp32, fp_error)
    elements += full.size + last.size
    print(f"REAL_INT8_LINEAR_COMPARE name={name} shape={outputs}x{width} "
          f"reference_max_abs={error:.9g} fp32_max_abs={fp_error:.9g}", flush=True)
assert offset == actual.size == elements == 12288
print(f"REAL_INT8_LINEAR_REFERENCE PASS matrices=3 elements={elements} "
      f"reference_max_abs={max_error:.9g} fp32_max_abs={max_fp32:.9g} "
      f"atol=1e-6 rtol=1e-6", flush=True)

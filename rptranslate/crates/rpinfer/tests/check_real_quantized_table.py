"""Real shared weights, independent block quantization, gather and tied logits."""
import hashlib
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
state = load_checked(source)
weight = state["model.shared.weight"].numpy()
bias = state["final_logits_bias"].reshape(-1).numpy().copy()
del state
ids = [0, 1, 2047, 2048, 4095, 4096, 64999, 65000]
hidden = ((np.arange(1024) % 31).astype(np.float32) - 15) / 16
hidden = hidden.reshape(2, 512)
gather = np.empty((8, 512), np.float32)
ref_logits = np.empty((2, 65001), np.float32)
fp_logits = np.empty_like(ref_logits)
bound = np.empty_like(ref_logits)
max_weight_error = max_scaled_error = 0.
blocks = elements = 0
with tempfile.TemporaryDirectory() as directory:
    dest = Path(directory) / "shared.rpq"
    raw = subprocess.check_output([str(exe.resolve()), str(package), str(dest)], timeout=120)
    data = dest.read_bytes()
    magic, version, count, index_bytes, payload = struct.unpack_from("<8sIIQQ", data)
    assert (magic, version, count, payload) == (b"RPQ8BIN\0", 1, 1, 35360544)
    assert data[-64:] == hashlib.sha256(data[:-64]).hexdigest().encode()
    assert len(data) == 32 + index_bytes + payload + 64
    packed = np.frombuffer(data, dtype=np.dtype([("s", "<f4"), ("q", "i1", (64,))]),
                           count=65001*8, offset=32+index_bytes).reshape(65001, 8)
    for start in range(0, 65001, 2048):
        end = min(65001, start+2048)
        original = weight[start:end]
        chunks = original.reshape(-1, 64).astype(np.float64)
        peak = np.max(np.abs(chunks), axis=1)
        scales = np.where(peak == 0, 1., peak/127.).astype(np.float32)
        scales = np.maximum(scales, np.nextafter(np.float32(0), np.float32(1)))
        q = np.clip(np.rint(chunks / scales[:, None].astype(np.float64)), -127, 127).astype(np.int8)
        stored = packed[start:end].reshape(-1)
        assert np.array_equal(stored["s"].view(np.uint32), scales.view(np.uint32))
        assert np.array_equal(stored["q"], q)
        restored = (q.astype(np.float32) * scales[:, None]).reshape(-1, 512)
        error = np.abs(restored.astype(np.float64) - original)
        normalized_error = error.reshape(-1, 64) / scales[:, None]
        assert np.max(normalized_error) <= 0.5001
        max_scaled_error = max(max_scaled_error, float(np.max(normalized_error)))
        max_weight_error = max(max_weight_error, float(np.max(error)))
        for slot, token in enumerate(ids):
            if start <= token < end:
                gather[slot] = restored[token-start] * np.float32(512**0.5)
        ref_logits[:, start:end] = hidden @ restored.T + bias[start:end]
        fp_logits[:, start:end] = hidden @ original.T + bias[start:end]
        bound[:, start:end] = np.abs(hidden) @ error.T
        blocks += len(scales)
        elements += q.size
    assert elements == 33280512 and blocks == 520008
    values = np.frombuffer(raw, dtype="<f4")
    assert values.size == 4096 + 3*65001
    actual_gather = values[:4096].reshape(8, 512)
    actual_logits = values[4096:4096+2*65001].reshape(2, 65001)
    last = values[-65001:]
    assert np.array_equal(actual_gather, gather)
    np.testing.assert_allclose(actual_logits, ref_logits, atol=1e-5, rtol=1e-5)
    assert np.array_equal(last, actual_logits[-1])
    assert np.all(np.abs(actual_logits-fp_logits) <= bound + 1e-4)
    argmax = int(np.sum(actual_logits.argmax(axis=-1) == fp_logits.argmax(axis=-1)))
    print(f"REAL_QTABLE_REFERENCE PASS elements={elements} blocks={blocks} "
          f"codes_scales_bitwise=true gather_elements=4096 logits_elements={3*65001} "
          f"payload_bytes={payload} file_bytes={len(data)} max_weight_error={max_weight_error:.9g} "
          f"max_error_over_scale={max_scaled_error:.9g} "
          f"quantized_reference_max_abs={np.max(np.abs(actual_logits-ref_logits)):.9g} "
          f"fp32_logits_max_abs={np.max(np.abs(actual_logits-fp_logits)):.9g} "
          f"fp32_argmax={argmax}/2", flush=True)

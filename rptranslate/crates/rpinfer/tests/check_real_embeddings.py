"""Independent PyTorch comparison using original verified OPUS-MT checkpoint."""
import subprocess
import sys
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
state = load_checked(source)
weight = state["model.shared.weight"]
enc = state["model.encoder.embed_positions.weight"]
dec = state["model.decoder.embed_positions.weight"]
bias = state["final_logits_bias"].reshape(-1)
ids = torch.tensor([0, 1, 2047, 2048, 4095, 4096, 64999, 65000])
scale = 512 ** 0.5
tokens = F.embedding(ids, weight) * scale
hidden = ((torch.arange(3 * 512) % 31).float() - 15.) / 16.
hidden = hidden.reshape(3, 512)
logits = F.linear(hidden, weight) + bias
counts = {"bits": 0, "comparisons": 0, "elements": 0}
max_abs = 0.0
max_projection = 0.0

with subprocess.Popen([str(exe.resolve()), str(package)], stdout=subprocess.PIPE) as proc:
    def read(expected):
        ref = expected.detach().contiguous().numpy().astype("<f4", copy=False)
        raw = proc.stdout.read(ref.nbytes)
        assert len(raw) == ref.nbytes, (len(raw), ref.nbytes)
        return np.frombuffer(raw, dtype="<f4").reshape(ref.shape), ref

    def bits(expected):
        actual, ref = read(expected)
        assert actual.tobytes() == ref.tobytes()
        counts["bits"] += ref.size

    def close(name, expected, projection=False):
        global max_abs, max_projection
        actual, ref = read(expected)
        assert np.isfinite(actual).all()
        error = float(np.max(np.abs(actual.astype(np.float64) - ref)))
        np.testing.assert_allclose(actual, ref, atol=1e-5, rtol=1e-5, err_msg=name)
        if projection:
            assert np.array_equal(actual.argmax(axis=-1), ref.argmax(axis=-1)), name
            max_projection = max(max_projection, error)
        max_abs = max(max_abs, error)
        counts["comparisons"] += 1
        counts["elements"] += ref.size
        print(f"REAL_EMBEDDING_COMPARE {name} elements={ref.size} max_abs={error:.9g}", flush=True)

    try:
        for start in range(0, len(weight), 2048):
            bits(weight[start:start + 2048])
        for table in (enc, dec, bias):
            bits(table)
        close("scaled_tokens", tokens)
        for side, table in (("encoder", enc), ("decoder", dec)):
            for offset in (0, 37, 504):
                close(f"{side}_offset_{offset}", tokens + table[offset:offset + len(ids)])
        close("all_logits", logits, True)
        close("last_logits", logits[-1:], True)
        assert proc.stdout.read(1) == b""
        assert proc.wait() == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
assert counts["bits"] * 4 == 135479204
print(f"REAL_EMBEDDING_REFERENCE PASS blocks=32 bitwise_elements={counts['bits']} "
      f"comparisons={counts['comparisons']} computed_elements={counts['elements']} "
      f"max_abs={max_abs:.9g} max_projection_abs={max_projection:.9g} "
      "argmax_rows=4 atol=1e-5 rtol=1e-5", flush=True)

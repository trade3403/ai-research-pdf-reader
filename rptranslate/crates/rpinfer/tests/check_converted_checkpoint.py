"""Check canonical file, manifest, official reader and every Rust-decoded bit."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import torch
from safetensors import safe_open

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import (ALIASES, FILES, METADATA, checked_metadata,
                                     digest, load_checked)

source, output, exe = map(Path, sys.argv[1:4])
manifest = json.loads((output / "manifest.json").read_text(encoding="utf-8"))
assert manifest["format"] == "RPModel" and manifest["version"] == 1
assert manifest["aliases"] == ALIASES
assert manifest["tensor_count"] == 256
assert manifest["payload_bytes"] == 312033188
assert set(manifest["files"]) == set(FILES) | {"weights.safetensors"}
for name, item in manifest["files"].items():
    with (output / name).open("rb") as fp:
        assert digest(fp, 400 * 1024 * 1024) == (item["bytes"], item["sha256"])
assert checked_metadata(output) == checked_metadata(source)
state = load_checked(source)
canonical = {k: v for k, v in state.items() if k not in ALIASES}
weights = output / "weights.safetensors"
total = 0
reference_hash = hashlib.sha256()
with safe_open(str(weights), framework="pt", device="cpu") as reader:
    assert reader.metadata() == METADATA
    assert set(reader.keys()) == set(canonical)
    with subprocess.Popen([str(exe.resolve()), str(weights)],
                          stdout=subprocess.PIPE) as proc:
        try:
            for name, expected in sorted(canonical.items()):
                actual = reader.get_tensor(name)
                assert actual.shape == expected.shape and actual.dtype == expected.dtype
                assert torch.equal(actual.view(torch.int32), expected.view(torch.int32)), name
                flat = expected.reshape(-1)
                for start in range(0, flat.numel(), 1024 * 1024):
                    ref = flat[start:start + 1024 * 1024].numpy().astype("<f4", copy=False).tobytes()
                    raw = proc.stdout.read(len(ref))
                    assert raw == ref, (name, start, len(raw))
                    reference_hash.update(ref)
                    total += len(ref)
            assert proc.stdout.read(1) == b""
            assert proc.wait() == 0
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
assert total == 312033188
print(f"RPMODEL_ROUNDTRIP PASS tensors={len(canonical)} "
      f"elements={total // 4} bytes={total} bitwise=true "
      f"canonical_stream_sha256={reference_hash.hexdigest()}", flush=True)

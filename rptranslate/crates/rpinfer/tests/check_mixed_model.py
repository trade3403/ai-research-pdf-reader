"""Independent complete RPQ8/F32 conversion and fixed-profile package audit."""
import hashlib
import json
import mmap
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from safetensors import safe_open

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, package, exe = map(Path, sys.argv[1:4])
exe = exe.resolve()
state = load_checked(source)
for name in ("model.encoder.embed_tokens.weight", "model.decoder.embed_tokens.weight"):
    del state[name]
expected_q = {name for name, t in state.items()
              if name == "model.shared.weight" or
              (t.ndim == 2 and ".layers." in name and name.endswith(".weight"))}
expected_f = set(state) - expected_q
assert (len(expected_q), len(expected_f)) == (97, 159)

with tempfile.TemporaryDirectory() as temp:
    dest = Path(temp) / "mixed"
    subprocess.run([str(exe), "convert", str(package), str(dest)], check=True, timeout=120)
    subprocess.run([str(exe), "verify", str(dest)], check=True, timeout=120)
    subprocess.run([str(exe), "batch-reference", str(dest)], check=True, timeout=120)
    qpath = dest / "weights.rpq"
    for path in (package/"weights.safetensors", qpath, dest/"auxiliary.safetensors"):
        report = subprocess.check_output([str(exe), "sha-reference", str(path)],
                                         text=True, timeout=120).strip()
        assert report.startswith("SHA_READ_REFERENCE PASS ")
        fields = dict(field.split("=", 1) for field in report.split()[2:])
        expected = hashlib.sha256()
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                expected.update(chunk)
        assert int(fields["bytes"]) == path.stat().st_size
        assert fields["digest"] == expected.hexdigest()
        print(f"{report} file={path.name} hashlib_match=true", flush=True)
    q_elements = q_blocks = f_elements = 0
    with qpath.open("rb") as file, mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as data:
        magic, version, count, index_size, payload = struct.unpack_from("<8sIIQQ", data)
        assert (magic, version, count, payload) == (b"RPQ8BIN\0", 1, 97, 82153248)
        assert len(data) == 32 + index_size + payload + 64
        assert data[-64:] == hashlib.sha256(data[:-64]).hexdigest().encode()
        pos, end = 32, 0
        seen = set()
        for _ in range(count):
            name_len, rank, reserved, block, offset, size = struct.unpack_from("<HBBIQQ", data, pos)
            pos += 24
            shape = struct.unpack_from("<" + "Q"*rank, data, pos)
            pos += 8*rank
            name = data[pos:pos+name_len].decode("ascii")
            pos += name_len
            assert name in expected_q and name not in seen
            seen.add(name)
            original = state[name].numpy()
            assert tuple(original.shape) == shape and block == 64 and reserved == 0
            assert offset == end and size == original.size + original.size//64*4
            end += size
            blocks = original.reshape(-1, 64)
            packed = np.frombuffer(data, dtype=np.dtype([("s", "<f4"), ("q", "i1", (64,))]),
                                   count=len(blocks), offset=32+index_size+offset)
            for start in range(0, len(blocks), 16384):
                raw = blocks[start:start+16384].astype(np.float64)
                peak = np.max(np.abs(raw), axis=1)
                scale = np.where(peak == 0., 1., peak/127.).astype(np.float32)
                scale = np.maximum(scale, np.nextafter(np.float32(0), np.float32(1)))
                q = np.clip(np.rint(raw/scale[:, None]), -127, 127).astype(np.int8)
                stored = packed[start:start+len(raw)]
                assert np.array_equal(stored["s"].view(np.uint32), scale.view(np.uint32)), name
                assert np.array_equal(stored["q"], q), name
                del stored
                q_elements += q.size
                q_blocks += len(scale)
            del packed
        assert seen == expected_q and pos == 32+index_size and end == payload
    with safe_open(str(dest/"auxiliary.safetensors"), framework="np") as f:
        assert set(f.keys()) == expected_f
        for name in f.keys():
            actual = f.get_tensor(name)
            original = state[name].numpy()
            assert actual.shape == original.shape and actual.dtype == np.float32
            assert np.array_equal(actual.view(np.uint32), original.view(np.uint32)), name
            f_elements += actual.size
    assert (q_elements, q_blocks, f_elements) == (77320704, 1208136, 687593)
    for name in ("config.json", "generation_config.json", "source.spm", "target.spm", "vocab.json"):
        assert (dest/name).read_bytes() == (package/name).read_bytes()
    total = sum(p.stat().st_size for p in dest.iterdir())
    # Test integrity failures in place; restore exact bytes after each case.
    def reject(path, offset):
        with path.open("r+b") as handle:
            handle.seek(offset)
            old = handle.read(1)
            handle.seek(offset)
            handle.write(bytes([old[0]^1]))
        try:
            result = subprocess.run([str(exe), "verify", str(dest)], capture_output=True, timeout=120)
            assert result.returncode != 0, path.name
        finally:
            with path.open("r+b") as handle:
                handle.seek(offset)
                handle.write(old)
    reject(dest/"mixed.manifest", 0)
    reject(dest/"config.json", 0)
    reject(qpath, qpath.stat().st_size//2)
    reject(dest/"auxiliary.safetensors", 100)
    subprocess.run([str(exe), "verify", str(dest)], check=True, timeout=120)
    print(f"MIXED_MODEL_REFERENCE PASS q_tensors=97 f_tensors=159 q_elements={q_elements} "
          f"q_blocks={q_blocks} f_elements={f_elements} bitwise=true payload_bytes=84903620 "
          f"package_bytes={total} negatives=4", flush=True)

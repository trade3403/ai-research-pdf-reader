"""Offline conversion of the exact audited OPUS-MT revision to RPModel v1.

This development tool requires torch==2.6.0 and safetensors==0.7.0.
It does not download anything and never uses unrestricted pickle loading.
Input files must be stable during conversion; run in a private local directory.
"""
import hashlib
import json
import shutil
import sys
import tempfile
from pathlib import Path

import torch
import safetensors
from safetensors.torch import save_file

REVISION = "a4193836671069f1f80ce341f9227a850ffb52d4"
SOURCE = "Helsinki-NLP/opus-mt-en-zh"
CHECKPOINT_SHA = "69a1d6ec829cee349360b3b677ac0aa99a7d88822d1a6578370029efffdca3f5"
FILES = {
    "config.json": (1403, "bcae8ed74fed77fb51c58462b62397fee6b1a1a34aece79183a0dd02ad329e71"),
    "generation_config.json": (293, "524dc91570823feb51fbd33a35e5aee3050a4441bdf65e66f6f5e945e5838e40"),
    "source.spm": (806435, "5775ddc9e3ff2fae91554da56468ad35ff56edaba870fea74447bc7234bfdaa8"),
    "target.spm": (804600, "81dc94efa84e4025ef38d25d5d07429fe41e3eb29d44003f1db6fe98487b0052"),
    "vocab.json": (1617791, "f1832ca38f7aa158b9a944ff583e4901e6b37d9d4ad9241f03838e636e9cfb03"),
}
ALIASES = {
    "model.encoder.embed_tokens.weight": "model.shared.weight",
    "model.decoder.embed_tokens.weight": "model.shared.weight",
}
METADATA = {
    "format": "pt",
    "rpmodel_format": "1",
    "source_revision": REVISION,
    "source_sha256": CHECKPOINT_SHA,
}


def digest(fp, limit):
    h = hashlib.sha256()
    n = 0
    while chunk := fp.read(1024 * 1024):
        n += len(chunk)
        if n > limit:
            raise ValueError("file exceeds size limit")
        h.update(chunk)
    return n, h.hexdigest()


def checked_metadata(root):
    result = {}
    for name, (size, expected) in FILES.items():
        with (root / name).open("rb") as fp:
            data = fp.read(size + 1)
        if len(data) != size or hashlib.sha256(data).hexdigest() != expected:
            raise ValueError("metadata checksum mismatch: " + name)
        result[name] = data
    return result


def expected_shapes():
    shapes = {
        "model.shared.weight": (65001, 512),
        "final_logits_bias": (1, 65001),
    }
    for side in ("encoder", "decoder"):
        prefix = "model." + side + "."
        shapes[prefix + "embed_positions.weight"] = (512, 512)
        shapes[prefix + "embed_tokens.weight"] = (65001, 512)
        for i in range(6):
            layer = prefix + f"layers.{i}."
            for attn in (["self_attn"] if side == "encoder" else ["self_attn", "encoder_attn"]):
                for projection in ("q_proj", "k_proj", "v_proj", "out_proj"):
                    name = layer + attn + "." + projection
                    shapes[name + ".weight"] = (512, 512)
                    shapes[name + ".bias"] = (512,)
                shapes[layer + attn + "_layer_norm.weight"] = (512,)
                shapes[layer + attn + "_layer_norm.bias"] = (512,)
            shapes[layer + "fc1.weight"] = (2048, 512)
            shapes[layer + "fc1.bias"] = (2048,)
            shapes[layer + "fc2.weight"] = (512, 2048)
            shapes[layer + "fc2.bias"] = (512,)
            shapes[layer + "final_layer_norm.weight"] = (512,)
            shapes[layer + "final_layer_norm.bias"] = (512,)
    return shapes


def load_checked(root):
    if torch.__version__.split("+")[0] != "2.6.0" or safetensors.__version__ != "0.7.0":
        raise ValueError("use the pinned conversion environment")
    torch.set_num_threads(1)
    with (root / "pytorch_model.bin").open("rb") as fp:
        if digest(fp, 312087009) != (312087009, CHECKPOINT_SHA):
            raise ValueError("checkpoint checksum mismatch")
        fp.seek(0)
        state = torch.load(fp, map_location="cpu", weights_only=True)
    shapes = expected_shapes()
    if not isinstance(state, dict) or state.keys() != shapes.keys():
        raise ValueError("unexpected checkpoint keys")
    for name, shape in shapes.items():
        value = state[name]
        if (not isinstance(value, torch.Tensor) or tuple(value.shape) != shape
                or value.dtype != torch.float32 or not value.is_contiguous()
                or not bool(torch.isfinite(value).all())):
            raise ValueError("invalid tensor: " + name)
    for alias, canonical in ALIASES.items():
        # Compare bits, including signed zero, not merely numeric equality.
        if not torch.equal(state[alias].view(torch.int32), state[canonical].view(torch.int32)):
            raise ValueError("shared weight mismatch: " + alias)
    return state


def convert(source, output):
    if output.exists() or output.is_symlink():
        raise FileExistsError(output)
    metadata = checked_metadata(source)
    state = load_checked(source)
    canonical = {k: v for k, v in sorted(state.items()) if k not in ALIASES}
    payload = sum(v.numel() * 4 for v in canonical.values())
    if len(canonical) != 256 or payload != 312033188:
        raise ValueError("unexpected canonical size")
    output.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".rpmodel-", dir=output.parent))
    try:
        for name, data in metadata.items():
            (stage / name).write_bytes(data)
        weights = stage / "weights.safetensors"
        save_file(canonical, str(weights), metadata=METADATA)
        manifest_files = {}
        for name in sorted([*metadata, weights.name]):
            with (stage / name).open("rb") as fp:
                n, sha = digest(fp, 400 * 1024 * 1024)
            manifest_files[name] = {"bytes": n, "sha256": sha}
        manifest = {
            "format": "RPModel", "version": 1, "source": SOURCE,
            "revision": REVISION, "license": "Apache-2.0",
            "checkpoint_sha256": CHECKPOINT_SHA, "dtype": "F32",
            "tensor_count": len(canonical), "payload_bytes": payload,
            "aliases": ALIASES, "files": manifest_files,
        }
        (stage / "manifest.json").write_text(
            json.dumps(manifest, sort_keys=True, indent=2) + "\n", encoding="utf-8"
        )
        if output.exists() or output.is_symlink():
            raise FileExistsError(output)
        stage.rename(output)
        print(f"RPMODEL_CONVERT PASS tensors={len(canonical)} aliases={len(ALIASES)} "
              f"payload_bytes={payload} file_bytes={manifest_files[weights.name]['bytes']} "
              f"sha256={manifest_files[weights.name]['sha256']}", flush=True)
    finally:
        if stage.exists():
            shutil.rmtree(stage)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: convert_opus_checkpoint.py INPUT_DIR NEW_OUTPUT_DIR")
    convert(Path(sys.argv[1]), Path(sys.argv[2]))

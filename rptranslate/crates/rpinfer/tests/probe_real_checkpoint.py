"""D1.8a: bounded inspection of one hash-verified official model."""
import hashlib
import json
import sys
import time
from pathlib import Path
from urllib.request import urlopen

import torch

REV = "a4193836671069f1f80ce341f9227a850ffb52d4"
BASE = "https://huggingface.co/Helsinki-NLP/opus-mt-en-zh/resolve/" + REV + "/"
EXPECTED = {
    "source.spm": "5775ddc9e3ff2fae91554da56468ad35ff56edaba870fea74447bc7234bfdaa8",
    "target.spm": "81dc94efa84e4025ef38d25d5d07429fe41e3eb29d44003f1db6fe98487b0052",
    "vocab.json": "f1832ca38f7aa158b9a944ff583e4901e6b37d9d4ad9241f03838e636e9cfb03",
    "pytorch_model.bin": "69a1d6ec829cee349360b3b677ac0aa99a7d88822d1a6578370029efffdca3f5",
}
root = Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=True)


def download(name, limit):
    path = root / name
    digest = hashlib.sha256()
    size = 0
    deadline = time.monotonic() + 150
    with urlopen(BASE + name, timeout=30) as response, path.open("wb") as fp:
        while chunk := response.read(1024 * 1024):
            size += len(chunk)
            if size > limit or time.monotonic() > deadline:
                raise ValueError("download limit exceeded: " + name)
            digest.update(chunk)
            fp.write(chunk)
    sha = digest.hexdigest()
    if name in EXPECTED and sha != EXPECTED[name]:
        raise ValueError(f"checksum mismatch {name}: {sha}")
    print(f"VERIFIED_FILE {name} bytes={size} sha256={sha}", flush=True)
    return path


cfg = json.loads(download("config.json", 65536).read_bytes())
gen = json.loads(download("generation_config.json", 65536).read_bytes())
for name in ("source.spm", "target.spm", "vocab.json"):
    download(name, 16 * 1024 * 1024)
vocab = json.loads((root / "vocab.json").read_bytes())
for field, expected in {
    "model_type": "marian", "vocab_size": 65001, "d_model": 512,
    "encoder_layers": 6, "decoder_layers": 6,
    "encoder_attention_heads": 8, "decoder_attention_heads": 8,
    "encoder_ffn_dim": 2048, "decoder_ffn_dim": 2048,
    "max_position_embeddings": 512, "activation_function": "swish",
    "scale_embedding": True, "decoder_start_token_id": 65000,
    "pad_token_id": 65000, "eos_token_id": 0,
}.items():
    assert cfg[field] == expected, (field, cfg[field])
assert len(vocab) == 65001 and vocab["<pad>"] == 65000 and vocab["</s>"] == 0
assert gen["num_beams"] == 4 and gen["max_length"] == 512
print("ARCHITECTURE", json.dumps(cfg, sort_keys=True), flush=True)
checkpoint = download("pytorch_model.bin", 400 * 1024 * 1024)
assert torch.__version__.split("+")[0] == "2.6.0"
torch.set_num_threads(1)
# No unrestricted pickle fallback. Only the verified official checkpoint.
state = torch.load(checkpoint, map_location="cpu", weights_only=True)
assert isinstance(state, dict) and len(state) <= 1000
storages = {}
logical = 0
for key, tensor in sorted(state.items()):
    assert isinstance(key, str) and isinstance(tensor, torch.Tensor)
    assert tensor.dtype == torch.float32 and tensor.is_contiguous(), key
    assert bool(torch.isfinite(tensor).all()), key
    storage = tensor.untyped_storage()
    storages[storage.data_ptr()] = storage.nbytes()
    logical += tensor.numel()
    print(f"WEIGHT {key} shape={list(tensor.shape)} dtype={tensor.dtype}", flush=True)
assert state["model.shared.weight"].shape == (65001, 512)
assert state["final_logits_bias"].shape == (1, 65001)
for side in ("encoder", "decoder"):
    for i in range(6):
        prefix = f"model.{side}.layers.{i}."
        assert state[prefix + "fc1.weight"].shape == (2048, 512)
        assert state[prefix + "fc2.weight"].shape == (512, 2048)
        for name in ("q_proj", "k_proj", "v_proj", "out_proj"):
            assert state[prefix + "self_attn." + name + ".weight"].shape == (512, 512)
            if side == "decoder":
                assert state[prefix + "encoder_attn." + name + ".weight"].shape == (512, 512)
print(f"CHECKPOINT_REFERENCE PASS tensors={len(state)} logical_elements={logical} "
      f"unique_storage_bytes={sum(storages.values())} shared_embedding_bytes={65001*512*4}", flush=True)

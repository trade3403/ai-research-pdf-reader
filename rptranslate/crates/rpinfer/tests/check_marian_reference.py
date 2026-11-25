"""CI-only independent PyTorch/Transformers tiny Marian forward comparison."""
import sys
import subprocess
from pathlib import Path

import numpy as np
import torch
import transformers
from transformers import MarianConfig, MarianMTModel

assert torch.__version__.split("+")[0] == "2.6.0"
assert transformers.__version__ == "4.48.3"
torch.manual_seed(612)
torch.set_num_threads(1)
cfg = MarianConfig(
    vocab_size=17, decoder_vocab_size=17, d_model=8,
    encoder_layers=2, decoder_layers=2, encoder_attention_heads=2,
    decoder_attention_heads=2, encoder_ffn_dim=13, decoder_ffn_dim=13,
    max_position_embeddings=16, activation_function="relu",
    dropout=0., attention_dropout=0., activation_dropout=0.,
    init_std=.15, pad_token_id=16, decoder_start_token_id=16,
    eos_token_id=0, scale_embedding=True, tie_word_embeddings=True,
    share_encoder_decoder_embeddings=True, use_cache=False,
    attn_implementation="eager",
)
model = MarianMTModel(cfg).eval()
with torch.no_grad():
    # Nontrivial affine norms, projection biases, and final logits bias.
    for name, param in model.named_parameters():
        if "layer_norm.weight" in name:
            param.copy_(.8 + .4 * torch.rand_like(param))
        elif name.endswith(".bias"):
            param.copy_(.1 * torch.randn_like(param))
    model.final_logits_bias.copy_(.1 * torch.randn_like(model.final_logits_bias))
assert model.model.shared.weight.data_ptr() == model.lm_head.weight.data_ptr()
assert model.model.shared.weight.data_ptr() == model.model.encoder.embed_tokens.weight.data_ptr()
assert model.model.shared.weight.data_ptr() == model.model.decoder.embed_tokens.weight.data_ptr()
path = Path(sys.argv[1])
exe = Path(sys.argv[2]).resolve()
with path.open("w") as fp:
    for name, tensor in model.state_dict().items():
        if name != "model.shared.weight" and name != "final_logits_bias" and ".layers." not in name:
            continue
        a = tensor.detach().cpu().numpy()
        if name == "final_logits_bias":
            a = a.reshape(-1)
        fp.write(name + "|" + ",".join(map(str, a.shape)) + "|" +
                 ",".join(str(float(v)) for v in a.ravel()) + "\n")

trace = {}
def hook(name):
    def save(module, args, out):
        trace[name] = out.detach().cpu().numpy()[0]
    return save

for i, layer in enumerate(model.model.encoder.layers):
    for field, name in [("self_attn_layer_norm", "self_norm"),
                        ("fc2", "ffn"), ("final_layer_norm", "output")]:
        getattr(layer, field).register_forward_hook(hook(f"enc.{i}.{name}"))
for i, layer in enumerate(model.model.decoder.layers):
    for field, name in [("self_attn_layer_norm", "self_norm"),
                        ("encoder_attn_layer_norm", "cross_norm"),
                        ("fc2", "ffn"), ("final_layer_norm", "output")]:
        getattr(layer, field).register_forward_hook(hook(f"dec.{i}.{name}"))

cases = [
    ([2,5,16], [1,1,0], [16,4,7], [1,0,1]),
    ([3], [1], [16], [1]),
    ([2,8,9,0], [1,1,1,1], [16,6], [1,1]),
]
total = elements = 0
overall = relative = 0.
for case, (src, sm, tgt, tm) in enumerate(cases):
    trace.clear()
    with torch.inference_mode():
        result = model(input_ids=torch.tensor([src]), attention_mask=torch.tensor([sm]),
                       decoder_input_ids=torch.tensor([tgt]),
                       decoder_attention_mask=torch.tensor([tm]),
                       output_hidden_states=True, use_cache=False, return_dict=True)
    trace["enc.embedding"] = result.encoder_hidden_states[0][0].numpy()
    trace["dec.embedding"] = result.decoder_hidden_states[0][0].numpy()
    trace["logits"] = result.logits[0].numpy()
    trace["next_logits"] = result.logits[0,-1:].numpy()
    trace["probabilities"] = torch.softmax(result.logits[0,-1:], dim=-1).numpy()
    for kernel in ("scalar", "blocked", "auto", "parallel"):
        stdout = subprocess.run([str(exe), str(path), str(case), kernel],
                                check=True, text=True, capture_output=True).stdout
        actual = {}
        for line in stdout.splitlines():
            key, shape, values = line.split("|")
            assert key not in actual
            actual[key] = np.array([float(v) for v in values.split(",")],
                                   np.float32).reshape(tuple(map(int, shape.split(","))))
        assert set(actual) == set(trace), (set(actual), set(trace))
        for name, expected in trace.items():
            a = actual[name]
            assert a.shape == expected.shape and np.isfinite(a).all()
            assert np.isfinite(expected).all()
            diff = np.abs(a.astype(np.float64)-expected.astype(np.float64))
            err = float(diff.max())
            rel = float((diff / np.maximum(np.abs(expected), 1e-6)).max())
            print(f"case={case} kernel={kernel} stage={name} max_abs={err:.9g} max_rel_floor={rel:.9g}", flush=True)
            np.testing.assert_allclose(a, expected, atol=1e-5, rtol=1e-5,
                                       err_msg=f"{case}/{kernel}/{name}")
            overall = max(overall, err)
            relative = max(relative, rel)
            total += 1
            elements += a.size
        assert np.argmax(actual["next_logits"]) == np.argmax(trace["next_logits"])
print(f"MARIAN_REFERENCE PASS comparisons={total} elements={elements} max_abs={overall:.9g} max_rel_floor={relative:.9g}", flush=True)

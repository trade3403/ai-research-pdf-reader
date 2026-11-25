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
    trace["cached_logits"] = result.logits[0].numpy()
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


# Independent exhaustive per-depth candidate expansion. Unlike Rust partial
# selection, enumerate every admissible continuation before global sorting.
# Uses the independent torch model, not Rust logits.
from functools import lru_cache
from transformers import GenerationConfig

generation_checks = hypotheses = hf_greedy_checks = 0
max_score_error = 0.
for case, (src, sm, _, _) in enumerate(cases):
    src_t = torch.tensor([src])
    sm_t = torch.tensor([sm])
    with torch.inference_mode():
        enc = model.model.encoder(input_ids=src_t, attention_mask=sm_t,
                                  return_dict=True)

    @lru_cache(maxsize=None)
    def log_probs(prefix):
        with torch.inference_mode():
            result = model(encoder_outputs=enc, attention_mask=sm_t,
                           decoder_input_ids=torch.tensor([prefix]),
                           use_cache=False, return_dict=True)
            return torch.log_softmax(result.logits[0, -1].double(), -1).tolist()

    def reference(width, setting):
        beam = [((16,), 0., 0., False)]
        for step in range(1, 5 + setting):
            candidates = []
            for tokens, lp, score, ended in beam:
                if ended:
                    candidates.append((tokens, lp, score, ended))
                    continue
                for token, delta in enumerate(log_probs(tokens)):
                    if token == 16 or (token == 0 and step < 2):
                        continue
                    if setting == 0 and step == 4 and token != 0:
                        continue
                    total_lp = lp + delta
                    candidates.append((tokens + (token,), total_lp,
                                       total_lp / step**setting, token == 0))
            beam = sorted(candidates, key=lambda h: (-h[2], h[0]))[:width]
            if all(h[3] for h in beam):
                break
        return beam

    stdout = subprocess.run([str(exe), str(path), str(case), "auto", "generate"],
                            check=True, text=True, capture_output=True).stdout
    actual = {}
    for line in stdout.splitlines():
        setting, width, mode, rank, tokens, lp, score, ended = line.split("|")
        key = (int(setting), int(width), mode)
        actual.setdefault(key, []).append(
            (int(rank), tuple(map(int, tokens.split(","))), float(lp),
             float(score), bool(int(ended))))
    assert set(actual) == {(s, w, m) for s in (0, 1) for w in (1, 2, 4)
                           for m in ("full", "cached")}
    for setting in (0, 1):
        for width in (1, 2, 4):
            expected = reference(width, setting)
            for mode in ("full", "cached"):
                rows = actual[(setting, width, mode)]
                assert len(rows) == len(expected)
                for rank, (a, b) in enumerate(zip(rows, expected)):
                    assert a[0] == rank and a[1] == b[0] and a[4] == b[3], (case, setting, width, mode, a, b)
                    np.testing.assert_allclose(a[2:4], b[1:3], atol=1e-5, rtol=1e-5)
                    max_score_error = max(max_score_error, abs(a[2]-b[1]), abs(a[3]-b[2]))
                    hypotheses += 1
                generation_checks += 1
            if width == 1 and setting == 0:
                gc = GenerationConfig(
                    decoder_start_token_id=16, pad_token_id=16, eos_token_id=0,
                    forced_eos_token_id=0, min_length=2, max_length=5,
                    num_beams=1, do_sample=False, suppress_tokens=[16], use_cache=False)
                with torch.inference_mode():
                    hf = tuple(model.generate(input_ids=src_t, attention_mask=sm_t,
                                              generation_config=gc)[0].tolist())
                assert hf == expected[0][0], (case, hf, expected)
                hf_greedy_checks += 1
    log_probs.cache_clear()
print(f"GENERATION_REFERENCE PASS cases={generation_checks} hypotheses={hypotheses} "
      f"hf_greedy={hf_greedy_checks} max_score_error={max_score_error:.9g}", flush=True)

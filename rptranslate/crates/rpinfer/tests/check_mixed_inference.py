"""Independent mixed Marian forward reference, integer block math and search.
FP32 differences are observations, not equality gates against quantized weights.
"""
import json
import subprocess
import sys
from functools import lru_cache
from pathlib import Path
import numpy as np
import torch
from transformers import MarianTokenizer

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "tools"))
from convert_opus_checkpoint import load_checked

source, packed, fp32, exe = map(Path, sys.argv[1:5])
state = load_checked(source)
tokenizer = MarianTokenizer(source_spm=str(source/"source.spm"),
                            target_spm=str(source/"target.spm"),
                            vocab=str(source/"vocab.json"), source_lang="en", target_lang="zh")
text = "The result is stable."
tokenizer._switch_to_input_mode()
real_ids = tokenizer.build_inputs_with_special_tokens(
    tokenizer.convert_tokens_to_ids(tokenizer._tokenize(">>cmn_Hans<< " + text)))
def quantize(x):
    b = x.reshape(x.shape[0], -1, 64).astype(np.float64)
    peak = np.max(np.abs(b), axis=-1)
    s = np.where(peak == 0, 1., peak/127.).astype(np.float32)
    s = np.maximum(s, np.nextafter(np.float32(0), np.float32(1)))
    q = np.clip(np.rint(b/s[..., None]), -127, 127).astype(np.int32)
    return q, s
@lru_cache(maxsize=None)
def weight(name):
    return quantize(state[name].numpy())
def linear(x, name, head=False):
    a, sa = quantize(x)
    w, sw = weight("model.shared.weight" if head else name+".weight")
    y = np.zeros((len(x), len(w)), np.float64)
    for b in range(a.shape[1]):
        dot = a[:, b, :] @ w[:, b, :].T
        y += dot.astype(np.float64) * sa[:, b, None].astype(np.float64) * sw[None, :, b].astype(np.float64)
    y += state["final_logits_bias" if head else name+".bias"].numpy().reshape(-1)
    return y.astype(np.float32)
def input_rows(ids, side):
    q, s = weight("model.shared.weight")
    x = (q[ids].astype(np.float32)*s[ids, :, None]).reshape(len(ids), 512)
    return x*np.float32(512**.5) + state[f"model.{side}.embed_positions.weight"].numpy()[:len(ids)]
def norm(x, branch, name):
    v = (x+branch).astype(np.float64)
    mean = v.mean(axis=1, keepdims=True)
    var = ((v-mean)**2).mean(axis=1, keepdims=True)
    return ((v-mean)/np.sqrt(var+float(np.float32(1e-5)))
            *state[name+".weight"].numpy()+state[name+".bias"].numpy()).astype(np.float32)
def attention(x, mem, name, keep, causal=False):
    q = linear(x, name+".q_proj").reshape(-1,8,64).transpose(1,0,2)
    k = linear(mem, name+".k_proj").reshape(-1,8,64).transpose(1,0,2)
    v = linear(mem, name+".v_proj").reshape(-1,8,64).transpose(1,0,2)
    # FP32 increasing-K summation, independent array implementation.
    scores = np.zeros((8,len(x),len(mem)), np.float32)
    for j in range(64):
        scores += q[:,:,j,None]*k[:,None,:,j]
    scores *= np.float32(0.125)
    allowed = np.broadcast_to(np.asarray(keep, bool), (len(x),len(mem))).copy()
    if causal:
        allowed &= np.arange(len(mem))[None,:] <= np.arange(len(x))[:,None]
    z = np.where(allowed[None,:,:], scores.astype(np.float64), -np.inf)
    exp = np.exp(z-z.max(axis=-1, keepdims=True))
    prob = (exp/exp.sum(axis=-1, keepdims=True)).astype(np.float32)
    result = np.zeros((8,len(x),64), np.float32)
    for j in range(len(mem)):
        result += prob[:,:,j,None]*v[:,None,j,:]
    return linear(result.transpose(1,0,2).reshape(len(x),512), name+".out_proj")
def finish(x, p):
    a = linear(x, p+".fc1").astype(np.float64)
    return norm(x, linear((a/(1+np.exp(-a))).astype(np.float32), p+".fc2"), p+".final_layer_norm")
def encode(ids):
    keep = [v != 65000 for v in ids]
    x = input_rows(ids, "encoder")
    for i in range(6):
        p = f"model.encoder.layers.{i}"
        x = norm(x, attention(x,x,p+".self_attn",keep),p+".self_attn_layer_norm")
        x = finish(x,p)
    return x, keep
def decode(ids, memory, source_keep, keep=None, last=False):
    if keep is None:
        keep = [True]*len(ids)
    x = input_rows(list(ids), "decoder")
    for i in range(6):
        p = f"model.decoder.layers.{i}"
        x = norm(x,attention(x,x,p+".self_attn",keep,True),p+".self_attn_layer_norm")
        x = norm(x,attention(x,memory,p+".encoder_attn",source_keep),p+".encoder_attn_layer_norm")
        x = finish(x,p)
    return linear(x[-1:] if last else x, "", True)

cases = [
    ([2047,2048,4096,0,65000],[65000,4,65000,4096],[True,True,False,True]),
    (real_ids,[65000,4,4096],None),
]
comparisons = elements = hypotheses = 0
max_abs = max_score_error = 0.
with subprocess.Popen([str(exe.resolve()), str(packed), str(fp32)], stdout=subprocess.PIPE) as proc:
    def read_line():
        line = proc.stdout.readline().decode("ascii").strip()
        assert line, "unexpected EOF"
        return line
    def compare(expected, label):
        global comparisons, elements, max_abs
        raw = proc.stdout.read(expected.nbytes)
        assert len(raw) == expected.nbytes, label
        actual = np.frombuffer(raw, dtype="<f4").reshape(expected.shape)
        assert np.isfinite(actual).all()
        err = float(np.max(np.abs(actual-expected)))
        print(f"MIXED_COMPARE {label} elements={actual.size} max_abs={err:.9g}", flush=True)
        np.testing.assert_allclose(actual, expected, atol=1e-4, rtol=1e-4, err_msg=label)
        comparisons += 1
        elements += actual.size
        max_abs = max(max_abs, err)
        return actual
    try:
        for case, (src,prefix,mask) in enumerate(cases):
            kind, ids = read_line().split("|")
            assert kind == "SRC" and list(map(int,ids.split(","))) == src
            memory, keep = encode(src)
            logits = decode(prefix,memory,keep,mask)
            compare(memory,f"{case}/encoder")
            a = compare(logits,f"{case}/full")
            b = compare(logits,f"{case}/cached")
            c = compare(logits,f"{case}/scalar")
            assert np.array_equal(a.view(np.uint32),b.view(np.uint32))
            assert np.array_equal(a.view(np.uint32),c.view(np.uint32))
            assert np.array_equal(a.argmax(axis=-1),logits.argmax(axis=-1))
            @lru_cache(maxsize=None)
            def log_probs(tokens):
                out = decode(tokens,memory,keep,last=True)[0].astype(np.float64)
                out -= out.max()
                return out-np.log(np.exp(out).sum())
            def generate(width, limit):
                beams = [((65000,),0.,0.,False)]
                for step in range(1,limit+1):
                    candidates = []
                    for tokens,lp,score,ended in beams:
                        if ended:
                            candidates.append((tokens,lp,score,ended))
                            continue
                        probs = log_probs(tokens)
                        legal = [0] if step == limit else [int(t) for t in np.argsort(-probs,kind="stable") if t != 65000][:width]
                        for token in legal:
                            total = lp+float(probs[token])
                            candidates.append((tokens+(token,),total,total/step,token==0))
                    beams = sorted(candidates,key=lambda h:(-h[2],h[0]))[:width]
                    if all(h[3] for h in beams):
                        break
                return beams
            for width in (1,4):
                expected = generate(width,4)
                for rank,h in enumerate(expected):
                    tag,w,i,t,lp,score,ended = read_line().split("|")
                    tokens = tuple(map(int,t.split(",")))
                    assert (tag,int(w),int(i),tokens,bool(int(ended))) == ("GEN",width,rank,h[0],h[3])
                    np.testing.assert_allclose([float(lp),float(score)],h[1:3],atol=1e-4,rtol=1e-4)
                    max_score_error = max(max_score_error,abs(float(lp)-h[1]),abs(float(score)-h[2]))
                    hypotheses += 1
            assert read_line() == "END"
            log_probs.cache_clear()
        expected = generate(1,32)[0]
        for mode in ("mixed","fp32"):
            tag,name,t,ended,hex_text = read_line().split("|")
            tokens = tuple(map(int,t.split(",")))
            actual = bytes.fromhex(hex_text).decode("utf-8")
            assert tag == "TEXT" and name == mode and ended == "1" and len(tokens)<33
            assert actual == tokenizer.decode(list(tokens),skip_special_tokens=True,clean_up_tokenization_spaces=False)
            assert any("\u4e00" <= char <= "\u9fff" for char in actual)
            if mode == "mixed":
                assert tokens == expected[0] and expected[3]
                mixed_tokens = tokens
            else:
                print(f"MIXED_FP32_TEXT token_equal={tokens==mixed_tokens}",flush=True)
            print("MIXED_TEXT "+json.dumps({"mode":mode,"source":text,"target":actual,"tokens":len(tokens)-1},ensure_ascii=False),flush=True)
        assert proc.stdout.read(1) == b"" and proc.wait() == 0
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
assert comparisons == 8 and hypotheses == 10
print(f"MIXED_INFERENCE_REFERENCE PASS cases=2 comparisons={comparisons} elements={elements} "
      f"hypotheses={hypotheses} text_cases=1 max_abs={max_abs:.9g} "
      f"max_score_error={max_score_error:.9g} cache_scalar_bitwise=true",flush=True)

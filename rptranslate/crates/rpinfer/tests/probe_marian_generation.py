"""Probe pinned public metadata only; no neural weights or inference."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from urllib.request import urlopen
from urllib.error import HTTPError

from transformers import GenerationConfig, MarianConfig

base = "https://huggingface.co/Helsinki-NLP/opus-mt-en-zh/resolve/a4193836671069f1f80ce341f9227a850ffb52d4/"

def fetch(name, optional=False):
    try:
        with urlopen(base + name, timeout=40) as response:
            data = response.read(65537)
    except HTTPError as exc:
        if optional and exc.code == 404:
            print(f"{name}: absent at pinned revision (404)")
            return None
        raise
    assert len(data) <= 65536
    print(f"{name} sha256={hashlib.sha256(data).hexdigest()}")
    return json.loads(data)

config = fetch("config.json")
explicit = fetch("generation_config.json")
g = (GenerationConfig.from_model_config(MarianConfig.from_dict(config))
     if explicit is None else GenerationConfig.from_dict(explicit))
print("GENERATION_METADATA", json.dumps(g.to_diff_dict(), sort_keys=True))
# Fail closed for non-default generation processors not implemented here.
assert not g.do_sample and g.num_beam_groups == 1
assert not g.renormalize_logits, "post-processor renormalization is not implemented"
assert g.repetition_penalty == 1 and g.encoder_repetition_penalty == 1
assert g.no_repeat_ngram_size == 0 and g.encoder_no_repeat_ngram_size == 0
assert g.diversity_penalty == 0 and not g.constraints and not g.force_words_ids
assert g.forced_bos_token_id is None and not g.begin_suppress_tokens
assert g.max_new_tokens is None and g.min_new_tokens is None
assert g.num_return_sequences == 1
assert all(isinstance(x, int) for x in
           (g.decoder_start_token_id, g.eos_token_id, g.pad_token_id))
assert g.forced_eos_token_id is None or isinstance(g.forced_eos_token_id, int)
bad = g.bad_words_ids or []
assert all(len(x) == 1 for x in bad), "multi-token bad words not supported"
suppressed = sorted(set((g.suppress_tokens or []) + [x[0] for x in bad
                                                    if x[0] != g.eos_token_id]))
vocab = config.get("decoder_vocab_size") or config["vocab_size"]
args = [vocab, g.decoder_start_token_id, g.eos_token_id, g.pad_token_id,
        g.num_beams, g.min_length, g.max_length, g.length_penalty,
        g.forced_eos_token_id, ",".join(map(str, suppressed))]
subprocess.run([str(Path(sys.argv[1]).resolve()), *map(str, args)], check=True)
print(f"POLICY_DIFFERENCE hf_early_stopping={g.early_stopping!r}; "
      "custom joint live/completed beam pool, original forced EOS log probability; "
      "not HF beam scorer equivalence", flush=True)

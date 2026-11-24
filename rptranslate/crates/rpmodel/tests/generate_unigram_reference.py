"""Reference generation only; requires sentencepiece==0.2.1, not used at runtime."""
from pathlib import Path

import sentencepiece as sp
from sentencepiece import sentencepiece_model_pb2 as pb

model = pb.ModelProto()
t = model.trainer_spec
t.model_type = pb.TrainerSpec.UNIGRAM
t.unk_id, t.bos_id, t.eos_id, t.pad_id = 0, 1, 2, 3
n = model.normalizer_spec
n.name = "identity"
n.add_dummy_prefix = True
n.remove_extra_whitespaces = True
n.escape_whitespaces = True
for line in Path(__file__).with_name("unigram_reference.tsv").read_text(
    encoding="utf-8"
).splitlines():
    i, kind, score, piece = line.split("\t", 3)
    assert int(i) == len(model.pieces)
    p = model.pieces.add()
    p.piece, p.score, p.type = piece, float(score), int(kind)
t.vocab_size = len(model.pieces)
s = sp.SentencePieceProcessor(model_proto=model.SerializeToString())
for text in ["", "  ab  学术。  ", "a b", "ab12", "学术。", "é β", "a😀🧪b",
             "a\tb", "  ", "a\nb", "▁ab"]:
    ids = s.encode(text, out_type=int)
    print(repr(text), repr(s.normalize(text)), ids, repr(s.decode(ids)))

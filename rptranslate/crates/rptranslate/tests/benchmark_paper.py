"""Bounded paper measurement; PDF and translations stay in runner temp."""
import collections
import hashlib
import pathlib
import re
import sys
import unicodedata

SHA = "c7812a97aa6ec2805e56e064bf02090c972c8c102b7c789321fc8f2a12a9286b"

def normalize(text):
    text = unicodedata.normalize("NFKC", text)
    text = re.sub(r"(?<=\w)-\s*\n\s*(?=\w)", "-", text)
    return " ".join(text.split())

def prepare(pdf, folder):
    import pymupdf
    data = pdf.read_bytes()
    if hashlib.sha256(data).hexdigest() != SHA:
        raise SystemExit("Public PDF differs from uploaded fixture; benchmark stopped")
    body, captions = [], []
    with pymupdf.open(pdf) as doc:
        assert len(doc) == 14
        for n, page in enumerate(doc, 1):
            if not 2 <= n <= 12:
                continue
            blocks = [
                b for b in page.get_text("blocks")
                if b[6] == 0 and b[0] < 565 and b[1] >= 40 and b[3] < 735
            ]
            captions.extend(b[4] for b in blocks if b[4].startswith(("Figure ", "Table 1.")))
            if n == 2:
                blocks = [b for b in blocks if 70 <= b[1] < 115 or 245 <= b[1] < 727]
                blocks.sort(key=lambda b: (0 if b[1] < 560 else 1 if b[0] < 300 else 2, b[1]))
            elif n == 12:
                blocks = [b for b in blocks if b[0] < 300 and b[1] < 394]
                blocks.sort(key=lambda b: b[1])
            else:
                blocks = [
                    b for b in blocks if b[2] - b[0] < 250
                    and b[1] > (130 if n == 5 else 0)
                    and not b[4].startswith(("Figure ", "Table "))
                    and not re.fullmatch(r"\s*(?:\([a-d]\)\s*)+", b[4])
                ]
                blocks.sort(key=lambda b: (b[0] >= 300, b[1]))
            body.extend(b[4] for b in blocks)
    segments = []
    for group in [" ".join(map(normalize, body)), *map(normalize, captions)]:
        for sentence in re.split(r"(?<=[.!?])\s+(?=[A-Z])", group):
            words = sentence.split()
            for i in range(0, len(words), 45):
                segments.append(" ".join(words[i:i + 45]))
    assert len(segments) > 100 and 5000 < sum(len(s.split()) for s in segments) < 7500
    rows = list(enumerate(segments))
    sample = [rows[i * (len(rows) - 1) // 7] for i in range(8)]
    for name, chosen in [("full", rows), ("sample", sample)]:
        (folder / (name + ".tsv")).write_text(
            "".join(f"{i}\t{s}\n" for i, s in chosen), encoding="utf-8"
        )
    print(f"PAPER_INPUT sha256={SHA} pages=14 segments={len(rows)} "
          f"source_words={sum(len(s.split()) for s in segments)} sample_segments={len(sample)} "
          "scope=title_abstract_keywords_body_captions exclusions=table_cells_references_figures_supplement")

def report(folder):
    source = dict(line.split("\t", 1) for line in (folder / "full.tsv").read_text().splitlines())
    sample = dict(line.split("\t", 1) for line in (folder / "sample.tsv").read_text().splitlines())
    def read(mode, expected):
        lines = (folder / (mode + ".out")).read_text().splitlines()
        load = lines[0].split("\t")
        assert len(load) == 4 and load[0] == "LOAD" and load[3] == "true"
        rows = {}
        for line in lines[1:]:
            label, key, src, tgt, sec, cap, encoded = line.split("\t")
            assert label == "ROW" and key not in rows and key in expected
            rows[key] = (int(src), int(tgt), float(sec), int(cap), bytes.fromhex(encoded).decode())
            assert rows[key][2] > 0 and rows[key][4]
        assert rows.keys() == expected.keys()
        rss = int((folder / (mode + ".rss")).read_text().strip())
        print(f"PAPER_LOAD precision={mode} model_ms={load[1]} session_ms={load[2]} rss_kib={rss}")
        return rows
    int8 = read("int8", source)
    fp32 = read("fp32", sample)
    nums = re.compile(r"\d+(?:[.,]\d+)?")
    def metrics(name, rows):
        seconds = sum(r[2] for r in rows.values())
        words = sum(len(source[k].split()) for k in rows)
        tokens = sum(r[1] for r in rows.values())
        counts = [collections.Counter(nums.findall(source[k])) for k in rows]
        total = sum(sum(c.values()) for c in counts)
        hit = sum(sum((c & collections.Counter(nums.findall(rows[k][4]))).values())
                  for k, c in zip(rows, counts))
        print(f"PAPER_RESULT mode={name} segments={len(rows)} source_words={words} "
              f"source_tokens={sum(r[0] for r in rows.values())} generated_tokens={tokens} "
              f"seconds={seconds:.6f} source_words_per_second={words / seconds:.6f} "
              f"tokens_per_second={tokens / seconds:.6f} capped={sum(r[3] for r in rows.values())} "
              f"numeric_string_matches={hit}/{total}")
        return seconds
    metrics("int8_full", int8)
    a = metrics("fp32_sample", fp32)
    b = metrics("int8_same_sample", {k: int8[k] for k in fp32})
    print(f"PAPER_PAIRED sample_segments={len(fp32)} speedup={a / b:.6f} "
          f"identical_text={sum(fp32[k][4] == int8[k][4] for k in fp32)}/{len(fp32)} "
          "quality=observational no_reference_translation=true")
    # Bounded preview only; full paper and translations remain in runner temp.
    import json
    for key in sample:
        print("PAPER_PREVIEW " + json.dumps({
            "segment": int(key),
            "source": source[key],
            "fp32": fp32[key][4],
            "int8": int8[key][4],
        }, ensure_ascii=False))


if __name__ == "__main__":
    if sys.argv[1] == "prepare":
        prepare(pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3]))
    elif sys.argv[1] == "report":
        report(pathlib.Path(sys.argv[2]))
    else:
        raise SystemExit("unknown operation")

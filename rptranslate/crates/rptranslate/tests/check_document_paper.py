"""Independent coverage and numeric-string audit of all extracted paper blocks.

No reference translation: coverage and lexical checks are not semantic quality.
No paper or translation text is emitted in CI logs.
"""
import collections
import math
import re
import sys
from pathlib import Path

folder = Path(sys.argv[1])
numbers = re.compile(r"[0-9]+(?:\.[0-9]+)*")


def inputs(name):
    rows = []
    for line in (folder / name).read_text(encoding="utf-8").splitlines():
        key, text = line.split("\t", 1)
        rows.append((int(key), text))
    assert rows and all(a[0] < b[0] for a, b in zip(rows, rows[1:]))
    return rows


def decode(value):
    return bytes.fromhex(value).decode("utf-8")


def check(mode, expected):
    lines = iter((folder / (mode + ".document")).read_text(encoding="utf-8").splitlines())
    label, precision, load, seconds = next(lines).split("\t")
    assert label == "DOCUMENT" and precision == mode
    load, seconds = float(load), float(seconds)
    assert all(math.isfinite(v) and v > 0 for v in (load, seconds))
    accepted, rejected, reasons = {}, {}, collections.Counter()
    strategies = collections.Counter()
    total_words = accepted_words = accepted_numbers = segment_count = 0
    for key, source in expected:
        row = next(lines).split("\t")
        assert int(row[1]) == key
        words = len(source.split())
        total_words += words
        if row[0] == "REJECT":
            assert len(row) == 3
            reason = decode(row[2])
            assert reason
            rejected[key] = reason
            reasons[reason.split(" ", 1)[0].split("(", 1)[0]] += 1
            continue
        assert row[0] == "ACCEPT" and len(row) == 4
        count, text = int(row[2]), decode(row[3])
        assert 0 < count <= 64 and text.strip()
        raw = source.encode("utf-8")
        end = 0
        translated = []
        for _ in range(count):
            label, segkey, start, stop, strategy, encoded = next(lines).split("\t")
            start, stop = int(start), int(stop)
            assert label == "SEGMENT" and int(segkey) == key
            assert end <= start < stop <= len(raw)
            assert not raw[end:start].decode("utf-8").strip()
            part = raw[start:stop].decode("utf-8")
            result = decode(encoded)
            assert part.strip() and result.strip()
            assert collections.Counter(numbers.findall(part)) == collections.Counter(numbers.findall(result))
            assert strategy in ("Whole", "Constrained", "ProtectedGaps")
            strategies[strategy] += 1
            segment_count += 1
            accepted_numbers += len(numbers.findall(part))
            translated.append(result)
            end = stop
        assert not raw[end:].decode("utf-8").strip()
        assert text == "\n".join(translated)
        accepted[key] = text
        accepted_words += words
    assert next(lines, None) is None
    assert len(accepted) + len(rejected) == len(expected)
    assert accepted, "no accepted blocks; inspect translation quality"
    rss = int((folder / (mode + ".document.rss")).read_text().strip())
    assert rss > 0
    print(f"DOCUMENT_REFERENCE PASS precision={mode} blocks={len(expected)} "
          f"accepted={len(accepted)} rejected={len(rejected)} "
          f"source_words={total_words} accepted_words={accepted_words} "
          f"accepted_numbers={accepted_numbers} segments={segment_count} "
          f"whole={strategies['Whole']} constrained={strategies['Constrained']} "
          f"gaps={strategies['ProtectedGaps']} seconds={seconds:.6f} "
          f"model_ms={load:.6f} rss_kib={rss} "
          "ordered=true lexical_only=true semantic_quality=unmeasured", flush=True)
    print(f"DOCUMENT_REJECTIONS precision={mode} categories={dict(reasons)}", flush=True)
    return accepted, rejected


full = inputs("full.tsv")
sample = inputs("sample.tsv")
assert len(full) > 100 and len(sample) == 8
assert all(dict(full)[key] == text for key, text in sample)
a, ar = check("int8", full)
b, br = check("fp32", sample)
keys = {key for key, _ in sample}
common = keys & a.keys() & b.keys()
print(f"DOCUMENT_PAIRED sample={len(sample)} both_accepted={len(common)} "
      f"identical_accepted={sum(a[key] == b[key] for key in common)} "
      f"same_status={sum((key in a) == (key in b) for key in keys)} "
      "quality=observational", flush=True)

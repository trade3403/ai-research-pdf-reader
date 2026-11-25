"""Compare protected and unprotected paper outputs; report failures, never hide them."""
import collections
import json
import pathlib
import re
import sys

folder = pathlib.Path(sys.argv[1])
source = dict(line.split("\t", 1) for line in (folder / "full.tsv").read_text().splitlines())
sample = dict(line.split("\t", 1) for line in (folder / "sample.tsv").read_text().splitlines())
baseline = {}
for line in (folder / "int8.out").read_text().splitlines()[1:]:
    label, key, src, tgt, seconds, cap, encoded = line.split("\t")
    assert label == "ROW"
    baseline[key] = (float(seconds), bytes.fromhex(encoded).decode())
protected, errors = {}, {}
for line in (folder / "scientific.out").read_text().splitlines():
    if line.startswith("SCIENCE_ROW\t"):
        _, key, spans, calls, seconds, encoded = line.split("\t")
        assert key in source and key not in protected and key not in errors
        protected[key] = (int(spans), int(calls), float(seconds), bytes.fromhex(encoded).decode())
    elif line.startswith("SCIENCE_ERROR\t"):
        _, key, error = line.split("\t", 2)
        assert key in source and key not in protected and key not in errors
        errors[key] = error
    elif line.startswith(("SCIENCE_REFERENCE ", "SCIENCE_FIXTURE ")):
        print(line)
    else:
        raise AssertionError("unexpected output row")
assert baseline.keys() == source.keys()
assert set(protected) | set(errors) == set(source)
assert protected and sum(v[0] for v in protected.values()) > 0
nums = re.compile(r"\d+(?:[.,]\d+)?")
def numeric_matches(values):
    hits, total = 0, 0
    for key, text in values.items():
        expected = collections.Counter(nums.findall(source[key]))
        total += sum(expected.values())
        hits += sum((expected & collections.Counter(nums.findall(text))).values())
    return f"{hits}/{total}"
before = {k: baseline[k][1] for k in protected}
after = {k: v[3] for k, v in protected.items()}
print(f"SCIENCE_PAPER observed={len(source)} successful={len(protected)} rejected={len(errors)} "
      f"recognized_spans={sum(v[0] for v in protected.values())} "
      f"model_calls={sum(v[1] for v in protected.values())} "
      f"baseline_seconds_same_success_set={sum(baseline[k][0] for k in protected):.6f} "
      f"protected_seconds_same_success_set={sum(v[2] for v in protected.values()):.6f} "
      f"baseline_numeric_strings={numeric_matches(before)} "
      f"protected_numeric_strings={numeric_matches(after)} "
      f"peak_rss_kib={(folder / 'scientific.rss').read_text().strip()} "
      "quality=observational context_fragmentation_possible")
for key, error in errors.items():
    print(f"SCIENCE_REJECTED segment={key} error={error}")
for key in sample:
    print("SCIENCE_PREVIEW " + json.dumps({
        "segment": int(key), "source": source[key], "baseline": baseline[key][1],
        "protected": after.get(key), "error": errors.get(key),
    }, ensure_ascii=False))

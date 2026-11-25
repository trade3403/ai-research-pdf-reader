"""Stdlib-only independent FP32-bit reference for block INT8."""
import math
import random
import struct
import subprocess
import sys
from pathlib import Path

def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]

def bits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]

def from_bits(x):
    return struct.unpack("<f", struct.pack("<I", x))[0]

max_f32 = from_bits(0x7f7fffff)
tiny = from_bits(1)
rng = random.Random(1901)
cases = [
    (7, [-127., -2.5, -1.5, 0., 1.5, 2.5, 127., 0., 0.]),
    (3, [1., -.25, .5, 100., -25., 50., .125]),
    (3, [max_f32, -max_f32, 0., tiny, -tiny, 2*tiny]),
    (1, [0., -0., tiny, -tiny, max_f32, -max_f32]),
    (32, [0.] * 65),
]
for block in (16, 32, 64, 128, 256, 512, 4096):
    cases.append((block, [f32(rng.uniform(-1, 1) * 10**rng.randint(-4, 4))
                          for _ in range(block * 2 + 7)]))
lines = [" ".join([str(block)] + [f"{bits(v):08x}" for v in data])
         for block, data in cases]
result = subprocess.run([str(Path(sys.argv[1]).resolve())], input="\n".join(lines)+"\n",
                        capture_output=True, text=True, check=True, timeout=30)
rows = result.stdout.splitlines()
assert len(rows) == len(cases)
total = blocks = 0
max_normalized = 0.
for (block, data), row in zip(cases, rows):
    codes, scales, restored, payload = row.split("|")
    q, s, y = [], [], []
    for begin in range(0, len(data), block):
        chunk = data[begin:begin+block]
        peak = max(abs(v) for v in chunk)
        scale = 1. if peak == 0 else max(tiny, f32(peak / 127))
        if scale * 127 > max_f32:
            scale = from_bits(bits(scale)-1)
        s.append(bits(scale))
        for x in chunk:
            code = max(-127, min(127, round(x / scale)))
            actual = f32(code * scale)
            assert math.isfinite(actual)
            error = abs(actual-x)
            # Include final f32 rounding, especially for subnormal values.
            assert error <= scale/2 + abs(actual)*2**-23 + tiny
            max_normalized = max(max_normalized, error/scale)
            q.append(code)
            y.append(bits(actual))
    assert list(map(int, codes.split(","))) == q
    assert [int(x, 16) for x in scales.split(",")] == s
    assert [int(x, 16) for x in restored.split(",")] == y
    assert int(payload) == len(q) + 4*len(s)
    total += len(q)
    blocks += len(s)
print(f"INT8_REFERENCE PASS cases={len(cases)} elements={total} blocks={blocks} "
      f"codes_scales_dequant_bitwise=true max_error_over_scale={max_normalized:.9g}", flush=True)

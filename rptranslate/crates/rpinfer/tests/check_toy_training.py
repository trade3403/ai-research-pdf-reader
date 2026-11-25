"""Independent stdlib reference for synthetic softmax SGD and checkpoint."""
import hashlib
import math
import struct
import subprocess
import sys

data = [([1., 0., 1.], 0), ([2., 0., 1.], 0),
        ([0., 1., 1.], 1), ([0., 2., 1.], 1),
        ([-1., -1., 1.], 2), ([-2., -2., 1.], 2)]
state = 42
w = []
for _ in range(9):
    state = (state * 6364136223846793005 + 1) % (1 << 64)
    w.append(((state >> 32) / 2**32 - .5) * .2)

def evaluate(weights):
    loss = 0.
    grad = [0.] * 9
    for x, label in data:
        z = [sum(weights[3*c+j]*x[j] for j in range(3)) for c in range(3)]
        peak = max(z)
        exp = [math.exp(v-peak) for v in z]
        denom = sum(exp)
        loss += peak-z[label]+math.log(denom)
        for c in range(3):
            for j in range(3):
                grad[3*c+j] += (exp[c]/denom-int(c==label))*x[j]
    return loss/6, [v/6 for v in grad]

rows = subprocess.check_output([sys.argv[1]], text=True).splitlines()
assert len(rows) == 6
observed = {}
for row in rows:
    fields = row.split()
    if fields[0] == "STEP":
        assert len(fields) == 12
        step = int(fields[1])
        assert step not in observed
        observed[step] = list(map(float, fields[2:]))
    elif fields[0] == "CHECKPOINT":
        checkpoint = bytes.fromhex(fields[1])
    else:
        assert fields == ["RESUME", "100", "bitwise=true"]
assert set(observed) == {0, 1, 37, 100}
assert len(checkpoint) == 168
body, digest = checkpoint[:-64], checkpoint[-64:]
assert hashlib.sha256(body).hexdigest().encode() == digest
magic, seed, steps, rate, *stored = struct.unpack("<8sQQd9d", body)
assert (magic, seed, steps, rate) == (b"RPTNY001", 42, 37, .2)
worst = 0.
for step in range(101):
    loss, gradient = evaluate(w)
    if step in observed:
        errors = [abs(a-b) for a,b in zip([loss]+w, observed[step])]
        worst = max(worst, *errors)
        assert all(e < 1e-12 for e in errors)
    if step == 37:
        assert all(abs(a-b) < 1e-12 for a,b in zip(w, stored))
    if step < 100:
        w = [a-.2*g for a,g in zip(w, gradient)]
assert observed[100][0] < observed[0][0]*.1
print(f"TOY_TRAINING PASS steps=100 weights=9 initial_loss={observed[0][0]:.9f} "
      f"final_loss={observed[100][0]:.9f} max_error={worst:.3g} "
      "resume_bitwise=true checkpoint_sha256=true synthetic_only=true")

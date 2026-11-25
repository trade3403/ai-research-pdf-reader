"""Independent RPQ8 byte encoder, digest and adversarial fixtures."""
import hashlib
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

exe = Path(sys.argv[1]).resolve()

def seal(prefix):
    return prefix + hashlib.sha256(prefix).hexdigest().encode("ascii")

def record(name, dims, block, offset, length, reserved=0):
    name = name.encode()
    return (struct.pack("<HBBIQQ", len(name), len(dims), reserved, block, offset, length)
            + struct.pack("<" + "Q"*len(dims), *dims) + name)

a = record("a.weight", [2, 3], 4, 0, 14)
b = record("b.weight", [1], 1, 14, 5)
payload = struct.pack("<f4bf2bfb", .5, 127, -127, 1, 0, .25, 2, -3, 1., 0)

def build(index=a+b, body=payload, count=2, version=1):
    return seal(struct.pack("<8sIIQQ", b"RPQ8BIN\0", version, count, len(index), len(body))
                + index + body)

with tempfile.TemporaryDirectory() as temp:
    root = Path(temp)
    rust = root/"rust.rpq"
    subprocess.run([str(exe), "write", str(rust)], check=True)
    expected = build()
    assert rust.read_bytes() == expected
    fixture = root/"python.rpq"
    fixture.write_bytes(expected)
    out = subprocess.check_output([str(exe), "read", str(fixture)], text=True)
    assert out.splitlines() == [
        "a.weight|0|3f000000|[127, -127, 1, 0]",
        "a.weight|1|3e800000|[2, -3]",
        "b.weight|0|3f800000|[0]",
    ]
    negatives = [
        expected[:-1],
        expected+b"\0",
        expected[:40]+bytes([expected[40]^1])+expected[41:],
        build(version=2),
        build(count=4097),
        build(index=a+a),
        build(index=a+record("b.weight", [1], 1, 13, 5)),
        build(index=a+record("b.weight", [1], 1, 15, 5)),
        build(index=a+record("b.weight", [1], 1, 14, 4)),
        build(index=record("../a", [2, 3], 4, 0, 14)+b),
        build(index=record("a.weight", [2**63, 3], 4, 0, 14)+b),
        build(index=record("a.weight", [2, 3], 0, 0, 14)+b),
        build(index=record("a.weight", [2, 3], 4, 0, 14, 1)+b),
        build(body=struct.pack("<f", float("nan"))+payload[4:]),
        build(body=struct.pack("<f", 0.)+payload[4:]),
        build(body=payload[:4]+b"\x80"+payload[5:]),
        build(body=struct.pack("<I", 0x7f7fffff)+payload[4:]),
    ]
    for n, data in enumerate(negatives):
        fixture.write_bytes(data)
        result = subprocess.run([str(exe), "read", str(fixture)], capture_output=True)
        assert result.returncode != 0, n
print(f"RPQ8_REFERENCE PASS tensors=2 blocks=3 byte_exact=true rejected={len(negatives)}")

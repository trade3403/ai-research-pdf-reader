"""Use the real converted directory; reject same-size corruption and bad manifest."""
import json
import subprocess
import sys
from pathlib import Path

root, exe = map(Path, sys.argv[1:3])
exe = exe.resolve()
subprocess.run([str(exe), str(root)], check=True)
cfg = root / "config.json"
manifest = root / "manifest.json"
original = cfg.read_bytes()
original_manifest = manifest.read_bytes()

def rejected():
    result = subprocess.run([str(exe), str(root)], capture_output=True, text=True)
    assert result.returncode != 0, result.stdout

try:
    cfg.write_bytes(bytes([original[0] ^ 1]) + original[1:])
    rejected()
finally:
    cfg.write_bytes(original)
try:
    data = json.loads(original_manifest)
    data["version"] = 2
    manifest.write_text(json.dumps(data), encoding="utf-8")
    rejected()
    data["version"] = 1
    data["files"]["weights.safetensors"]["sha256"] = "0" * 64
    manifest.write_text(json.dumps(data), encoding="utf-8")
    rejected()
finally:
    manifest.write_bytes(original_manifest)
print("RPMODEL_PACKAGE_NEGATIVE PASS cases=3", flush=True)

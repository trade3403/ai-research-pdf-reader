"""Choose an available iPhone from the newest installed iOS runtime."""
import json
import sys

with open(sys.argv[1], encoding="utf-8") as source:
    devices = json.load(source)["devices"]

candidates = []
for runtime, entries in devices.items():
    if not runtime.startswith("com.apple.CoreSimulator.SimRuntime.iOS-"):
        continue
    version = tuple(int(n) for n in runtime.split("iOS-", 1)[1].split("-"))
    for device in entries:
        if device.get("isAvailable") and device["name"].startswith("iPhone"):
            candidates.append((version, device.get("state") == "Booted",
                               device["name"], device["udid"]))
if not candidates:
    sys.exit("No available iPhone simulator found")
print(max(candidates)[-1])

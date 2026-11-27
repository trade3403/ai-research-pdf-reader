#!/usr/bin/env python3
"""Test-only external GPOS reference; Python standard library + system HarfBuzz.

Compares native Latin kern on/off deltas at UPEM scale, not full text
shaping. HB supplies glyph IDs; no production dependency is introduced.
A mismatch fails CI and retains the per-case diagnostic JSON.
"""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path
import subprocess

class Info(C.Structure):
    _fields_ = [("codepoint", C.c_uint32), ("mask", C.c_uint32),
                ("cluster", C.c_uint32), ("var1", C.c_uint32), ("var2", C.c_uint32)]

class Position(C.Structure):
    _fields_ = [("x_advance", C.c_int32), ("y_advance", C.c_int32),
                ("x_offset", C.c_int32), ("y_offset", C.c_int32), ("var", C.c_uint32)]

class Feature(C.Structure):
    _fields_ = [("tag", C.c_uint32), ("value", C.c_uint32),
                ("start", C.c_uint), ("end", C.c_uint)]

class HB:
    def __init__(self):
        name = ctypes.util.find_library("harfbuzz")
        if not name:
            raise RuntimeError("test requires system HarfBuzz shared library")
        self.lib = C.CDLL(name)
        self.library = name
        def bind(name, result, *args):
            f = getattr(self.lib, "hb_" + name)
            f.restype = result
            f.argtypes = list(args)
            setattr(self, name, f)
        ptr, uint, i32 = C.c_void_p, C.c_uint, C.c_int
        bind("version_string", C.c_char_p)
        bind("blob_create", ptr, C.c_char_p, uint, i32, ptr, ptr)
        bind("blob_destroy", None, ptr)
        bind("face_create", ptr, ptr, uint)
        bind("face_destroy", None, ptr)
        bind("face_get_upem", uint, ptr)
        bind("font_create", ptr, ptr)
        bind("font_destroy", None, ptr)
        bind("ot_font_set_funcs", None, ptr)
        bind("font_set_scale", None, ptr, i32, i32)
        bind("buffer_create", ptr)
        bind("buffer_destroy", None, ptr)
        bind("buffer_add_utf8", None, ptr, C.c_char_p, i32, uint, i32)
        bind("buffer_set_direction", None, ptr, i32)
        bind("script_from_string", C.c_uint32, C.c_char_p, i32)
        bind("buffer_set_script", None, ptr, C.c_uint32)
        bind("language_from_string", ptr, C.c_char_p, i32)
        bind("buffer_set_language", None, ptr, ptr)
        bind("shape_full", i32, ptr, ptr, C.POINTER(Feature), uint, C.POINTER(C.c_char_p))
        bind("buffer_get_glyph_infos", C.POINTER(Info), ptr, C.POINTER(uint))
        bind("buffer_get_glyph_positions", C.POINTER(Position), ptr, C.POINTER(uint))

    def shape(self, data, text, kern, mark=False):
        blob = self.blob_create(data, len(data), 0, None, None)  # duplicate
        face = self.face_create(blob, 0)
        font = self.font_create(face)
        buf = self.buffer_create()
        try:
            upem = self.face_get_upem(face)
            if not upem:
                raise RuntimeError("invalid units per em")
            self.ot_font_set_funcs(font)
            self.font_set_scale(font, upem, upem)
            raw = text.encode("utf-8")
            self.buffer_add_utf8(buf, raw, len(raw), 0, len(raw))
            self.buffer_set_direction(buf, 4)  # HB_DIRECTION_LTR
            self.buffer_set_script(buf, self.script_from_string(b"Latn", -1))
            self.buffer_set_language(buf, self.language_from_string(b"und", -1))
            values = {"kern": int(kern), "liga": 0, "clig": 0, "calt": 0,
                      "rlig": 0, "dlig": 0, "hlig": 0, "mark": int(mark), "mkmk": 0}
            features = (Feature * len(values))(*[
                Feature(int.from_bytes(tag.encode("ascii"), "big"), value, 0, 0xFFFFFFFF)
                for tag, value in values.items()])
            shapers = (C.c_char_p * 2)(b"ot", None)
            if not self.shape_full(font, buf, features, len(features), shapers):
                raise RuntimeError("HarfBuzz OT shaper failed")
            ni, np = C.c_uint(), C.c_uint()
            infos = self.buffer_get_glyph_infos(buf, C.byref(ni))
            positions = self.buffer_get_glyph_positions(buf, C.byref(np))
            if ni.value != np.value:
                raise RuntimeError("inconsistent HB buffer")
            glyphs = [(infos[i].codepoint, infos[i].cluster) for i in range(ni.value)]
            metrics = [[p.x_advance, p.y_advance, p.x_offset, p.y_offset]
                       for p in positions[:np.value]]
            return glyphs, metrics, upem
        finally:
            self.buffer_destroy(buf)
            self.font_destroy(font)
            self.face_destroy(face)
            self.blob_destroy(blob)

TEXTS = ("AV", "VA", "To", "Ta", "WA", "Yo", "LT", "AVATAR", "ToTo", "hello")
FONTS = ("ABeeZee-Regular.ttf", "ABeeZee-Italic.ttf")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--font-dir", type=Path, default=Path("core/tests/fixtures/abeezee"))
    args = parser.parse_args()
    hb = HB()
    report = {"harfbuzz": hb.version_string().decode(), "library": hb.library,
              "scope": "native Latin GPOS kern toggle; HB glyph IDs; LTR design units",
              "fonts": [], "cases": 0, "nonzero_cases": 0, "mismatches": 0,
              "max_absolute_error": 0}
    print("HarfBuzz", report["harfbuzz"], flush=True)
    try:
        for name in FONTS:
            path = args.font_dir / name
            data = path.read_bytes()
            font = {"name": name, "sha256": hashlib.sha256(data).hexdigest(), "cases": []}
            report["fonts"].append(font)
            groups = []
            for text in TEXTS:
                a, on, upem = hb.shape(data, text, True)
                b, off, off_upem = hb.shape(data, text, False)
                if a != b or upem != off_upem or not a or any(g == 0 for g, _ in a):
                    raise RuntimeError(f"invalid isolated kern sample {name} {text}")
                expected = [[x-y for x, y in zip(p, q)] for p, q in zip(on, off)]
                groups.append(",".join(str(g) for g, _ in a))
                font["cases"].append({"text": text, "glyphs": a, "upem": upem,
                                      "reference_delta": expected})
            proc = subprocess.run([str(args.probe.resolve()), str(path), *groups],
                                  check=True, capture_output=True, text=True, timeout=60)
            lines = proc.stdout.splitlines()
            if len(lines) != len(groups):
                raise RuntimeError(f"probe row count {len(lines)} != {len(groups)}")
            for case, line in zip(font["cases"], lines):
                actual = [[int(x) for x in part.split(",")] for part in line.split(";")]
                expected = case["reference_delta"]
                if len(actual) != len(expected) or any(len(a) != 4 for a in actual):
                    raise RuntimeError("probe glyph/field count")
                errors = [abs(x-y) for a,b in zip(actual,expected) for x,y in zip(a,b)]
                case["rwcore_delta"] = actual
                case["max_absolute_error"] = max(errors, default=0)
                report["cases"] += 1
                report["nonzero_cases"] += int(any(v for a in expected for v in a))
                report["mismatches"] += int(actual != expected)
                report["max_absolute_error"] = max(report["max_absolute_error"], *errors)
                print(name, case["text"], "max_error", case["max_absolute_error"], flush=True)
        if report["cases"] != len(FONTS)*len(TEXTS) or not report["nonzero_cases"]:
            raise RuntimeError("vacuous reference comparison")
        if report["mismatches"]:
            raise RuntimeError(f"{report['mismatches']} GPOS reference mismatches")
    except Exception as err:
        report["error"] = str(err)
        raise
    finally:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print("HB_GPOS_REFERENCE", json.dumps({k:v for k,v in report.items() if k != "fonts"}), flush=True)

if __name__ == "__main__":
    main()

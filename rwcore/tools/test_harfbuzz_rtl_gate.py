"""Unit tests of the external RTL acceptance policy, no shaping library needed."""
import unittest
from harfbuzz_rtl_reference import MIN_EVIDENCE, BASES, MARKS, CURSIVE, validate_gate

def baseline():
    r=dict(candidate_count=114,admitted=0,compared=0,exact=0,mismatches=0,
           probe_errors=0,excluded=0,max_absolute_error=0,max_world_error=0,fonts=[])
    for name, floors in MIN_EVIDENCE.items():
        font=dict(name=name,features=[])
        r["fonts"].append(font)
        for tag,n in floors.items():
            rows=[]
            for _ in range(n):
                rows.append(dict(status="exact",max_absolute_error=0,max_world_error=0,
                                 reference_delta=[[0,0,1,0]],rwcore_delta=[[0,0,1,0]],
                                 reference_world_delta=[[1,0]],rwcore_world_delta=[[1,0]]))
            count=len(BASES)*len(MARKS) if tag=="mark" else len(CURSIVE)
            rows.extend(dict(status="excluded",reason="synthetic scope exclusion") for _ in range(count-n))
            font["features"].append(dict(tag=tag,rows=rows))
            for k in ("admitted","compared","exact"):
                r[k]+=n
            r["excluded"]+=count-n
    return r

class GateTests(unittest.TestCase):
    def test_baseline(self):
        validate_gate(baseline())
    def test_summary_precision(self):
        for key in ("mismatches","probe_errors","max_absolute_error","max_world_error"):
            with self.subTest(key=key):
                r=baseline()
                r[key]=1
                with self.assertRaises(RuntimeError):
                    validate_gate(r)
    def test_row_precision(self):
        r=baseline()
        r["fonts"][0]["features"][0]["rows"][0]["rwcore_delta"]=[[0,0,2,0]]
        with self.assertRaises(RuntimeError):
            validate_gate(r)
    def test_coverage_drop(self):
        r=baseline()
        r["fonts"][0]["features"][0]["rows"][0]=dict(status="excluded",reason="scope")
        for k in ("admitted","compared","exact"):
            r[k]-=1
        r["excluded"]+=1
        with self.assertRaises(RuntimeError):
            validate_gate(r)
    def test_nonzero_drop(self):
        r=baseline()
        row=r["fonts"][0]["features"][0]["rows"][0]
        row.update(reference_delta=[[0,0,0,0]],rwcore_delta=[[0,0,0,0]])
        with self.assertRaises(RuntimeError):
            validate_gate(r)
    def test_missing_font(self):
        r=baseline()
        r["fonts"].pop()
        with self.assertRaises(RuntimeError):
            validate_gate(r)
    def test_exclusion_reason(self):
        r=baseline()
        row=r["fonts"][0]["features"][0]["rows"][-1]
        row.pop("reason")
        with self.assertRaises(RuntimeError):
            validate_gate(r)
    def test_accounting(self):
        r=baseline()
        r["excluded"]+=1
        with self.assertRaises(RuntimeError):
            validate_gate(r)

if __name__=="__main__":
    unittest.main()

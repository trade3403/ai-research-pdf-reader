import unittest
from formula_benchmark import outcome, normalize, load_cases, exit_code, ROOT
import json

class FormulaBenchmarkTests(unittest.TestCase):
    def test_scoring(self):
        self.assertEqual(outcome({"status":"ok","latex":r"x_{2}"},[r"x_2"]),"mismatch")
        self.assertEqual(outcome({"status":"ok","latex":"x + y"},["x+y"]),"exact_match")
        self.assertEqual(outcome({"status":"unsupported","math_tree":"ok"},[]),"latex_rejected")
        self.assertEqual(outcome({"status":"unsupported","math_geometry":"ok"},[]),"tree_rejected")
        self.assertEqual(outcome({"status":"unsupported"},[]),"geometry_rejected")
        self.assertEqual(outcome({"status":"timeout"},[]),"timeout")
    def test_manifest(self):
        papers={x["id"]:x for x in json.loads((ROOT/"papers.json").read_text(encoding="utf-8"))}
        baseline=load_cases(ROOT/"formulas.json",papers)
        extension=load_cases(ROOT/"formulas_extended.json",papers)
        self.assertEqual(len(baseline),4)
        self.assertEqual(len(extension),4)
        self.assertFalse({x["id"] for x in baseline}&{x["id"] for x in extension})
    def test_gate(self):
        self.assertEqual(exit_code([{"outcome":"exact_match"}],True),0)
        self.assertEqual(exit_code([{"outcome":"mismatch"}],True),1)
        self.assertEqual(exit_code([{"outcome":"tree_rejected"}],False),0)
        self.assertEqual(exit_code([{"outcome":"download_error"}],False),3)
        self.assertEqual(exit_code([],True),3)

if __name__=="__main__":
    unittest.main()

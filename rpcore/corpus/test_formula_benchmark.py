import unittest
from formula_benchmark import outcome, normalize, load_cases, ROOT
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
        self.assertEqual(len(load_cases(ROOT/"formulas.json",papers)),4)

if __name__=="__main__":
    unittest.main()

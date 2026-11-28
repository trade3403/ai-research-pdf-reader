import unittest
import tempfile
from pathlib import Path
from PIL import Image
from chart_benchmark import chart_metrics, pixel_box, load_cases, synthetic_pdf
from color_benchmark import patches, fixture, measure, neutral_lab

class ChartTests(unittest.TestCase):
    def test_color_union(self):
        a=Image.new("RGB",(2,1),"white")
        b=a.copy()
        a.putpixel((0,0),(255,0,0));b.putpixel((1,0),(0,0,255))
        c,_=chart_metrics(a,b)
        self.assertEqual(c["chroma_union_pixels"],2)
        self.assertEqual(c["chroma_iou"],0.)
        self.assertEqual(c["chroma_union_mae"],170.)
        c,_=chart_metrics(a,a)
        self.assertEqual(c["chroma_iou"],1.)
        self.assertEqual(c["chroma_union_mae"],0.)
        c,_=chart_metrics(Image.new("RGB",(1,1),"white"),Image.new("RGB",(1,1),"white"))
        self.assertIsNone(c["chroma_union_mae"])
    def test_manifest_and_fixture(self):
        self.assertEqual(len(load_cases()["cases"]),4)
        self.assertEqual(pixel_box((100,100),[.1,.2,.8,.9]),(10,20,80,90))
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"fixture.pdf"
            synthetic_pdf(p)
            b=p.read_bytes()
            synthetic_pdf(p)
            self.assertEqual(b,p.read_bytes())
            self.assertTrue(b.startswith(b"%PDF-1.4"))
            self.assertIn(b"/Half gs",b)
    def test_color_patches(self):
        rows=patches()
        self.assertEqual(len(rows),36)
        self.assertEqual(sum(x["expected"] is not None for x in rows),18)
        self.assertEqual(neutral_lab(50),[119]*3)
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"patches.pdf"
            self.assertEqual(fixture(p,rows),(288,288))
            self.assertEqual(rows[0]["sample"],[16,16,32,32])
            self.assertEqual(measure(Image.new("RGB",(288,288),(119,119,119)),rows[0]["sample"]),([119.]*3,0))
if __name__=="__main__":
    unittest.main()

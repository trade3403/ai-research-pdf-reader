import unittest
from PIL import Image
from visual import compare

class VisualTests(unittest.TestCase):
    def test_identical_and_blank(self):
        a=Image.new("RGB",(4,4),"white")
        c,h=compare(a,a)
        self.assertEqual(c["mae_rgb_0_255"],0)
        self.assertEqual(c["ink_iou"],1)
        self.assertEqual(c["exact_pixel_fraction"],1)
        self.assertEqual(h.getpixel((0,0)),(255,255,255))

    def test_missing_ink_and_extra_ink(self):
        a=Image.new("RGB",(4,4),"white")
        b=a.copy()
        b.putpixel((1,1),(0,0,0))
        a.putpixel((2,2),(0,0,0))
        c,h=compare(a,b)
        self.assertEqual(c["fraction_gt16"],0.125)
        self.assertEqual(c["ink_iou"],0)
        self.assertEqual(c["reference_ink_missing_fraction"],1)
        self.assertEqual(c["native_extra_ink_fraction"],1)
        self.assertEqual(c["mae_rgb_0_255"],31.875)
        self.assertEqual(h.getpixel((1,1)),(255,0,0))
        self.assertEqual(c["worst_tiles"][0]["box"],[0,0,4,4])

    def test_edge_shift_tolerance_does_not_change_raw_metrics(self):
        a=Image.new("RGB",(9,9),"white")
        b=a.copy()
        a.putpixel((3,3),(0,0,0))
        b.putpixel((4,4),(0,0,0))
        c,_=compare(a,b)
        self.assertEqual(c["ink_iou"],0)
        self.assertEqual(c["reference_ink_missing_fraction"],1)
        self.assertEqual(c["reference_ink_unmatched_1px_fraction"],0)
        self.assertEqual(c["native_ink_unmatched_1px_fraction"],0)
        self.assertLess(c["blurred_mae_rgb_0_255"],c["mae_rgb_0_255"])

    def test_distant_missing_ink_not_hidden(self):
        a=Image.new("RGB",(9,9),"white")
        b=a.copy()
        a.putpixel((1,1),(0,0,0))
        b.putpixel((7,7),(0,0,0))
        c,_=compare(a,b)
        self.assertEqual(c["reference_ink_unmatched_1px_fraction"],1)
        self.assertEqual(c["native_ink_unmatched_1px_fraction"],1)

    def test_no_silent_rescale(self):
        c,h=compare(Image.new("RGB",(2,2)),Image.new("RGB",(3,2)))
        self.assertEqual(c["status"],"dimension_mismatch")
        self.assertIsNone(h)

    def test_bright_colored_ink(self):
        a=Image.new("RGB",(1,1),(255,255,0))
        c,_=compare(a,a)
        self.assertEqual(c["native_ink_fraction"],1)
        self.assertEqual(c["ink_iou"],1)

if __name__=="__main__":
    unittest.main()

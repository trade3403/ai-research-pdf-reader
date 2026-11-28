import unittest
from PIL import Image
from visual import compare, parse_image_list, jpx_summary

IMAGE_HEADER = 'page num type width height color comp bpc enc interp object ID x-ppi y-ppi size ratio\n-----\n'

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

class JpxInventoryTests(unittest.TestCase):
    def test_inventory_distinguishes_encoding_column(self):
        rows = IMAGE_HEADER + "1 0 image 4 4 gray 1 8 jpx no 5 0 72 72 214B 1338%\n2 1 image 4 4 rgb 3 8 jpeg no 6 0 72 72 100B 200%\n"
        v = parse_image_list(rows,2)
        self.assertEqual(v["image_rows"],2)
        self.assertEqual(len(v["jpx_rows"]),1)
        self.assertEqual(v["jpx_rows"][0]["page"],1)
    def test_inventory_rejects_bad_rows(self):
        for value in ["",IMAGE_HEADER+"1 2 image",IMAGE_HEADER+"3 0 image 4 4 gray 1 8 jpx no"]:
            with self.assertRaises(ValueError):
                parse_image_list(value,2)
    def test_zero_hits_is_not_success(self):
        d={"jpx_inventory":{"status":"ok","complete":True,"jpx_rows":[]}}
        self.assertEqual(jpx_summary([d])["status"],"coverage_gap")
        self.assertEqual(jpx_summary([d,{}])["status"],"inventory_incomplete")
    def test_compare_is_observational(self):
        d={"jpx_inventory":{"status":"ok","complete":True,"jpx_rows":[{"page":1},{"page":1}]}}
        self.assertEqual(jpx_summary([d])["status"],"comparison_incomplete")
        d["jpx_pages"]=[{"comparison":{"status":"compared"}}]
        self.assertEqual(jpx_summary([d])["status"],"compared_observational")
        self.assertEqual(jpx_summary([d])["jpx_image_rows"],2)

if __name__=="__main__":
    unittest.main()

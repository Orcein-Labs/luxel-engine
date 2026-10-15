import importlib.util
import os
import tempfile
import unittest


MODULE_PATH = os.path.join(os.path.dirname(__file__), "..", "tools", "shadow_consistency_measure.py")
SPEC = importlib.util.spec_from_file_location("shadow_consistency_measure", MODULE_PATH)
measure = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(measure)


def write_ppm(path, width, height, pixels):
    with open(path, "wb") as stream:
        stream.write(f"P6\n{width} {height}\n255\n".encode("ascii"))
        stream.write(bytes(pixels))


class ShadowConsistencyMeasurementTests(unittest.TestCase):
    def test_synthetic_centroid_points_with_packet_direction_and_rejects_reverse(self):
        with tempfile.TemporaryDirectory() as temp:
            bare_path = os.path.join(temp, "receiver.ppm")
            caster_path = os.path.join(temp, "caster.ppm")
            width = height = 64
            bare = bytearray([180] * (width * height * 3))
            caster = bytearray(bare)
            for y in range(48, 52):
                for x in range(44, 48):
                    offset = 3 * (y * width + x)
                    caster[offset : offset + 3] = bytes((80, 80, 80))
            write_ppm(bare_path, width, height, bare)
            write_ppm(caster_path, width, height, caster)
            case = {
                "pixels_per_meter": 4,
                "camera_center_xz": [0, 0],
                "caster_base_xz": [0, 0],
                "direction_xyz": [3.5 / measure.math.hypot(3.5, 4.5), -0.7,
                                  4.5 / measure.math.hypot(3.5, 4.5)],
                "caster_height_m": 4,
            }
            result = measure.measure_pair(bare_path, caster_path, case)
            self.assertGreaterEqual(result["shadow_pixels"], 8)
            self.assertAlmostEqual(result["displacement_m"], measure.math.hypot(3.5, 4.5), places=2)
            self.assertGreaterEqual(result["direction_cosine"], 0.99)
            self.assertTrue(result["accepted"])
            self.assertTrue(result["opposite_direction_rejected"])
            wrong = measure.measure_pair(bare_path, caster_path, case, reverse_direction=True)
            self.assertFalse(wrong["accepted"])
            self.assertLess(wrong["direction_cosine"], -0.99)

    def test_no_shadow_is_not_reported_as_a_direction_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            path = os.path.join(temp, "same.ppm")
            pixels = bytes([160] * (64 * 64 * 3))
            write_ppm(path, 64, 64, pixels)
            result = measure.measure_pair(path, path, {
                "pixels_per_meter": 4,
                "camera_center_xz": [0, 0],
                "caster_base_xz": [0, 0],
                "direction_xyz": [0.6, -0.7, 0.8],
            })
            self.assertEqual(result["shadow_pixels"], 0)
            self.assertNotIn("accepted", result)

    def test_malformed_and_wrong_size_ppm_are_refused(self):
        with tempfile.TemporaryDirectory() as temp:
            bad_header = os.path.join(temp, "bad-header.ppm")
            with open(bad_header, "wb") as stream:
                stream.write(b"P3\n1 1\n255\n0 0 0\n")
            with self.assertRaises(ValueError):
                measure.read_ppm(bad_header)
            short_pixels = os.path.join(temp, "short.ppm")
            with open(short_pixels, "wb") as stream:
                stream.write(b"P6\n2 2\n255\n\x00\x00\x00")
            with self.assertRaises(ValueError):
                measure.read_ppm(short_pixels)

    def test_builtin_synthetic_negative_gate(self):
        self.assertTrue(measure.synthetic_wrong_direction_rejected())


if __name__ == "__main__":
    unittest.main()

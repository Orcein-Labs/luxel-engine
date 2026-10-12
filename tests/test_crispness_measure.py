"""Properties for R-2 resolution capture comparison diagnostics."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
from PIL import Image

TOOLS = Path(__file__).resolve().parents[1] / "tools"
sys.path.insert(0, str(TOOLS))

from crispness_measure import measure  # noqa: E402


def _write_rgb(path: Path, image: np.ndarray) -> Path:
    Image.fromarray(np.asarray(image, dtype=np.uint8), mode="RGB").save(path)
    return path


def _packet(width: int, height: int, *, exposure: float = 1.0) -> dict:
    return {
        "schema_version": "luxel.graphics-scene-packet/v6",
        "packet_id": f"id-{width}x{height}",
        "world_artifact_id": "world-fixed",
        "world_artifact_sha256": "sha256:world-fixed",
        "spatial_fields_sha256": "sha256:fields-fixed",
        "frame_seed": 7,
        "coordinate_system": {"up_axis": "y", "handedness": "right", "units_per_meter": 1.0},
        "camera": {
            "camera_id": "camera-fixed",
            "projection": {"kind": "perspective", "fov_y_degrees": 55.0},
            "position_xyz_m": [1.0, 2.0, 3.0],
            "forward_xyz": [0.0, 0.0, -1.0],
            "up_xyz": [0.0, 1.0, 0.0],
            "near_plane_m": 0.1,
            "far_plane_m": 1000.0,
            "width_px": width,
            "height_px": height,
        },
        "capture": {
            "capture_id": f"capture-{width}x{height}",
            "camera_id": "camera-fixed",
            "width_px": width,
            "height_px": height,
            "format": "rgba8_srgb",
            "include_depth": False,
            "deterministic": True,
        },
        "environment": {"exposure": exposure},
        "terrain": {},
        "materials": [],
        "textures": [],
        "meshes": [],
        "instances": [],
        "lights": [],
        "overlays": [],
    }


class CrispnessMeasureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def _packet_file(self, name: str, packet: dict, digest_claim: str = "sha256:synthetic-unverified") -> Path:
        path = self.root / name
        path.write_text(
            json.dumps({"body": packet, "packet_sha256": digest_claim}),
            encoding="utf-8",
        )
        return path

    def test_identical_capture_has_zero_normalized_difference(self):
        y, x = np.indices((48, 64))
        rgb = np.stack(((x * 3) % 256, (y * 5) % 256, ((x + y) * 2) % 256), axis=-1)
        base = _write_rgb(self.root / "base.png", rgb)
        cand = _write_rgb(self.root / "candidate.png", rgb.copy())

        report = measure(base, cand)
        difference = report["normalized_comparison"]["luminance_difference"]
        self.assertEqual(0.0, difference["mean_absolute"])
        self.assertEqual(0.0, difference["root_mean_square"])
        self.assertEqual(0.0, difference["maximum_absolute"])
        self.assertEqual("unverified", report["provenance"]["status"])

    def test_flat_capture_has_zero_edge_and_detail_diagnostics(self):
        rgb = np.full((32, 48, 3), 128, dtype=np.uint8)
        base = _write_rgb(self.root / "flat-base.png", rgb)
        cand = _write_rgb(self.root / "flat-candidate.png", rgb)

        report = measure(base, cand)
        native = report["native_diagnostics"]["baseline"]
        self.assertEqual(0.0, native["mean_gradient_magnitude"])
        self.assertEqual(0.0, native["edge_fraction_over_threshold"])
        self.assertEqual(0.0, native["laplacian_rms"])

    def test_shifted_capture_and_resolution_normalization_are_measurable(self):
        height, width = 48, 64
        y, x = np.indices((height, width))
        low = np.repeat((((x // 4 + y // 4) % 2) * 220 + 18).astype(np.uint8)[..., None], 3, axis=2)
        high = np.repeat(np.kron(low[..., 0], np.ones((2, 2), dtype=np.uint8))[..., None], 3, axis=2)
        shifted = np.roll(high, 2, axis=1)
        base_path = _write_rgb(self.root / "low.png", low)
        cand_path = _write_rgb(self.root / "high-shifted.png", shifted)

        report = measure(base_path, cand_path)
        comparison = report["normalized_comparison"]
        self.assertEqual((width, height), (comparison["width_px"], comparison["height_px"]))
        self.assertGreater(comparison["luminance_difference"]["root_mean_square"], 0.01)
        self.assertEqual(2 * width, report["inputs"]["candidate"]["width_px"])

    def test_packet_scene_fields_match_across_resolutions_with_fixed_exposure(self):
        low = np.full((32, 48, 3), 96, dtype=np.uint8)
        high = np.full((64, 96, 3), 96, dtype=np.uint8)
        base = _write_rgb(self.root / "low.png", low)
        cand = _write_rgb(self.root / "high.png", high)
        base_packet = self._packet_file(
            "base-packet.json", _packet(48, 32), "sha256:baseline-claim"
        )
        cand_packet = self._packet_file(
            "candidate-packet.json", _packet(96, 64), "sha256:candidate-claim"
        )

        report = measure(base, cand, base_packet, cand_packet)
        self.assertEqual("scene_fields_match", report["provenance"]["status"])
        self.assertTrue(report["provenance"]["scene_identity_match"])
        self.assertTrue(report["provenance"]["fixed_exposure_match"])
        self.assertEqual("not_verified", report["provenance"]["packet_integrity"])
        self.assertEqual("not_verified", report["provenance"]["capture_packet_binding"])
        self.assertEqual("sha256:baseline-claim", report["provenance"]["baseline_packet_sha256_claim"])
        self.assertEqual("sha256:candidate-claim", report["provenance"]["candidate_packet_sha256_claim"])

    def test_packet_dimensions_must_match_image_dimensions(self):
        image = _write_rgb(self.root / "capture.png", np.zeros((32, 48, 3), dtype=np.uint8))
        packet_a = self._packet_file("a.json", _packet(48, 32))
        packet_b = self._packet_file("b.json", _packet(96, 64))
        with self.assertRaisesRegex(ValueError, "dimensions"):
            measure(image, image, packet_a, packet_b)

    def test_different_aspect_ratio_is_rejected(self):
        base = _write_rgb(self.root / "base.png", np.zeros((32, 48, 3), dtype=np.uint8))
        candidate = _write_rgb(self.root / "candidate.png", np.zeros((32, 64, 3), dtype=np.uint8))
        with self.assertRaisesRegex(ValueError, "aspect ratios differ"):
            measure(base, candidate)

    def test_different_scene_or_exposure_is_rejected(self):
        rgb = np.full((32, 48, 3), 96, dtype=np.uint8)
        base = _write_rgb(self.root / "base.png", rgb)
        cand = _write_rgb(self.root / "candidate.png", rgb)
        base_packet = self._packet_file("base-packet.json", _packet(48, 32))
        changed_world = _packet(48, 32)
        changed_world["world_artifact_id"] = "world-other"
        scene_packet = self._packet_file("scene-packet.json", changed_world)
        exposure_packet = self._packet_file(
            "exposure-packet.json", _packet(48, 32, exposure=1.1)
        )
        with self.assertRaisesRegex(ValueError, "scene identity differs"):
            measure(base, cand, base_packet, scene_packet)
        with self.assertRaisesRegex(ValueError, "fixed exposure differs"):
            measure(base, cand, base_packet, exposure_packet)

    def test_packet_arguments_must_be_paired(self):
        image = _write_rgb(self.root / "capture.png", np.zeros((32, 48, 3), dtype=np.uint8))
        packet = self._packet_file("packet.json", _packet(48, 32))
        with self.assertRaisesRegex(ValueError, "both packet paths"):
            measure(image, image, packet, None)

    def test_malformed_packet_shapes_and_identity_fields_are_rejected(self):
        image = _write_rgb(self.root / "capture.png", np.zeros((32, 48, 3), dtype=np.uint8))
        good = _packet(48, 32)
        malformed = [
            ("top-level-list", [], "packet JSON must be a JSON object"),
            ("environment-list", {**good, "environment": []}, "environment must be an object"),
            ("boolean-exposure", {**good, "environment": {"exposure": True}}, "exposure must be finite"),
            (
                "boolean-width",
                {**good, "camera": {**good["camera"], "width_px": True}},
                "dimensions must be positive integers",
            ),
            (
                "missing-camera-id",
                {
                    **good,
                    "camera": {**good["camera"], "camera_id": None},
                    "capture": {**good["capture"], "camera_id": None},
                },
                "camera.camera_id must be a non-empty string",
            ),
            (
                "missing-schema",
                {key: value for key, value in good.items() if key != "schema_version"},
                "schema_version must be one of",
            ),
            (
                "unsupported-schema",
                {**good, "schema_version": "luxel.graphics-scene-packet/v5"},
                "schema_version must be one of",
            ),
            (
                "non-string-schema",
                {**good, "schema_version": []},
                "schema_version must be one of",
            ),
        ]
        for name, payload, message in malformed:
            with self.subTest(packet=name):
                path = self.root / f"{name}.json"
                wire = payload if not isinstance(payload, dict) else {
                    "body": payload,
                    "packet_sha256": "sha256:synthetic-unverified",
                }
                path.write_text(json.dumps(wire), encoding="utf-8")
                with self.assertRaisesRegex(ValueError, message):
                    measure(image, image, path, path)

    def test_capture_camera_id_must_match_a_real_camera_identity(self):
        image = _write_rgb(self.root / "capture.png", np.zeros((32, 48, 3), dtype=np.uint8))
        packet = _packet(48, 32)
        packet["capture"]["camera_id"] = ""
        path = self._packet_file("detached-camera.json", packet)
        with self.assertRaisesRegex(ValueError, "capture camera_id is detached"):
            measure(image, image, path, path)

    def test_scene_identity_binds_camera_pose_render_policy_and_unknown_root_fields(self):
        image = _write_rgb(self.root / "capture.png", np.zeros((32, 48, 3), dtype=np.uint8))
        base_path = self._packet_file("base-packet.json", _packet(48, 32))

        changed_pose = _packet(48, 32)
        changed_pose["camera"]["position_xyz_m"][0] += 1.0
        changed_policy = _packet(48, 32)
        changed_policy["render_policy"] = {"ibl": {"enabled": True}}
        changed_unknown_root = _packet(48, 32)
        changed_unknown_root["content_sha256"] = "sha256:unexpected-root-field"

        for name, changed_body in (
            ("camera-pose", changed_pose),
            ("render-policy", changed_policy),
            ("unknown-root", changed_unknown_root),
        ):
            with self.subTest(change=name):
                changed_path = self._packet_file(f"{name}.json", changed_body)
                with self.assertRaisesRegex(ValueError, "scene identity differs"):
                    measure(image, image, base_path, changed_path)

    def test_v7_packet_schema_is_supported(self):
        image = _write_rgb(self.root / "capture.png", np.zeros((32, 48, 3), dtype=np.uint8))
        base = _packet(48, 32)
        candidate = _packet(48, 32)
        base["schema_version"] = "luxel.graphics-scene-packet/v7"
        candidate["schema_version"] = "luxel.graphics-scene-packet/v7"
        base_path = self._packet_file("v7-base.json", base)
        candidate_path = self._packet_file("v7-candidate.json", candidate)

        report = measure(image, image, base_path, candidate_path)
        self.assertEqual("scene_fields_match", report["provenance"]["status"])


if __name__ == "__main__":
    unittest.main()

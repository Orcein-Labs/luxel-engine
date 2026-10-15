"""End-to-end Blender provider controls validated by Rust asset authority."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "pipeline"))

from rigging_provider import generate_rigged_character_control  # noqa: E402


BLENDER = os.environ.get("LUXEL_BLENDER_BIN") or shutil.which("blender")
ASSET_CONTRACT = Path(
    os.environ.get(
        "LUXEL_ASSET_CONTRACT_BIN",
        ROOT / "world_core" / "target" / "debug" / "luxel-asset-contract",
    )
)
# The negative-control GLB is an external sample kept out of the repo (research/ is gitignored).
BAD_CONTROL = Path(
    os.environ.get(
        "LUXEL_RIGGING_BAD_GLB",
        ROOT / "research" / "2026-10-15_rigging-control" / "bad-control.glb",
    )
)
BAD_CONTROL_SHA256 = "858fa104880822d081405579fb5b39d533d3b3b341d38aa1490a44b634f5e2b4"
REQUEST_TEMPLATE = ROOT / "tests" / "fixtures" / "rigging" / "blender_control_request.json"


@unittest.skipUnless(BLENDER, "Blender is required for the rigging-provider smoke test")
@unittest.skipUnless(ASSET_CONTRACT.is_file(), "build luxel-asset-contract before running provider tests")
class RiggingProviderIntegrationTest(unittest.TestCase):
    def _rust_inspect(self, glb: Path) -> dict:
        result = subprocess.run(
            (str(ASSET_CONTRACT), str(glb), "--kind", "character"),
            capture_output=True,
            text=True,
            check=False,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def _rust_prepare(self, glb: Path, request_template: dict, directory: Path, label: str):
        inspection = self._rust_inspect(glb)
        request = dict(request_template)
        request["expected_source_sha256"] = inspection["inspection"]["identity"]["source_sha256"]
        request_path = directory / f"{label}.request.json"
        request_path.write_text(json.dumps(request, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        return subprocess.run(
            (str(ASSET_CONTRACT), "prepare", str(glb), str(request_path)),
            capture_output=True,
            text=True,
            check=False,
            timeout=30,
        )

    def test_blender_control_is_deterministic_and_rust_promotes_measured_package(self):
        with tempfile.TemporaryDirectory(prefix="luxel-rigging-test-") as temporary:
            directory = Path(temporary)
            first_glb = directory / "control-a.glb"
            second_glb = directory / "control-b.glb"
            editable = directory / "control.blend"
            for output in (first_glb, second_glb):
                generated = generate_rigged_character_control(
                    output, editable_blend=editable if output == first_glb else None
                )
                self.assertEqual(generated.returncode, 0, generated.stdout + generated.stderr)
                self.assertGreater(output.stat().st_size, 1024)
            self.assertEqual(first_glb.read_bytes(), second_glb.read_bytes())
            probe = directory / "inspect_editable.py"
            facts = directory / "editable.json"
            probe.write_text("""import bpy, json, sys
rig = bpy.data.objects['control_skeleton']
mesh = bpy.data.objects['body_lod0']
facts = {'bones': list(rig.data.bones.keys()), 'actions': list(bpy.data.actions.keys()),
         'groups': list(mesh.vertex_groups.keys()),
         'armature': mesh.modifiers['skin'].object.name,
         'socket': bpy.data.objects['weapon_mount'].parent_bone}
with open(sys.argv[sys.argv.index('--') + 1], 'w') as stream: json.dump(facts, stream)
""")
            environment = dict(os.environ, ALSOFT_DRIVERS="null", OPENBLAS_NUM_THREADS="1")
            reopened = subprocess.run(
                [str(BLENDER), "-b", str(editable), "-t", "2", "-noaudio",
                 "--python-exit-code", "1", "--python", str(probe), "--", str(facts)],
                capture_output=True, text=True, timeout=30, env=environment,
            )
            self.assertEqual(reopened.returncode, 0, reopened.stdout + reopened.stderr)
            self.assertEqual(json.loads(facts.read_text()), {
                "bones": ["root", "spine", "hand_r"], "actions": ["attack", "idle", "locomotion"],
                "groups": ["root", "spine", "hand_r"], "armature": "control_skeleton",
                "socket": "hand_r",
            })
            inspection = self._rust_inspect(first_glb)
            self.assertIn("Blender", inspection["inspection"]["gltf"]["generator"])

            template = json.loads(REQUEST_TEMPLATE.read_text(encoding="utf-8"))
            first = self._rust_prepare(first_glb, template, directory, "first")
            second = self._rust_prepare(second_glb, template, directory, "second")
            self.assertEqual(first.returncode, 0, first.stderr + first.stdout)
            self.assertEqual(first.stdout, second.stdout)
            receipt = json.loads(first.stdout)
            self.assertEqual(receipt["schema_version"], "luxel.asset-runtime-receipt/v1")
            self.assertEqual(receipt["status"], "ready")
            self.assertEqual(receipt["findings"], [])
            source_digest = receipt["source_identity"]["source_sha256"]
            self.assertEqual(source_digest, inspection["inspection"]["identity"]["source_sha256"])
            self.assertEqual(source_digest, receipt["package"]["provenance"]["source_sha256"])
            self.assertEqual(len(source_digest), 64)
            package = receipt["package"]
            self.assertEqual(package["rig"]["joint_names"], ["root", "spine", "hand_r"])
            self.assertEqual(
                {clip["clip_name"] for clip in package["animations"]},
                {"idle", "locomotion", "attack"},
            )
            self.assertTrue(all(clip["has_sampled_motion"] for clip in package["animations"]))
            self.assertEqual(package["sockets"][0]["node_name"], "weapon_mount")
            self.assertEqual([lod["level"] for lod in package["lods"]], [0, 1])
            self.assertEqual(package["collision"]["shape"], "capsule")
            self.assertTrue(receipt["receipt_sha256"])

    def test_editable_job_failure_publishes_nothing_and_existing_source_is_preserved(self):
        with tempfile.TemporaryDirectory(prefix="luxel-editable-failure-") as temporary:
            directory = Path(temporary)
            glb, blend = directory / "control.glb", directory / "control.blend"
            with self.assertRaises(subprocess.TimeoutExpired):
                generate_rigged_character_control(glb, editable_blend=blend, timeout_seconds=0.001)
            self.assertEqual(list(directory.iterdir()), [])
            blend.write_bytes(b"existing editable source")
            with self.assertRaises(FileExistsError):
                generate_rigged_character_control(glb, editable_blend=blend)
            self.assertEqual(blend.read_bytes(), b"existing editable source")
            self.assertFalse(glb.exists())

    def test_supplied_bad_glb_stays_a_hard_rust_rejection_control(self):
        if not BAD_CONTROL.is_file():
            self.skipTest(f"permanent external negative control is unavailable: {BAD_CONTROL}")
        with tempfile.TemporaryDirectory(prefix="luxel-rigging-negative-") as temporary:
            directory = Path(temporary)
            inspection = self._rust_inspect(BAD_CONTROL)
            self.assertEqual(inspection["inspection"]["identity"]["source_sha256"], BAD_CONTROL_SHA256)
            request = json.loads(REQUEST_TEMPLATE.read_text(encoding="utf-8"))
            result = self._rust_prepare(BAD_CONTROL, request, directory, "bad-source")
            self.assertEqual(result.returncode, 3, result.stderr + result.stdout)
            receipt = json.loads(result.stdout)
            self.assertEqual(receipt["status"], "rejected")
            self.assertIsNone(receipt["package"])
            rejected_for = {finding["code"] for finding in receipt["findings"]}
            self.assertIn("invalid_skin", rejected_for)
            self.assertIn("missing_required_animation", rejected_for)
            self.assertIn("unsupported_required_extension", rejected_for)


if __name__ == "__main__":
    unittest.main()

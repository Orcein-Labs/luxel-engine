#!/usr/bin/env python3
"""Compare fixed-scene captures across output resolutions (CONVERGE-4 R-2).

This is an attribution aid, not a quality score. It compares full-frame,
normalized image domains by downsampling the larger capture to the smaller
capture's raster. It reports luminance difference plus edge/detail diagnostics;
more edge energy can mean useful detail, sharpening, noise, or aliasing.

Usage: crispness_measure.py BASELINE CANDIDATE [--baseline-packet JSON]
       [--candidate-packet JSON] [--json OUT]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image


SCHEMA = "luxel.crispness-comparison/v1"
SUPPORTED_PACKET_SCHEMAS = {
    "luxel.graphics-scene-packet/v6",
    "luxel.graphics-scene-packet/v7",
}
EDGE_THRESHOLD = 0.02  # linear-luma change per normalized analysis pixel


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_image(path: str | Path) -> np.ndarray:
    """Load an RGB image as sRGB8, rejecting empty or degenerate rasters."""
    path = Path(path)
    try:
        with Image.open(path) as source:
            image = source.convert("RGB")
            image.load()
            rgb = np.asarray(image, dtype=np.uint8)
    except (OSError, ValueError) as exc:
        raise ValueError(f"cannot read image {path}: {exc}") from exc
    if rgb.ndim != 3 or rgb.shape[2] != 3 or rgb.shape[0] < 2 or rgb.shape[1] < 2:
        raise ValueError(f"image must have at least 2x2 RGB pixels: {path}")
    return rgb


def linear_luminance(rgb8: np.ndarray) -> np.ndarray:
    """Convert sRGB8 RGB to Rec.709 linear luminance."""
    rgb = np.asarray(rgb8, dtype=np.float64) / 255.0
    linear = np.where(rgb <= 0.04045, rgb / 12.92, ((rgb + 0.055) / 1.055) ** 2.4)
    return 0.2126 * linear[..., 0] + 0.7152 * linear[..., 1] + 0.0722 * linear[..., 2]


def _linear_rgb(rgb8: np.ndarray) -> np.ndarray:
    rgb = np.asarray(rgb8, dtype=np.float64) / 255.0
    return np.where(rgb <= 0.04045, rgb / 12.92, ((rgb + 0.055) / 1.055) ** 2.4)


def _gradient_stats(luma: np.ndarray) -> dict[str, float | int]:
    # Pixel differences are in linear luminance. The sampling raster is included
    # beside these metrics; the values describe that raster, not perceived quality.
    gx = np.gradient(luma, axis=1)
    gy = np.gradient(luma, axis=0)
    magnitude = np.hypot(gx, gy)
    laplacian = np.gradient(gx, axis=1) + np.gradient(gy, axis=0)
    return {
        "mean_gradient_magnitude": float(magnitude.mean()),
        "rms_gradient_magnitude": float(np.sqrt(np.mean(magnitude * magnitude))),
        "edge_fraction_over_threshold": float((magnitude >= EDGE_THRESHOLD).mean()),
        "edge_threshold_linear_luma": EDGE_THRESHOLD,
        "laplacian_rms": float(np.sqrt(np.mean(laplacian * laplacian))),
    }


def _read_packet(path: str | Path) -> tuple[dict[str, Any], str, str]:
    source = Path(path)
    try:
        payload = json.loads(source.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read packet JSON {source}: {exc}") from exc
    if not isinstance(payload, dict):
        raise ValueError(f"packet JSON must be a JSON object: {source}")
    body = payload.get("body")
    if not isinstance(body, dict):
        raise ValueError(f"packet body must be a JSON object: {source}")
    claimed_digest = payload.get("packet_sha256")
    if not isinstance(claimed_digest, str) or not claimed_digest:
        raise ValueError(f"packet JSON must include a packet_sha256 string: {source}")
    schema_version = body.get("schema_version")
    if not isinstance(schema_version, str) or schema_version not in SUPPORTED_PACKET_SCHEMAS:
        supported = ", ".join(sorted(SUPPORTED_PACKET_SCHEMAS))
        raise ValueError(
            f"packet body schema_version must be one of {supported}, got {schema_version!r}: {source}"
        )
    # This is intentionally a minimal packet-shape/schema check, not the Rust
    # protocol validator. The outer packet_sha256 remains a claim; only the
    # file bytes are hashed here, and the seal is not recomputed or validated.
    return body, _sha256(source), claimed_digest


def _scene_identity(body: dict[str, Any]) -> str:
    """Hash packet content after removing output-resolution identity fields."""
    normalized = json.loads(json.dumps(body))
    normalized.pop("packet_id", None)
    camera = normalized.get("camera")
    if isinstance(camera, dict):
        camera.pop("width_px", None)
        camera.pop("height_px", None)
    capture = normalized.get("capture")
    if isinstance(capture, dict):
        capture.pop("capture_id", None)
        capture.pop("width_px", None)
        capture.pop("height_px", None)
    encoded = json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _packet_facts(body: dict[str, Any], image_shape: tuple[int, int], label: str) -> dict[str, Any]:
    height, width = image_shape
    camera = body.get("camera")
    capture = body.get("capture")
    if not isinstance(camera, dict) or not isinstance(capture, dict):
        raise ValueError(f"{label} packet must include camera and capture objects")
    for obj, obj_label in ((camera, "camera"), (capture, "capture")):
        dimensions = (obj.get("width_px"), obj.get("height_px"))
        if any(type(value) is not int or value <= 0 for value in dimensions):
            raise ValueError(f"{label} packet {obj_label} dimensions must be positive integers")
        identity_key = "camera_id" if obj_label == "camera" else "capture_id"
        identity = obj.get(identity_key)
        if not isinstance(identity, str) or not identity.strip():
            raise ValueError(f"{label} packet {obj_label}.{identity_key} must be a non-empty string")
    camera_size = (camera["width_px"], camera["height_px"])
    capture_size = (capture["width_px"], capture["height_px"])
    if camera_size != (width, height) or capture_size != (width, height):
        raise ValueError(
            f"{label} image dimensions {width}x{height} disagree with packet camera/capture "
            f"dimensions {camera_size}/{capture_size}"
        )
    if not isinstance(capture.get("camera_id"), str) or capture["camera_id"] != camera["camera_id"]:
        raise ValueError(f"{label} packet capture camera_id is detached from camera")
    environment = body.get("environment")
    if not isinstance(environment, dict):
        raise ValueError(f"{label} packet environment must be an object")
    exposure = environment.get("exposure")
    if (
        type(exposure) not in (int, float)
        or not math.isfinite(float(exposure))
        or not 0.01 <= float(exposure) <= 16.0
    ):
        raise ValueError(f"{label} packet environment.exposure must be finite and within [0.01, 16]")
    if capture.get("deterministic") is not True:
        raise ValueError(f"{label} packet capture must declare deterministic=true")
    return {
        "camera_id": camera.get("camera_id"),
        "capture_id": capture.get("capture_id"),
        "exposure": float(exposure),
        "deterministic": capture.get("deterministic"),
    }


def _downsample_luminance(rgb: np.ndarray, size: tuple[int, int]) -> np.ndarray:
    height, width = rgb.shape[:2]
    target_width, target_height = size
    if target_width > width or target_height > height:
        raise ValueError("normalized comparison may downsample only; it never upscales")
    linear = _linear_rgb(rgb)
    if (width, height) == size:
        return 0.2126 * linear[..., 0] + 0.7152 * linear[..., 1] + 0.0722 * linear[..., 2]
    resized = np.stack(
        [
            np.asarray(
                Image.fromarray(linear[..., channel].astype(np.float32), mode="F").resize(
                    size, Image.Resampling.LANCZOS
                ),
                dtype=np.float64,
            )
            for channel in range(3)
        ],
        axis=-1,
    )
    return 0.2126 * resized[..., 0] + 0.7152 * resized[..., 1] + 0.0722 * resized[..., 2]


def measure(
    baseline_path: str | Path,
    candidate_path: str | Path,
    baseline_packet_path: str | Path | None = None,
    candidate_packet_path: str | Path | None = None,
) -> dict[str, Any]:
    """Measure two captures over the same normalized full-frame domain."""
    baseline_path, candidate_path = Path(baseline_path), Path(candidate_path)
    base = read_image(baseline_path)
    cand = read_image(candidate_path)
    bh, bw = base.shape[:2]
    ch, cw = cand.shape[:2]
    if bw * ch != cw * bh:
        raise ValueError(
            f"capture aspect ratios differ ({bw}x{bh} vs {cw}x{ch}); "
            "the scene framing would not be comparable"
        )
    if (baseline_packet_path is None) != (candidate_packet_path is None):
        raise ValueError("supply both packet paths or neither")

    packet_info: dict[str, Any]
    scene_identity_match: bool | None = None
    exposure_match: bool | None = None
    if baseline_packet_path is None:
        packet_info = {"status": "unverified", "reason": "packet provenance was not supplied"}
    else:
        base_body, base_file_sha, base_claimed_sha = _read_packet(baseline_packet_path)
        cand_body, cand_file_sha, cand_claimed_sha = _read_packet(candidate_packet_path)  # type: ignore[arg-type]
        base_facts = _packet_facts(base_body, (bh, bw), "baseline")
        cand_facts = _packet_facts(cand_body, (ch, cw), "candidate")
        exposure_match = base_facts["exposure"] == cand_facts["exposure"]
        if not exposure_match:
            raise ValueError(
                f"fixed exposure differs ({base_facts['exposure']} vs {cand_facts['exposure']})"
            )
        scene_identity_match = _scene_identity(base_body) == _scene_identity(cand_body)
        if not scene_identity_match:
            raise ValueError("packet scene identity differs after removing output resolution fields")
        packet_info = {
            "status": "scene_fields_match",
            "scene_identity_sha256": _scene_identity(base_body),
            "scene_identity_match": True,
            "fixed_exposure_match": True,
            "fixed_exposure": base_facts["exposure"],
            "packet_integrity": "not_verified",
            "capture_packet_binding": "not_verified",
            "baseline_packet_file_sha256": base_file_sha,
            "candidate_packet_file_sha256": cand_file_sha,
            "baseline_packet_sha256_claim": base_claimed_sha,
            "candidate_packet_sha256_claim": cand_claimed_sha,
            "baseline_packet_facts": base_facts,
            "candidate_packet_facts": cand_facts,
        }

    common_width, common_height = min(bw, cw), min(bh, ch)
    base_common = _downsample_luminance(base, (common_width, common_height))
    cand_common = _downsample_luminance(cand, (common_width, common_height))
    delta = cand_common - base_common
    abs_delta = np.abs(delta)
    common_pixels = {
        "width_px": common_width,
        "height_px": common_height,
        "domain": "same full-frame normalized coordinates; larger raster Lanczos-downsampled",
        "baseline_luminance": _gradient_stats(base_common),
        "candidate_luminance": _gradient_stats(cand_common),
        "luminance_difference": {
            "mean_absolute": float(abs_delta.mean()),
            "root_mean_square": float(np.sqrt(np.mean(delta * delta))),
            "maximum_absolute": float(abs_delta.max()),
        },
    }
    return {
        "schema_version": SCHEMA,
        "interpretation": (
            "descriptive capture diagnostics only; edge/detail energy does not establish visual quality, "
            "and this still-image comparison cannot certify temporal stability"
        ),
        "inputs": {
            "baseline": {"path": str(baseline_path), "sha256": _sha256(baseline_path), "width_px": bw, "height_px": bh},
            "candidate": {"path": str(candidate_path), "sha256": _sha256(candidate_path), "width_px": cw, "height_px": ch},
        },
        "provenance": packet_info,
        "normalized_comparison": common_pixels,
        "native_diagnostics": {
            "baseline": {"sampling_width_px": bw, "sampling_height_px": bh, **_gradient_stats(linear_luminance(base))},
            "candidate": {"sampling_width_px": cw, "sampling_height_px": ch, **_gradient_stats(linear_luminance(cand))},
            "caveat": "native pixel gradients are sampling-scale diagnostics and must not be ranked as quality",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--baseline-packet", type=Path)
    parser.add_argument("--candidate-packet", type=Path)
    parser.add_argument("--json", type=Path, help="also write the report to this path")
    args = parser.parse_args()
    try:
        report = measure(args.baseline, args.candidate, args.baseline_packet, args.candidate_packet)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    print(rendered, end="")
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(rendered, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

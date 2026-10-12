#!/usr/bin/env python3
"""Measure a sparse cast shadow against the packet's directional light.

The Julia GPU test writes P6 captures of a bare receiver and the same receiver
with one prism caster. This tool creates reviewable PNGs, subtracts those real
captures, and finds the darkness-weighted shadow centroid outside the caster
footprint. The deliberately reversed sun vector is evaluated through the same
acceptance rule and must be rejected.
"""
import argparse
import hashlib
import json
import math
import os
from PIL import Image


def read_ppm(path):
    with open(path, "rb") as stream:
        magic = stream.readline().strip()
        dimensions = stream.readline().strip()
        maximum = stream.readline().strip()
        if magic != b"P6" or maximum != b"255":
            raise ValueError(f"unsupported PPM header in {path}")
        width, height = (int(part) for part in dimensions.split())
        pixels = stream.read()
    if len(pixels) != width * height * 3:
        raise ValueError(f"invalid PPM pixel count in {path}")
    return width, height, pixels


def write_png(path, width, height, rgb):
    Image.frombytes("RGB", (width, height), rgb).save(path, format="PNG", compress_level=6)


def luminance(rgb, offset):
    def linear(component):
        srgb = component / 255.0
        return srgb / 12.92 if srgb <= 0.04045 else ((srgb + 0.055) / 1.055) ** 2.4

    return 0.2126 * linear(rgb[offset]) + 0.7152 * linear(rgb[offset + 1]) + 0.0722 * linear(rgb[offset + 2])


def accepts_direction(displacement, cosine, shadow_pixels):
    return shadow_pixels >= 8 and 1.0 <= displacement <= 8.0 and cosine >= 0.85


def synthetic_wrong_direction_rejected():
    # Known 3-4-5 m cast offset aligned with +X/+Z. Its reversed sun must fail
    # the same gate used for measured pixels.
    dx, dz = 3.0, 4.0
    displacement = math.hypot(dx, dz)
    good_cosine = (dx * 0.6 + dz * 0.8) / displacement
    bad_cosine = (dx * -0.6 + dz * -0.8) / displacement
    return accepts_direction(displacement, good_cosine, 32) and not accepts_direction(
        displacement, bad_cosine, 32
    )


def measure_pair(bare_path, caster_path, case, reverse_direction=False):
    width, height, bare = read_ppm(bare_path)
    other_width, other_height, caster = read_ppm(caster_path)
    if (width, height) != (other_width, other_height):
        raise ValueError("receiver and caster captures have different dimensions")
    ppm = float(case["pixels_per_meter"])
    camera_x, camera_z = map(float, case["camera_center_xz"])
    base_x, base_z = map(float, case["caster_base_xz"])
    weight_sum = dx_sum = dz_sum = 0.0
    shadow_pixels = 0
    for y in range(height):
        world_z = camera_z + (y + 0.5 - 0.5 * height) / ppm
        for x in range(width):
            world_x = camera_x + (x + 0.5 - 0.5 * width) / ppm
            dx, dz = world_x - base_x, world_z - base_z
            # The projected 1.6 m caster footprint and a small edge margin are
            # excluded; only the cast shadow on the receiver is measured.
            if abs(dx) < 1.45 and abs(dz) < 1.45:
                continue
            offset = 3 * (y * width + x)
            delta = luminance(bare, offset) - luminance(caster, offset)
            if delta < 0.02:
                continue
            weight = delta - 0.01
            shadow_pixels += 1
            weight_sum += weight
            dx_sum += weight * dx
            dz_sum += weight * dz
    if not shadow_pixels or weight_sum <= 0.0:
        return {"shadow_pixels": shadow_pixels, "displacement_m": 0.0,
                "direction_cosine": -1.0, "opposite_direction_rejected": True,
                "reason": "no measurable shadow pixels"}
    measured_x, measured_z = dx_sum / weight_sum, dz_sum / weight_sum
    displacement = math.hypot(measured_x, measured_z)
    direction = case["direction_xyz"]
    sun_x, sun_y, sun_z = (float(direction[index]) for index in range(3))
    if reverse_direction:
        sun_x, sun_z = -sun_x, -sun_z
    sun_length = math.hypot(sun_x, sun_z)
    cosine = (measured_x * sun_x + measured_z * sun_z) / max(1.0e-12, displacement * sun_length)
    height = float(case["caster_height_m"])
    predicted_x = -0.5 * height * sun_x / sun_y
    predicted_z = -0.5 * height * sun_z / sun_y
    vector_error = math.hypot(measured_x - predicted_x, measured_z - predicted_z)
    # This finite displacement band plus a strong directional cosine makes
    # the test reject swapped/reversed lights instead of merely finding dark pixels.
    accepted = accepts_direction(displacement, cosine, shadow_pixels)
    opposite_cosine = -cosine
    opposite_accepted = accepts_direction(displacement, opposite_cosine, shadow_pixels)
    return {
        "shadow_pixels": shadow_pixels,
        "shadow_centroid_displacement_xz_m": [round(measured_x, 4), round(measured_z, 4)],
        "displacement_m": round(displacement, 4),
        "unmasked_geometric_shadow_center_xz_m": [round(predicted_x, 4), round(predicted_z, 4)],
        "unmasked_geometric_reference_error_m": round(vector_error, 4),
        "direction_cosine": round(cosine, 5),
        "expected_direction_xz": [sun_x, sun_z],
        "reversed_direction_cosine": round(opposite_cosine, 5),
        "accepted": accepted,
        "opposite_direction_rejected": not opposite_accepted,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest")
    parser.add_argument("--output", required=True)
    parser.add_argument("--negative-control", action="store_true",
                        help="evaluate the observed shadow against the reversed packet sun")
    parser.add_argument("--strict", action="store_true",
                        help="exit nonzero if any sparse direction measurement fails")
    args = parser.parse_args()
    manifest_path = os.path.abspath(args.manifest)
    run_dir = os.path.dirname(manifest_path)
    with open(manifest_path, "r", encoding="utf-8") as stream:
        manifest = json.load(stream)
    by_name = {entry["name"]: entry for entry in manifest["cases"]}
    results = []
    for case in manifest["cases"]:
        source = os.path.join(run_dir, case["ppm"])
        width, height, rgb = read_ppm(source)
        png_name = os.path.splitext(case["ppm"])[0] + ".png"
        write_png(os.path.join(run_dir, png_name), width, height, rgb)
        with open(source, "rb") as stream:
            digest = hashlib.sha256(stream.read()).hexdigest()
        row = {"name": case["name"], "ppm": case["ppm"], "png": png_name,
               "render_capture_sha256": case["render_capture_sha256"],
               "ppm_sha256": digest,
               "width_px": width, "height_px": height}
        if case["name"].endswith("-caster"):
            receiver_name = case["name"][: -len("-caster")] + "-receiver"
            receiver = by_name.get(receiver_name)
            if receiver is None:
                raise ValueError(f"missing receiver control for {case['name']}")
            row.update(measure_pair(os.path.join(run_dir, receiver["ppm"]), source, case,
                                    reverse_direction=args.negative_control))
        results.append(row)
    result_by_name = {row["name"]: row for row in results}
    warm = result_by_name.get("camera-b-warm-after-a")
    cold = result_by_name.get("camera-b-cold")
    output = {
        "schema": "luxel.shadow-consistency-measurement/v1",
        "source_manifest": os.path.basename(manifest_path),
        "negative_control": args.negative_control,
        "synthetic_wrong_direction_rejected": synthetic_wrong_direction_rejected(),
        "displacement_reference_note": (
            "The vector reference is the ideal prism-shadow midpoint before caster-footprint masking. "
            "The measured centroid may shift because of footprint exclusion, PCF filtering, and depth bias; "
            "the 2 m vector-error bound is diagnostic. Direction cosine is the primary consistency gate."
        ),
        "cases": results,
        "warm_cold_camera_b_equal": bool(warm and cold and
                                          warm["render_capture_sha256"] == cold["render_capture_sha256"] and
                                          warm["ppm_sha256"] == cold["ppm_sha256"]),
    }
    measured = [row for row in results if row["name"].endswith("-caster")]
    output["negative_control_passed"] = bool(args.negative_control and measured and
                                            all("accepted" in row and not row["accepted"] for row in measured))
    output["all_directions_accepted"] = bool(measured and all(row.get("accepted", False) for row in measured))
    with open(args.output, "w", encoding="utf-8") as stream:
        json.dump(output, stream, indent=2)
        stream.write("\n")
    print(json.dumps(output, indent=2))
    if args.negative_control:
        # Exit nonzero only when the intentionally wrong direction was rejected
        # for every real capture. This is the end-to-end negative gate.
        return 2 if output["negative_control_passed"] else 0
    if args.strict and not output["all_directions_accepted"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

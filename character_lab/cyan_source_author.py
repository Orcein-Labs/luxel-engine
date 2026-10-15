"""CYAN-SOURCE-1: layered rest assembly and concealed-material plates.

Reads the immutable female-v2 reference, writes exclusive visible layers,
concealed overlap plates, a coverage ledger, four-background reviews, and a
PSD import candidate. Native raster receipt schemas are unchanged.
"""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/avatars/cyan/source/cyan-source-art-female-v2.png"
OUT = ROOT / "assets/avatars/cyan/layered"
FEMALE_V2_SHA256 = "82da3196532fc000fdc6ce4569a8fe031c1fb260f5d5dacb3e3f1a9abd53b3b8"
REQUIRED_SURFACES = (
    "lids",
    "sclera",
    "pupils",
    "lips",
    "mouth_interior",
    "face_behind_bangs",
    "neck_behind_jaw_collar",
    "hair_overlaps",
)

# Measured from the frozen reference (see recipe landmarks).
LEFT_IRIS = (435, 340)
RIGHT_IRIS = (577, 322)
MOUTH_CENTER = (515, 442)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_source() -> np.ndarray:
    digest = sha256(SOURCE)
    if digest != FEMALE_V2_SHA256:
        raise SystemExit(f"female-v2 digest mismatch: {digest}")
    image = np.array(Image.open(SOURCE).convert("RGBA"), dtype=np.uint8)
    if image.shape != (1536, 1024, 4):
        raise SystemExit(f"unexpected source shape {image.shape}")
    return image


def connected_from(seed_mask: np.ndarray, y: int, x: int) -> np.ndarray:
    h, w = seed_mask.shape
    seen = np.zeros((h, w), dtype=bool)
    if not seed_mask[y, x]:
        raise SystemExit("seed is outside character classifier")
    stack = [(y, x)]
    seen[y, x] = True
    while stack:
        cy, cx = stack.pop()
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            ny, nx = cy + dy, cx + dx
            if 0 <= ny < h and 0 <= nx < w and not seen[ny, nx] and seed_mask[ny, nx]:
                seen[ny, nx] = True
                stack.append((ny, nx))
    return seen


def fill_holes(component: np.ndarray) -> np.ndarray:
    h, w = component.shape
    outside = np.zeros((h, w), dtype=bool)
    stack = []
    for x in range(w):
        if not component[0, x]:
            outside[0, x] = True
            stack.append((0, x))
        if not component[h - 1, x]:
            outside[h - 1, x] = True
            stack.append((h - 1, x))
    for y in range(h):
        if not component[y, 0]:
            outside[y, 0] = True
            stack.append((y, 0))
        if not component[y, w - 1]:
            outside[y, w - 1] = True
            stack.append((y, w - 1))
    while stack:
        cy, cx = stack.pop()
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            ny, nx = cy + dy, cx + dx
            if 0 <= ny < h and 0 <= nx < w and not outside[ny, nx] and not component[ny, nx]:
                outside[ny, nx] = True
                stack.append((ny, nx))
    return ~outside


def ellipse_mask(h: int, w: int, cx: float, cy: float, rx: float, ry: float) -> np.ndarray:
    yy, xx = np.ogrid[:h, :w]
    return ((xx - cx) / rx) ** 2 + ((yy - cy) / ry) ** 2 <= 1.0


def sample_mean(image: np.ndarray, mask: np.ndarray) -> np.ndarray:
    if not mask.any():
        return np.array([200, 150, 140, 255], dtype=np.uint8)
    return image[mask].mean(axis=0).astype(np.uint8)


def nearest_fill(image: np.ndarray, destination: np.ndarray, donor: np.ndarray) -> np.ndarray:
    """Copy nearest donor pixel into destination pixels (concealed underpaint)."""
    donor_y, donor_x = np.where(donor)
    dest_y, dest_x = np.where(destination)
    out = np.zeros_like(image)
    if donor_y.size == 0 or dest_y.size == 0:
        return out
    if donor_y.size > 2500:
        pick = np.linspace(0, donor_y.size - 1, 2500).astype(int)
        donor_y, donor_x = donor_y[pick], donor_x[pick]
    chunk = 2000
    for start in range(0, dest_y.size, chunk):
        sl = slice(start, start + chunk)
        dy = dest_y[sl][:, None].astype(np.int32)
        dx = dest_x[sl][:, None].astype(np.int32)
        dist = (dy - donor_y[None, :]) ** 2 + (dx - donor_x[None, :]) ** 2
        nearest = dist.argmin(axis=1)
        out[dest_y[sl], dest_x[sl]] = image[donor_y[nearest], donor_x[nearest]]
        out[dest_y[sl], dest_x[sl], 3] = 255
    return out


def crop_layer(rgba: np.ndarray) -> tuple[np.ndarray, list[int]]:
    alpha = rgba[:, :, 3]
    ys, xs = np.where(alpha > 0)
    if xs.size == 0:
        return rgba[:1, :1].copy(), [0, 0]
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    y0, y1 = int(ys.min()), int(ys.max()) + 1
    return rgba[y0:y1, x0:x1].copy(), [x0, y0]


def save_png(path: Path, pixels: np.ndarray) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(pixels).save(path, format="PNG", optimize=True)


def layer_rgba(image: np.ndarray, mask: np.ndarray) -> np.ndarray:
    out = np.zeros_like(image)
    out[mask] = image[mask]
    return out


def paint_rgba(mask: np.ndarray, color: np.ndarray) -> np.ndarray:
    out = np.zeros(mask.shape + (4,), dtype=np.uint8)
    out[mask] = color
    return out


def build_silhouette(image: np.ndarray) -> np.ndarray:
    r, g, b, a = image[:, :, 0], image[:, :, 1], image[:, :, 2], image[:, :, 3]
    luma = image[:, :, :3].mean(axis=2)
    sat = image[:, :, :3].max(axis=2) - image[:, :, :3].min(axis=2)
    classified = (a >= 40) & (
        (luma > 70)
        | ((sat > 25) & (luma > 28))
        | ((a > 200) & (luma > 18) & ((b > r + 8) | (sat > 12)))
        | ((a > 200) & (luma > 12) & (luma < 90) & (sat < 40))
    )
    component = connected_from(classified, 768, 512)
    return fill_holes(component)


def exclusive_visible(image: np.ndarray, silhouette: np.ndarray) -> dict[str, np.ndarray]:
    r, g, b, a = image[:, :, 0], image[:, :, 1], image[:, :, 2], image[:, :, 3]
    luma = image[:, :, :3].mean(axis=2)
    sat = image[:, :, :3].max(axis=2) - image[:, :, :3].min(axis=2)
    h, w = silhouette.shape
    left_eye = ellipse_mask(h, w, LEFT_IRIS[0], LEFT_IRIS[1], 42, 18)
    right_eye = ellipse_mask(h, w, RIGHT_IRIS[0], RIGHT_IRIS[1], 42, 18)
    eyes = left_eye | right_eye
    iris = (
        ellipse_mask(h, w, LEFT_IRIS[0], LEFT_IRIS[1], 14, 14)
        | ellipse_mask(h, w, RIGHT_IRIS[0], RIGHT_IRIS[1], 14, 14)
    ) & silhouette
    sclera = eyes & silhouette & ~iris & (luma > 130)
    lips = ellipse_mask(h, w, MOUTH_CENTER[0], MOUTH_CENTER[1], 48, 16) & silhouette
    lips &= (r > 160) & (r > g + 8) & (r > b + 5)
    face_ellipse = ellipse_mask(h, w, 512, 390, 145, 195)
    skin = silhouette & (r > 150) & (g > 95) & (b > 80) & (r > b + 12) & (luma < 235)
    hair = silhouette & (b > r + 8) & (luma < 170)
    white = silhouette & (luma > 200) & (sat < 45)
    remaining = silhouette.copy()

    def take(mask: np.ndarray) -> np.ndarray:
        chosen = mask & remaining
        remaining[chosen] = False
        return chosen

    layers = {
        "iris_left": take(iris & left_eye),
        "iris_right": take(iris & right_eye),
        "sclera_left": take(sclera & left_eye),
        "sclera_right": take(sclera & right_eye),
        "lip_upper": take(lips & (np.indices(silhouette.shape)[0] <= MOUTH_CENTER[1])),
        "lip_lower": take(lips & (np.indices(silhouette.shape)[0] > MOUTH_CENTER[1])),
        "hair_front": take(hair & face_ellipse),
        "face": take(skin & face_ellipse),
        "neck": take(skin & ~face_ellipse & (np.indices(silhouette.shape)[0] < 760)),
        "torso": take(white | (silhouette & (luma > 170) & (sat < 55) & (np.indices(silhouette.shape)[0] > 520))),
        "collar": take(silhouette & (np.indices(silhouette.shape)[0] > 620) & (np.indices(silhouette.shape)[0] < 860) & (sat > 20) & (luma < 140) & (b + 10 > r)),
        "coat": take(silhouette & (np.indices(silhouette.shape)[0] > 560) & (luma < 120)),
        "hair_back": take(hair),
    }
    leftover = remaining
    if leftover.any():
        # Keep every silhouette pixel owned: remaining fringe follows nearest hair or coat.
        layers["hair_back"] |= leftover & (np.indices(silhouette.shape)[0] < 700)
        layers["coat"] |= leftover & (np.indices(silhouette.shape)[0] >= 700)
        remaining[leftover] = False
    return layers


def concealed_plates(image: np.ndarray, silhouette: np.ndarray, visible: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
    h, w = silhouette.shape
    face_hull = ellipse_mask(h, w, 512, 390, 155, 210) & fill_holes(visible["face"] | visible["hair_front"] | visible["neck"])
    bangs = visible["hair_front"] & face_hull
    face_donor = visible["face"]
    face_behind = bangs & ~visible["face"]
    face_rgba = nearest_fill(image, face_behind, face_donor)

    neck_hull = ellipse_mask(h, w, 512, 620, 120, 160)
    jaw = ellipse_mask(h, w, 512, 470, 90, 55)
    collar_zone = visible["collar"] | visible["torso"]
    neck_dest = (neck_hull | jaw) & silhouette & ~visible["neck"] & ~visible["face"]
    neck_rgba = nearest_fill(image, neck_dest, visible["neck"] if visible["neck"].any() else face_donor)

    sclera_color = np.array([248, 242, 236, 255], dtype=np.uint8)
    left_white = ellipse_mask(h, w, LEFT_IRIS[0], LEFT_IRIS[1], 40, 16)
    right_white = ellipse_mask(h, w, RIGHT_IRIS[0], RIGHT_IRIS[1], 40, 16)
    sclera_left_full = paint_rgba(left_white, sclera_color)
    sclera_right_full = paint_rgba(right_white, sclera_color)
    # Concealed sclera is the plate under the iris; visible sclera stays the source layer.
    sclera_left_full[visible["iris_left"] | visible["sclera_left"]] = 0
    sclera_right_full[visible["iris_right"] | visible["sclera_right"]] = 0

    lid_color = sample_mean(image, visible["face"] & ellipse_mask(h, w, 512, 300, 80, 40))
    lids = {}
    for name, cx, cy in (("left", LEFT_IRIS[0], LEFT_IRIS[1]), ("right", RIGHT_IRIS[0], RIGHT_IRIS[1])):
        upper = ellipse_mask(h, w, cx, cy - 4, 42, 16) & ~ellipse_mask(h, w, cx, cy + 10, 42, 14)
        lower = ellipse_mask(h, w, cx, cy + 8, 38, 10) & ~ellipse_mask(h, w, cx, cy - 6, 40, 14)
        lids[f"lid_upper_{name}"] = paint_rgba(upper, lid_color)
        lids[f"lid_lower_{name}"] = paint_rgba(lower, lid_color)

    interior = ellipse_mask(h, w, MOUTH_CENTER[0], MOUTH_CENTER[1] + 2, 28, 10)
    interior &= ~visible["lip_upper"] & ~visible["lip_lower"]
    mouth_interior = paint_rgba(interior, np.array([92, 42, 48, 255], dtype=np.uint8))

    hair_overlap = visible["hair_front"] & (visible["face"] | visible["neck"] | visible["torso"] | face_hull)
    hair_under = nearest_fill(image, hair_overlap, visible["hair_back"] if visible["hair_back"].any() else visible["hair_front"])

    plates = {
        "face_behind_bangs": face_rgba,
        "neck_behind_jaw_collar": neck_rgba,
        "sclera_left_under": sclera_left_full,
        "sclera_right_under": sclera_right_full,
        "mouth_interior": mouth_interior,
        "hair_underpaint": hair_under,
        **lids,
    }
    return plates


def composite(image: np.ndarray, stack: list[tuple[str, np.ndarray]]) -> np.ndarray:
    out = np.zeros_like(image)
    for _, rgba in stack:
        alpha = rgba[:, :, 3] > 0
        out[alpha] = rgba[alpha]
    return out


def four_backgrounds(rest: np.ndarray) -> dict[str, np.ndarray]:
    h, w = rest.shape[:2]
    alpha = rest[:, :, 3] > 0
    checker = np.zeros((h, w, 4), dtype=np.uint8)
    yy, xx = np.indices((h, w))
    light = ((xx // 16) + (yy // 16)) % 2 == 0
    checker[light] = (240, 240, 240, 255)
    checker[~light] = (40, 40, 40, 255)
    backs = {
        "dark": np.full((h, w, 4), (12, 12, 16, 255), dtype=np.uint8),
        "light": np.full((h, w, 4), (245, 245, 248, 255), dtype=np.uint8),
        "cyan": np.full((h, w, 4), (40, 180, 200, 255), dtype=np.uint8),
        "checker": checker,
    }
    reviews = {}
    for name, bg in backs.items():
        frame = bg.copy()
        frame[alpha] = rest[alpha]
        reviews[name] = frame
    return reviews


def _u8(value: int) -> bytes:
    return bytes([value & 255])


def _u16(value: int) -> bytes:
    return value.to_bytes(2, "big")


def _u32(value: int) -> bytes:
    return value.to_bytes(4, "big")


def _i16(value: int) -> bytes:
    return value.to_bytes(2, "big", signed=True)


def _pascal_name(name: str) -> bytes:
    raw = name.encode("ascii")[:255]
    body = bytes([len(raw)]) + raw
    pad = (4 - (len(body) % 4)) % 4
    return body + b"\x00" * pad


def _packbits_row(row: bytes) -> bytes:
    out = bytearray()
    i = 0
    n = len(row)
    while i < n:
        j = i + 1
        while j < n and row[j] == row[i] and (j - i) < 128:
            j += 1
        run = j - i
        if run >= 3:
            out.append((257 - run) & 255)
            out.append(row[i])
            i = j
            continue
        start = i
        i += 1
        while i < n and (i - start) < 128:
            if i + 2 < n and row[i] == row[i + 1] == row[i + 2]:
                break
            i += 1
        length = i - start
        out.append(length - 1)
        out.extend(row[start:i])
    return bytes(out)


def _channel_plane(rgba: np.ndarray, index: int) -> bytes:
    plane = np.ascontiguousarray(rgba[:, :, index])
    height, width = plane.shape
    lengths = []
    payload = bytearray()
    for y in range(height):
        encoded = _packbits_row(plane[y].tobytes())
        lengths.append(len(encoded))
        payload.extend(encoded)
    return _u16(1) + b"".join(_u16(n) for n in lengths) + bytes(payload)


def _image_resource(res_id: int, data: bytes, name: bytes = b"") -> bytes:
    """One Photoshop image-resource block. Cubism's parser always reads at least one 8BIM."""
    name_bytes = bytes([len(name)]) + name
    if len(name_bytes) % 2:
        name_bytes += b"\x00"
    block = b"8BIM" + _u16(res_id) + name_bytes + _u32(len(data)) + data
    if len(data) % 2:
        block += b"\x00"
    return block


def _resolution_info() -> bytes:
    # 72 dpi as 16.16 fixed, unit 1 = pixels/inch, 1 = inches.
    dpi = _u32(0x00480000)
    return dpi + _u16(1) + _u16(1) + dpi + _u16(1) + _u16(1)


def write_psd(full_layers: list[tuple[str, np.ndarray]], canvas: tuple[int, int], dest: Path) -> None:
    """Write an 8-bit RGB+alpha Photoshop file Cubism can import as ArtMeshes."""
    width, height = canvas
    records = []
    channels_blob = b""
    for name, rgba in reversed(full_layers):  # PSD stores bottom layer first.
        if rgba.shape[0] != height or rgba.shape[1] != width:
            canvas_rgba = np.zeros((height, width, 4), dtype=np.uint8)
            canvas_rgba[: rgba.shape[0], : rgba.shape[1]] = rgba
            rgba = canvas_rgba
        planes = [
            (-1, _channel_plane(rgba, 3)),
            (0, _channel_plane(rgba, 0)),
            (1, _channel_plane(rgba, 1)),
            (2, _channel_plane(rgba, 2)),
        ]
        packed = []
        extra = _u32(0) + _u32(0) + _pascal_name(name)
        header = _u32(0) + _u32(0) + _u32(height) + _u32(width) + _u16(4)
        for channel_id, plane in planes:
            packed.append((_i16(channel_id), _u32(len(plane)), plane))
            channels_blob += plane
        for channel_id, length, _payload in packed:
            header += channel_id + length
        # flags bit 3 marks Photoshop 5+ extra; visible/unprotected otherwise.
        header += b"8BIM" + b"norm" + _u8(255) + _u8(0) + _u8(8) + _u8(0)
        header += _u32(len(extra)) + extra
        records.append(header)
    layer_info = _u16(len(full_layers)) + b"".join(records) + channels_blob
    if len(layer_info) % 2:
        layer_info += b"\x00"
    layer_mask_section = _u32(len(layer_info)) + layer_info
    resources = _image_resource(0x03ED, _resolution_info())
    composite = np.zeros((height, width, 4), dtype=np.uint8)
    for _name, rgba in full_layers:
        alpha = rgba[:, :, 3] > 0
        composite[alpha] = rgba[alpha]
    row_table = b""
    payload = b""
    for plane in (0, 1, 2, 3):
        plane_arr = np.ascontiguousarray(composite[:, :, plane])
        for y in range(height):
            encoded = _packbits_row(plane_arr[y].tobytes())
            row_table += _u16(len(encoded))
            payload += encoded
    image_data = _u16(1) + row_table + payload
    header = (
        b"8BPS"
        + _u16(1)
        + b"\x00" * 6
        + _u16(4)
        + _u32(height)
        + _u32(width)
        + _u16(8)
        + _u16(3)
        + _u32(0)
        + _u32(len(resources))
        + resources
        + _u32(len(layer_mask_section))
        + layer_mask_section
        + image_data
    )
    dest.write_bytes(header)


def main() -> int:
    image = load_source()
    silhouette = build_silhouette(image)
    visible_masks = exclusive_visible(image, silhouette)
    visible_rgba = {name: layer_rgba(image, mask) for name, mask in visible_masks.items()}
    concealed = concealed_plates(image, silhouette, visible_masks)

    part_stack = [
        "hair_back",
        "coat",
        "torso",
        "collar",
        "hair_underpaint",
        "neck_behind_jaw_collar",
        "neck",
        "face_behind_bangs",
        "face",
        "sclera_left_under",
        "sclera_right_under",
        "sclera_left",
        "sclera_right",
        "iris_left",
        "iris_right",
        "mouth_interior",
        "lid_lower_left",
        "lid_lower_right",
        "lip_lower",
        "lip_upper",
        "lid_upper_left",
        "lid_upper_right",
        "hair_front",
    ]
    kind = {name: "visible" for name in visible_rgba}
    kind.update({name: "concealed" for name in concealed})
    all_rgba = {**visible_rgba, **concealed}

    OUT.mkdir(parents=True, exist_ok=True)
    layer_dir = OUT / "layers"
    if layer_dir.exists():
        for old in layer_dir.glob("*.png"):
            old.unlink()
    layer_dir.mkdir(exist_ok=True)

    layers_meta = []
    full_for_psd = []
    for name in part_stack:
        rgba = all_rgba[name]
        cropped, origin = crop_layer(rgba)
        rel = f"layers/{name}.png"
        save_png(OUT / rel, cropped)
        surfaces = []
        if name.startswith("lid_"):
            surfaces.append("lids")
        if name.startswith("sclera_"):
            surfaces.append("sclera")
        if name.startswith("iris_"):
            surfaces.append("pupils")
        if name.startswith("lip_"):
            surfaces.append("lips")
        if name == "mouth_interior":
            surfaces.append("mouth_interior")
        if name == "face_behind_bangs":
            surfaces.append("face_behind_bangs")
        if name == "neck_behind_jaw_collar":
            surfaces.append("neck_behind_jaw_collar")
        if name in {"hair_front", "hair_back", "hair_underpaint"}:
            surfaces.append("hair_overlaps")
        layers_meta.append({
            "id": name,
            "label": name.replace("_", " "),
            "kind": kind[name],
            "surfaces": surfaces,
            "png": rel,
            "origin": origin,
            "size": [int(cropped.shape[1]), int(cropped.shape[0])],
        })
        full_for_psd.append((name, rgba))

    rest_visible = composite(image, [(n, visible_rgba[n]) for n in part_stack if n in visible_rgba])
    # Ownership silhouette is the painted rest, so hair gaps stay unowned holes
    # instead of demanding fill for every flood-closed interior.
    visible_union = rest_visible[:, :, 3] > 0
    save_png(OUT / "rest-visible.png", rest_visible)
    save_png(OUT / "silhouette.png", (visible_union.astype(np.uint8) * 255))
    artifacts = ROOT / "artifacts/cyan-source-1-20261013/visual-review"
    artifacts.mkdir(parents=True, exist_ok=True)
    for name, frame in four_backgrounds(rest_visible).items():
        save_png(artifacts / f"rest-{name}.png", frame)

    coverage = []
    surface_layers = {surface: [] for surface in REQUIRED_SURFACES}
    for layer in layers_meta:
        alpha = all_rgba[layer["id"]][:, :, 3] > 0
        if not alpha.any():
            continue
        for surface in layer["surfaces"]:
            surface_layers[surface].append(layer["id"])
    for surface in REQUIRED_SURFACES:
        ids = surface_layers[surface]
        coverage.append({
            "surface": surface,
            "status": "present" if ids else "missing",
            "layer_ids": ids,
            "finding": (
                f"Authored layer(s) {', '.join(ids)} cover {surface}."
                if ids else f"{surface} has no authored pixels."
            ),
        })

    recipe = {
        "schema": "luxel.cyan-source-assembly/v1",
        "source": "assets/avatars/cyan/source/cyan-source-art-female-v2.png",
        "source_digest": FEMALE_V2_SHA256,
        "canvas": [1024, 1536],
        "boundary_policy_px": 2,
        "landmarks": {
            "left_iris_xy": list(LEFT_IRIS),
            "right_iris_xy": list(RIGHT_IRIS),
            "mouth_center_xy": list(MOUTH_CENTER),
        },
        "part_stack": part_stack,
        "layers": layers_meta,
        "coverage": coverage,
        "provenance": {
            "kind": "cyan-source-1-rest-assembly",
            "reference_preserved": True,
            "flattened_underlay": False,
            "cubism_import": "assets/avatars/cyan/layered/cyan-cubism-import.psd",
            "quality": "Source preparation; human appearance review remains separate",
        },
    }
    (OUT / "recipe.json").write_text(json.dumps(recipe, indent=2) + "\n")
    write_psd(full_for_psd, (1024, 1536), OUT / "cyan-cubism-import.psd")
    print(json.dumps({
        "source_digest": FEMALE_V2_SHA256,
        "visible_pixels": {name: int(mask.sum()) for name, mask in visible_masks.items()},
        "coverage": {row["surface"]: row["status"] for row in coverage},
        "psd": str(OUT / "cyan-cubism-import.psd"),
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

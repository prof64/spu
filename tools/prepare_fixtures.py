#!/usr/bin/env python3
"""Prepare private material-study fixtures from local native Gen 5 SPUs.

Run with existing Pillow/numpy, or:
    uv run --with pillow --with numpy python tools/prepare_fixtures.py

The input game artwork and generated archives belong in ignored directories.
Only the conversion recipe is intended for version control. This is a one-way
prototype importer, not the new format specification or a general SPU editor.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import io
import json
from pathlib import Path
import shutil
import zipfile

import numpy as np
from PIL import Image, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
ZIP_TIME = (1980, 1, 1, 0, 0, 0)
CHANNELS = {"visible", "image", "x", "y", "rotation", "scaleX", "scaleY", "z"}


def png_bytes(image: Image.Image) -> bytes:
    stream = io.BytesIO()
    image.save(stream, "PNG", optimize=False)
    return stream.getvalue()


def decode_palette(colors: list[str]) -> np.ndarray:
    rgba = []
    for color in colors:
        raw = bytes.fromhex(color.lstrip("#"))
        if len(raw) not in (3, 4):
            raise ValueError("An SPU palette color must be RGB or RGBA")
        rgba.append(tuple(raw) if len(raw) == 4 else (*raw, 255))
    rgba[0] = (0, 0, 0, 0)  # SPU defines index zero as transparent.
    return np.asarray(rgba, dtype=np.uint8)


def read_indexed(z: zipfile.ZipFile, member: str, palette: np.ndarray) -> Image.Image:
    with Image.open(io.BytesIO(z.read(member))) as png:
        if png.mode != "P":
            raise ValueError(f"Expected indexed native SPU drawing: {member}")
        indices = np.asarray(png)
        if indices.max(initial=0) >= len(palette):
            raise ValueError(f"Palette index outside the selected palette: {member}")
        return Image.fromarray(palette[indices])


def drawing_with_open_eyes(
    z: zipfile.ZipFile, view: int, index: int, drawing: dict, palette: np.ndarray
) -> tuple[Image.Image, int, int]:
    eye = drawing.get("eyes")
    if not drawing["width"] or not drawing["height"]:
        # Some native sprites (including Cryogonal) store an entire eye only
        # in the overlay. A zero-sized base is not an empty drawing in that case.
        if not eye or not eye["width"] or not eye["height"]:
            raise ValueError("A drawing with neither base nor eye pixels is empty")
        overlay = read_indexed(z, f"images/{view}/{index}.eyes-open.png", palette)
        if overlay.size != (eye["width"], eye["height"]):
            raise ValueError("SPU eye dimensions differ from its PNG")
        return overlay, eye["x"], eye["y"]
    base = read_indexed(z, f"images/{view}/{index}.png", palette)
    if base.size != (drawing["width"], drawing["height"]):
        raise ValueError("SPU drawing dimensions differ from its PNG")
    x, y = drawing["x"], drawing["y"]
    if not eye or not eye["width"] or not eye["height"]:
        return base, x, y
    overlay = read_indexed(z, f"images/{view}/{index}.eyes-open.png", palette)
    if overlay.size != (eye["width"], eye["height"]):
        raise ValueError("SPU eye dimensions differ from its PNG")
    # Eye origins are part-local, just like drawing origins. Draw the eye over
    # the base with source-over, as the native evaluator does. Transparent eye
    # pixels do not erase the base; expansion also handles an eye beyond it.
    left, top = min(x, eye["x"]), min(y, eye["y"])
    right = max(x + base.width, eye["x"] + overlay.width)
    bottom = max(y + base.height, eye["y"] + overlay.height)
    combined = Image.new("RGBA", (right - left, bottom - top))
    combined.alpha_composite(base, (x - left, y - top))
    combined.alpha_composite(overlay, (eye["x"] - left, eye["y"] - top))
    return combined, left, top


def material(identifier: str, name: str, **overrides: float) -> dict:
    return {
        "id": identifier,
        "name": name,
        "roughness": 0.8,
        "metalness": 0.0,
        "transmission": 0.0,
        "ior": 1.45,
        "thickness": 0.5,
        "emission": 0.0,
        "normalStrength": 0.65,
        "opacity": 1.0,
        **overrides,
    }


MATERIALS = {
    "charizard": [
        material("skin", "Matte scales"),
        material("wing", "Transmitting wing membrane", roughness=0.65,
                 transmission=0.24, thickness=0.16, normalStrength=0.4),
        material("flame", "Emissive tail flame", roughness=1.0,
                 emission=3.0, normalStrength=0.0),
    ],
    "glalie": [
        material("ice", "Glossy transmitting ice", roughness=0.28,
                 transmission=0.38, ior=1.31, thickness=0.6, normalStrength=1.0),
        material("eye", "Cyan eye emission", roughness=0.36,
                 emission=1.25, normalStrength=0.35),
        material("detail", "Opaque facial details", roughness=0.65,
                 normalStrength=0.3),
    ],
    "cryogonal": [
        material("facet-ice", "Faceted transmitting ice", roughness=0.19,
                 transmission=0.46, ior=1.31, thickness=0.55, normalStrength=1.0),
        material("crystal-energy", "Cyan eyes and ice particles", roughness=0.28,
                 transmission=0.12, ior=1.31, thickness=0.12,
                 emission=1.7, normalStrength=0.55),
        material("crystal-core", "Opaque inner face", roughness=0.85,
                 normalStrength=0.0),
    ],
}


def drawing_material(slug: str, view: int, index: int, name: str) -> str:
    if slug == "charizard":
        if name.startswith("hi") or name.startswith("ef01"):
            return "flame"
        if "hane" in name or "wings" in name:
            return "wing"
        return "skin"
    if slug == "cryogonal":
        if name.startswith("eye") or name.startswith("ice "):
            return "crystal-energy"
        if name == "body b":
            return "crystal-core"
        return "facet-ice"
    if index == 0:
        return "ice"
    if view == 0 and index in (1, 3):
        return "eye"
    return "detail"


def blur_array(array: np.ndarray, radius: float) -> np.ndarray:
    image = Image.fromarray(np.clip(array * 255, 0, 255).astype(np.uint8))
    return np.asarray(image.filter(ImageFilter.GaussianBlur(radius)), dtype=np.float64) / 255


def faceted_crystal_normals(image: Image.Image) -> np.ndarray:
    """Six planar bevels around a flat hexagonal face, in local image space.

    These facets are an explicit material-study interpretation of a crystal,
    not a claim to recover the original sprite's 3D geometry. The pale bevel
    band gets a steeper planar slope; no radial spherical smoothing is used.
    """
    pixels = np.asarray(image, dtype=np.float64) / 255
    alpha = pixels[..., 3]
    luminance = pixels[..., :3] @ np.array([0.2126, 0.7152, 0.0722])
    opaque_y, opaque_x = np.nonzero(alpha)
    normals = np.full((*alpha.shape, 4), 255, dtype=np.uint8)
    normals[..., :3] = (128, 128, 255)
    if not len(opaque_x):
        return normals
    left, right = opaque_x.min(), opaque_x.max() + 1
    top, bottom = opaque_y.min(), opaque_y.max() + 1
    yy, xx = np.mgrid[:image.height, :image.width]
    x = (xx + 0.5 - (left + right) * 0.5) / max(1, (right - left) * 0.5)
    y = -(yy + 0.5 - (top + bottom) * 0.5) / max(1, (bottom - top) * 0.5)
    angles = np.arange(6) * np.pi / 3 + np.pi / 6
    directions = np.stack((np.cos(angles), np.sin(angles)), axis=-1)
    projection = x[..., None] * directions[:, 0] + y[..., None] * directions[:, 1]
    facet = np.argmax(projection, axis=-1)
    distance = projection.max(axis=-1)
    slope = np.where(luminance > 0.72, 0.82, 0.58)
    slope = np.where(distance < 0.43, 0, slope)
    vectors = np.stack((directions[facet, 0] * slope,
                        directions[facet, 1] * slope,
                        np.sqrt(1 - slope * slope)), axis=-1)
    normals[..., :3] = np.rint((vectors * 0.5 + 0.5) * 255).astype(np.uint8)
    normals[alpha == 0, :3] = (128, 128, 255)
    return normals


def maps_for(image: Image.Image, kind: str) -> dict[str, Image.Image]:
    pixels = np.asarray(image, dtype=np.float64) / 255
    rgb, alpha = pixels[..., :3], pixels[..., 3]
    luminance = rgb @ np.array([0.2126, 0.7152, 0.0722])
    # A shallow rounded height estimate from coverage plus a small amount of
    # smoothed source luminance. This is an authored approximation, not recovered
    # geometry. Pad before blurring so cropped edges don't create flat ridges.
    pad = max(6, int(min(image.size) * 0.2))
    radius = max(1.0, min(image.size) * 0.1)
    coverage = blur_array(np.pad(alpha, pad), radius)[pad:-pad, pad:-pad]
    detail = blur_array(np.pad(luminance * alpha, pad), 1.0)[pad:-pad, pad:-pad]
    height = 0.86 * coverage + 0.14 * detail
    dy = np.gradient(height, axis=0) if image.height > 1 else np.zeros_like(height)
    dx = np.gradient(height, axis=1) if image.width > 1 else np.zeros_like(height)
    strength = max(2.0, min(image.size) * 0.45)
    vectors = np.dstack((-dx * strength, dy * strength, np.ones_like(height)))
    vectors /= np.linalg.norm(vectors, axis=-1, keepdims=True)
    normals = np.full((*alpha.shape, 4), 255, dtype=np.uint8)
    normals[..., :3] = np.rint((vectors * 0.5 + 0.5) * 255).astype(np.uint8)
    normals[alpha == 0, :3] = (128, 128, 255)

    surface = np.ones((*alpha.shape, 4), dtype=np.float64)
    emission = np.zeros((*alpha.shape, 4), dtype=np.float64)
    emission[..., 3] = 1
    surface[..., 0] = 0.78 + 0.22 * (1 - luminance)
    if kind == "flame":
        # Keep dark outline pixels dark, with stronger yellow/white cores.
        hot = np.clip((luminance - 0.12) / 0.72, 0, 1)
        emission[..., :3] = rgb * hot[..., None] * alpha[..., None]
        normals[..., :3] = (128, 128, 255)
    elif kind == "wing":
        # Teal membranes transmit; orange supporting bones remain opaque.
        membrane = (rgb[..., 2] > rgb[..., 0] * 1.1) & (rgb[..., 1] > rgb[..., 0])
        surface[..., 2] = membrane.astype(float)
        surface[..., 3] = 0.35 + 0.35 * coverage
    elif kind == "ice":
        # The pale shell transmits while the dark horns, holes and line work do
        # not. Thickness increases toward the estimated interior of the shell.
        ice = np.clip((luminance - 0.18) / 0.5, 0, 1)
        surface[..., 0] = 1 - 0.52 * ice
        surface[..., 2] = ice
        surface[..., 3] = 0.3 + 0.7 * coverage
    elif kind == "eye":
        cyan = np.clip((rgb[..., 2] - rgb[..., 0]) * 2.5, 0, 1)
        emission[..., :3] = rgb * cyan[..., None] * alpha[..., None]
    elif kind == "facet-ice":
        normals = faceted_crystal_normals(image)
        ice = np.clip((luminance - 0.1) / 0.5, 0, 1)
        surface[..., 0] = 0.95 - 0.38 * ice
        surface[..., 2] = ice
        surface[..., 3] = 0.25 + 0.65 * coverage
    elif kind == "crystal-energy":
        normals = faceted_crystal_normals(image)
        cyan = np.clip((rgb[..., 2] - rgb[..., 0] - 0.05) * 3, 0, 1)
        emission[..., :3] = rgb * cyan[..., None] * alpha[..., None]
        surface[..., 2] = cyan
        surface[..., 3] = 0.5
    elif kind == "crystal-core":
        normals[..., :3] = (128, 128, 255)
        surface[..., 2] = 0
    # Uncovered texels retain well-defined neutral data. Geometry coverage
    # comes only from the color texture alpha, not material-map alpha.
    surface[alpha == 0] = 1
    return {
        "normal": Image.fromarray(normals),
        "surface": Image.fromarray(np.rint(np.clip(surface, 0, 1) * 255).astype(np.uint8)),
        "emission": Image.fromarray(np.rint(np.clip(emission, 0, 1) * 255).astype(np.uint8)),
    }


def convert(slug: str, source: Path) -> tuple[dict, dict[str, bytes]]:
    raw = source.read_bytes()
    with zipfile.ZipFile(io.BytesIO(raw)) as z:
        old = json.loads(z.read("sprite.json"))
        if old.get("format") != "spu" or old.get("version") != 0:
            raise ValueError(f"{source}: expected a native SPU v0 archive")
        palette = decode_palette(old["palettes"][0]["colors"])
        package = {
            "format": "sprite-lab", "version": 0, "name": slug.title(),
            "ticksPerSecond": old["ticksPerSecond"], "required": ["core-0"],
            "materials": copy.deepcopy(MATERIALS[slug]), "views": [],
            "extras": {
                "provenance": {
                    "sourceFormat": "Smeargle SPU v0",
                    "sourceSha256": hashlib.sha256(raw).hexdigest(),
                    "sourceGame": old.get("gen5", {}).get("game"),
                    "sourceEntry": old.get("gen5", {}).get("entry"),
                    "sourceClips": "Native Gen 5 timelines, unretimed",
                    "appearance": "Normal palette, open eyes",
                    "subset": "Shiny and editable blinking are not imported in this prototype.",
                    "usage": "Private local test fixture; do not publish game artwork.",
                },
                "materialStudy": {
                    "origin": "Authored procedural estimates; inputs contain no material or normal maps.",
                    "normals": "Rounded silhouette with a small smoothed luminance contribution; +X right, +Y up, +Z toward viewer.",
                    "surfaceChannels": ["roughness multiplier", "metalness multiplier", "transmission multiplier", "thickness multiplier"],
                    "emission": "RGB emission mask multiplied by material emission and animation emission.",
                    "authoredClips": "Glow pulse copies native Idle motion and adds only emission keyframes.",
                    "limitation": "Existing pixel colors include painted shading; this is a material experiment, not physically measured albedo.",
                },
            },
        }
        if slug == "cryogonal":
            package["extras"]["materialStudy"]["normals"] = (
                "Authored six planar bevels around a flat hexagonal face, with "
                "a steeper pale edge band; +X right, +Y up, +Z toward viewer. "
                "An estimate for the crystal study, not recovered 3D geometry."
            )
        emissive_materials = {m["id"] for m in package["materials"] if m["emission"] > 0}
        payloads = {}
        for vi, old_view in enumerate(old["views"]):
            vid = f"view-{vi}"
            image_ids = {
                ii: f"{vid}.image-{ii:03d}" if (
                    (im["width"] and im["height"])
                    or ((im.get("eyes") or {}).get("width") and (im.get("eyes") or {}).get("height"))
                ) else None
                for ii, im in enumerate(old_view["images"])
            }
            part_ids = [f"{vid}.part-{pi:03d}" for pi in range(len(old_view["parts"]))]
            view = {
                "id": vid, "name": old_view["name"],
                "canvas": copy.deepcopy(old_view["canvas"]),
                "images": [], "parts": [], "animations": [],
            }
            material_by_image = {}
            for ii, old_image in enumerate(old_view["images"]):
                identifier = image_ids[ii]
                if identifier is None:
                    continue
                color, x, y = drawing_with_open_eyes(z, vi, ii, old_image, palette)
                kind = drawing_material(slug, vi, ii, old_image["name"])
                material_by_image[identifier] = kind
                image = {"id": identifier, "name": old_image["name"], "x": x, "y": y,
                         "color": f"images/{vid}/{ii:03d}.png", "material": kind}
                payloads[image["color"]] = png_bytes(color)
                for channel, bitmap in maps_for(color, kind).items():
                    image[channel] = f"maps/{vid}/{ii:03d}.{channel}.png"
                    payloads[image[channel]] = png_bytes(bitmap)
                view["images"].append(image)
            for pi, part in enumerate(old_view["parts"]):
                parent = part.get("parent")
                view["parts"].append({"id": part_ids[pi], "name": part["name"],
                                      "parent": None if parent is None else part_ids[parent]})
            for ai, old_clip in enumerate(old_view["animations"]):
                clip = {"id": f"{vid}.animation-{ai:03d}", "name": old_clip["name"],
                        "length": old_clip["length"], "looping": old_clip["looping"], "tracks": []}
                if len(old_clip["tracks"]) != len(part_ids):
                    raise ValueError("Native track count differs from part count")
                for pi, old_track in enumerate(old_clip["tracks"]):
                    unknown = set(old_track) - CHANNELS
                    if unknown:
                        raise ValueError(f"Unrecognized source channels: {sorted(unknown)}")
                    track = {"part": part_ids[pi]}
                    for channel, keys in old_track.items():
                        converted = []
                        for key in keys:
                            key = copy.deepcopy(key)
                            key.setdefault("ease", "step")
                            if channel == "image" and key["value"] is not None:
                                key["value"] = image_ids[key["value"]]
                            converted.append(key)
                        track[channel] = converted
                    clip["tracks"].append(track)
                view["animations"].append(clip)
            if view["animations"]:
                glow = copy.deepcopy(view["animations"][0])
                glow["id"] = f"{vid}.glow-pulse"
                glow["name"] = "Glow pulse"
                glow_parts = 0
                for track in glow["tracks"]:
                    if any(material_by_image.get(key["value"]) in emissive_materials
                           for key in track.get("image", [])):
                        length = glow["length"]
                        track["emission"] = [
                            {"tick": 0, "value": 1.0, "ease": "easeInOut"},
                            {"tick": max(1, length // 2), "value": 2.0, "ease": "easeInOut"},
                            {"tick": max(2, length - 1), "value": 1.0, "ease": "step"},
                        ]
                        glow_parts += 1
                # Glalie's Back view contains no emissive eye, so do not add a
                # misleading duplicate clip with no visible authored change.
                if glow_parts:
                    view["animations"].append(glow)
            package["views"].append(view)
    return package, payloads


def write_package(path: Path, document: dict, payloads: dict[str, bytes]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    members = {"sprite.json": (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode(), **payloads}
    with zipfile.ZipFile(path, "w") as z:
        for name, data in sorted(members.items()):
            member = zipfile.ZipInfo(name, ZIP_TIME)
            member.create_system = 3
            member.external_attr = 0o100644 << 16
            # Recompressing PNG adds cost without useful savings. Compress JSON.
            member.compress_type = zipfile.ZIP_DEFLATED if name.endswith(".json") else zipfile.ZIP_STORED
            z.writestr(member, data, compresslevel=6)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, default=ROOT / "artifacts/source")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "web/fixtures")
    parser.add_argument("--skip-previews", action="store_true",
                        help="Convert archives without requiring existing native reference PNGs")
    parser.add_argument("--assets", nargs="+", choices=tuple(MATERIALS),
                        default=list(MATERIALS), help="Select which local fixtures to rebuild")
    args = parser.parse_args()
    results = []
    for slug in args.assets:
        document, payloads = convert(slug, args.source_dir / f"{slug}.spu")
        path = args.output_dir / f"{slug}.sprite"
        write_package(path, document, payloads)
        if not args.skip_previews:
            preview = args.source_dir / f"{slug}-native/view-0/animation-0/normal/0000.png"
            if not preview.is_file():
                raise FileNotFoundError(f"Native preview missing: {preview}; use --skip-previews for archives only")
            shutil.copyfile(preview, args.output_dir / f"{slug}.png")
        results.append({"name": slug, "path": str(path), "bytes": path.stat().st_size,
                        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                        "views": len(document["views"]),
                        "drawings": sum(len(v["images"]) for v in document["views"]),
                        "animations": sum(len(v["animations"]) for v in document["views"])})
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Imports the terrain materials from ambientCG 1K-PNG downloads (CC0) into res/core/models/terrain/<name>/.

Albedos are recolored to a shared Mars palette: their luminance is remapped between a dark and a light tint per
material, keeping a little of the original color. Normal maps use the OpenGL convention (+Y = +V = world +Z).

Usage: python3 import_materials.py <folder with the unzipped <AssetId>/ downloads>
"""
import os
import sys

import numpy as np
from PIL import Image

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "core", "models", "terrain")

# (engine name, ambientCG asset, dark sRGB tint, light sRGB tint, kept original color)
MATERIALS = [
    ("mars_dust", "Ground096A", (0.50, 0.29, 0.17), (0.86, 0.62, 0.43), 0.15),
    ("mars_regolith", "Ground093C", (0.40, 0.20, 0.10), (0.80, 0.50, 0.31), 0.15),
    ("mars_pebbles", "Ground079S", (0.27, 0.14, 0.09), (0.82, 0.53, 0.36), 0.2),
    ("mars_gravel", "Gravel025", (0.30, 0.13, 0.07), (0.86, 0.46, 0.25), 0.45),
    ("mars_sediment", "Ground100", (0.38, 0.23, 0.15), (0.87, 0.63, 0.45), 0.15),
    ("mars_cliff", "Rock029", (0.28, 0.12, 0.07), (0.86, 0.49, 0.30), 0.4),
    ("mars_basalt", "Rock061", (0.10, 0.07, 0.06), (0.47, 0.31, 0.25), 0.2),
    ("mars_compacted", "Ground102", (0.33, 0.24, 0.19), (0.76, 0.59, 0.47), 0.2),
]
SIZE = 1024
# Albedo contrast kept after recoloring: soft, natural ground rather than strong patterns
CONTRAST = 0.6


def load(path, mode):
    image = Image.open(path).convert(mode)
    if image.size != (SIZE, SIZE):
        image = image.resize((SIZE, SIZE), Image.LANCZOS)
    return np.asarray(image).astype(np.float32) / 255.0


def recolor(rgb, dark, light, keep):
    lum = rgb @ np.array([0.2126, 0.7152, 0.0722])
    lo, hi = np.percentile(lum, [2, 98])
    t = np.clip((lum - lo) / max(hi - lo, 1e-4), 0, 1)
    t = (0.5 + (t - 0.5) * CONTRAST)[..., None]
    tinted = np.array(dark) * (1 - t) + np.array(light) * t
    # Keep some of the original hue variation, at the tinted brightness
    original = rgb / np.maximum(lum[..., None], 1e-3) * (tinted @ np.array([0.2126, 0.7152, 0.0722]))[..., None]
    return np.clip(tinted * (1 - keep) + original * keep, 0, 1)


def save(array, path, mode):
    Image.fromarray((np.clip(array, 0, 1) * 255 + 0.5).astype(np.uint8), mode).save(path)


def main(source):
    for name, asset, dark, light, keep in MATERIALS:
        base = os.path.join(source, asset, f"{asset}_1K-PNG")
        out = os.path.join(OUT, name)
        os.makedirs(out, exist_ok=True)
        save(recolor(load(base + "_Color.png", "RGB"), dark, light, keep), os.path.join(out, "albedo.png"), "RGB")
        save(load(base + "_NormalGL.png", "RGB"), os.path.join(out, "normal.png"), "RGB")
        for src, dst in (("Roughness", "roughness"), ("AmbientOcclusion", "ambient_occlusion"),
                         ("Displacement", "displacement")):
            save(load(f"{base}_{src}.png", "L"), os.path.join(out, f"{dst}.png"), "L")
        print(f"[material] {name} <- {asset}")


if __name__ == "__main__":
    main(sys.argv[1])

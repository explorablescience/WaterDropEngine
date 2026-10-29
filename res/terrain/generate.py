#!/usr/bin/env python3
"""Generates a terrain of res/terrain/ from its config folder.

Reads `<terrain>/config/terrain.json` (chunk layout, height encoding, map files and mask rules,
shared with wde-terrain) and `<terrain>/config/generation.json` (seed and hand-placed landmarks),
then writes the heightmaps and splat maps where terrain.json says.

The ground is either buildable or clearly not. Buildable plains are flat at exactly height 0;
every landform rises or sinks from them through slopes no one mistakes for flat ground: walkable
talus, ramps, dunes and crater flanks (above ~15 degrees), or cliffs. Ground materials follow the
masks the engine derives (see `wde-terrain/src/masks.rs`): regolith and compacted ground where
buildable, never elsewhere.

Usage: python3 generate.py [TERRAIN] [--preview DIR] [--no-write]
"""
import argparse
import json
import math
import os

import numpy as np
from PIL import Image
from scipy import ndimage

ROOT = os.path.dirname(os.path.abspath(__file__))
# Slopes between flat and this are the "shallow" ones the terrain avoids (reported only)
SHALLOW_SLOPE = 0.2


def load_config(path):
    with open(path) as f:
        return json.load(f)["data"]


class Layout:
    def __init__(self, terrain):
        layout = terrain["layout"]
        self.chunks = layout["chunk_count"]
        self.chunk_size = layout["chunk_size"]
        self.res = layout["chunk_resolution"]
        self.n = self.chunks * self.res + 1
        self.step = self.chunk_size / self.res
        # World X/Z of sample 0: chunk (px, pz) is centered at (px, pz) * chunk_size
        self.origin = -(self.chunks // 2) * self.chunk_size - self.chunk_size / 2
        self.min_height = terrain["heights"]["min"]
        self.height_range = terrain["heights"]["range"]
        self.masks = terrain["masks"]
        self.maps = terrain["maps"]


class Window:
    """Samples around a landform: slices of the global arrays and their world coordinates."""

    def __init__(self, L, x0, z0, x1, z1):
        j0 = max(int(math.floor((x0 - L.origin) / L.step)), 0)
        j1 = min(int(math.ceil((x1 - L.origin) / L.step)) + 1, L.n)
        i0 = max(int(math.floor((z0 - L.origin) / L.step)), 0)
        i1 = min(int(math.ceil((z1 - L.origin) / L.step)) + 1, L.n)
        self.empty = i0 >= i1 or j0 >= j1
        self.sl = (slice(i0, i1), slice(j0, j1))
        self.x = (L.origin + np.arange(j0, j1) * L.step)[None, :]
        self.z = (L.origin + np.arange(i0, i1) * L.step)[:, None]
        self.shape = (i1 - i0, j1 - j0)

    @staticmethod
    def around(L, center, reach):
        return Window(L, center[0] - reach, center[1] - reach, center[0] + reach, center[1] + reach)


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def noise_field(rng, L, scale):
    """Smooth noise over the whole terrain with features of about `scale` world units, unit deviation."""
    coarse_step = scale / 2
    m = int(math.ceil((L.n - 1) * L.step / coarse_step)) + 1
    f = ndimage.gaussian_filter(rng.standard_normal((m, m)), 1.0, mode="wrap")
    f = (f - f.mean()) / f.std()
    out = ndimage.zoom(f, (L.n - 1) / (m - 1), order=3)[:L.n, :L.n]
    return out.astype(np.float32)


def signed_distance(L, inside):
    """Signed distance (positive inside) to the outline of a boolean mask, smoothed of its aliasing."""
    d = ndimage.distance_transform_edt(inside) - ndimage.distance_transform_edt(~inside)
    return ndimage.gaussian_filter(d * L.step, 2.0)


def relief(d, height, cliff, talus, apron):
    """Landform profile over the signed distance `d` into its outline: a talus apron from
    `-apron`, a cliff from 0, up to `height`."""
    return np.clip(np.maximum(d + apron, 0) * talus + np.maximum(d, 0) * (cliff - talus), 0, height)


def blob_inside(win, center, radius, rng, warp, rough=0.22):
    """Irregular disc: angular lobes and a domain warp."""
    px = win.x + warp[0][win.sl] * radius * 0.12
    pz = win.z + warp[1][win.sl] * radius * 0.12
    angle = np.arctan2(pz - center[1], px - center[0])
    lobes = sum(rng.uniform(-rough, rough) / k ** 0.6 * np.cos(k * angle + rng.uniform(0, math.tau)) for k in range(2, 8))
    return np.hypot(px - center[0], pz - center[1]) < radius * (1 + lobes)


def curve(points, subdivisions=12):
    """Catmull-Rom curve through the points, sampled densely."""
    p = np.array(points, dtype=np.float64)
    p = np.vstack([2 * p[0] - p[1], p, 2 * p[-1] - p[-2]])
    out = []
    for i in range(1, len(p) - 2):
        for t in np.linspace(0, 1, subdivisions, endpoint=False):
            t2, t3 = t * t, t * t * t
            out.append(0.5 * ((2 * p[i]) + (-p[i - 1] + p[i + 1]) * t
                              + (2 * p[i - 1] - 5 * p[i] + 4 * p[i + 1] - p[i + 2]) * t2
                              + (-p[i - 1] + 3 * p[i] - 3 * p[i + 1] + p[i + 2]) * t3))
    out.append(p[-2])
    return np.array(out)


def curve_distance(L, win, pts):
    """Distance from each sample of the window to the curve, and the curve parameter (0..1) of the
    closest point."""
    seg = np.diff(pts, axis=0)
    lengths = np.hypot(seg[:, 0], seg[:, 1])
    total = lengths.sum()
    # Rasterize the curve every quarter sample, then a distance transform finds the closest point
    marks = np.zeros(win.shape, dtype=bool)
    params = np.zeros(win.shape)
    acc = 0.0
    x0, z0 = win.x[0, 0], win.z[0, 0]
    for (a, d, length) in zip(pts[:-1], seg, lengths):
        count = max(int(length / (L.step * 0.25)), 1)
        t = np.arange(count) / count
        xs, zs = a[0] + d[0] * t, a[1] + d[1] * t
        j = np.round((xs - x0) / L.step).astype(int)
        i = np.round((zs - z0) / L.step).astype(int)
        ok = (i >= 0) & (i < win.shape[0]) & (j >= 0) & (j < win.shape[1])
        marks[i[ok], j[ok]] = True
        params[i[ok], j[ok]] = (acc + t[ok] * length) / total
        acc += length
    dist, (ii, jj) = ndimage.distance_transform_edt(~marks, return_indices=True)
    return ndimage.gaussian_filter(dist * L.step, 2.0), params[ii, jj]


def ramp(win, a, b, ha, hb, half_width, side_slope, cut):
    """Straight ramp from `a` (height `ha`) to `b` (height `hb`) with steep sides: filling (raised
    onto a landform) or cutting (sunk into the ground)."""
    a, b = np.asarray(a), np.asarray(b)
    ab = b - a
    t = np.clip(((win.x - a[0]) * ab[0] + (win.z - a[1]) * ab[1]) / ab.dot(ab), 0, 1)
    dist = np.hypot(win.x - (a[0] + t * ab[0]), win.z - (a[1] + t * ab[1]))
    side = np.maximum(dist - half_width, 0) * side_slope
    return ha + (hb - ha) * t + (side if cut else -side)


def dunes(win, wind, wavelength, amplitude, rng, shape_noise):
    """Transverse dunes: gentle windward faces, steep slip faces. Crests meander across the wind
    only, so that their spacing, hence their slopes, stay constant along it."""
    w = np.asarray(wind, dtype=np.float64)
    w /= np.linalg.norm(w)
    along = win.x * w[0] + win.z * w[1]
    across = -win.x * w[1] + win.z * w[0]
    u = along / wavelength
    for period, amp in ((60.0, 0.35), (23.0, 0.15), (9.0, 0.05)):
        u = u + amp * np.sin(across / period + rng.uniform(0, math.tau))
    f = u - np.floor(u)
    windward = 0.65
    saw = np.where(f < windward, f / windward, (1 - f) / (1 - windward))
    a = amplitude[0] + (amplitude[1] - amplitude[0]) * smoothstep(-1.0, 1.0, shape_noise[win.sl])
    return a * saw, a


class Generator:
    def __init__(self, L, gen):
        self.L = L
        self.gen = gen
        self.shape = gen["shape"]
        self.rng = np.random.default_rng(gen["seed"])
        n = L.n
        self.h = np.zeros((n, n), dtype=np.float32)
        # Material hints, in [0, 1]: sand (dunes, sand floors) and dark rock (crater and crevasse walls)
        self.sand = np.zeros((n, n), dtype=np.float32)
        self.dark = np.zeros((n, n), dtype=np.float32)
        rng = self.rng
        self.warp = (noise_field(rng, L, 40.0), noise_field(rng, L, 40.0))
        self.detail = noise_field(rng, L, 7.0)
        self.broad = noise_field(rng, L, 120.0)
        self.start = gen["start_area"]

    # --- Landforms ---

    def mesa(self, spec):
        L, s = self.L, self.shape
        c, radius, height = spec["center"], spec["radius"], spec["height"]
        win = Window.around(L, c, radius * 1.5 + 50)
        inside = blob_inside(win, c, radius, self.rng, self.warp)
        # Gullies and spurs along the cliffs
        d = signed_distance(L, inside) + self.detail[win.sl] * 1.2
        top = relief(d, height, s["cliff_slope"], s["talus_slope"], s["talus_width"])
        if "tier" in spec:
            tier = spec["tier"]
            inner = blob_inside(win, c, radius * tier["radius"], self.rng, self.warp)
            d2 = signed_distance(L, inner) + self.detail[win.sl] * 1.2
            top = top + relief(d2, tier["height"], s["cliff_slope"], s["talus_slope"], s["talus_width"])
        for angle in spec.get("ramps", []):
            top = np.maximum(top, self.mesa_ramp(win, c, d, height, angle))
        self.h[win.sl] = np.maximum(self.h[win.sl], top)

    def mesa_ramp(self, win, c, d, height, angle):
        """Talus ramp leaning against the cliff, from the plains up to the top."""
        L, s = self.L, self.shape
        u = np.array([math.cos(math.radians(angle)), math.sin(math.radians(angle))])
        # Outline along the ray from the center
        edge = 0.0
        for r in np.arange(0, win.shape[0] * L.step, L.step):
            p = np.asarray(c) + u * r
            i = int(round((p[1] - win.z[0, 0]) / L.step))
            j = int(round((p[0] - win.x[0, 0]) / L.step))
            if not (0 <= i < win.shape[0] and 0 <= j < win.shape[1]) or d[i, j] < 0:
                edge = r
                break
        top_depth = (height - s["talus_width"] * s["talus_slope"]) / s["cliff_slope"] + 2.0
        b = np.asarray(c) + u * (edge - top_depth)
        a = b + u * (height / s["ramp_slope"])
        return ramp(win, a, b, 0.0, height, s["ramp_width"] / 2, s["ramp_side_slope"], cut=False)

    def pillar_field(self, spec):
        L, s, rng = self.L, self.shape, self.rng
        placed, tries = 0, 0
        while placed < spec["count"] and tries < spec["count"] * 50:
            tries += 1
            angle, r = rng.uniform(0, math.tau), spec["radius"] * math.sqrt(rng.uniform())
            c = (spec["center"][0] + r * math.cos(angle), spec["center"][1] + r * math.sin(angle))
            radius = rng.uniform(*spec["size"])
            height = rng.uniform(*spec["height"])
            reach = radius * 1.4 + 4.0
            if not self.free(c, reach):
                continue
            win = Window.around(L, c, reach + 6)
            inside = blob_inside(win, c, radius, rng, self.warp, rough=0.3)
            d = signed_distance(L, inside) + self.detail[win.sl] * 0.4
            top = relief(d, height, s["cliff_slope"] * 1.5, s["talus_slope"], 1.5)
            self.h[win.sl] = np.maximum(self.h[win.sl], top)
            placed += 1

    def canyon(self, spec):
        L, s, rng = self.L, self.shape, self.rng
        pts = curve(spec["path"])
        depth = spec["depth"]
        w0, w1 = spec["floor_width"]
        reach = w1 / 2 + depth / s["cliff_slope"] + s["talus_width"] + 60
        win = Window(L, pts[:, 0].min() - reach, pts[:, 1].min() - reach, pts[:, 0].max() + reach, pts[:, 1].max() + reach)
        dist, t = curve_distance(L, win, pts)
        phase = rng.uniform(0, math.tau)
        half = 0.5 * (w0 + (w1 - w0) * (0.5 + 0.5 * np.sin(t * 9.0 + phase)))
        g = dist - half + self.detail[win.sl] * 1.5
        cut = -depth + relief(g, depth, s["cliff_slope"], s["talus_slope"], s["talus_width"])
        floor = self.sand_floor(win, -depth)
        cut = np.maximum(cut, floor)
        for frac, side in spec.get("ramps", []):
            cut = np.minimum(cut, self.canyon_ramp(win, pts, frac, side, depth, w0 / 2))
        self.carve(win, cut)
        self.sand[win.sl] = np.maximum(self.sand[win.sl], (cut < -depth + 2.2) * 1.0)

    def canyon_ramp(self, win, pts, frac, side, depth, half_floor):
        """Gully sunk into the plains beside the canyon, down to its floor."""
        s = self.shape
        i = int(frac * (len(pts) - 1))
        tangent = pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]
        tangent /= np.linalg.norm(tangent)
        normal = np.array([-tangent[1], tangent[0]]) * side
        b = pts[i] + normal * half_floor * 0.5
        a = b + normal * (depth / s["ramp_slope"])
        return ramp(win, a, b, 0.0, -depth, s["ramp_width"] / 2, s["ramp_side_slope"], cut=True)

    def crevasse(self, spec):
        L, s = self.L, self.shape
        pts = curve(spec["path"])
        reach = spec["width"] + 20
        win = Window(L, pts[:, 0].min() - reach, pts[:, 1].min() - reach, pts[:, 0].max() + reach, pts[:, 1].max() + reach)
        dist, t = curve_distance(L, win, pts)
        # Widest in the middle, closing at both tips
        half = spec["width"] / 2 * np.sqrt(np.clip(np.sin(t * math.pi) * 1.6, 0, 1))
        half = half + self.detail[win.sl] * 0.4
        cut = -np.minimum(spec["depth"], np.maximum(half - dist, 0) * s["crevasse_wall_slope"])
        self.carve(win, cut)
        self.dark[win.sl] = np.maximum(self.dark[win.sl], smoothstep(0.0, 0.5, -cut))

    def crater(self, c, radius):
        L, s = self.L, self.shape
        reach = radius * 1.3 + 10
        win = Window.around(L, c, reach)
        px = win.x + self.warp[0][win.sl] * radius * 0.04
        pz = win.z + self.warp[1][win.sl] * radius * 0.04
        r = np.hypot(px - c[0], pz - c[1])
        flank = s["crater_flank_slope"]
        if radius >= 25:
            # Complex crater: cliff walls with a talus foot, sand floor, central peak when large
            depth, rim = 0.16 * radius, 0.05 * radius
            wall = (depth + rim - s["talus_width"] * s["talus_slope"]) / s["cliff_slope"]
            inner = -depth + relief(r - (radius - wall), depth + rim, s["cliff_slope"], s["talus_slope"], s["talus_width"])
            inner = np.maximum(inner, self.sand_floor(win, -depth))
            if radius >= 45:
                inner = np.maximum(inner, -depth + 0.7 * depth - r * 0.7)
            self.sand[win.sl] = np.maximum(self.sand[win.sl], (r < radius - wall) * (inner < -depth + 2.2))
        else:
            # Simple bowl
            depth, rim = 0.3 * radius, 0.06 * radius
            inner = -depth + (depth + rim) * r / radius
        outer = np.maximum(rim - (r - radius) * flank, 0)
        profile = np.where(r < radius, inner, outer)
        self.h[win.sl] += profile.astype(np.float32)
        self.dark[win.sl] = np.maximum(self.dark[win.sl], (r < radius) * smoothstep(0.05, 0.5, -profile))

    def crater_field(self, spec):
        rng = self.rng
        placed, tries = 0, 0
        while placed < spec["count"] and tries < spec["count"] * 50:
            tries += 1
            angle, r = rng.uniform(0, math.tau), spec["radius"] * math.sqrt(rng.uniform())
            c = (spec["center"][0] + r * math.cos(angle), spec["center"][1] + r * math.sin(angle))
            radius = rng.uniform(*spec["size"])
            if not self.free(c, radius * 1.3 + 4):
                continue
            self.crater(c, radius)
            placed += 1

    def dune_sea(self, spec):
        L = self.L
        c, radius = spec["center"], spec["radius"]
        win = Window.around(L, c, radius * 1.5)
        inside = blob_inside(win, c, radius, self.rng, self.warp, rough=0.3)
        d = signed_distance(L, inside)
        field, amplitude = dunes(win, spec["wind"], spec["wavelength"], spec["amplitude"], self.rng, self.broad)
        # A sand sheet rising from the plains, then dunes that sink into it toward the edge rather
        # than flatten: their slopes stay steep
        sheet = relief(d, 1.5, 0.35, 0.35, 0.0)
        sink = amplitude * np.clip(1 - d / 50.0, 0, 1)
        top = sheet + np.maximum(field - sink, 0)
        self.h[win.sl] = np.maximum(self.h[win.sl], top)
        self.sand[win.sl] = np.maximum(self.sand[win.sl], smoothstep(-6.0, 6.0, d))

    def sand_floor(self, win, base):
        spec = self.gen["sand_floors"]
        field, _ = dunes(win, (1.0, 0.3), spec["wavelength"], (spec["amplitude"], spec["amplitude"]), self.rng, self.broad)
        return base + field

    def carve(self, win, cut):
        """Sinks the ground to `cut` where it is negative, leaving the rest of the window as is."""
        h = self.h[win.sl]
        self.h[win.sl] = np.where(cut < 0, np.minimum(h, cut), h)

    def free(self, c, reach):
        """Whether the plains around `c` are untouched and outside the start area."""
        start = self.start
        if math.dist(c, start["center"]) < start["radius"] + reach:
            return False
        win = Window.around(self.L, c, reach)
        if win.empty or win.shape[0] < 2 or win.shape[1] < 2:
            return False
        return not np.any(self.h[win.sl] != 0)

    def run(self):
        g = self.gen
        for spec in g.get("dune_seas", []):
            self.dune_sea(spec)
        for spec in g.get("mesas", []):
            self.mesa(spec)
        for spec in g.get("craters", []):
            self.crater(spec["center"], spec["radius"])
        for spec in g.get("canyons", []):
            self.canyon(spec)
        for spec in g.get("crevasses", []):
            self.crevasse(spec)
        for spec in g.get("pillar_fields", []):
            self.pillar_field(spec)
        for spec in g.get("crater_fields", []):
            self.crater_field(spec)
        L = self.L
        self.h = np.clip(self.h, L.min_height, L.min_height + L.height_range)
        # Heights as the engine decodes them from the 16-bit heightmaps
        q = np.round((self.h - L.min_height) / L.height_range * 65535).clip(0, 65535).astype(np.uint16)
        self.h = (L.min_height + q.astype(np.float32) / 65535 * L.height_range).astype(np.float32)
        return q


# --- Masks and materials ---

def masks(L, h):
    """Slope and buildable masks, as wde-terrain derives them."""
    gz, gx = np.gradient(h, L.step)
    slope = np.hypot(gx, gz)
    m = L.masks
    flat = (np.abs(h) <= m["build_height_tolerance"]) & (slope <= m["build_max_slope"])
    margin = int(math.ceil(m["build_margin"] / L.step))
    buildable = ndimage.minimum_filter(flat, size=2 * margin + 1, mode="nearest")
    return slope, buildable


def materials(gen, L, h, slope, buildable):
    """Splat weights of the 8 materials, in the order of `MATERIALS` in
    `wde-terrain/src/render/dependencies/materials.rs`:
    dust, regolith, pebbles, gravel, sediment, cliff, basalt, compacted."""
    walk = L.masks["walk_max_slope"]
    build = ndimage.gaussian_filter(buildable.astype(np.float32), 1.0) * buildable
    cliff = smoothstep(walk * 0.8, walk * 1.15, slope)
    sloped = smoothstep(0.03, 0.15, slope) * (1 - cliff)
    flat = np.clip(1 - cliff - sloped, 0, 1) * (1 - build)
    sand = gen.sand * (1 - cliff)
    dark = gen.dark
    patches = smoothstep(0.0, 1.2, gen.broad + 0.6 * gen.detail) * 0.7
    cap = flat * (np.abs(h) >= 0.5)
    strata = 0.5 + 0.5 * np.sin(h * 1.4 + gen.detail * 0.8)

    w = np.zeros((8, L.n, L.n), dtype=np.float32)
    w[1] = build * (1 - patches)                                             # regolith
    w[7] = build * patches                                                    # compacted ground
    w[0] = sand                                                               # dust: dunes, sand floors
    rest = 1 - sand
    w[2] = rest * (sloped * 0.6 + flat * (np.abs(h) < 0.5) * (1 - dark) + cap * 0.3)  # pebbles: talus, cliff feet
    w[3] = rest * sloped * 0.4 * (1 - dark)                                   # gravel
    w[4] = rest * cliff * strata * (1 - dark)                                 # layered sediment
    w[5] = rest * (cliff * (1 - strata) * (1 - dark) + cap * 0.5 * (1 - dark))  # orange rock: cliffs, caps
    w[6] = rest * (cap * (0.2 + 0.5 * dark) + (cliff + sloped) * dark)         # basalt: crater walls
    # Weights of non-buildable ground never leak onto the buildable plains, and back
    w[[0, 2, 3, 4, 5, 6]] *= (1 - build)
    w /= np.maximum(w.sum(axis=0), 1e-6)
    return w


def write(L, folder, q, w):
    s = np.round(w * 255).clip(0, 255).astype(np.uint8)
    half = L.chunks // 2
    for cz in range(L.chunks):
        for cx in range(L.chunks):
            px, pz = cx - half, cz - half
            rows = slice(cz * L.res, cz * L.res + L.res + 1)
            cols = slice(cx * L.res, cx * L.res + L.res + 1)
            path = os.path.join(folder, L.maps["heightmap"].format(x=px, z=pz))
            os.makedirs(os.path.dirname(path), exist_ok=True)
            Image.fromarray(np.ascontiguousarray(q[rows, cols])).save(path)
            for k in range(2):
                rgba = np.moveaxis(s[4 * k:4 * k + 4, rows, cols], 0, -1)
                path = os.path.join(folder, L.maps["splatmap"].format(x=px, z=pz, k=k))
                os.makedirs(os.path.dirname(path), exist_ok=True)
                Image.fromarray(np.ascontiguousarray(rgba)).save(path)


def report(L, h, slope, buildable):
    walk = L.masks["walk_max_slope"]
    total = h.size
    flat = slope <= L.masks["build_max_slope"]
    shallow = (slope > L.masks["build_max_slope"]) & (slope < SHALLOW_SLOPE)
    print(f"heights: {h.min():.1f} .. {h.max():.1f}")
    print(f"buildable:            {buildable.sum() / total:6.1%}")
    print(f"flat, not buildable:  {(flat & ~buildable).sum() / total:6.1%}")
    print(f"shallow slopes:       {shallow.sum() / total:6.1%}")
    print(f"walkable slopes:      {((slope >= SHALLOW_SLOPE) & (slope <= walk)).sum() / total:6.1%}")
    print(f"too steep to walk:    {(slope > walk).sum() / total:6.1%}")


def preview(L, out, h, slope, buildable, w):
    """Hillshaded overviews: tinted by materials, and by masks (green buildable, sand walkable,
    red too steep to walk, blue shallow)."""
    f = 1024 / L.n
    hs = ndimage.zoom(h, f, order=1)
    gz, gx = np.gradient(hs, L.step / f)
    light = np.array([-0.6, 0.6, 0.55])
    n = np.stack([-gx, np.ones_like(gx), -gz])
    n /= np.linalg.norm(n, axis=0)
    shade = np.clip(np.tensordot(light / np.linalg.norm(light), n, 1), 0, 1) * 0.75 + 0.25
    palette = np.array([[0.86, 0.6, 0.4], [0.8, 0.48, 0.3], [0.62, 0.36, 0.24], [0.7, 0.3, 0.16],
                        [0.66, 0.46, 0.34], [0.72, 0.36, 0.2], [0.32, 0.22, 0.2], [0.75, 0.6, 0.5]])
    ws = np.stack([ndimage.zoom(wi, f, order=1) for wi in w])
    color = np.tensordot(ws, palette, axes=(0, 0)) * shade[..., None]
    Image.fromarray((color.clip(0, 1) * 255).astype(np.uint8)[::-1]).save(os.path.join(out, "preview.png"))

    walk = L.masks["walk_max_slope"]
    s = ndimage.zoom(slope, f, order=0)
    b = ndimage.zoom(buildable.astype(np.uint8), f, order=0) > 0
    color = np.empty(s.shape + (3,))
    color[...] = [0.85, 0.7, 0.5]
    color[s > walk] = [0.8, 0.2, 0.15]
    color[(s > L.masks["build_max_slope"]) & (s < SHALLOW_SLOPE)] = [0.2, 0.4, 0.9]
    color[b] = [0.3, 0.75, 0.3]
    color *= shade[..., None]
    Image.fromarray((color.clip(0, 1) * 255).astype(np.uint8)[::-1]).save(os.path.join(out, "preview_masks.png"))


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("terrain", nargs="?", default="mars", help="terrain folder in res/terrain/")
    ap.add_argument("--preview", help="directory to write overview images to (north up)")
    ap.add_argument("--no-write", action="store_true", help="skip writing the maps")
    args = ap.parse_args()
    folder = os.path.join(ROOT, args.terrain)
    L = Layout(load_config(os.path.join(folder, "config", "terrain.json")))
    gen = Generator(L, load_config(os.path.join(folder, "config", "generation.json")))
    q = gen.run()
    slope, buildable = masks(L, gen.h)
    report(L, gen.h, slope, buildable)
    w = materials(gen, L, gen.h, slope, buildable)
    if not args.no_write:
        write(L, folder, q, w)
    if args.preview:
        os.makedirs(args.preview, exist_ok=True)
        preview(L, args.preview, gen.h, slope, buildable, w)

#!/usr/bin/env python3
"""Generates the Mars terrain loaded by wde-terrain: per chunk, a 16-bit heightmap and two RGBA splat maps.

Layout (must match `wde-terrain/src/manager.rs`): CHUNKS x CHUNKS chunks of CHUNK_SIZE world units, chunk (px, pz)
centered at (px, pz) * CHUNK_SIZE with px, pz in [-CHUNKS / 2, CHUNKS / 2). Each map holds SUBDIV + 1 samples per
side, edges included, so neighbor chunks share their border samples. Heights in world units are remapped from
[MIN_HEIGHT, MIN_HEIGHT + HEIGHT_RANGE] to the full 16-bit range. The landscape is mostly gentle plains with a few
landmarks (mountain massifs, mesas, craters). Wherever the plains come close to height 0 they flatten to exactly 0:
these patches, all over the map, are the buildable ground (the start area around the origin is one of them).

Splat weights (sum 1): `splatmap_X_Z-0.png` RGBA = materials 0-3, `splatmap_X_Z-1.png` RGBA = materials 4-7, in the
order of `MATERIALS` in `wde-terrain/src/render/dependencies/materials.rs`.

Usage: python3 generate_mars.py [--seed N] [--out DIR] [--preview PNG]
"""
import argparse
import math
import os

import numpy as np
from PIL import Image
from scipy import ndimage

CHUNKS = 16
CHUNK_SIZE = 64.0
SUBDIV = 256
MIN_HEIGHT = -24.0
HEIGHT_RANGE = 64.0

N = CHUNKS * SUBDIV + 1               # samples per side of the whole terrain
STEP = CHUNK_SIZE / SUBDIV            # world units between samples
ORIGIN = -(CHUNKS // 2) * CHUNK_SIZE - CHUNK_SIZE / 2  # world X/Z of sample 0
COARSE = 4                            # coarse pass: one sample every COARSE fine samples
NC = (N - 1) // COARSE + 1
STEP_C = STEP * COARSE

# Start area: flat within BASIN_FLAT, blending into the land up to BASIN_EDGE (world units)
BASIN_CENTER = (-2.0, 4.0)
BASIN_FLAT = 45.0
BASIN_EDGE = 140.0
# Plains within this height of 0 flatten to exactly 0 (buildable), with a smooth transition
FLAT_BAND = 0.5


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def spectral(rng, n, beta, step=1.0):
    """Gaussian noise with a 1/f^beta power spectrum (fBm), normalized to unit deviation."""
    fx = np.fft.fftfreq(n)[:, None]
    fy = np.fft.rfftfreq(n)[None, :]
    f = np.sqrt(fx * fx + fy * fy)
    f[0, 0] = 1.0
    amp = f ** (-beta / 2.0)
    amp[0, 0] = 0.0
    phase = rng.standard_normal((n, n // 2 + 1)) + 1j * rng.standard_normal((n, n // 2 + 1))
    out = np.fft.irfft2(amp * phase, s=(n, n))
    return (out - out.mean()) / out.std()


def grid(n, step):
    coords = ORIGIN + np.arange(n) * step
    return np.meshgrid(coords, coords)  # x varies along columns, z along rows


def ridged(rng, n, octaves=5):
    out = np.zeros((n, n))
    amp, total = 1.0, 0.0
    for o in range(octaves):
        r = 1.0 - np.abs(spectral(rng, n, 2.0 + 0.3 * o) * 0.45)
        out += amp * np.clip(r, 0, 1) ** 3
        total += amp
        amp *= 0.5
    return out / total


def polyline_distance(x, z, points):
    """Distance from each (x, z) to a polyline, and the curve parameter of the closest point (0..1)."""
    best = np.full(x.shape, np.inf)
    param = np.zeros(x.shape)
    lengths = [math.dist(points[i], points[i + 1]) for i in range(len(points) - 1)]
    total = sum(lengths)
    acc = 0.0
    for i in range(len(points) - 1):
        (ax, az), (bx, bz) = points[i], points[i + 1]
        dx, dz = bx - ax, bz - az
        t = np.clip(((x - ax) * dx + (z - az) * dz) / (dx * dx + dz * dz), 0, 1)
        d = np.hypot(x - (ax + t * dx), z - (az + t * dz))
        closer = d < best
        best = np.where(closer, d, best)
        param = np.where(closer, (acc + t * lengths[i]) / total, param)
        acc += lengths[i]
    return best, param


def meander(rng, start, end, count, amplitude):
    (sx, sz), (ex, ez) = start, end
    nx, nz = -(ez - sz), ex - sx
    norm = math.hypot(nx, nz)
    pts = []
    phase = rng.uniform(0, math.tau)
    for i in range(count + 1):
        t = i / count
        off = amplitude * (math.sin(t * 5.1 + phase) * 0.7 + rng.uniform(-0.4, 0.4)) * math.sin(math.pi * t)
        pts.append((sx + (ex - sx) * t + nx / norm * off, sz + (ez - sz) * t + nz / norm * off))
    return pts


def crater(h, x0, z0, radius, step, origin_x, origin_z, depth_ratio=0.2, peak=False, mark=None):
    """Adds an impact crater (bowl, raised rim, ejecta) to the heightmap window it covers."""
    reach = radius * 2.6
    n = h.shape[0]
    i0 = max(int((z0 - reach - origin_z) / step), 0)
    i1 = min(int((z0 + reach - origin_z) / step) + 2, n)
    j0 = max(int((x0 - reach - origin_x) / step), 0)
    j1 = min(int((x0 + reach - origin_x) / step) + 2, n)
    if i0 >= i1 or j0 >= j1:
        return
    zz = origin_z + np.arange(i0, i1)[:, None] * step
    xx = origin_x + np.arange(j0, j1)[None, :] * step
    r = np.hypot(xx - x0, zz - z0) / radius
    depth = depth_ratio * radius
    rim = 0.06 * radius
    bowl = np.where(r < 1.0, depth * (r * r - 1.0), 0.0)
    if radius > 30:
        bowl = np.maximum(bowl, -depth * 0.78)  # flat floor of complex craters
    profile = bowl + rim * np.exp(-((r - 1.0) / 0.22) ** 2) + np.where(r > 1.0, rim * 0.5 * r ** -3.0, 0.0)
    if peak:
        profile += depth * 0.45 * np.exp(-(r / 0.16) ** 2)
    h[i0:i1, j0:j1] += profile
    if mark is not None:
        # Crater walls and ejecta, for the dark rock / pebbles materials
        mark[0][i0:i1, j0:j1] = np.maximum(mark[0][i0:i1, j0:j1], smoothstep(0.55, 0.85, r) * (1 - smoothstep(1.0, 1.25, r)))
        mark[1][i0:i1, j0:j1] = np.maximum(mark[1][i0:i1, j0:j1], smoothstep(0.95, 1.1, r) * (1 - smoothstep(1.6, 2.4, r)))
        mark[2][i0:i1, j0:j1] = np.maximum(mark[2][i0:i1, j0:j1], 1 - smoothstep(0.35, 0.6, r))


def thermal_erosion(h, step, iterations, talus_deg=33.0, rate=0.3):
    """Material slides from slopes steeper than the talus angle onto the lower neighbors."""
    talus = math.tan(math.radians(talus_deg)) * step
    n, m = h.shape
    neighbors = ((0, 1), (2, 1), (1, 0), (1, 2))
    for _ in range(iterations):
        p = np.pad(h, 1, mode="edge")
        incoming = np.zeros((n + 2, m + 2))
        for di, dj in neighbors:
            move = np.clip(h - p[di:di + n, dj:dj + m] - talus, 0, None) * rate * 0.25
            h = h - move
            incoming[di:di + n, dj:dj + m] += move
        h = h + incoming[1:-1, 1:-1]
    return h


def blob_distance(x, z, cx, cz, radius, rng, roughness=0.25):
    """Distance to (cx, cz) relative to an irregular radius: 1 on the outline."""
    angle = np.arctan2(z - cz, x - cx)
    wobble = sum(rng.uniform(-roughness, roughness) * np.cos(k * angle + rng.uniform(0, math.tau)) for k in (2, 3, 5))
    return np.hypot(x - cx, z - cz) / (radius * (1 + wobble))


def blob(x, z, cx, cz, radius, rng, roughness=0.25):
    """Soft, irregular disc mask (1 inside, 0 outside) around (cx, cz)."""
    return 1 - smoothstep(0.75, 1.0, blob_distance(x, z, cx, cz, radius, rng, roughness))


def far_from_basin(rng, margin, count):
    """Random landmark positions outside the colony basin."""
    points = []
    while len(points) < count:
        x, z = rng.uniform(-500, 440), rng.uniform(-500, 440)
        if math.hypot(x - BASIN_CENTER[0], z - BASIN_CENTER[1]) > BASIN_EDGE + margin:
            points.append((x, z))
    return points


def generate(seed):
    rng = np.random.default_rng(seed)
    xc, zc = grid(NC, STEP_C)

    # --- Coarse landforms (1 sample per STEP_C units) ---
    # Broad, gentle undulation and a slight regional tilt: the plains
    h = spectral(rng, NC, 3.6) * 3.0 + spectral(rng, NC, 2.8) * 0.2
    # Flat patches wherever the plains come close to 0, all over the map, without a step at their edge
    h = np.sign(h) * np.maximum(np.abs(h) - FLAT_BAND, 0.0)
    h = ndimage.gaussian_filter(h, 2.0)

    # A few mountain clusters, sharp ridges fading into the plains
    ridges = ridged(rng, NC)
    for cx, cz in far_from_basin(rng, 140, 3):
        r = blob_distance(xc, zc, cx, cz, rng.uniform(80, 130), rng, 0.3)
        cone = (1 - smoothstep(0.0, 1.0, r)) ** 1.6
        h += cone * (0.45 + 0.55 * ridges) * rng.uniform(28, 38)

    # Mesas: flat tops, steep scarps
    for cx, cz in far_from_basin(rng, 120, 2):
        mask = blob(xc, zc, cx, cz, rng.uniform(45, 75), rng, 0.25)
        top = smoothstep(0.25, 0.5, mask)
        h += top * rng.uniform(9, 13) + smoothstep(0.75, 1.0, mask) * 2.0

    # Craters: a handful of large ones, some small ones
    crater_marks = [np.zeros_like(h) for _ in range(3)]
    for (x0, z0), radius in zip(far_from_basin(rng, 70, 6), rng.uniform(22, 60, 6)):
        crater(h, x0, z0, float(radius), STEP_C, ORIGIN, ORIGIN, depth_ratio=0.12,
               peak=radius > 45, mark=crater_marks)
    for (x0, z0), radius in zip(far_from_basin(rng, 10, 30), rng.uniform(4, 12, 30)):
        crater(h, x0, z0, float(radius), STEP_C, ORIGIN, ORIGIN, depth_ratio=0.12, mark=crater_marks)

    h = thermal_erosion(h, STEP_C, 30)

    # --- Fine resolution ---
    zoom = (N / NC, N / NC)
    h = ndimage.zoom(h, zoom, order=3)[:N, :N]
    marks = [ndimage.zoom(m, zoom, order=1)[:N, :N] for m in crater_marks]
    x, z = grid(N, STEP)

    # Light detail, a bit rougher on slopes
    gz, gx = np.gradient(h, STEP)
    slope = np.hypot(gx, gz)
    h += spectral(rng, N, 2.8) * 0.2 * smoothstep(0.3, 1.2, slope)

    # Soft dunes on crater floors only
    dune_mask = marks[2] * 0.8
    warp = spectral(rng, N, 3.0) * 9.0
    dunes = np.abs(np.sin((x * 0.8 + z * 0.6) / 7.5 + warp)) ** 1.6
    h += dunes * dune_mask * 0.35

    # Start area: exactly flat at 0, fading into the plains
    basin_r = np.hypot(x - BASIN_CENTER[0], z - BASIN_CENTER[1]) + spectral(rng, N, 3.2) * 12.0
    h = h * smoothstep(BASIN_FLAT, BASIN_EDGE, basin_r)
    # Detail stays off the flat ground, so that it remains buildable
    h = np.where(np.abs(h) < 0.02, 0.0, h)

    h = np.clip(h, MIN_HEIGHT + 0.5, MIN_HEIGHT + HEIGHT_RANGE - 0.5)

    # --- Materials ---
    gz, gx = np.gradient(h, STEP)
    slope = np.hypot(gx, gz)
    curvature = ndimage.laplace(ndimage.gaussian_filter(h, 3.0))
    patches = spectral(rng, N, 3.0)
    sand = spectral(rng, N, 3.4)
    w = np.zeros((8, N, N), dtype=np.float32)
    cliff = smoothstep(0.75, 1.25, slope)
    steep = smoothstep(0.32, 0.75, slope) * (1 - cliff)
    gentle = smoothstep(0.12, 0.32, slope) * (1 - steep - cliff)
    flat = np.clip(1 - cliff - steep - gentle, 0, 1)
    dark = marks[0]
    # Plains: regolith and dust in broad, soft areas
    dusty = smoothstep(0.3, 1.3, sand)
    w[1] = flat * (1 - dusty)                                   # regolith
    w[0] = flat * np.clip(dusty + dune_mask + smoothstep(0.05, 0.2, -curvature) * 0.5, 0, 1)  # dust
    w[2] = gentle * 0.8 + flat * marks[1] * 0.5                 # pebbly ground, crater ejecta
    w[3] = gentle * 0.2 + flat * smoothstep(1.2, 2.0, patches) * 0.4  # a few gravel patches
    w[4] = steep * (1 - dark * 0.7)                             # layered sediment
    w[5] = cliff * (1 - dark)                                   # orange cliff rock
    w[6] = cliff * dark + steep * dark * 0.7                    # dark rock on crater walls
    dust = np.eye(8, dtype=np.float32)[0][:, None, None]
    w = w * (1 - dune_mask * 0.85) + dust * dune_mask * 0.85
    w /= np.maximum(w.sum(axis=0), 1e-6)
    return h.astype(np.float32), w


def write(out, h, w):
    os.makedirs(out, exist_ok=True)
    q = np.round((h - MIN_HEIGHT) / HEIGHT_RANGE * 65535).clip(0, 65535).astype(np.uint16)
    s = np.round(w * 255).clip(0, 255).astype(np.uint8)
    half = CHUNKS // 2
    for cz in range(CHUNKS):
        for cx in range(CHUNKS):
            px, pz = cx - half, cz - half
            rows = slice(cz * SUBDIV, cz * SUBDIV + SUBDIV + 1)
            cols = slice(cx * SUBDIV, cx * SUBDIV + SUBDIV + 1)
            Image.fromarray(np.ascontiguousarray(q[rows, cols])).save(os.path.join(out, f"heightmap_{px}_{pz}.png"))
            for k in range(2):
                rgba = np.moveaxis(s[4 * k:4 * k + 4, rows, cols], 0, -1)
                Image.fromarray(np.ascontiguousarray(rgba)).save(os.path.join(out, f"splatmap_{px}_{pz}-{k}.png"))


def preview(path, h, w):
    """Hillshaded overview tinted by the dominant materials."""
    small = 1024
    f = small / N
    hs = ndimage.zoom(h, f, order=1)
    gz, gx = np.gradient(hs, STEP / f)
    light = np.array([-0.6, 0.6, 0.55])
    n = np.stack([-gx, np.ones_like(gx), -gz])
    n /= np.linalg.norm(n, axis=0)
    shade = np.clip(np.tensordot(light / np.linalg.norm(light), n, 1), 0, 1) * 0.75 + 0.25
    palette = np.array([[0.86, 0.6, 0.4], [0.8, 0.48, 0.3], [0.62, 0.36, 0.24], [0.7, 0.3, 0.16],
                        [0.66, 0.46, 0.34], [0.72, 0.36, 0.2], [0.32, 0.22, 0.2], [0.55, 0.45, 0.4]])
    ws = np.stack([ndimage.zoom(wi, f, order=1) for wi in w])
    color = np.tensordot(ws, palette, axes=(0, 0)) * shade[..., None]
    Image.fromarray((color.clip(0, 1) * 255).astype(np.uint8)).save(path)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=2026)
    ap.add_argument("--out", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "mars"))
    ap.add_argument("--preview")
    args = ap.parse_args()
    heights, weights = generate(args.seed)
    print(f"heights: {heights.min():.1f} .. {heights.max():.1f}")
    write(args.out, heights, weights)
    if args.preview:
        preview(args.preview, heights, weights)

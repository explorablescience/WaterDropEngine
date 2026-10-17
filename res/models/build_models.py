"""Procedural generator for the WDE outpost models.

Run headless:
    blender -b --python res/models/build_models.py -- [model ...] [--preview DIR] [--no-bake]

Each placement model keeps the `%P%` property nodes (Footprint, Anchors, Center) of the
existing glTF, rebuilds the render geometry, bakes an AO texture and re-exports the glTF.
Buildings come in two variants: `<name>.gltf` (closed, seen through transparent glass) and
`<name>_cut.gltf` (open top, to see and place things inside).
Engine constraints: flat node list, single buffer, PBR factors only (no emissive), alpha
blending only through `alphaMode: BLEND`, textures sampled ClampToEdge without mips or sRGB decode.
"""

import bpy
import bmesh
import json
import math
import os
import random
import sys
from math import cos, pi, radians, sin, sqrt
from mathutils import Matrix, Vector

ROOT = os.path.dirname(os.path.abspath(__file__))
TAU = 2 * pi

# ---------------------------------------------------------------------------------------
# Palette (linear base color, metallic, roughness)
# ---------------------------------------------------------------------------------------
PALETTE = {
    "Hull_White": ((0.74, 0.72, 0.66), 0.0, 0.45),
    "Hull_Grey": ((0.30, 0.31, 0.32), 0.0, 0.55),
    "Frame_Graphite": ((0.035, 0.038, 0.042), 0.3, 0.40),
    "Accent_Orange": ((0.85, 0.20, 0.03), 0.0, 0.40),
    "Hazard_Yellow": ((0.85, 0.55, 0.02), 0.0, 0.50),
    "Glass_Teal": ((0.08, 0.30, 0.34), 0.0, 0.05),
    "Light_Cyan": ((0.30, 0.95, 1.00), 0.0, 0.25),
    "Floor_Deck": ((0.16, 0.16, 0.17), 0.1, 0.65),
    "Concrete": ((0.40, 0.35, 0.30), 0.0, 0.90),
    "Soil": ((0.10, 0.05, 0.025), 0.0, 1.00),
    "Plant_Green": ((0.10, 0.32, 0.05), 0.0, 0.75),
    "Plant_Dark": ((0.03, 0.14, 0.04), 0.0, 0.80),
    "Bark": ((0.09, 0.05, 0.03), 0.0, 0.90),
    "Solar_Blue": ((0.01, 0.025, 0.08), 0.4, 0.15),
    "Cargo_Teal": ((0.03, 0.25, 0.26), 0.1, 0.55),
    "Cargo_Rust": ((0.30, 0.06, 0.02), 0.1, 0.60),
    "Crate_Tan": ((0.45, 0.30, 0.15), 0.0, 0.80),
    "Visor_Gold": ((0.80, 0.50, 0.15), 0.9, 0.12),
    "Window_Dark": ((0.012, 0.045, 0.055), 0.3, 0.08),
    "Grass": ((0.10, 0.28, 0.05), 0.0, 0.9),
    "Water": ((0.04, 0.22, 0.28), 0.0, 0.05),
    "Steel": ((0.33, 0.34, 0.36), 0.4, 0.35),
}

# Opacity of the alpha-blended materials (exported with `alphaMode: BLEND`)
GLASS_ALPHA = {"Glass_Teal": 0.3}

# Variant being built: True for the open-top cutaway model
CUT = True


def material(name):
    mat = bpy.data.materials.get(name)
    if mat:
        return mat
    color, metallic, roughness = PALETTE[name]
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    mat.diffuse_color = (*color, 1.0)
    if name in GLASS_ALPHA:
        bsdf.inputs["Alpha"].default_value = GLASS_ALPHA[name]
        mat.surface_render_method = "BLENDED"
    return mat


# ---------------------------------------------------------------------------------------
# Geometry helpers. Every helper returns a mesh object with an identity transform.
# ---------------------------------------------------------------------------------------
PARTS = []


def to_obj(name, bm, mat, sharp_angle=40.0, flat=False):
    me = bpy.data.meshes.new(name)
    bm.normal_update()
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    me.materials.append(material(mat))
    if flat:
        me.shade_flat()
    else:
        me.shade_smooth()
        me.set_sharp_from_angle(angle=radians(sharp_angle))
    PARTS.append(obj)
    return obj


def xform(obj, loc=(0, 0, 0), rot_z=0.0, rot=None, scale=None):
    m = Matrix.Translation(Vector(loc)) @ Matrix.Rotation(rot_z, 4, "Z")
    if rot is not None:
        m = m @ rot
    if scale is not None:
        m = m @ Matrix.Diagonal((*scale, 1.0))
    obj.data.transform(m)
    return obj


def box(name, size, loc, mat, bevel=0.0, rot_z=0.0, segments=2):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=Vector(size), verts=bm.verts)
    if bevel > 0:
        bmesh.ops.bevel(
            bm, geom=bm.edges[:], offset=bevel, segments=segments, profile=0.5,
            affect="EDGES", clamp_overlap=True
        )
    return xform(to_obj(name, bm, mat), loc, rot_z)


def lathe(name, prof, mat, segs=32, loc=(0, 0, 0), closed=False, angle0=0.0,
          sharp_angle=40.0, flat=False, arc=None):
    """Revolve a (r, z) profile around Z. `arc` = (a0, a1) for a partial revolve."""
    bm = bmesh.new()
    full = arc is None
    n = segs if full else segs + 1
    a0, a1 = (angle0, angle0 + TAU) if full else arc
    angles = [a0 + (a1 - a0) * i / segs for i in range(n)]
    rings = []
    for r, z in prof:
        if r <= 1e-6:
            rings.append([bm.verts.new((0, 0, z))])
        else:
            rings.append([bm.verts.new((r * cos(a), r * sin(a), z)) for a in angles])
    pairs = list(zip(rings, rings[1:]))
    if closed:
        pairs.append((rings[-1], rings[0]))
    for A, B in pairs:
        if len(A) == 1 and len(B) == 1:
            continue
        span = segs if full else segs
        for i in range(span):
            j = (i + 1) % n
            if len(A) == 1:
                bm.faces.new((A[0], B[i], B[j]))
            elif len(B) == 1:
                bm.faces.new((A[i], A[j], B[0]))
            else:
                bm.faces.new((A[i], A[j], B[j], B[i]))
    if not full:
        for ring_idx in (0, n - 1):
            cap = [r[0] if len(r) == 1 else r[ring_idx] for r in rings]
            uniq = []
            for v in cap:
                if v not in uniq:
                    uniq.append(v)
            if len(uniq) >= 3:
                bm.faces.new(uniq)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return xform(to_obj(name, bm, mat, sharp_angle, flat), loc)


def extrude_profile(name, pts, x0, x1, mat, caps=True, sharp_angle=40.0):
    """Extrude a closed (y, z) polygon along X from x0 to x1."""
    bm = bmesh.new()
    A = [bm.verts.new((x0, y, z)) for y, z in pts]
    B = [bm.verts.new((x1, y, z)) for y, z in pts]
    n = len(pts)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((A[i], A[j], B[j], B[i]))
    if caps:
        bm.faces.new(A)
        bm.faces.new(list(reversed(B)))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return to_obj(name, bm, mat, sharp_angle)


def rounded_rect(hw, hh, r, cy=0.0, cz=0.0, steps=4):
    """Closed (y, z) rounded rectangle polygon."""
    pts = []
    corners = [(hw - r, hh - r, 0), (-hw + r, hh - r, 90), (-hw + r, -hh + r, 180), (hw - r, -hh + r, 270)]
    for x, y, a in corners:
        for i in range(steps + 1):
            t = radians(a + 90 * i / steps)
            pts.append((cy + x + r * cos(t), cz + y + r * sin(t)))
    return pts


def ico(name, radius, loc, mat, subdiv=1, scale=(1, 1, 1), jitter=0.0, seed=0, flat=True):
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=subdiv, radius=radius)
    rnd = random.Random(seed)
    for v in bm.verts:
        if jitter:
            v.co *= 1.0 + rnd.uniform(-jitter, jitter)
        v.co = Vector((v.co.x * scale[0], v.co.y * scale[1], v.co.z * scale[2]))
    return xform(to_obj(name, bm, mat, flat=flat), loc)


def cyl(name, r, z0, z1, mat, loc=(0, 0), segs=24, bevel=0.0):
    b = min(bevel, r * 0.5, (z1 - z0) * 0.5)
    if b > 0:
        prof = [(0, z0), (r - b, z0), (r, z0 + b), (r, z1 - b), (r - b, z1), (0, z1)]
    else:
        prof = [(0, z0), (r, z0), (r, z1), (0, z1)]
    return lathe(name, prof, mat, segs, (loc[0], loc[1], 0))


def ring(name, r_in, r_out, z0, z1, mat, loc=(0, 0), segs=48, arc=None):
    prof = [(r_in, z0), (r_out, z0), (r_out, z1), (r_in, z1)]
    return lathe(name, prof, mat, segs, (loc[0], loc[1], 0), closed=True, arc=arc)


def polygon_prism(name, radius, n, z0, z1, mat, angle0=0.0, loc=(0, 0)):
    return lathe(name, [(0, z0), (radius, z0), (radius, z1), (0, z1)], mat, n,
                 (loc[0], loc[1], 0), angle0=angle0, sharp_angle=20)


def rotated(objs, angle):
    for o in objs:
        o.data.transform(Matrix.Rotation(angle, 4, "Z"))
    return objs


def beam(name, p0, p1, w, h, mat):
    """Box of section (w, h) spanning p0 -> p1."""
    p0, p1 = Vector(p0), Vector(p1)
    d = p1 - p0
    obj = box(name, (d.length, w, h), (0, 0, 0), mat, 0.008)
    obj.data.transform(Matrix.Translation((p0 + p1) / 2) @ d.to_track_quat("X", "Z").to_matrix().to_4x4())
    return obj


def track(func):
    """Run a builder and return the parts it created."""
    start = len(PARTS)
    func()
    return PARTS[start:]


# ---------------------------------------------------------------------------------------
# Shared tunnel cross-section (runs along X). Every tunnel-type piece uses it so modules
# connect seamlessly at tile edges.
# ---------------------------------------------------------------------------------------
T_ZC = 0.50     # tube axis height
T_RO = 0.62     # outer radius
T_RI = 0.56     # inner radius
T_CUT = 0.88    # cutaway height
T_FLOOR = 0.14  # interior floor height
T_HALF_OPEN = 0.62  # half-width of door openings in walls/domes


def _arc(r, a0, a1, n):
    return [(r * cos(a0 + (a1 - a0) * i / n), T_ZC + r * sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]


def tube_shell_profile():
    tc_o = math.asin((T_CUT - T_ZC) / T_RO)
    tc_i = math.asin((T_CUT - T_ZC) / T_RI)
    tf = math.asin((T_FLOOR - T_ZC) / T_RI)
    outer = _arc(T_RO, tc_o, -pi - tc_o, 28)
    outer = [(y, max(z, -0.10)) for y, z in outer]
    inner_l = _arc(T_RI, pi - tc_i, pi - tf, 8)
    inner_r = _arc(T_RI, TAU + tf, TAU + tc_i, 8)
    return outer + inner_l + inner_r


def band_profile(r_in, r_out, a0, a1, n=8):
    return _arc(r_out, a0, a1, n) + list(reversed(_arc(r_in, a0, a1, n)))


def tube_segment(prefix, x0, x1, ribs=(), collars=(), feet=True, stripe=True, deck=True):
    extrude_profile(f"{prefix}_Shell", tube_shell_profile(), x0, x1, "Hull_White")
    tc_o = math.asin((T_CUT - T_ZC) / T_RO)
    if stripe:
        for s in (1, -1):
            a0, a1 = (0.05, 0.28) if s == 1 else (pi - 0.28, pi - 0.05)
            extrude_profile(f"{prefix}_Stripe", band_profile(T_RO - 0.01, T_RO + 0.014, a0, a1, 4),
                            x0, x1, "Accent_Orange")
    if deck:
        box(f"{prefix}_Deck", (x1 - x0, 0.74, 0.03), ((x0 + x1) / 2, 0, T_FLOOR + 0.012), "Floor_Deck", 0.006)
        for s in (1, -1):
            box(f"{prefix}_Light", (x1 - x0, 0.025, 0.012), ((x0 + x1) / 2, s * 0.385, T_FLOOR + 0.03), "Light_Cyan")
    yc = (sqrt(T_RO**2 - (T_CUT - T_ZC) ** 2) + sqrt(T_RI**2 - (T_CUT - T_ZC) ** 2)) / 2
    for s in (1, -1):
        box(f"{prefix}_Rail", (x1 - x0, 0.085, 0.035), ((x0 + x1) / 2, s * yc, T_CUT + 0.012), "Frame_Graphite", 0.012)
    for x in ribs:
        extrude_profile(f"{prefix}_Rib", band_profile(T_RO - 0.02, T_RO + 0.05, tc_o + 0.02, -pi - tc_o - 0.02, 20),
                        x - 0.035, x + 0.035, "Frame_Graphite")
        if feet:
            box(f"{prefix}_Foot", (0.16, 1.5, 0.22), (x, 0, 0.0), "Concrete", 0.03)
    for x, d in collars:
        xa, xb = (x - 0.07, x) if d > 0 else (x, x + 0.07)
        extrude_profile(f"{prefix}_Collar", band_profile(T_RI + 0.015, T_RO + 0.08, tc_o + 0.05, -pi - tc_o - 0.05, 20),
                        xa, xb, "Frame_Graphite")
        box(f"{prefix}_CollarBase", (0.07, 1.62, 0.18), ((xa + xb) / 2, 0, 0.0), "Concrete", 0.02)
    if not CUT:
        tube_roof(prefix, x0, x1, ribs, collars)


def tube_roof(prefix, x0, x1, ribs=(), collars=()):
    """Glass vault closing the tube above the cutaway height."""
    tc_o = math.asin((T_CUT - T_ZC) / T_RO)
    extrude_profile(f"{prefix}_Glass", band_profile(T_RI + 0.02, T_RO - 0.02, tc_o, pi - tc_o, 16), x0, x1,
                    "Glass_Teal")
    for x in ribs:
        extrude_profile(f"{prefix}_RibTop", band_profile(T_RO - 0.03, T_RO + 0.03, tc_o, pi - tc_o, 16),
                        x - 0.03, x + 0.03, "Frame_Graphite")
    for x, d in collars:
        xa, xb = (x - 0.07, x) if d > 0 else (x, x + 0.07)
        extrude_profile(f"{prefix}_CollarTop", band_profile(T_RI + 0.015, T_RO + 0.08, tc_o, pi - tc_o, 16),
                        xa, xb, "Frame_Graphite")


def door_portal(prefix, x, thickness=0.1):
    """Frame around a wall opening facing X, centered on the tube section."""
    h = T_CUT + 0.06
    box(f"{prefix}_PortalL", (thickness, 0.09, h), (x, T_HALF_OPEN + 0.03, h / 2), "Frame_Graphite", 0.015)
    box(f"{prefix}_PortalR", (thickness, 0.09, h), (x, -T_HALF_OPEN - 0.03, h / 2), "Frame_Graphite", 0.015)
    for i in range(6):
        y = -0.5 + i * 0.2
        box(f"{prefix}_Sill", (thickness, 0.1, 0.02), (x, y, T_FLOOR + 0.03),
            "Hazard_Yellow" if i % 2 == 0 else "Frame_Graphite")


# ---------------------------------------------------------------------------------------
# Props
# ---------------------------------------------------------------------------------------
def tree(prefix, loc, height=1.2, seed=0):
    x, y, z = loc
    rnd = random.Random(seed)
    lathe(f"{prefix}_Trunk", [(0, z), (0.05, z), (0.03, z + height * 0.6), (0, z + height * 0.6)], "Bark", 8,
          (x, y, 0))
    for i in range(3):
        r = height * (0.28 - i * 0.05)
        ico(f"{prefix}_Leaves", r, (x + rnd.uniform(-0.08, 0.08), y + rnd.uniform(-0.08, 0.08),
                                    z + height * (0.55 + i * 0.17)),
            "Plant_Green" if i % 2 == 0 else "Plant_Dark", 1, (1, 1, 0.85), 0.18, seed + i)


def shrub(prefix, loc, r=0.15, seed=0):
    ico(f"{prefix}_Shrub", r, loc, "Plant_Dark" if seed % 2 else "Plant_Green", 1, (1, 1, 0.75), 0.2, seed)


def planter_round(prefix, loc, r, h=0.22):
    x, y = loc
    ring(f"{prefix}_Planter", r - 0.05, r, 0.0, h, "Hull_White", (x, y), 32)
    box_cap = ring(f"{prefix}_PlanterCap", r - 0.06, r + 0.01, h, h + 0.025, "Frame_Graphite", (x, y), 32)
    cyl(f"{prefix}_PlanterSoil", r - 0.05, 0.0, h - 0.03, "Soil", (x, y), 32)
    return box_cap


def planter_box(prefix, loc, size, h=0.2, rot_z=0.0, crops=0, seed=0):
    sx, sy = size
    cx, cy = loc
    parts = track(lambda: (
        box(f"{prefix}_Bed", (sx, sy, h), (0, 0, h / 2), "Hull_White", 0.02),
        box(f"{prefix}_BedTrim", (sx + 0.02, sy + 0.02, 0.03), (0, 0, h), "Frame_Graphite", 0.008),
        box(f"{prefix}_BedSoil", (sx - 0.08, sy - 0.08, 0.02), (0, 0, h + 0.025), "Soil"),
        box(f"{prefix}_BedStripe", (sx + 0.016, sy + 0.016, 0.03), (0, 0, h * 0.45), "Accent_Orange"),
    ))
    rnd = random.Random(seed)
    if crops:
        rows = max(1, int(sy / 0.22))
        for r in range(rows):
            yy = -sy / 2 + (r + 0.5) * sy / rows
            for i in range(crops):
                xx = -sx / 2 + (i + 0.5) * sx / crops
                parts.append(ico(f"{prefix}_Crop", 0.07 + rnd.uniform(-0.015, 0.02), (xx, yy, h + 0.05),
                                 "Plant_Green" if rnd.random() > 0.3 else "Plant_Dark", 1, (1, 1, 0.8), 0.25,
                                 seed * 100 + r * 10 + i))
    for p in parts:
        xform(p, (cx, cy, 0), rot_z)
    return parts


def bench(prefix, loc, rot_z=0.0, length=0.6):
    parts = track(lambda: (
        box(f"{prefix}_Seat", (length, 0.16, 0.035), (0, 0, 0.12), "Hull_Grey", 0.01),
        box(f"{prefix}_LegA", (0.04, 0.14, 0.11), (-length / 2 + 0.06, 0, 0.055), "Frame_Graphite", 0.008),
        box(f"{prefix}_LegB", (0.04, 0.14, 0.11), (length / 2 - 0.06, 0, 0.055), "Frame_Graphite", 0.008),
    ))
    for p in parts:
        xform(p, (*loc, 0), rot_z)


def lamp_post(prefix, loc, h=0.9):
    x, y = loc
    cyl(f"{prefix}_Post", 0.025, 0.0, h, "Frame_Graphite", (x, y), 8)
    cyl(f"{prefix}_LampBase", 0.06, 0.0, 0.05, "Frame_Graphite", (x, y), 12, 0.01)
    cyl(f"{prefix}_Lamp", 0.055, h, h + 0.05, "Light_Cyan", (x, y), 12, 0.015)
    cyl(f"{prefix}_LampCap", 0.07, h + 0.05, h + 0.07, "Hull_White", (x, y), 12, 0.01)


def container(prefix, loc, rot_z, mat, size=(2.0, 0.82, 0.86)):
    L, W, H = size

    def build():
        box(f"{prefix}_Body", (L - 0.04, W - 0.02, H - 0.02), (0, 0, H / 2), mat, 0.01)
        n = 14
        for i in range(n):
            x = -L / 2 + 0.12 + i * (L - 0.24) / (n - 1)
            for s in (1, -1):
                box(f"{prefix}_Rib", (0.05, 0.02, H - 0.1), (x, s * (W / 2 - 0.005), H / 2), mat, 0.006)
            box(f"{prefix}_RoofRib", (0.06, W - 0.1, 0.015), (x, 0, H - 0.005), mat, 0.004)
        for sx in (1, -1):
            for sy in (1, -1):
                box(f"{prefix}_Post", (0.075, 0.075, H + 0.01), (sx * (L / 2 - 0.03), sy * (W / 2 - 0.03), H / 2),
                    "Frame_Graphite", 0.008)
            box(f"{prefix}_RailTop", (0.06, W, 0.05), (sx * (L / 2 - 0.03), 0, H - 0.025), "Frame_Graphite", 0.008)
            box(f"{prefix}_RailBot", (0.06, W, 0.05), (sx * (L / 2 - 0.03), 0, 0.025), "Frame_Graphite", 0.008)
        for s in (1, -1):
            box(f"{prefix}_SideTop", (L, 0.05, 0.05), (0, s * (W / 2 - 0.02), H - 0.025), "Frame_Graphite", 0.008)
            box(f"{prefix}_SideBot", (L, 0.05, 0.05), (0, s * (W / 2 - 0.02), 0.025), "Frame_Graphite", 0.008)
        for y in (-0.25, -0.1, 0.1, 0.25):
            box(f"{prefix}_DoorBar", (0.02, 0.018, H - 0.16), (L / 2 + 0.005, y * W / 0.82, H / 2), "Hull_Grey")
        box(f"{prefix}_Label", (0.5, 0.012, 0.12), (0.3, -W / 2 - 0.004, H * 0.7), "Hull_White")

    for p in track(build):
        xform(p, loc, rot_z)


def crate(prefix, loc, s=0.32, rot_z=0.0, mat="Crate_Tan"):
    def build():
        box(f"{prefix}_Crate", (s, s, s), (0, 0, s / 2), mat, 0.012)
        for z in (0.04, s - 0.04):
            box(f"{prefix}_CrateBand", (s + 0.012, s + 0.012, 0.03), (0, 0, z), "Frame_Graphite", 0.005)

    for p in track(build):
        xform(p, loc, rot_z)


def barrel(prefix, loc, mat="Accent_Orange"):
    x, y, z = loc
    lathe(f"{prefix}_Barrel", [(0, z), (0.1, z), (0.11, z + 0.02), (0.11, z + 0.26), (0.1, z + 0.28), (0, z + 0.28)],
          mat, 16, (x, y, 0))
    for zz in (0.09, 0.19):
        ring(f"{prefix}_BarrelHoop", 0.105, 0.118, z + zz, z + zz + 0.015, "Frame_Graphite", (x, y), 16)


def fuel_tank(prefix, loc, rot_z, length=2.0, r=0.42):
    def build():
        prof = [(0, -length / 2 - r * 0.55)]
        for i in range(1, 7):
            a = pi / 2 * (1 - i / 6)
            prof.append((r * cos(a), -length / 2 + r * 0.55 * (-sin(a))))
        prof += [(r, length / 2)]
        for i in range(1, 7):
            a = pi / 2 * i / 6
            prof.append((r * cos(a), length / 2 + r * 0.55 * sin(a)))
        tank = lathe(f"{prefix}_Tank", prof, "Hull_White", 32)
        tank.data.transform(Matrix.Rotation(pi / 2, 4, "Y"))
        tank.data.transform(Matrix.Translation((0, 0, r + 0.12)))
        for x in (-length * 0.3, length * 0.3):
            band = ring(f"{prefix}_TankBand", r - 0.01, r + 0.015, x - 0.05, x + 0.05, "Accent_Orange", segs=32)
            band.data.transform(Matrix.Translation((0, 0, 0)))
            band.data.transform(Matrix.Rotation(pi / 2, 4, "Y"))
            band.data.transform(Matrix.Translation((0, 0, r + 0.12)))
            box(f"{prefix}_Saddle", (0.14, r * 1.7, r * 0.7), (x, 0, r * 0.35), "Frame_Graphite", 0.02)
        box(f"{prefix}_Valve", (0.12, 0.12, 0.12), (length / 2 + r * 0.45, 0, r + 0.12), "Frame_Graphite", 0.02)

    for p in track(build):
        xform(p, loc, rot_z)


def console(prefix, loc, rot_z):
    """Desk with an angled screen, facing -X in local space (operator sits at -X)."""
    def build():
        box(f"{prefix}_Desk", (0.34, 0.7, 0.26), (0, 0, 0.13), "Hull_White", 0.02)
        box(f"{prefix}_DeskTop", (0.4, 0.74, 0.03), (-0.02, 0, 0.275), "Frame_Graphite", 0.01)
        scr = box(f"{prefix}_Screen", (0.03, 0.62, 0.26), (0, 0, 0), "Frame_Graphite", 0.008)
        scr.data.transform(Matrix.Rotation(radians(-18), 4, "Y"))
        scr.data.transform(Matrix.Translation((0.1, 0, 0.43)))
        glow = box(f"{prefix}_ScreenGlow", (0.01, 0.56, 0.2), (0, 0, 0), "Light_Cyan")
        glow.data.transform(Matrix.Rotation(radians(-18), 4, "Y"))
        glow.data.transform(Matrix.Translation((0.08, 0, 0.43)))
        for y in (-0.22, 0.0, 0.22):
            box(f"{prefix}_Key", (0.08, 0.12, 0.008), (-0.08, y, 0.294), "Accent_Orange" if y == 0 else "Light_Cyan")
        lathe(f"{prefix}_Chair", [(0, 0.0), (0.09, 0.0), (0.09, 0.02), (0.02, 0.03), (0.02, 0.15), (0.11, 0.15),
                                  (0.11, 0.19), (0, 0.19)], "Frame_Graphite", 12, (-0.38, 0, 0))
        box(f"{prefix}_ChairBack", (0.04, 0.2, 0.18), (-0.48, 0, 0.27), "Accent_Orange", 0.015)

    for p in track(build):
        xform(p, loc, rot_z)


def dish(prefix, loc, rot_z, scale=1.0):
    def build():
        s = scale
        cyl(f"{prefix}_DishMast", 0.05 * s, 0.0, 1.0 * s, "Frame_Graphite", (0, 0), 12)
        box(f"{prefix}_DishBase", (0.4 * s, 0.4 * s, 0.12 * s), (0, 0, 0.06 * s), "Concrete", 0.02)
        box(f"{prefix}_DishHousing", (0.18 * s, 0.16 * s, 0.14 * s), (0, 0, 1.0 * s), "Hull_White", 0.02)
        R = 0.5 * s
        prof = []
        for i in range(9):
            r = R * i / 8
            prof.append((r, 0.35 * r * r / R))
        prof += [(R + 0.015, 0.35 * R + 0.01)]
        for i in reversed(range(9)):
            r = R * i / 8
            prof.append((r, 0.35 * r * r / R - 0.03 * s))
        bowl = lathe(f"{prefix}_Dish", prof, "Hull_White", 32, sharp_angle=60)
        rim = ring(f"{prefix}_DishRim", R - 0.02, R + 0.03, 0.35 * R - 0.01, 0.35 * R + 0.03, "Accent_Orange", segs=32)
        arm = cyl(f"{prefix}_DishFeed", 0.015 * s, 0.0, 0.42 * s, "Frame_Graphite", (0, 0), 8)
        tip = cyl(f"{prefix}_DishTip", 0.04 * s, 0.4 * s, 0.47 * s, "Light_Cyan", (0, 0), 12, 0.01)
        tilt = Matrix.Translation((0, 0, 1.08 * s)) @ Matrix.Rotation(radians(40), 4, "Y")
        for o in (bowl, rim, arm, tip):
            o.data.transform(tilt)

    for p in track(build):
        xform(p, loc, rot_z)


def solar_panel(prefix, loc, rot_z, w=1.1, d=0.6):
    def build():
        cyl(f"{prefix}_SolarPost", 0.035, 0.0, 0.42, "Frame_Graphite", (0, 0), 10)
        panel = box(f"{prefix}_SolarPanel", (d, w, 0.03), (0, 0, 0), "Solar_Blue", 0.006)
        frame = box(f"{prefix}_SolarFrame", (d + 0.04, w + 0.04, 0.02), (0, 0, -0.012), "Hull_White", 0.006)
        lines = [box(f"{prefix}_SolarLine", (d, 0.012, 0.034), (0, -w / 2 + w * i / 4, 0), "Hull_Grey")
                 for i in range(1, 4)]
        tilt = Matrix.Translation((0, 0, 0.48)) @ Matrix.Rotation(radians(28), 4, "Y")
        for o in [panel, frame, *lines]:
            o.data.transform(tilt)

    for p in track(build):
        xform(p, loc, rot_z)


def geodesic_shell(prefix, R, Rz, zcut, door_x, subdiv=3, strut=0.05):
    """Cutaway geodesic dome band (z in [0, zcut]) with door openings toward ±X/±Y."""
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=subdiv, radius=1.0)
    bmesh.ops.rotate(bm, cent=(0, 0, 0), matrix=Matrix.Rotation(radians(18), 3, "Z"), verts=bm.verts)
    for v in bm.verts:
        v.co = Vector((v.co.x * R, v.co.y * R, v.co.z * Rz))
    geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
    bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(0, 0, 0.0), plane_no=(0, 0, 1), clear_inner=True)
    if zcut < Rz:
        geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(0, 0, zcut), plane_no=(0, 0, 1), clear_outer=True)
    for ang in (0, pi / 2, pi, 3 * pi / 2):
        d = Vector((cos(ang), sin(ang), 0))
        for side in (1, -1):
            n = Vector((-sin(ang), cos(ang), 0)) * side
            geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
            bmesh.ops.bisect_plane(bm, geom=geom, plane_co=n * T_HALF_OPEN, plane_no=n)
        geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
        bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(0, 0, T_CUT + 0.06), plane_no=(0, 0, 1))
        kill = [f for f in bm.faces
                if f.calc_center_median().dot(d) > door_x
                and abs(f.calc_center_median().dot(Vector((-sin(ang), cos(ang), 0)))) < T_HALF_OPEN
                and f.calc_center_median().z < T_CUT + 0.06]
        bmesh.ops.delete(bm, geom=kill, context="FACES")
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=0.004)

    glass_bm = bm.copy()
    glass = to_obj(f"{prefix}_Glass", glass_bm, "Glass_Teal", flat=True)
    mod = glass.modifiers.new("solid", "SOLIDIFY")
    mod.thickness = 0.025
    mod.offset = 0.0
    apply_modifiers(glass)

    frame = to_obj(f"{prefix}_Struts", bm, "Hull_White", flat=True)
    mod = frame.modifiers.new("wire", "WIREFRAME")
    mod.thickness = strut
    mod.use_even_offset = True
    mod.use_boundary = True
    mod.use_replace = True
    apply_modifiers(frame)
    frame.data.shade_smooth()
    frame.data.set_sharp_from_angle(angle=radians(35))


def apply_modifiers(obj):
    dg = bpy.context.evaluated_depsgraph_get()
    ev = obj.evaluated_get(dg)
    me = bpy.data.meshes.new_from_object(ev)
    old = obj.data
    obj.modifiers.clear()
    obj.data = me
    bpy.data.meshes.remove(old)


# ---------------------------------------------------------------------------------------
# Models
# ---------------------------------------------------------------------------------------
def build_tunnel_2():
    tube_segment("Tunnel", -1.0, 1.0, ribs=(-0.5, 0.0, 0.5), collars=((1.0, 1), (-1.0, -1)))


def build_tunnel_4():
    box("Junction_Base", (2.0, 2.0, 0.24), (0, 0, T_FLOOR - 0.12), "Concrete", 0.02)
    box("Junction_Deck", (1.9, 1.9, 0.025), (0, 0, T_FLOOR + 0.012), "Floor_Deck", 0.005)
    ring("Junction_HazardRing", 0.36, 0.41, T_FLOOR + 0.02, T_FLOOR + 0.03, "Hazard_Yellow", segs=40)
    cyl("Junction_Hatch", 0.14, T_FLOOR + 0.02, T_FLOOR + 0.045, "Hull_Grey", (0, 0), 16, 0.008)
    r = 1.0 - T_RI
    for k in range(4):
        a = k * pi / 2 + pi / 4
        cx, cy = sqrt(2) * cos(a), sqrt(2) * sin(a)
        arc = (a + pi - pi / 4, a + pi + pi / 4)
        lathe("Junction_Corner", [(0, -0.1), (r, -0.1), (r, T_CUT), (0, T_CUT)], "Hull_White", 12, (cx, cy, 0),
              arc=arc)
        lathe("Junction_CornerCap", [(0, T_CUT), (r + 0.03, T_CUT), (r + 0.03, T_CUT + 0.035), (0, T_CUT + 0.035)],
              "Frame_Graphite", 12, (cx, cy, 0), arc=arc)
        lathe("Junction_CornerStripe", [(r - 0.01, 0.5), (r + 0.014, 0.5), (r + 0.014, 0.6), (r - 0.01, 0.6)],
              "Accent_Orange", 12,
              (cx, cy, 0), closed=True, arc=arc)
        lathe("Junction_CornerLight", [(r - 0.01, T_FLOOR + 0.03), (r + 0.014, T_FLOOR + 0.03),
                                       (r + 0.014, T_FLOOR + 0.045), (r - 0.01, T_FLOOR + 0.045)], "Light_Cyan", 12,
              (cx, cy, 0), closed=True, arc=arc)
        rotated(track(lambda: tube_segment("Arm", 0.9, 1.0, collars=((1.0, 1),), ribs=(), deck=False)), k * pi / 2)
    if not CUT:
        for k in range(2):
            rotated(track(lambda: tube_roof("Junction", -0.9, 0.9, ribs=(-0.56, 0.56))), k * pi / 2)


def build_airlock():
    # Tunnel side (+X, AnchorIn) -> airlock chamber -> apron toward the path (-X, AnchorOut)
    tube_segment("Tunnel", 0.32, 1.0, ribs=(0.62,), collars=((1.0, 1),))
    x0, x1, hw = -0.5, 0.36, 0.72
    h = 1.0 if CUT else 1.25
    xm = (x0 + x1) / 2
    box("Lock_Plinth", (x1 - x0 + 0.12, 2 * hw + 0.12, 0.3), (xm, 0, 0.0), "Concrete", 0.03)
    if CUT:
        t = 0.08
        box("Lock_Deck", (x1 - x0 - t, 2 * hw - t, 0.03), (xm, 0, T_FLOOR + 0.01), "Floor_Deck", 0.005)
        box("Lock_WallOut", (t, 2 * hw, h - 0.1), (x0 + t / 2, 0, (h + 0.1) / 2), "Hull_White", 0.02)
        for s in (1, -1):
            box("Lock_WallSide", (x1 - x0, t, h - 0.1), (xm, s * (hw - t / 2), (h + 0.1) / 2), "Hull_White", 0.02)
            yc = (T_HALF_OPEN + hw) / 2
            box("Lock_WallIn", (t, hw - T_HALF_OPEN, h - 0.1), (x1 - t / 2, s * yc, (h + 0.1) / 2), "Hull_White", 0.02)
            box("Lock_StripeSide", (x1 - x0 + 0.02, 0.02, 0.1), (xm, s * hw, 0.82), "Accent_Orange")
            box("Lock_CapSide", (x1 - x0 + 0.02, t + 0.03, 0.035), (xm, s * (hw - t / 2), h), "Frame_Graphite", 0.008)
            box("Lock_CapIn", (t + 0.03, hw - T_HALF_OPEN + 0.02, 0.035), (x1 - t / 2, s * yc, h), "Frame_Graphite",
                0.008)
            box("Lock_Locker", (0.36, 0.14, 0.62), (xm - 0.05, s * (hw - t - 0.07), T_FLOOR + 0.31), "Hull_Grey", 0.015)
            box("Lock_LockerLed", (0.06, 0.012, 0.03), (xm - 0.05, s * (hw - t - 0.145), T_FLOOR + 0.5), "Light_Cyan")
        box("Lock_CapOut", (t + 0.03, 2 * hw + 0.02, 0.035), (x0 + t / 2, 0, h), "Frame_Graphite", 0.008)
        box("Lock_StripeOut", (0.02, 2 * hw, 0.1), (x0, 0, 0.82), "Accent_Orange")
        bx, by = x0 + 0.06, -hw + 0.06
    else:
        box("Lock_Body", (x1 - x0, 2 * hw, h - 0.1), (xm, 0, (h + 0.1) / 2), "Hull_White", 0.1, segments=3)
        box("Lock_Stripe", (x1 - x0 + 0.02, 2 * hw + 0.02, 0.1), (xm, 0, 0.82), "Accent_Orange", 0.01)
        box("Lock_RoofTrim", (x1 - x0 - 0.1, 2 * hw - 0.1, 0.06), (xm, 0, h + 0.01), "Frame_Graphite", 0.02)
        box("Lock_Vent", (0.3, 0.4, 0.1), (0.05, 0.15, h + 0.07), "Hull_Grey", 0.02)
        for i in range(4):
            box("Lock_VentSlat", (0.26, 0.03, 0.02), (0.05, 0.0 + i * 0.1, h + 0.13), "Frame_Graphite")
        cyl("Lock_Antenna", 0.012, h, h + 0.55, "Frame_Graphite", (0.2, -0.45), 6)
        cyl("Lock_AntennaTip", 0.025, h + 0.55, h + 0.59, "Light_Cyan", (0.2, -0.45), 8, 0.008)
        bx, by = -0.25, -0.35
    cyl("Lock_BeaconBase", 0.09, h, h + 0.06, "Frame_Graphite", (bx, by), 16, 0.01)
    lathe("Lock_Beacon", [(0, h + 0.06), (0.07, h + 0.06), (0.07, h + 0.12), (0.045, h + 0.17), (0, h + 0.18)],
          "Accent_Orange", 16, (bx, by, 0))
    # Outer door on the -X face
    dx = x0 - 0.015
    box("Door_Frame", (0.06, 0.84, 0.9 if CUT else 1.0), (dx, 0, 0.55 if CUT else 0.6), "Frame_Graphite", 0.03)
    box("Door_Panel", (0.07, 0.62, 0.86), (dx - 0.01, 0, 0.58), "Hull_Grey", 0.02)
    box("Door_Window", (0.075, 0.3, 0.16), (dx - 0.012, 0, 0.82), "Glass_Teal", 0.02)
    box("Door_Seam", (0.075, 0.012, 0.84), (dx - 0.013, 0, 0.58), "Frame_Graphite")
    for i in range(7):
        z = 0.14 + i * 0.13
        for s in (1, -1):
            box("Door_Hazard", (0.065, 0.05, 0.065), (dx - 0.005, s * 0.39, z),
                "Hazard_Yellow" if i % 2 == 0 else "Frame_Graphite")
    box("Door_Lamp", (0.04, 0.2, 0.04), (dx - 0.03, 0, 1.12 if not CUT else 0.96), "Light_Cyan", 0.01)
    box("Door_Panel_Ctrl", (0.03, 0.08, 0.12), (dx - 0.01, -0.5, 0.6), "Light_Cyan", 0.008)
    # Side windows
    for s in (1, -1):
        box("Lock_Window", (0.42, 0.03, 0.18), ((x0 + x1) / 2, s * hw, 0.6), "Glass_Teal", 0.03)
        box("Lock_WindowFrame", (0.48, 0.02, 0.24), ((x0 + x1) / 2, s * (hw - 0.004), 0.6), "Frame_Graphite", 0.03)
    # Apron ramp down to path level
    ramp = extrude_profile("Apron", [(-0.5, -0.12), (0.5, -0.12), (0.5, 0.045), (-0.5, 0.045)], -1.0, x0, "Concrete")
    for v in ramp.data.vertices:
        if v.co.z > 0 and v.co.x > -0.6:
            v.co.z = 0.045 + (v.co.x + 0.6) / (x0 + 0.6) * 0.07
    for s in (1, -1):
        box("Apron_Curb", (1.0 + x0, 0.07, 0.16), ((-1.0 + x0) / 2, s * 0.455, 0.0), "Frame_Graphite", 0.015)
    for i in range(5):
        box("Apron_Stripe", (0.06, 0.7, 0.006), (-0.9 + i * 0.09, 0, 0.05), "Hazard_Yellow" if i % 2 == 0 else "Concrete")


def path_slabs(prefix, x0, x1, n, width=0.84):
    L = (x1 - x0) / n
    for i in range(n):
        cx = x0 + (i + 0.5) * L
        box(f"{prefix}_Slab", (L - 0.025, width, 0.16), (cx, 0, -0.04), "Concrete", 0.012)


def path_curbs(prefix, x0, x1, studs=()):
    for s in (1, -1):
        box(f"{prefix}_Curb", (x1 - x0, 0.07, 0.19), ((x0 + x1) / 2, s * 0.455, -0.035), "Frame_Graphite", 0.015)
        for x in studs:
            box(f"{prefix}_Stud", (0.05, 0.072, 0.02), (x, s * 0.455, 0.055), "Accent_Orange", 0.005)


def build_path_2():
    path_slabs("Path", -1.0, 1.0, 4)
    path_curbs("Path", -1.0, 1.0, studs=(-0.5, 0.5))


def build_path_4():
    box("Path_Center", (0.84, 0.84, 0.16), (0, 0, -0.04), "Concrete", 0.012)
    ring("Path_Marking", 0.22, 0.26, 0.04, 0.045, "Hazard_Yellow", segs=32)
    for k in range(4):
        rotated(track(lambda: (path_slabs("Arm", 0.42, 1.0, 1), path_curbs("Arm", 0.49, 1.0, studs=(0.75,)))),
                k * pi / 2)
    for sx in (1, -1):
        for sy in (1, -1):
            box("Path_Corner", (0.07, 0.07, 0.19), (sx * 0.455, sy * 0.455, -0.035), "Frame_Graphite", 0.015)
            cyl("Path_Bollard", 0.03, 0.0, 0.16, "Hull_White", (sx * 0.455, sy * 0.455), 10, 0.01)
            cyl("Path_BollardCap", 0.032, 0.16, 0.19, "Accent_Orange", (sx * 0.455, sy * 0.455), 10, 0.01)


# ---------------------------------------------------------------------------------------
# Domes: empty, with a buildable tile grid for dome modules (see `%P%Interior`)
# ---------------------------------------------------------------------------------------
DOME_ZCUT = 1.5
DOME_FLOOR = T_FLOOR + 0.03  # top of the module slots


def dome_metrics(n):
    """(base radius, height, buildable radius) of an n x n tiles dome."""
    R = n - 0.28
    return R, 0.75 * R, R - 0.45


def dome_tiles(n):
    """Tile centers fully inside the buildable radius."""
    _, _, rb = dome_metrics(n)
    centers = [2 * i for i in range(-(n // 2), n // 2 + 1)]
    return [(x, y) for x in centers for y in centers if sqrt((abs(x) + 1) ** 2 + (abs(y) + 1) ** 2) <= rb]


def dome(n, subdiv):
    R, Rz, rb = dome_metrics(n)
    half = float(n)
    lathe("Dome_Foundation", [(0, -0.25), (R + 0.14, -0.25), (R + 0.14, 0.06), (R + 0.06, 0.12), (0, 0.12)],
          "Concrete", 96)
    cyl("Dome_Floor", R - 0.02, 0.10, T_FLOOR, "Concrete", (0, 0), 96)
    ring("Dome_BaseRing", R - 0.1, R + 0.08, 0.08, 0.24, "Frame_Graphite", segs=96)
    ring("Dome_EdgeLine", rb + 0.05, rb + 0.12, T_FLOOR, T_FLOOR + 0.012, "Accent_Orange", segs=96)
    strut = 0.04 + 0.005 * n
    if CUT:
        geodesic_shell("Dome", R, Rz, DOME_ZCUT, R * 0.8, subdiv, strut)
        r_top = R * sqrt(1 - (DOME_ZCUT / Rz) ** 2)
        ring("Dome_TopRing", r_top - 0.08, r_top + 0.08, DOME_ZCUT - 0.05, DOME_ZCUT + 0.05, "Accent_Orange",
             segs=96)
    else:
        geodesic_shell("Dome", R, Rz, Rz + 1.0, R * 0.8, subdiv, strut)
        a = 0.1 * R
        lathe("Dome_Apex", [(0, Rz - 0.1), (a, Rz - 0.1), (a, Rz - 0.03), (a * 0.65, Rz + 0.06), (0, Rz + 0.08)],
              "Hull_White", 24)
        ring("Dome_ApexRing", a - 0.03, a + 0.02, Rz - 0.11, Rz - 0.04, "Accent_Orange", segs=24)
    tiles = dome_tiles(n)
    for x, y in tiles:
        box("Dome_Slot", (1.9, 1.9, 0.03), (x, y, T_FLOOR + 0.015), "Floor_Deck", 0.008)
    edge = max(x for x, y in tiles if y == 0) + 1.0
    x_in = R * sqrt(1 - ((T_CUT * 0.6) / Rz) ** 2) - 0.35
    for k in range(4):
        def door():
            tube_segment("Dome_Tunnel", x_in, half, ribs=(), collars=((half, 1), (x_in, -1)))
            box("Dome_Walk", (x_in - edge, 0.84, 0.02), ((x_in + edge) / 2, 0, T_FLOOR + 0.01), "Hull_Grey", 0.005)

        rotated(track(door), k * pi / 2)
        a = k * pi / 2 + pi / 4
        lamp_post("Dome_Lamp", (cos(a) * (rb + 0.12), sin(a) * (rb + 0.12)), 0.6)


def dome_props(n):
    _, _, rb = dome_metrics(n)
    return dict(extent=(n, n), ins=[(n, 0), (-n, 0), (0, n), (0, -n)], interior=(rb, DOME_FLOOR))


def build_dome_5x5():
    dome(5, 3)


def build_dome_7x7():
    dome(7, 4)


def build_dome_9x9():
    dome(9, 4)


def octagon_wall(a, h, t, z0, door, windows):
    side = 2 * a * math.tan(pi / 8)
    m = a - t / 2
    parts = []
    for k in range(8):
        phi = k * pi / 4
        c, s = cos(phi), sin(phi)

        def seg(along0, along1, zz0, zz1, mat, depth=t, off=0.0, bevel=0.015, name="Ctrl_Wall"):
            mid = (along0 + along1) / 2
            o = box(name, (depth, along1 - along0, zz1 - zz0), (0, 0, 0), mat, bevel)
            o.data.transform(Matrix.Translation((m + off, mid, (zz0 + zz1) / 2)))
            o.data.transform(Matrix.Rotation(phi, 4, "Z"))
            parts.append(o)

        if k % 2 == 0 and door:
            hw = T_HALF_OPEN
            seg(-side / 2, -hw, z0, h, "Hull_White")
            seg(hw, side / 2, z0, h, "Hull_White")
            seg(-hw, hw, T_CUT + 0.06, h, "Hull_White")
            seg(-side / 2, -hw, 0.82, 0.92, "Accent_Orange", 0.012, t / 2 + 0.004, 0.0, "Ctrl_Stripe")
            seg(hw, side / 2, 0.82, 0.92, "Accent_Orange", 0.012, t / 2 + 0.004, 0.0, "Ctrl_Stripe")
        else:
            seg(-side / 2, side / 2, z0, 0.42, "Hull_White")
            seg(-side / 2, side / 2, 1.0, h, "Hull_White")
            for w in range(windows):
                w0 = -side / 2 + 0.06 + w * (side - 0.12) / windows
                w1 = w0 + (side - 0.12) / windows
                seg(w0 + 0.03, w1 - 0.03, 0.42, 1.0, "Glass_Teal", t * 0.5, 0.0, 0.0, "Ctrl_Glass")
                seg(w0 - 0.03, w0 + 0.03, 0.42, 1.0, "Frame_Graphite", t + 0.02, 0.0, 0.008, "Ctrl_Mullion")
            seg(side / 2 - 0.09, side / 2 - 0.03, 0.42, 1.0, "Frame_Graphite", t + 0.02, 0.0, 0.008, "Ctrl_Mullion")
            seg(-side / 2, side / 2, 0.82, 0.92, "Accent_Orange", 0.012, t / 2 + 0.004, 0.0, "Ctrl_Stripe")
        seg(-side / 2 - 0.02, side / 2 + 0.02, h, h + 0.045, "Frame_Graphite", t + 0.05, 0.0, 0.012, "Ctrl_Cap")
    rr = a / cos(pi / 8)
    for k in range(8):
        phi = k * pi / 4 + pi / 8
        parts.append(cyl("Ctrl_Pillar", 0.09, z0 - 0.05, h + 0.06, "Frame_Graphite",
                         (cos(phi) * (rr - 0.04), sin(phi) * (rr - 0.04)), 8, 0.02))
    return parts


def build_control_room():
    a, h, t = 2.45, 1.3, 0.14
    polygon_prism("Ctrl_Foundation", 2.95 * sqrt(2), 4, -0.25, 0.06, "Concrete", angle0=pi / 4)
    polygon_prism("Ctrl_Plinth", (a + 0.2) / cos(pi / 8), 8, 0.0, 0.1, "Hull_Grey", angle0=pi / 8)
    polygon_prism("Ctrl_Deck", (a - 0.05) / cos(pi / 8), 8, 0.1, T_FLOOR, "Floor_Deck", angle0=pi / 8)
    octagon_wall(a, h, t, T_FLOOR - 0.02, door=True, windows=3)
    if not CUT:
        rr = a / cos(pi / 8) + 0.02
        apex = h + 0.6
        lathe("Ctrl_Roof", [(0, apex), (rr, h + 0.05), (rr - 0.02, h + 0.03), (0, apex - 0.025)], "Glass_Teal", 8,
              angle0=pi / 8, closed=True, sharp_angle=20)
        for k in range(8):
            phi = k * pi / 4 + pi / 8
            beam("Ctrl_RoofRib", (cos(phi) * (rr - 0.06), sin(phi) * (rr - 0.06), h + 0.06), (0, 0, apex), 0.06, 0.05,
                 "Frame_Graphite")
        lathe("Ctrl_RoofHub", [(0, apex - 0.05), (0.18, apex - 0.05), (0.18, apex + 0.02), (0.1, apex + 0.07),
                               (0, apex + 0.08)], "Hull_White", 16)
        ring("Ctrl_RoofHubBand", 0.175, 0.19, apex - 0.03, apex + 0.0, "Accent_Orange", segs=16)
    for k in range(4):
        def door():
            tube_segment("Ctrl_Tunnel", a - 0.05, 3.0, collars=((3.0, 1),), ribs=(), feet=False)
            door_portal("Ctrl_Door", a + t / 2 + 0.02)
        rotated(track(door), k * pi / 2)
    # Command centre
    lathe("Ctrl_Holo", [(0, T_FLOOR), (0.5, T_FLOOR), (0.5, 0.2), (0.38, 0.32), (0.55, 0.36), (0.55, 0.4),
                        (0, 0.4)], "Hull_White", 40)
    cyl("Ctrl_HoloTop", 0.45, 0.4, 0.415, "Light_Cyan", (0, 0), 40)
    ring("Ctrl_HoloRing", 0.5, 0.56, 0.38, 0.42, "Frame_Graphite", segs=40)
    lathe("Ctrl_HoloProj", [(0, 0.41), (0.1, 0.41), (0.04, 0.6), (0, 0.62)], "Light_Cyan", 12)
    ring("Ctrl_FloorRing", 0.9, 0.98, T_FLOOR, T_FLOOR + 0.008, "Accent_Orange", segs=48)
    for k in range(4):
        phi = k * pi / 2 + pi / 4
        c, s = cos(phi), sin(phi)
        console("Ctrl_Console", (c * 1.85, s * 1.85, T_FLOOR), phi)
    # Exterior corners: dish, mast, solar, batteries
    q = 2.62
    dish("Ctrl_Dish", (q, q, 0), pi + pi / 4, 0.85)
    cyl("Ctrl_Mast", 0.035, 0.0, 2.1, "Frame_Graphite", (-q, q), 8)
    for z in (0.7, 1.3, 1.8):
        box("Ctrl_MastArm", (0.3, 0.03, 0.03), (-q, q, z), "Frame_Graphite", 0.005, rot_z=pi / 4)
    cyl("Ctrl_MastTip", 0.06, 2.1, 2.18, "Accent_Orange", (-q, q), 12, 0.02)
    box("Ctrl_MastBase", (0.35, 0.35, 0.14), (-q, q, 0.07), "Concrete", 0.02)
    solar_panel("Ctrl_Solar", (-q + 0.05, -q + 0.05, 0), pi / 4 + pi, 0.8, 0.5)
    for i, (dx, dy) in enumerate(((0.0, 0.0), (0.22, -0.22))):
        box("Ctrl_Battery", (0.3, 0.42, 0.34), (q - 0.08 + dx, -q + 0.08 + dy, 0.17), "Hull_White", 0.03,
            rot_z=-pi / 4)
        box("Ctrl_BatteryBand", (0.31, 0.43, 0.05), (q - 0.08 + dx, -q + 0.08 + dy, 0.25), "Accent_Orange", 0.01,
            rot_z=-pi / 4)
        box("Ctrl_BatteryLed", (0.04, 0.04, 0.02), (q - 0.08 + dx, -q + 0.08 + dy, 0.35), "Light_Cyan", 0.005)


def build_storage_3x3():
    half = 2.96
    box("Yard_Slab", (2 * half, 2 * half, 0.28), (0, 0, -0.1), "Concrete", 0.04)
    # Painted bay outlines and lanes
    for k in range(4):
        def lane():
            box("Yard_Lane", (half - 0.5, 0.84, 0.01), ((half + 0.5) / 2, 0, 0.042), "Hull_Grey")
            for i in range(5):
                box("Yard_Arrow", (0.18, 0.04, 0.012), (1.0 + i * 0.38, 0, 0.046), "Hull_White")
            for s in (1, -1):
                box("Yard_LaneLine", (half - 0.5, 0.04, 0.012), ((half + 0.5) / 2, s * 0.44, 0.046),
                    "Hazard_Yellow")
        rotated(track(lane), k * pi / 2)
    box("Yard_Center", (1.0, 1.0, 0.01), (0, 0, 0.042), "Hull_Grey")
    ring("Yard_CenterMark", 0.3, 0.36, 0.04, 0.05, "Hazard_Yellow", segs=32)
    # Fence posts along the perimeter, gaps at the lane entrances
    for k in range(4):
        def fence():
            for x in (-2.75, -2.15, -1.55, -0.95, 0.95, 1.55, 2.15, 2.75):
                cyl("Yard_FencePost", 0.03, 0.0, 0.5, "Frame_Graphite", (x, half - 0.08), 8, 0.008)
            for x0, x1 in ((-2.78, -0.92), (0.92, 2.78)):
                box("Yard_FenceRail", (x1 - x0, 0.03, 0.03), ((x0 + x1) / 2, half - 0.08, 0.46), "Hazard_Yellow",
                    0.008)
                box("Yard_FenceRail", (x1 - x0, 0.03, 0.03), ((x0 + x1) / 2, half - 0.08, 0.26), "Frame_Graphite",
                    0.008)
            for x in (-0.9, 0.9):
                cyl("Yard_GatePost", 0.06, 0.0, 0.7, "Hull_White", (x, half - 0.08), 12, 0.015)
                cyl("Yard_GateLight", 0.065, 0.7, 0.76, "Accent_Orange", (x, half - 0.08), 12, 0.015)
        rotated(track(fence), k * pi / 2)
    # Bay (+,+): stacked containers
    container("CtrA", (1.75, 1.2, 0.04), 0.0, "Accent_Orange")
    container("CtrB", (1.75, 2.15, 0.04), 0.0, "Hull_White")
    container("CtrC", (1.75, 1.65, 0.9), 0.0, "Cargo_Teal")
    # Bay (-,+): crates and barrels
    rnd = random.Random(7)
    for i, (x, y, z, s) in enumerate(((-2.2, 1.1, 0.04, 0.42), (-1.72, 1.1, 0.04, 0.42), (-2.2, 1.58, 0.04, 0.42),
                                      (-1.95, 1.3, 0.46, 0.36), (-1.2, 2.3, 0.04, 0.34))):
        crate(f"Crate{i}", (x, y, z), s, rnd.uniform(-0.1, 0.1), "Crate_Tan" if i % 2 == 0 else "Hull_Grey")
    for i, (x, y) in enumerate(((-1.25, 1.2), (-1.0, 1.45), (-1.3, 1.55), (-2.3, 2.3), (-2.05, 2.4))):
        barrel(f"Barrel{i}", (x, y, 0.04), "Accent_Orange" if i % 3 else "Hull_White")
    # Bay (-,-): fuel tanks
    fuel_tank("TankA", (-1.75, -1.25, 0.04), 0.0, 1.9, 0.38)
    fuel_tank("TankB", (-1.75, -2.2, 0.04), 0.0, 1.9, 0.38)
    # Bay (+,-): rust container + crate pile + loader crane
    container("CtrD", (1.75, -2.15, 0.04), 0.0, "Cargo_Rust")
    for i, (x, y) in enumerate(((1.2, -1.2), (1.6, -1.25), (2.0, -1.2))):
        crate(f"CrateD{i}", (x, y, 0.04), 0.34, 0.05 * i, "Crate_Tan")
    cyl("Crane_Base", 0.22, 0.0, 0.25, "Frame_Graphite", (2.55, -1.2), 16, 0.03)
    cyl("Crane_Mast", 0.07, 0.25, 1.6, "Hazard_Yellow", (2.55, -1.2), 12, 0.01)
    box("Crane_Jib", (1.5, 0.1, 0.1), (2.0, -1.2, 1.6), "Hazard_Yellow", 0.02)
    box("Crane_Counter", (0.3, 0.22, 0.22), (2.75, -1.2, 1.6), "Frame_Graphite", 0.03)
    cyl("Crane_Cable", 0.008, 0.95, 1.55, "Frame_Graphite", (1.4, -1.2), 6)
    box("Crane_Hook", (0.1, 0.06, 0.08), (1.4, -1.2, 0.92), "Accent_Orange", 0.01)
    cyl("Crane_Cab", 0.12, 1.45, 1.75, "Hull_White", (2.55, -1.2), 12, 0.03)
    for k in range(4):
        a = k * pi / 2 + pi / 4
        lamp_post("Yard_Lamp", (cos(a) * 0.9, sin(a) * 0.9), 1.1)


HUMAN_SCALE = 2.0  # authored at half size, scaled to the in-game colonist height (~0.63)


def build_human():
    """Stylized astronaut colonist. Feet at z=0, faces +X, ~0.63 tall."""
    for s in (1, -1):
        y = s * 0.022
        lathe("Leg", [(0, 0.025), (0.017, 0.025), (0.019, 0.07), (0.021, 0.12), (0, 0.12)], "Hull_White", 12,
              (0.0, y, 0))
        box("Boot", (0.05, 0.03, 0.028), (0.006, y, 0.014), "Frame_Graphite", 0.008)
        box("BootToe", (0.012, 0.031, 0.012), (0.03, y, 0.008), "Accent_Orange", 0.004)
        ring("KneePad", 0.0, 0.025, 0.064, 0.074, "Accent_Orange", (0.002, y), 12)
    lathe("Torso", [(0, 0.105), (0.038, 0.105), (0.044, 0.13), (0.045, 0.19), (0.04, 0.215), (0.025, 0.225),
                    (0, 0.225)], "Hull_White", 16, (0, 0, 0))
    PARTS[-1].data.transform(Matrix.Diagonal((0.9, 1.1, 1.0, 1.0)))
    ring("Belt", 0.0, 0.05, 0.115, 0.13, "Frame_Graphite", segs=16)
    PARTS[-1].data.transform(Matrix.Diagonal((0.9, 1.1, 1.0, 1.0)))
    box("ChestPanel", (0.012, 0.04, 0.03), (0.04, 0.0, 0.165), "Frame_Graphite", 0.004)
    box("ChestLight", (0.004, 0.012, 0.008), (0.047, -0.008, 0.17), "Light_Cyan")
    box("ChestLight2", (0.004, 0.012, 0.008), (0.047, 0.01, 0.17), "Accent_Orange")
    box("Pack", (0.035, 0.07, 0.09), (-0.05, 0, 0.17), "Hull_Grey", 0.01)
    box("PackCap", (0.037, 0.072, 0.015), (-0.05, 0, 0.21), "Accent_Orange", 0.005)
    for s in (1, -1):
        y = s * 0.058
        arm = lathe("Arm", [(0, 0.0), (0.013, 0.004), (0.015, 0.05), (0.012, 0.085), (0, 0.09)], "Hull_White", 10)
        arm.data.transform(Matrix.Rotation(s * radians(12), 4, "X"))
        arm.data.transform(Matrix.Translation((0.0, y, 0.12)))
        ico("Shoulder", 0.02, (0.0, y * 0.95, 0.205), "Accent_Orange", 1, (1, 1, 0.9), 0.0, 0, flat=False)
        ico("Glove", 0.014, (0.0, y + s * 0.009, 0.118), "Frame_Graphite", 1, (1, 1, 1.1), 0.0, 0, flat=False)
    ring("Collar", 0.02, 0.034, 0.215, 0.232, "Frame_Graphite", segs=16)
    ico("Helmet", 0.05, (0.0, 0, 0.263), "Hull_White", 2, (1, 1, 1), 0.0, 0, flat=False)
    ico("Visor", 0.04, (0.026, 0, 0.266), "Visor_Gold", 2, (0.7, 1.05, 0.72), 0.0, 0, flat=False)
    cyl("HelmetLamp", 0.008, 0.0, 0.016, "Light_Cyan", (0, 0), 8)
    lampo = PARTS[-1]
    lampo.data.transform(Matrix.Rotation(radians(90), 4, "Y"))
    lampo.data.transform(Matrix.Translation((0.03, 0.035, 0.295)))
    cyl("Antenna", 0.002, 0.29, 0.335, "Frame_Graphite", (-0.02, -0.03), 6)
    ico("AntennaTip", 0.005, (-0.02, -0.03, 0.337), "Accent_Orange", 1, flat=False)
    for p in PARTS:
        p.data.transform(Matrix.Scale(HUMAN_SCALE, 4))


# ---------------------------------------------------------------------------------------
# Shared building parts
# ---------------------------------------------------------------------------------------
STOREY = 0.75  # module storey height (~1.2 colonists)
GROUND = 0.06  # top of the exterior foundations


def foundation(hx, hy):
    box("Foundation", (2 * hx - 0.1, 2 * hy - 0.1, 0.3), (0, 0, GROUND - 0.15), "Concrete", 0.03)


def pipe(name, p0, p1, r, mat="Steel", segs=12):
    p0, p1 = Vector(p0), Vector(p1)
    d = p1 - p0
    obj = lathe(name, [(0, 0), (r, 0), (r, d.length), (0, d.length)], mat, segs, sharp_angle=60)
    obj.data.transform(Matrix.Translation(p0) @ d.to_track_quat("Z", "Y").to_matrix().to_4x4())
    return obj


def vtank(prefix, loc, r, h, mat="Hull_White", bands=(0.3, 0.7)):
    x, y, z = loc
    k = min(0.3 * r, 0.25 * h)
    lathe(f"{prefix}_Tank", [(0, z), (r * 0.7, z), (r, z + k), (r, z + h - k), (r * 0.7, z + h), (0, z + h)], mat,
          24, (x, y, 0))
    for f in bands:
        ring(f"{prefix}_Band", r - 0.01, r + 0.02, z + h * f - 0.04, z + h * f + 0.04, "Accent_Orange", (x, y), 24)


def lattice_tower(prefix, loc, w0, w1, h, levels, mat="Frame_Graphite", t=0.05):
    x, y = loc
    signs = ((1, 1), (-1, 1), (-1, -1), (1, -1))

    def corner(i, f):
        w = (w0 + (w1 - w0) * f) / 2
        return (x + signs[i][0] * w, y + signs[i][1] * w, GROUND + h * f)

    for i in range(4):
        beam(f"{prefix}_Leg", corner(i, 0), corner(i, 1), t, t, mat)
    for lv in range(1, levels + 1):
        f0, f1 = (lv - 1) / levels, lv / levels
        for i in range(4):
            j = (i + 1) % 4
            beam(f"{prefix}_Ring", corner(i, f1), corner(j, f1), t * 0.8, t * 0.8, mat)
            beam(f"{prefix}_Brace", corner(i, f0), corner(j, f1), t * 0.6, t * 0.6, mat)


def fan(prefix, loc, r):
    x, y, z = loc
    ring(f"{prefix}_FanHousing", r - 0.04, r + 0.02, z, z + 0.14, "Frame_Graphite", (x, y), 24)
    cyl(f"{prefix}_FanHub", r * 0.22, z, z + 0.1, "Light_Cyan", (x, y), 12, 0.01)
    for i in range(5):
        blade = box(f"{prefix}_FanBlade", (r * 0.7, r * 0.24, 0.015), (0, 0, 0), "Hull_Grey", 0.004)
        blade.data.transform(Matrix.Rotation(radians(20), 4, "X"))
        blade.data.transform(Matrix.Translation((x, y, z + 0.07)) @ Matrix.Rotation(i * TAU / 5, 4, "Z")
                             @ Matrix.Translation((r * 0.45, 0, 0)))


def drone(prefix, loc, rot_z=0.0):
    def build():
        box(f"{prefix}_Body", (0.26, 0.18, 0.08), (0, 0, 0.1), "Hull_White", 0.02)
        box(f"{prefix}_Stripe", (0.27, 0.19, 0.02), (0, 0, 0.11), "Accent_Orange", 0.005)
        box(f"{prefix}_Eye", (0.02, 0.08, 0.025), (0.135, 0, 0.1), "Light_Cyan", 0.005)
        for sx in (1, -1):
            for sy in (1, -1):
                beam(f"{prefix}_Arm", (0, 0, 0.11), (sx * 0.17, sy * 0.15, 0.13), 0.025, 0.02, "Frame_Graphite")
                ring(f"{prefix}_Rotor", 0.065, 0.08, 0.12, 0.15, "Frame_Graphite", (sx * 0.17, sy * 0.15), 16)
                box(f"{prefix}_Prop", (0.13, 0.02, 0.006), (sx * 0.17, sy * 0.15, 0.14), "Hull_Grey")
        for sy in (1, -1):
            box(f"{prefix}_Skid", (0.24, 0.02, 0.02), (0, sy * 0.07, 0.03), "Frame_Graphite", 0.004)
            for sx in (1, -1):
                box(f"{prefix}_SkidLeg", (0.015, 0.015, 0.05), (sx * 0.07, sy * 0.07, 0.06), "Frame_Graphite")

    for p in track(build):
        xform(p, loc, rot_z)


def window_row(prefix, face_x, y0, y1, z0, z1, count, depth=0.02):
    """Windows on a face normal to +X at `face_x` (rotate the result for other faces)."""
    w = (y1 - y0) / count
    for i in range(count):
        yc = y0 + (i + 0.5) * w
        box(f"{prefix}_Window", (depth, w * 0.7, z1 - z0), (face_x, yc, (z0 + z1) / 2), "Window_Dark", 0.01)
        box(f"{prefix}_WindowFrame", (depth * 0.5, w * 0.7 + 0.05, z1 - z0 + 0.05), (face_x - depth * 0.3, yc,
                                                                                   (z0 + z1) / 2), "Frame_Graphite",
            0.01)


def door(prefix, face_x, y, w=0.36, h=0.62, z0=0.0):
    box(f"{prefix}_DoorFrame", (0.04, w + 0.08, h + 0.05), (face_x, y, z0 + (h + 0.05) / 2), "Frame_Graphite", 0.01)
    box(f"{prefix}_Door", (0.05, w, h), (face_x + 0.005, y, z0 + h / 2), "Hull_Grey", 0.01)
    box(f"{prefix}_DoorLight", (0.06, w * 0.6, 0.03), (face_x + 0.005, y, z0 + h + 0.07), "Light_Cyan", 0.005)


# ---------------------------------------------------------------------------------------
# Exterior buildings
# ---------------------------------------------------------------------------------------
def build_solar_array():
    foundation(2, 2)
    for sx in (1, -1):
        for sy in (1, -1):
            box("Solar_Pad", (0.4, 0.4, 0.08), (sx, sy, GROUND + 0.04), "Concrete", 0.02)
            solar_panel("Solar", (sx, sy, GROUND), pi / 2, 1.7, 1.1)
    box("Solar_Inverter", (0.6, 0.36, 0.5), (0, -1.65, GROUND + 0.25), "Hull_White", 0.03)
    box("Solar_InverterBand", (0.62, 0.38, 0.06), (0, -1.65, GROUND + 0.38), "Accent_Orange", 0.01)
    box("Solar_InverterLed", (0.16, 0.02, 0.05), (0, -1.84, GROUND + 0.28), "Light_Cyan")
    box("Solar_ConduitX", (2.4, 0.08, 0.05), (0, 0, GROUND + 0.025), "Frame_Graphite", 0.01)
    box("Solar_ConduitY", (0.08, 1.6, 0.05), (0, -0.8, GROUND + 0.025), "Frame_Graphite", 0.01)


def build_battery():
    foundation(1, 1)
    box("Bat_Cabinet", (1.45, 1.05, 0.18), (0, 0, GROUND + 0.09), "Frame_Graphite", 0.02)
    for x in (-0.35, 0.35):
        for y in (-0.24, 0.24):
            z0 = GROUND + 0.18
            box("Bat_Cell", (0.62, 0.4, 0.8), (x, y, z0 + 0.4), "Hull_White", 0.04)
            box("Bat_CellBand", (0.64, 0.42, 0.07), (x, y, z0 + 0.62), "Accent_Orange", 0.01)
            s = 1 if y > 0 else -1
            box("Bat_Gauge", (0.05, 0.02, 0.4), (x + 0.18, y + s * 0.2, z0 + 0.36), "Light_Cyan")
            for f in range(4):
                box("Bat_Fin", (0.5, 0.025, 0.08), (x, y - 0.12 + f * 0.08, z0 + 0.84), "Frame_Graphite", 0.005)
    box("Bat_BusBar", (1.3, 0.08, 0.06), (0, 0, GROUND + 1.06), "Accent_Orange", 0.01)


def build_water_tank():
    foundation(1, 1)
    zc, r = 1.2, 0.62
    for k in range(4):
        a = k * pi / 2 + pi / 4
        c, s = cos(a), sin(a)
        beam("WTank_Leg", (c * 0.66, s * 0.66, GROUND), (c * 0.4, s * 0.4, zc - 0.25), 0.07, 0.07, "Frame_Graphite")
        cyl("WTank_Foot", 0.09, GROUND, GROUND + 0.06, "Concrete", (c * 0.66, s * 0.66), 12, 0.01)
    ico("WTank_Sphere", r, (0, 0, zc), "Hull_White", 3, flat=False)
    ring("WTank_Band", r - 0.03, r + 0.02, zc - 0.06, zc + 0.06, "Accent_Orange", segs=40)
    ring("WTank_Gauge", r - 0.05, r + 0.012, zc + 0.12, zc + 0.3, "Light_Cyan", segs=40, arc=(-0.35, 0.35))
    cyl("WTank_Pipe", 0.06, GROUND, zc - r + 0.05, "Steel", (0, 0), 12)
    cyl("WTank_Hatch", 0.14, zc + r - 0.04, zc + r + 0.04, "Frame_Graphite", (0, 0), 16, 0.01)
    box("WTank_Valve", (0.24, 0.24, 0.2), (0, 0, GROUND + 0.1), "Frame_Graphite", 0.03)


def build_water_extractor():
    foundation(2, 2)
    # Drill rig
    box("WEx_Deck", (1.7, 1.7, 0.12), (-0.9, 0.9, GROUND + 0.06), "Hull_Grey", 0.02)
    lattice_tower("WEx_Derrick", (-0.9, 0.9), 1.3, 0.35, 2.6, 4, "Hazard_Yellow")
    box("WEx_Crown", (0.45, 0.45, 0.16), (-0.9, 0.9, GROUND + 2.68), "Frame_Graphite", 0.02)
    cyl("WEx_DrillPipe", 0.07, GROUND + 0.1, GROUND + 2.6, "Steel", (-0.9, 0.9), 12)
    cyl("WEx_DrillHead", 0.26, GROUND + 0.1, GROUND + 0.5, "Frame_Graphite", (-0.9, 0.9), 20, 0.03)
    # Pump house
    box("WEx_Pump", (1.4, 0.9, 0.75), (-0.75, -1.05, GROUND + 0.375), "Hull_White", 0.05)
    box("WEx_PumpStripe", (1.42, 0.92, 0.08), (-0.75, -1.05, GROUND + 0.55), "Accent_Orange", 0.01)
    box("WEx_PumpWindow", (0.5, 0.02, 0.2), (-0.75, -1.51, GROUND + 0.42), "Window_Dark", 0.01)
    fan("WEx_Vent", (-0.75, -1.05, GROUND + 0.75), 0.28)
    # Storage tank
    vtank("WEx", (0.95, -0.85, GROUND), 0.6, 1.5)
    ring("WEx_Gauge", 0.595, 0.615, GROUND + 0.6, GROUND + 0.9, "Light_Cyan", (0.95, -0.85), 24, arc=(2.6, 3.4))
    vtank("WEx2", (0.95, 0.95, GROUND), 0.45, 1.1)
    # Pipes
    pipe("WEx_Pipe", (-0.9, 0.55, GROUND + 0.3), (-0.9, -0.6, GROUND + 0.3), 0.06)
    pipe("WEx_Pipe", (-0.05, -1.05, GROUND + 0.45), (0.38, -0.95, GROUND + 0.45), 0.06)
    pipe("WEx_Pipe", (0.95, -0.25, GROUND + 0.9), (0.95, 0.5, GROUND + 0.9), 0.05)


def build_oxygen_generator():
    foundation(2, 2)
    hy0 = -0.55
    box("O2Gen_Housing", (2.6, 1.6, 0.95), (0, hy0, GROUND + 0.475), "Hull_White", 0.08)
    box("O2Gen_Stripe", (2.62, 1.62, 0.08), (0, hy0, GROUND + 0.72), "Accent_Orange", 0.01)
    for i in range(7):
        box("O2Gen_Grille", (0.02, 1.2, 0.04), (1.31, hy0, GROUND + 0.2 + i * 0.07), "Frame_Graphite")
    for x in (-0.75, 0.0, 0.75):
        fan("O2Gen", (x, hy0, GROUND + 0.95), 0.32)
    for x in (-1.1, 1.1):
        cyl("O2Gen_Stack", 0.11, GROUND + 0.9, GROUND + 1.9, "Hull_Grey", (x, hy0 - 0.6), 12)
        cyl("O2Gen_StackTip", 0.13, GROUND + 1.85, GROUND + 1.95, "Accent_Orange", (x, hy0 - 0.6), 12, 0.01)
    for x in (-0.8, 0.8):
        box("O2Gen_Saddle", (0.6, 0.5, 0.25), (x, 1.2, GROUND + 0.125), "Frame_Graphite", 0.03)
        ico("O2Gen_Sphere", 0.48, (x, 1.2, GROUND + 0.7), "Hull_White", 3, flat=False)
        ring("O2Gen_SphereBand", 0.45, 0.5, GROUND + 0.66, GROUND + 0.74, "Light_Cyan", (x, 1.2), 32)
        pipe("O2Gen_Pipe", (x, 0.25, GROUND + 0.5), (x, 0.78, GROUND + 0.6), 0.06)
    box("O2Gen_Panel", (0.03, 0.4, 0.3), (-1.31, hy0, GROUND + 0.45), "Light_Cyan", 0.01)


def build_oxygen_tank():
    foundation(1, 1)
    box("O2Tank_Frame", (1.5, 0.8, 0.12), (0, 0, GROUND + 0.06), "Frame_Graphite", 0.02)
    for x in (-0.38, 0.38):
        vtank("O2Tank", (x, 0, GROUND + 0.12), 0.32, 1.35, bands=(0.25,))
        ring("O2Tank_Cyan", 0.31, 0.335, GROUND + 0.95, GROUND + 1.02, "Light_Cyan", (x, 0), 24)
    beam("O2Tank_Top", (-0.6, 0, GROUND + 1.3), (0.6, 0, GROUND + 1.3), 0.06, 0.06, "Frame_Graphite")
    pipe("O2Tank_Manifold", (-0.38, -0.4, GROUND + 0.35), (0.38, -0.4, GROUND + 0.35), 0.05)
    box("O2Tank_Valve", (0.18, 0.12, 0.18), (0, -0.42, GROUND + 0.35), "Accent_Orange", 0.02)


def build_landing_pad():
    polygon_prism("Pad_Base", 2.95 / cos(pi / 8), 8, -0.25, 0.08, "Concrete", angle0=pi / 8)
    polygon_prism("Pad_Top", 2.6 / cos(pi / 8), 8, 0.08, 0.1, "Hull_Grey", angle0=pi / 8)
    cyl("Pad_Scorch", 1.3, 0.1, 0.106, "Floor_Deck", (0, 0), 48)
    ring("Pad_Circle", 1.55, 1.72, 0.1, 0.112, "Hazard_Yellow", segs=64)
    for a in (pi / 4, -pi / 4):
        box("Pad_Cross", (1.8, 0.16, 0.012), (0, 0, 0.112), "Accent_Orange", rot_z=a)
    for k in range(8):
        a = k * pi / 4 + pi / 8
        rr = 2.75 / cos(pi / 8)
        cyl("Pad_Light", 0.07, 0.08, 0.17, "Light_Cyan", (cos(a) * rr, sin(a) * rr), 12, 0.02)
    for k in range(4):
        a = k * pi / 2
        for i in range(5):
            box("Pad_Hazard", (0.12, 0.3, 0.012), (cos(a) * 2.35, sin(a) * 2.35, 0.106), "Hazard_Yellow",
                rot_z=a + 0.0)
            PARTS[-1].data.transform(Matrix.Translation((-sin(a) * (i - 2) * 0.3, cos(a) * (i - 2) * 0.3, 0)))
    box("Pad_Pump", (0.4, 0.3, 0.6), (1.75, -1.75, 0.4), "Hull_White", 0.04, rot_z=pi / 4)
    box("Pad_PumpBand", (0.41, 0.31, 0.06), (1.75, -1.75, 0.56), "Accent_Orange", 0.01, rot_z=pi / 4)
    box("Pad_PumpScreen", (0.16, 0.02, 0.12), (1.64, -1.86, 0.5), "Light_Cyan", rot_z=pi / 4)
    pipe("Pad_Hose", (1.6, -1.6, 0.3), (1.3, -1.3, 0.12), 0.03, "Frame_Graphite")


def build_rocket():
    """Colony lander parked on the landing pad (entity, no grid properties)."""
    r = 0.6
    lathe("Rocket_Body", [(0, 0.9), (0.45, 0.9), (r, 1.05), (r, 3.7), (0.56, 3.95), (0.4, 4.45), (0.22, 4.8),
                          (0.07, 5.0), (0, 5.02)], "Hull_White", 40)
    for z in (1.3, 3.3):
        ring("Rocket_Band", r - 0.01, r + 0.015, z, z + 0.14, "Accent_Orange", segs=40)
    for k in range(4):
        a0 = k * pi / 2 + 0.25
        ring("Rocket_Window", r - 0.01, r + 0.012, 3.5, 3.62, "Window_Dark", segs=10, arc=(a0, a0 + 1.0))
    lathe("Rocket_Bell", [(0.14, 0.9), (0.22, 0.9), (0.46, 0.35), (0.42, 0.35), (0.18, 0.86)], "Frame_Graphite", 24,
          closed=True)
    cyl("Rocket_Mount", 0.32, 0.86, 0.96, "Frame_Graphite", (0, 0), 24)
    for k in range(4):
        a = k * pi / 2 + pi / 4
        c, s = cos(a), sin(a)
        beam("Rocket_Leg", (c * 0.5, s * 0.5, 1.7), (c * 1.55, s * 1.55, 0.1), 0.1, 0.1, "Frame_Graphite")
        beam("Rocket_Strut", (c * 0.55, s * 0.55, 1.05), (c * 1.1, s * 1.1, 0.62), 0.06, 0.06, "Hull_Grey")
        cyl("Rocket_Foot", 0.2, 0.0, 0.1, "Frame_Graphite", (c * 1.55, s * 1.55), 16, 0.02)
        a = k * pi / 2
        box("Rocket_Fin", (0.5, 0.06, 0.9), (cos(a) * (r + 0.2), sin(a) * (r + 0.2), 1.6), "Accent_Orange", 0.02,
            rot_z=a)
    box("Rocket_Hatch", (0.04, 0.34, 0.52), (r + 0.005, 0, 2.3), "Frame_Graphite", 0.02)
    box("Rocket_HatchLight", (0.05, 0.2, 0.04), (r + 0.01, 0, 2.62), "Light_Cyan", 0.01)


def build_drone_hub():
    polygon_prism("Hub_Base", 2.95 / cos(pi / 8), 8, -0.25, 0.08, "Concrete", angle0=pi / 8)
    lathe("Hub_Tower", [(0, 0.08), (0.8, 0.08), (0.8, 0.5), (0.65, 0.6), (0.65, 1.2), (0.5, 1.32), (0, 1.32)],
          "Hull_White", 32)
    ring("Hub_Band", 0.79, 0.82, 0.38, 0.48, "Accent_Orange", segs=32)
    for k in range(6):
        a0 = k * TAU / 6 + 0.15
        ring("Hub_Window", 0.64, 0.66, 0.78, 0.98, "Window_Dark", segs=6, arc=(a0, a0 + 0.7))
    dish("Hub_Dish", (0, 0, 1.32), pi / 3, 0.45)
    cyl("Hub_Mast", 0.03, 1.32, 2.5, "Frame_Graphite", (0.3, -0.3), 8)
    cyl("Hub_Beacon", 0.06, 2.5, 2.6, "Accent_Orange", (0.3, -0.3), 12, 0.02)
    door("Hub", 0.81, 0.0, 0.4, 0.62, 0.08)
    for i, (sx, sy) in enumerate(((1, 1), (-1, 1), (-1, -1), (1, -1))):
        x, y = sx * 1.65, sy * 1.65
        cyl("Hub_Pad", 0.45, 0.08, 0.12, "Hull_Grey", (x, y), 32, 0.01)
        ring("Hub_PadRing", 0.34, 0.4, 0.12, 0.125, "Hazard_Yellow", (x, y), 32)
        cyl("Hub_PadCore", 0.08, 0.12, 0.13, "Light_Cyan", (x, y), 12)
        box("Hub_Cable", (0.06, 0.06, 0.03), (0, 0, 0), "Frame_Graphite")
        beam("Hub_Cable", (sx * 0.8, sy * 0.8, 0.1), (x - sx * 0.35, y - sy * 0.35, 0.1), 0.05, 0.03,
             "Frame_Graphite")
        if i < 3:
            drone(f"Hub_Drone{i}", (x, y, 0.12), i * 1.3)


def build_fuel_refinery():
    foundation(2, 2)
    for (x, y, r, h) in ((-1.1, 1.0, 0.34, 2.6), (-0.3, 1.2, 0.28, 2.0)):
        vtank("Ref_Column", (x, y, GROUND), r, h, bands=(0.2, 0.5, 0.8))
        ring("Ref_Platform", r + 0.02, r + 0.18, GROUND + h * 0.62, GROUND + h * 0.62 + 0.04, "Frame_Graphite",
             (x, y), 24)
        cyl("Ref_Cap", r * 0.4, GROUND + h, GROUND + h + 0.12, "Frame_Graphite", (x, y), 12, 0.02)
    fuel_tank("Ref_Tank", (0.55, -0.9, GROUND), 0.0, 1.9, 0.4)
    for x in (-0.6, 0.6):
        box("Ref_RackPost", (0.07, 0.07, 1.25), (x * 1.2 + 0.2, 0.2, GROUND + 0.625), "Frame_Graphite", 0.01)
    beam("Ref_Rack", (-0.52, 0.2, GROUND + 1.22), (0.92, 0.2, GROUND + 1.22), 0.07, 0.07, "Frame_Graphite")
    pipe("Ref_Pipe", (-1.1, 0.66, GROUND + 1.25), (0.92, 0.66, GROUND + 1.25), 0.06)
    pipe("Ref_Pipe2", (-0.3, 0.92, GROUND + 1.1), (-0.3, 0.25, GROUND + 1.1), 0.05)
    pipe("Ref_Pipe3", (0.9, 0.66, GROUND + 1.25), (0.9, -0.6, GROUND + 0.75), 0.06)
    cyl("Ref_Flare", 0.06, GROUND, GROUND + 3.0, "Steel", (1.45, 1.35), 10)
    cyl("Ref_FlareTip", 0.1, GROUND + 2.95, GROUND + 3.1, "Accent_Orange", (1.45, 1.35), 12, 0.02)
    box("Ref_Control", (0.6, 0.45, 0.6), (1.35, 0.35, GROUND + 0.3), "Hull_White", 0.04)
    box("Ref_ControlScreen", (0.02, 0.3, 0.16), (1.66, 0.35, GROUND + 0.4), "Light_Cyan")


def build_stirling_generator():
    foundation(2, 2)
    lathe("Stirling_Core", [(0, GROUND), (0.95, GROUND), (0.95, GROUND + 0.3), (0.72, GROUND + 0.36),
                            (0.72, GROUND + 1.7), (0.52, GROUND + 1.95), (0, GROUND + 2.0)], "Hull_White", 40)
    ring("Stirling_Band", 0.94, 0.97, GROUND + 0.22, GROUND + 0.3, "Accent_Orange", segs=40)
    ring("Stirling_Glow", 0.5, 0.56, GROUND + 1.94, GROUND + 1.99, "Light_Cyan", segs=32)
    for k in range(20):
        a = k * TAU / 20
        box("Stirling_Fin", (0.36, 0.03, 1.2), (cos(a) * 0.86, sin(a) * 0.86, GROUND + 1.0), "Frame_Graphite", 0.008,
            rot_z=a)
    for sx in (1, -1):
        for sy in (1, -1):
            cyl("Stirling_Bollard", 0.06, GROUND, GROUND + 0.35, "Hazard_Yellow", (sx * 1.6, sy * 1.6), 10, 0.02)
    box("Stirling_Junction", (0.35, 0.3, 0.4), (1.55, 0, GROUND + 0.2), "Frame_Graphite", 0.03)
    box("Stirling_Conduit", (0.6, 0.1, 0.06), (1.15, 0, GROUND + 0.03), "Frame_Graphite", 0.01)


def build_sensor_tower():
    foundation(1, 1)
    lattice_tower("Sensor", (0, 0), 1.1, 0.26, 3.0, 5)
    box("Sensor_Platform", (0.55, 0.55, 0.06), (0, 0, GROUND + 3.03), "Frame_Graphite", 0.01)
    dish("Sensor_Dish", (0, 0, GROUND + 3.06), pi / 4, 0.4)
    cyl("Sensor_Beacon", 0.05, GROUND + 3.06, GROUND + 3.75, "Frame_Graphite", (-0.2, -0.2), 8)
    cyl("Sensor_BeaconTip", 0.08, GROUND + 3.75, GROUND + 3.85, "Accent_Orange", (-0.2, -0.2), 12, 0.02)
    box("Sensor_Cabinet", (0.6, 0.4, 0.45), (0.5, -0.55, GROUND + 0.225), "Hull_White", 0.03)
    box("Sensor_CabinetLed", (0.15, 0.02, 0.05), (0.5, -0.76, GROUND + 0.32), "Light_Cyan")


def build_concrete_extractor():
    foundation(2, 2)
    for i, (x, y, r, s) in enumerate(((-1.15, 1.1, 0.6, 0.5), (-0.55, 1.45, 0.35, 0.6), (-1.45, 0.45, 0.3, 0.6))):
        ico("Mine_Rocks", r, (x, y, GROUND), "Cargo_Rust" if i % 2 else "Concrete", 1, (1, 1, s), 0.25, i)
    box("Mine_Crusher", (1.2, 1.0, 0.95), (0.9, 0.95, GROUND + 0.475), "Hull_White", 0.05)
    box("Mine_CrusherStripe", (1.22, 1.02, 0.08), (0.9, 0.95, GROUND + 0.7), "Accent_Orange", 0.01)
    box("Mine_CrusherWindow", (0.02, 0.45, 0.2), (0.29, 0.95, GROUND + 0.55), "Window_Dark", 0.01)
    beam("Mine_Belt", (-0.75, 0.95, GROUND + 0.25), (0.35, 0.95, GROUND + 0.85), 0.3, 0.06, "Frame_Graphite")
    beam("Mine_Belt2", (0.9, 0.42, GROUND + 0.9), (0.9, -0.55, GROUND + 1.45), 0.3, 0.06, "Frame_Graphite")
    for x in (-0.4, 0.0):
        box("Mine_BeltLeg", (0.05, 0.25, 0.5), (x, 0.95, GROUND + 0.25), "Hazard_Yellow")
    lathe("Mine_Hopper", [(0.1, GROUND + 0.55), (0.62, GROUND + 1.3), (0.62, GROUND + 1.45), (0.55, GROUND + 1.45),
                          (0.05, GROUND + 0.7)], "Hazard_Yellow", 4, (0.9, -0.9, 0), closed=True, angle0=pi / 4,
          sharp_angle=20)
    ico("Mine_HopperLoad", 0.42, (0.9, -0.9, GROUND + 1.3), "Cargo_Rust", 1, (1, 1, 0.45), 0.2, 5)
    cyl("Mine_Chute", 0.1, GROUND + 0.3, GROUND + 0.6, "Frame_Graphite", (0.9, -0.9), 8)
    for sx in (1, -1):
        for sy in (1, -1):
            beam("Mine_HopperLeg", (0.9 + sx * 0.38, -0.9 + sy * 0.38, GROUND),
                 (0.9 + sx * 0.32, -0.9 + sy * 0.32, GROUND + 0.95), 0.06, 0.06, "Frame_Graphite")
    for i in range(3):
        for j in range(2):
            box("Mine_Block", (0.3, 0.3, 0.2), (-1.25 + i * 0.32, -1.15 + j * 0.32, GROUND + 0.1), "Concrete", 0.02)
    box("Mine_Block", (0.3, 0.3, 0.2), (-0.93, -0.99, GROUND + 0.3), "Concrete", 0.02)


# ---------------------------------------------------------------------------------------
# Dome modules (placed on the dome slots, base at z=0, door toward +X)
# ---------------------------------------------------------------------------------------
def module_base(hx, hy):
    box("Module_Base", (2 * hx - 0.08, 2 * hy - 0.08, 0.06), (0, 0, 0.03), "Hull_Grey", 0.015)


def side_windows(prefix, cx, half_y, t0, t1, z0, z1, count, sides=(1, -1)):
    """Window rows on the +Y/-Y faces of a block centered at x=cx."""
    for s in sides:
        for o in track(lambda: window_row(prefix, 0.0, t0, t1, z0, z1, count)):
            o.data.transform(Matrix.Translation((cx, s * (half_y + 0.01), 0)) @ Matrix.Rotation(s * pi / 2, 4, "Z"))


def build_living_quarters():
    module_base(2, 2)
    z0, h = 0.06, 2 * STOREY
    box("LQ_Block", (3.0, 2.8, h), (-0.2, 0, z0 + h / 2), "Hull_White", 0.06)
    box("LQ_Slab", (3.08, 2.88, 0.06), (-0.2, 0, z0 + STOREY), "Frame_Graphite", 0.01)
    box("LQ_Roof", (3.1, 2.9, 0.08), (-0.2, 0, z0 + h + 0.02), "Frame_Graphite", 0.02)
    box("LQ_Stripe", (3.02, 2.82, 0.07), (-0.2, 0, z0 + h - 0.12), "Accent_Orange", 0.01)
    for st in range(2):
        zb = z0 + st * STOREY
        side_windows("LQ", -0.2, 1.4, -1.25, 1.25, zb + 0.3, zb + 0.6, 4)
        if st == 0:
            window_row("LQ", 1.31, -1.25, -0.4, zb + 0.3, zb + 0.6, 2)
            window_row("LQ", 1.31, 0.4, 1.25, zb + 0.3, zb + 0.6, 2)
        else:
            window_row("LQ", 1.31, -1.25, 1.25, zb + 0.3, zb + 0.6, 4)
    door("LQ", 1.31, 0.0, 0.42, 0.62, z0)
    box("LQ_Canopy", (0.5, 0.9, 0.04), (1.5, 0, z0 + 0.74), "Accent_Orange", 0.01)
    box("LQ_Balcony", (2.0, 0.42, 0.05), (-0.3, -1.6, z0 + STOREY), "Frame_Graphite", 0.01)
    box("LQ_Rail", (2.0, 0.03, 0.03), (-0.3, -1.8, z0 + STOREY + 0.3), "Hull_White")
    for i in range(6):
        box("LQ_RailPost", (0.03, 0.03, 0.3), (-1.25 + i * 0.38, -1.8, z0 + STOREY + 0.15), "Hull_White")
    zr = z0 + h + 0.06
    for p in planter_box("LQ_RoofGarden", (-0.9, 0.6), (1.0, 0.5), 0.15, crops=4, seed=3):
        p.data.transform(Matrix.Translation((0, 0, zr)))
    box("LQ_AC", (0.5, 0.4, 0.25), (0.5, -0.6, zr + 0.125), "Hull_Grey", 0.03)
    fan("LQ_AC", (0.5, -0.6, zr + 0.25), 0.15)


def build_residence():
    module_base(1, 1)
    lathe("Res_Pod", [(0, 0.06), (0.7, 0.06), (0.76, 0.12), (0.76, 0.72), (0.62, 0.95), (0.32, 1.07), (0, 1.1)],
          "Hull_White", 32)
    ring("Res_Stripe", 0.75, 0.775, 0.66, 0.72, "Accent_Orange", segs=32)
    for k in range(3):
        a0 = pi / 2 + k * pi / 1.5 * 0.9 - 0.2
        ring("Res_Window", 0.75, 0.77, 0.36, 0.56, "Window_Dark", segs=6, arc=(a0, a0 + 0.8))
    door("Res", 0.76, 0.0, 0.34, 0.6, 0.06)
    cyl("Res_Skylight", 0.22, 1.06, 1.12, "Window_Dark", (0, 0), 16, 0.02)
    planter_round("Res_Planter", (0.6, -0.65), 0.2, 0.16)
    shrub("Res_Shrub", (0.6, -0.65, 0.2), 0.12, 2)


def build_farm():
    module_base(2, 2)
    for i, y in enumerate((-1.15, 0.0, 1.15)):
        planter_box("Farm_Bed", (-0.15, y), (3.0, 0.55), 0.24, crops=14, seed=20 + i)
        for x in (-1.5, 1.2):
            box("Farm_LightPost", (0.04, 0.04, 0.8), (x, y + 0.3, 0.46), "Frame_Graphite")
        box("Farm_GrowLight", (2.8, 0.08, 0.03), (-0.15, y, 0.86), "Light_Cyan", 0.005)
        box("Farm_LightBar", (2.8, 0.06, 0.04), (-0.15, y + 0.3, 0.86), "Frame_Graphite", 0.005)
        pipe("Farm_Drip", (-1.6, y - 0.22, 0.3), (1.3, y - 0.22, 0.3), 0.025)
    vtank("Farm", (1.55, 1.5, 0.06), 0.3, 0.9, bands=(0.5,))
    ring("Farm_TankGauge", 0.29, 0.31, 0.3, 0.6, "Light_Cyan", (1.55, 1.5), 24, arc=(3.5, 4.3))
    box("Farm_Pump", (0.35, 0.3, 0.3), (1.55, 0.9, 0.21), "Hull_White", 0.03)
    door("Farm", 1.9, -1.55, 0.3, 0.5, 0.06)


def build_research_lab():
    module_base(2, 1)
    box("Lab_Block", (2.9, 1.5, 1.0), (-0.4, 0, 0.56), "Hull_White", 0.06)
    box("Lab_Stripe", (2.92, 1.52, 0.07), (-0.4, 0, 0.92), "Accent_Orange", 0.01)
    side_windows("Lab", -0.4, 0.75, -1.25, 0.0, 0.4, 0.7, 3, sides=(-1,))
    side_windows("Lab", -0.4, 0.75, -1.25, 1.25, 0.4, 0.7, 4, sides=(1,))
    box("Lab_Annex", (1.0, 1.2, 0.7), (1.45, 0, 0.41), "Hull_White", 0.05)
    lathe("Lab_Observatory", [(0, 1.06)] + [(0.55 * cos(a), 1.06 + 0.55 * sin(a)) for a in
                                             [i * pi / 16 for i in range(9)]], "Hull_White", 32, (-1.2, 0, 0))
    box("Lab_Slit", (0.12, 0.62, 0.5), (0, 0, 0), "Frame_Graphite", 0.01)
    PARTS[-1].data.transform(Matrix.Translation((-1.2, 0, 1.06)) @ Matrix.Rotation(radians(30), 4, "Y")
                             @ Matrix.Translation((0, 0, 0.28)))
    ring("Lab_ObsRing", 0.55, 0.6, 1.06, 1.12, "Accent_Orange", (-1.2, 0), 32)
    cyl("Lab_Antenna", 0.025, 1.06, 1.8, "Frame_Graphite", (0.6, 0.5), 8)
    cyl("Lab_AntennaTip", 0.045, 1.8, 1.86, "Light_Cyan", (0.6, 0.5), 10, 0.01)
    box("Lab_Equipment", (0.5, 0.4, 0.2), (0.3, 0.3, 1.16), "Hull_Grey", 0.03)
    box("Lab_EquipLed", (0.2, 0.02, 0.05), (0.3, 0.09, 1.18), "Light_Cyan")
    for o in track(lambda: door("Lab", 0.0, 0.0, 0.42, 0.6, 0.06)):
        o.data.transform(Matrix.Translation((0.3, -0.76, 0)) @ Matrix.Rotation(-pi / 2, 4, "Z"))


def build_diner():
    module_base(1, 1)
    box("Diner_Kiosk", (0.95, 1.6, 0.85), (-0.45, 0, 0.485), "Hull_White", 0.05)
    box("Diner_Counter", (0.04, 1.2, 0.3), (0.035, 0, 0.62), "Window_Dark", 0.01)
    box("Diner_CounterTop", (0.18, 1.3, 0.04), (0.08, 0, 0.42), "Frame_Graphite", 0.01)
    awning = box("Diner_Awning", (0.55, 1.7, 0.04), (0, 0, 0), "Accent_Orange", 0.01)
    awning.data.transform(Matrix.Translation((0.2, 0, 0.92)) @ Matrix.Rotation(radians(-15), 4, "Y"))
    box("Diner_Sign", (0.04, 0.8, 0.18), (-0.45, 0, 1.0), "Light_Cyan", 0.02)
    for y in (-0.45, 0.45):
        cyl("Diner_TablePost", 0.025, 0.06, 0.3, "Frame_Graphite", (0.6, y), 8)
        cyl("Diner_Table", 0.2, 0.3, 0.33, "Hull_White", (0.6, y), 16, 0.01)
        for dx in (-0.28, 0.28):
            cyl("Diner_Stool", 0.08, 0.06, 0.2, "Accent_Orange", (0.6 + dx, y), 12, 0.02)


def build_infirmary():
    module_base(1, 1)
    box("Med_Block", (1.55, 1.55, 0.95), (-0.1, 0, 0.535), "Hull_White", 0.06)
    box("Med_Stripe", (1.57, 1.57, 0.07), (-0.1, 0, 0.8), "Accent_Orange", 0.01)
    box("Med_CrossA", (0.8, 0.22, 0.05), (-0.1, 0, 1.03), "Accent_Orange", 0.01)
    box("Med_CrossB", (0.22, 0.8, 0.05), (-0.1, 0, 1.03), "Accent_Orange", 0.01)
    side_windows("Med", -0.1, 0.775, -0.55, 0.55, 0.38, 0.62, 2)
    door("Med", 0.68, 0.0, 0.42, 0.62, 0.06)
    box("Med_Sign", (0.04, 0.5, 0.12), (0.69, 0, 0.88), "Light_Cyan", 0.02)


def build_garden():
    box("Garden_Border", (1.94, 1.94, 0.06), (0, 0, 0.03), "Concrete", 0.015)
    box("Garden_Lawn", (1.8, 1.8, 0.04), (0, 0, 0.07), "Grass", 0.01)
    box("Garden_PathX", (1.84, 0.3, 0.012), (0, 0, 0.096), "Concrete")
    box("Garden_PathY", (0.3, 1.84, 0.01), (0, 0, 0.095), "Concrete")
    ring("Garden_Basin", 0.24, 0.32, 0.09, 0.2, "Hull_White", segs=32)
    cyl("Garden_Water", 0.25, 0.09, 0.17, "Water", (0, 0), 32)
    cyl("Garden_Spout", 0.04, 0.17, 0.32, "Steel", (0, 0), 10)
    tree("Garden_Tree", (-0.55, 0.55, 0.09), 1.0, seed=7)
    tree("Garden_Tree2", (0.6, -0.6, 0.09), 0.8, seed=9)
    for i, (x, y) in enumerate(((0.55, 0.6), (-0.6, -0.55), (0.75, 0.35), (-0.35, -0.75))):
        shrub("Garden_Shrub", (x, y, 0.13), 0.13, i)
    bench("Garden_Bench", (0.55, 0.3), 0.0, 0.45)
    bench("Garden_Bench2", (-0.55, -0.3), 0.0, 0.45)


def build_grocery():
    module_base(1, 1)
    box("Shop_Block", (1.2, 1.6, 0.85), (-0.35, 0, 0.485), "Hull_White", 0.05)
    box("Shop_Window", (0.03, 1.0, 0.42), (0.26, 0.15, 0.5), "Window_Dark", 0.01)
    door("Shop", 0.26, -0.55, 0.3, 0.58, 0.06)
    for i in range(6):
        stripe = box("Shop_Awning", (0.5, 0.28, 0.03), (0, 0, 0), "Accent_Orange" if i % 2 == 0 else "Hull_White")
        stripe.data.transform(Matrix.Translation((0.45, -0.7 + i * 0.28, 0.9)) @ Matrix.Rotation(radians(-18), 4, "Y"))
    box("Shop_Sign", (0.04, 0.9, 0.16), (-0.35, 0, 1.0), "Light_Cyan", 0.02)
    for i, y in enumerate((-0.15, 0.25, 0.65)):
        crate(f"Shop_Crate{i}", (0.65, y, 0.06), 0.24, 0.1 * i)
        for j in range(3):
            ico("Shop_Produce", 0.05, (0.6 + (j - 1) * 0.07, y, 0.33), "Accent_Orange" if i == 1 else "Plant_Green",
                1, (1, 1, 1), 0.2, i * 3 + j)
    barrel("Shop_Barrel", (0.7, -0.45, 0.06), "Cargo_Teal")


# Model registry. `props`: None keeps the `%P%` nodes of the existing glTF, False exports none,
# a dict generates them (extent in tiles, anchor positions in Blender XY, interior (radius, floor)).
def P(extent, ins=(), outs=(), interior=None):
    return dict(extent=extent, ins=list(ins), outs=list(outs), interior=interior)


MODELS = {
    # Network
    "tunnel_2": dict(rel="placement/tunnel/tunnel_2", build=build_tunnel_2, tex=1024, ao=0.5, cut=True,
                     name="Tunnel"),
    "tunnel_4": dict(rel="placement/tunnel/tunnel_4", build=build_tunnel_4, tex=1024, ao=0.5, cut=True,
                     name="TunnelJunction"),
    "airlock": dict(rel="placement/airlock/airlock", build=build_airlock, tex=1024, ao=0.5, cut=True,
                    name="Airlock"),
    "path_2": dict(rel="placement/path/path_2", build=build_path_2, tex=512, ao=0.3, name="Path"),
    "path_4": dict(rel="placement/path/path_4", build=build_path_4, tex=512, ao=0.3, name="PathJunction"),
    # Domes
    "dome_5x5": dict(rel="placement/dome/dome_5x5", build=build_dome_5x5, tex=2048, ao=0.8, cut=True,
                     name="DomeSmall", props=dome_props(5)),
    "dome_7x7": dict(rel="placement/dome/dome_7x7", build=build_dome_7x7, tex=2048, ao=0.9, cut=True,
                     name="DomeMedium", props=dome_props(7)),
    "dome_9x9": dict(rel="placement/dome/dome_9x9", build=build_dome_9x9, tex=2048, ao=1.0, cut=True,
                     name="DomeLarge", props=dome_props(9)),
    # Exterior buildings
    "control_room": dict(rel="placement/control_room/control_room", build=build_control_room, tex=2048, ao=0.7,
                         cut=True, name="ControlRoom"),
    "storage_3x3": dict(rel="placement/storage/storage_3x3", build=build_storage_3x3, tex=2048, ao=0.7,
                        name="StorageYard"),
    "solar_array_2x2": dict(rel="placement/power/solar_array_2x2", build=build_solar_array, tex=1024, ao=0.5,
                            name="SolarArray", props=P((2, 2))),
    "battery_1x1": dict(rel="placement/power/battery_1x1", build=build_battery, tex=1024, ao=0.4,
                        name="Battery", props=P((1, 1))),
    "stirling_generator_2x2": dict(rel="placement/power/stirling_generator_2x2", build=build_stirling_generator,
                                   tex=1024, ao=0.5, name="StirlingGenerator", props=P((2, 2), outs=[(2, -1)])),
    "water_extractor_2x2": dict(rel="placement/water/water_extractor_2x2", build=build_water_extractor, tex=2048,
                                ao=0.6, name="WaterExtractor", props=P((2, 2), outs=[(-2, -1)])),
    "water_tank_1x1": dict(rel="placement/water/water_tank_1x1", build=build_water_tank, tex=1024, ao=0.5,
                           name="WaterTank", props=P((1, 1))),
    "oxygen_generator_2x2": dict(rel="placement/oxygen/oxygen_generator_2x2", build=build_oxygen_generator,
                                 tex=2048, ao=0.6, name="OxygenGenerator", props=P((2, 2), outs=[(-2, 1)])),
    "oxygen_tank_1x1": dict(rel="placement/oxygen/oxygen_tank_1x1", build=build_oxygen_tank, tex=1024, ao=0.4,
                            name="OxygenTank", props=P((1, 1))),
    "fuel_refinery_2x2": dict(rel="placement/industry/fuel_refinery_2x2", build=build_fuel_refinery, tex=2048,
                              ao=0.6, name="FuelRefinery", props=P((2, 2), outs=[(2, -1)])),
    "concrete_extractor_2x2": dict(rel="placement/industry/concrete_extractor_2x2", build=build_concrete_extractor,
                                   tex=2048, ao=0.6, name="ConcreteExtractor", props=P((2, 2), outs=[(-2, -1)])),
    "drone_hub_3x3": dict(rel="placement/logistics/drone_hub_3x3", build=build_drone_hub, tex=2048, ao=0.6,
                          name="DroneHub", props=P((3, 3), outs=[(3, 0), (-3, 0), (0, 3), (0, -3)])),
    "landing_pad_3x3": dict(rel="placement/logistics/landing_pad_3x3", build=build_landing_pad, tex=2048, ao=0.6,
                            name="LandingPad", props=P((3, 3), outs=[(3, 0), (-3, 0), (0, 3), (0, -3)])),
    "sensor_tower_1x1": dict(rel="placement/logistics/sensor_tower_1x1", build=build_sensor_tower, tex=1024,
                             ao=0.5, name="SensorTower", props=P((1, 1))),
    # Dome modules
    "living_quarters_2x2": dict(rel="modules/living_quarters/living_quarters_2x2", build=build_living_quarters,
                                tex=2048, ao=0.5, name="LivingQuarters", props=P((2, 2), ins=[(2, 0)])),
    "residence_1x1": dict(rel="modules/residence/residence_1x1", build=build_residence, tex=1024, ao=0.4,
                          name="Residence", props=P((1, 1), ins=[(1, 0)])),
    "farm_2x2": dict(rel="modules/farm/farm_2x2", build=build_farm, tex=2048, ao=0.4, name="HydroponicFarm",
                     props=P((2, 2), ins=[(2, -1)])),
    "research_lab_2x1": dict(rel="modules/research_lab/research_lab_2x1", build=build_research_lab, tex=1024,
                             ao=0.5, name="ResearchLab", props=P((2, 1), ins=[(0.3, -1)])),
    "diner_1x1": dict(rel="modules/diner/diner_1x1", build=build_diner, tex=1024, ao=0.4, name="Diner",
                      props=P((1, 1), ins=[(1, 0)])),
    "infirmary_1x1": dict(rel="modules/infirmary/infirmary_1x1", build=build_infirmary, tex=1024, ao=0.4,
                          name="Infirmary", props=P((1, 1), ins=[(1, 0)])),
    "garden_1x1": dict(rel="modules/garden/garden_1x1", build=build_garden, tex=1024, ao=0.4, name="Garden",
                       props=P((1, 1), ins=[(1, 0)])),
    "grocery_1x1": dict(rel="modules/grocery/grocery_1x1", build=build_grocery, tex=1024, ao=0.4, name="Grocery",
                        props=P((1, 1), ins=[(1, 0)])),
    # Entities
    "human": dict(rel="entities/human/human", build=build_human, tex=512, ao=0.08, name="Astronaut", props=False),
    "rocket": dict(rel="entities/rocket/rocket", build=build_rocket, tex=1024, ao=0.6, name="Rocket", props=False),
}


# ---------------------------------------------------------------------------------------
# Pipeline
# ---------------------------------------------------------------------------------------
def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    PARTS.clear()


def import_properties(gltf_path):
    """Import the existing glTF and keep only its `%P%` property nodes."""
    if not os.path.exists(gltf_path):
        return []
    bpy.ops.import_scene.gltf(filepath=gltf_path)
    keep = []
    for o in list(bpy.context.scene.objects):
        if o.name.startswith("%P%") and o.type == "MESH":
            keep.append(o)
        else:
            bpy.data.objects.remove(o, do_unlink=True)
    for coll in (bpy.data.meshes, bpy.data.materials, bpy.data.images):
        for d in list(coll):
            if d.users == 0:
                coll.remove(d)
    return keep


def make_props(extent, ins=(), outs=(), interior=None):
    """Create the `%P%` property nodes read by the engine (see wde-gltf and wde-terrain-grid)."""
    mat = bpy.data.materials.get("%P%") or bpy.data.materials.new("%P%")
    mat.diffuse_color = (0.8, 0.02, 0.66, 1.0)
    objs = []

    def node(name, loc, scale, plane):
        bm = bmesh.new()
        if plane:
            vs = [bm.verts.new((x, y, 0)) for x, y in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            bm.faces.new(vs)
        else:
            bmesh.ops.create_icosphere(bm, subdivisions=1, radius=1.0)
        me = bpy.data.meshes.new(name)
        bm.to_mesh(me)
        bm.free()
        me.materials.append(mat)
        obj = bpy.data.objects.new(name, me)
        bpy.context.scene.collection.objects.link(obj)
        obj.location, obj.scale = loc, scale
        objs.append(obj)

    # Blender (sx, sy) becomes glTF (scale.x, scale.z), read as the tile extent
    node("%P%Footprint", (0, 0, 0), (extent[0], extent[1], 1), True)
    node("%P%Center", (0, 0, 0.013), (0.2, 0.2, 0.2), False)
    for kind, anchors in (("AnchorIn", ins), ("AnchorOut", outs)):
        for i, (x, y) in enumerate(anchors):
            node(f"%P%{kind}" + (f".{i:03d}" if i else ""), (x, y, 0.013), (0.13, 0.13, 0.13), False)
    if interior:
        # Buildable dome floor: tiles fully inside `radius`, modules sit at the node height
        radius, floor = interior
        node("%P%Interior", (0, 0, floor), (radius, radius, 1), True)
    return objs


def join_objects(name, objs):
    for o in bpy.context.scene.objects:
        o.select_set(False)
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    obj.name = name
    obj.data.name = name
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    bm.to_mesh(me)
    bm.free()
    return obj


def is_glass(obj):
    return any(m.name in GLASS_ALPHA for m in obj.data.materials)


def join_parts(name):
    """Join opaque parts and glass parts separately (glass is kept out of the AO bake)."""
    opaque = [o for o in PARTS if not is_glass(o)]
    glass = [o for o in PARTS if is_glass(o)]
    return join_objects(name, opaque), (join_objects(name + "_Glass", glass) if glass else None)


def merge_glass(obj, glass):
    """Join the glass back into the model, sharing the opaque UV map name (its UVs are unused)."""
    if glass is None:
        return obj
    glass.data.uv_layers.new(name=obj.data.uv_layers.active.name)
    name = obj.name
    obj = join_objects(name, [obj, glass])
    obj.name = name
    return obj


def unwrap(obj):
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=radians(60), island_margin=0.0, area_weight=0.0, scale_to_bounds=False)
    bpy.ops.uv.pack_islands(margin=0.004, rotate=True)
    bpy.ops.object.mode_set(mode="OBJECT")


def bake_ao(obj, props, out_png, size, distance, hidden=()):
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 96
    scene.cycles.device = "CPU"
    if scene.world is None:
        scene.world = bpy.data.worlds.new("World")
    scene.world.light_settings.distance = distance
    img = bpy.data.images.new(os.path.basename(out_png), size, size, alpha=False)
    img.colorspace_settings.name = "Non-Color"
    nodes = []
    for mat in obj.data.materials:
        n = mat.node_tree.nodes.new("ShaderNodeTexImage")
        n.image = img
        mat.node_tree.nodes.active = n
        nodes.append((mat, n))
    bpy.ops.mesh.primitive_plane_add(size=200, location=(0, 0, 0.0))
    ground = bpy.context.active_object
    for p in [*props, *hidden]:
        p.hide_render = True
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.bake(type="AO", margin=6, use_clear=True)
    img.filepath_raw = out_png
    img.file_format = "PNG"
    img.save()
    for mat, n in nodes:
        mat.node_tree.nodes.remove(n)
    bpy.data.objects.remove(ground, do_unlink=True)
    bpy.data.images.remove(img)
    for p in [*props, *hidden]:
        p.hide_render = False


def export(gltf_path, ao_png):
    props = set(bpy.ops.export_scene.gltf.get_rna_type().properties.keys())
    kwargs = dict(
        filepath=gltf_path, export_format="GLTF_SEPARATE", use_selection=False, export_apply=True,
        export_yup=True, export_texcoord=True, export_normals=True, export_tangents=False,
        export_materials="EXPORT", export_cameras=False, export_lights=False, export_extras=False,
        export_animations=False, export_skins=False, export_morph=False, export_image_format="NONE",
        export_vertex_color="NONE",
    )
    bpy.ops.export_scene.gltf(**{k: v for k, v in kwargs.items() if k in props})
    with open(gltf_path) as f:
        g = json.load(f)
    if ao_png:
        g["samplers"] = [{"magFilter": 9729, "minFilter": 9729, "wrapS": 33071, "wrapT": 33071}]
        g["images"] = [{"name": "ao", "uri": os.path.basename(ao_png)}]
        g["textures"] = [{"sampler": 0, "source": 0}]
        for m in g.get("materials", []):
            if not m.get("name", "").startswith("%P%") and m.get("name") not in GLASS_ALPHA:
                m["occlusionTexture"] = {"index": 0}
    for m in g.get("materials", []):
        if m.get("name") in GLASS_ALPHA:
            m["alphaMode"] = "BLEND"
            m.setdefault("pbrMetallicRoughness", {}).setdefault("baseColorFactor", [1, 1, 1, 1])
            m["pbrMetallicRoughness"]["baseColorFactor"][3] = GLASS_ALPHA[m["name"]]
    validate(g, gltf_path)
    with open(gltf_path, "w") as f:
        json.dump(g, f, indent=2)


def validate(g, path):
    assert len(g["buffers"]) == 1, f"{path}: expected a single buffer"
    assert "scene" in g
    for idx in g["scenes"][g["scene"]]["nodes"]:
        node = g["nodes"][idx]
        assert "mesh" in node, f"{path}: node {node.get('name')} has no mesh"
        assert "children" not in node, f"{path}: node {node.get('name')} has children"
    assert len(g["scenes"][g["scene"]]["nodes"]) == len(g["nodes"])


def variant_base(key, cut):
    return os.path.join(ROOT, MODELS[key]["rel"]) + ("_cut" if cut else "")


def build(key, cut=False, bake=True):
    global CUT
    CUT = cut
    spec = MODELS[key]
    tex_size, ao_dist = spec["tex"], spec["ao"]
    base = variant_base(key, cut)
    os.makedirs(os.path.dirname(base), exist_ok=True)
    gltf_path = base + ".gltf"
    reset_scene()
    props_spec = spec.get("props")
    if props_spec is None:
        # Keep the `%P%` nodes of the existing (closed) model
        props = import_properties(variant_base(key, False) + ".gltf")
    elif props_spec is False:
        props = []
    else:
        props = make_props(**props_spec)
    spec["build"]()
    obj, glass = join_parts(spec["name"])
    unwrap(obj)
    ao_png = base + "_ao.png" if bake else None
    if bake:
        bake_ao(obj, props, ao_png, tex_size, ao_dist, hidden=[glass] if glass else [])
    obj = merge_glass(obj, glass)
    export(gltf_path, ao_png)
    tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"[build] {key}{'_cut' if cut else ''}: {len(obj.data.vertices)} verts, ~{tris} tris, "
          f"{len(obj.data.materials)} materials, props={[p.name for p in props]}")
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=base + ".blend", compress=True)


# ---------------------------------------------------------------------------------------
# Preview rendering (Eevee, approximates the in-game Mars lighting)
# ---------------------------------------------------------------------------------------
def preview(key, out_dir, cut=False):
    reset_scene()
    bpy.ops.import_scene.gltf(filepath=variant_base(key, cut) + ".gltf")
    lo = Vector((1e9,) * 3)
    hi = Vector((-1e9,) * 3)
    for o in bpy.context.scene.objects:
        if o.type != "MESH":
            continue
        if o.name.startswith("%P%"):
            o.hide_render = True
            continue
        for c in o.bound_box:
            w = o.matrix_world @ Vector(c)
            lo = Vector(map(min, lo, w))
            hi = Vector(map(max, hi, w))
    sc = bpy.context.scene
    sc.render.engine = "BLENDER_EEVEE"
    sc.render.resolution_x, sc.render.resolution_y = 900, 700
    sc.view_settings.view_transform = "AgX"
    bpy.ops.mesh.primitive_plane_add(size=300, location=(0, 0, 0))
    ground = bpy.context.active_object
    gm = bpy.data.materials.new("MarsGround")
    gm.use_nodes = True
    gb = gm.node_tree.nodes["Principled BSDF"]
    gb.inputs["Base Color"].default_value = (0.32, 0.12, 0.06, 1)
    gb.inputs["Roughness"].default_value = 0.95
    ground.data.materials.append(gm)
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sc.collection.objects.link(sun)
    sun.rotation_euler = (radians(50), radians(10), radians(35))
    sun.data.energy = 4.0
    sun.data.angle = radians(3)
    world = bpy.data.worlds.new("w")
    sc.world = world
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.55, 0.38, 0.30, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.6
    cen = (lo + hi) / 2
    radius = max((hi - lo).length / 2, 0.2)
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    sc.camera = cam
    cam.data.lens = 50
    for tag, d in (("iso", Vector((1.0, -1.25, 1.1))), ("top", Vector((0.35, -0.6, 1.6)))):
        if key == "human":
            d = Vector((1.4, -0.9, 0.45))
        cam.location = cen + d.normalized() * radius * 2.9
        cam.rotation_euler = (cen - cam.location).to_track_quat("-Z", "Y").to_euler()
        sc.render.filepath = os.path.join(out_dir, f"{key}{'_cut' if cut else ''}_{tag}.png")
        bpy.ops.render.render(write_still=True)


# ---------------------------------------------------------------------------------------
# UI icons (transparent isometric thumbnails for the placement menu)
# ---------------------------------------------------------------------------------------
ICON_DIR = os.path.normpath(os.path.join(ROOT, "..", "core", "icons", "buildings"))
ICON_SIZE = 256


def render_icon(key):
    reset_scene()
    bpy.ops.import_scene.gltf(filepath=variant_base(key, False) + ".gltf")
    lo = Vector((1e9,) * 3)
    hi = Vector((-1e9,) * 3)
    for o in list(bpy.context.scene.objects):
        if o.name.startswith("%P%"):
            bpy.data.objects.remove(o, do_unlink=True)
            continue
        for c in o.bound_box:
            w = o.matrix_world @ Vector(c)
            lo = Vector(map(min, lo, w))
            hi = Vector(map(max, hi, w))
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.samples = 48
    sc.cycles.use_denoising = True
    sc.render.film_transparent = True
    sc.render.resolution_x = sc.render.resolution_y = ICON_SIZE
    sc.view_settings.view_transform = "AgX"
    sc.view_settings.look = "AgX - Medium High Contrast"
    bpy.ops.mesh.primitive_plane_add(size=200, location=(0, 0, 0))
    bpy.context.active_object.is_shadow_catcher = True
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sc.collection.objects.link(sun)
    sun.rotation_euler = (radians(45), radians(-5), radians(30))
    sun.data.energy = 4.5
    sun.data.angle = radians(8)
    world = bpy.data.worlds.new("w")
    sc.world = world
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.62, 0.55, 0.52, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.9
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    sc.camera = cam
    cam.data.type = "ORTHO"
    cen = (lo + hi) / 2
    cam.location = cen + Vector((1.0, -1.0, 0.95)).normalized() * 50
    cam.rotation_euler = (cen - cam.location).to_track_quat("-Z", "Y").to_euler()
    # Fit the ortho frame to the projected bounding box
    bpy.context.view_layer.update()
    inv = cam.matrix_world.inverted()
    pts = [inv @ Vector((x, y, z)) for x in (lo.x, hi.x) for y in (lo.y, hi.y) for z in (lo.z, hi.z)]
    span = max(max(p.x for p in pts) - min(p.x for p in pts), max(p.y for p in pts) - min(p.y for p in pts))
    mid = Vector(((max(p.x for p in pts) + min(p.x for p in pts)) / 2, (max(p.y for p in pts) + min(p.y for p in pts)) / 2, 0))
    cam.location = cam.matrix_world @ mid
    cam.data.ortho_scale = span * 1.08
    os.makedirs(ICON_DIR, exist_ok=True)
    path = os.path.join(ICON_DIR, key + ".png")
    sc.render.filepath = path
    bpy.ops.render.render(write_still=True)
    premultiply(path)
    print(f"[icon] {key}")


def premultiply(path):
    """egui blends textures as premultiplied alpha."""
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "Non-Color"
    px = list(img.pixels)
    for i in range(0, len(px), 4):
        a = px[i + 3]
        px[i] *= a
        px[i + 1] *= a
        px[i + 2] *= a
    img.pixels = px
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = None
    if "--preview" in argv:
        i = argv.index("--preview")
        out = argv[i + 1]
        del argv[i:i + 2]
    bake = "--no-bake" not in argv
    icons = "--icons" in argv
    argv = [a for a in argv if not a.startswith("--")]
    keys = argv or list(MODELS)
    if icons:
        for k in keys:
            if k not in ("human", "rocket"):
                render_icon(k)
        sys.exit(0)
    for k in keys:
        for cut in (False, True) if MODELS[k].get("cut") else (False,):
            if "--only-preview" not in sys.argv:
                build(k, cut, bake)
            if out:
                os.makedirs(out, exist_ok=True)
                preview(k, out, cut)

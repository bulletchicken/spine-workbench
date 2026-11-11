"""Prepare the vertebra shapes the 3D view draws.

    python3 bone-prep/prepare_bones.py path/to/partof_BP3D_4.0_obj_99.zip [app/public/bones]

Reads the eighteen vertebrae from C7 to L5 out of the BodyParts3D 4.0 archive
(see README.md for where it comes from and its licence) and writes
`vertebrae.json` and `vertebrae.bin`. Standard library only.

Nothing is reshaped. Points the archive repeats are joined, and each bone is
moved into a frame of its own: origin at the middle of the vertebral body,
x forward, y to the patient's left, z up the body, in millimetres. The body's
height, half width and half depth are measured and stored beside it, so a
fitted vertebra can place the bone and stretch it to size.
"""

import hashlib
import json
import math
import struct
import sys
import zipfile

ARCHIVE_SHA256 = "9fbc713fffeee924a5a657d9813d84d7eb957bded63adb854931dd5e3eb61c97"
FOLDER = "partof_BP3D_4.0_obj_99"
CREDIT = "BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International"

# The archive names files by element id. These ids come from its own table,
# partof_element_parts.txt ("first thoracic vertebra" is FJ3158, and so on).
LEVELS = [
    ("C7", "FJ3172"), ("T1", "FJ3158"), ("T2", "FJ3160"), ("T3", "FJ3163"), ("T4", "FJ3166"), ("T5", "FJ3169"),
    ("T6", "FJ3171"), ("T7", "FJ3173"), ("T8", "FJ3174"), ("T9", "FJ3175"), ("T10", "FJ3154"), ("T11", "FJ3155"),
    ("T12", "FJ3156"), ("L1", "FJ3157"), ("L2", "FJ3159"), ("L3", "FJ3162"), ("L4", "FJ3165"), ("L5", "FJ3168"),
]


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def mul(a, k):
    return (a[0] * k, a[1] * k, a[2] * k)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(a):
    return mul(a, 1 / math.sqrt(dot(a, a)))


def read_obj(text):
    """Points in patient axes (forward, left, up) in millimetres, and triangles.

    The archive has x to the patient's left, y to the back and z up. It also
    repeats a point wherever the surface direction differs, which leaves the
    surface in separate patches; points that share a place are joined here.
    """
    seen, points, where, triangles = [], [], {}, []
    for line in text.splitlines():
        if line.startswith("v "):
            x, y, z = map(float, line.split()[1:4])
            point = (-y, x, z)
            key = (round(point[0], 3), round(point[1], 3), round(point[2], 3))
            if key not in where:
                where[key] = len(points)
                points.append(point)
            seen.append(where[key])
        elif line.startswith("f "):
            corners = [seen[int(token.split("/")[0]) - 1] for token in line.split()[1:]]
            for i in range(1, len(corners) - 1):
                # (x, y, z) -> (-y, x, z) is a turn, not a mirror: the winding stands.
                if len({corners[0], corners[i], corners[i + 1]}) == 3:
                    triangles.append((corners[0], corners[i], corners[i + 1]))
    return points, triangles


def is_closed(triangles):
    """True when every edge is shared by exactly two triangles."""
    uses = {}
    for a, b, c in triangles:
        for i, j in ((a, b), (b, c), (c, a)):
            edge = (min(i, j), max(i, j))
            uses[edge] = uses.get(edge, 0) + 1
    return all(count == 2 for count in uses.values())


def cut(points, triangles, origin, normal):
    """Cut the surface with a plane. Returns closed outlines as lists of 3D points in order."""
    # The small offset keeps a point from lying exactly on the plane.
    side = [dot(sub(p, origin), normal) + 1e-4 for p in points]
    joins, crossing = {}, {}
    for triangle in triangles:
        nodes = []
        for i, j in ((triangle[0], triangle[1]), (triangle[1], triangle[2]), (triangle[2], triangle[0])):
            if (side[i] > 0) != (side[j] > 0):
                edge = (min(i, j), max(i, j))
                if edge not in crossing:
                    t = side[i] / (side[i] - side[j])
                    crossing[edge] = add(points[i], mul(sub(points[j], points[i]), t))
                nodes.append(edge)
        if len(nodes) == 2:
            joins.setdefault(nodes[0], []).append(nodes[1])
            joins.setdefault(nodes[1], []).append(nodes[0])
    outlines, visited = [], set()
    for start in joins:
        if start in visited:
            continue
        outline, previous, here = [], None, start
        while here is not None and here not in visited:
            visited.add(here)
            outline.append(crossing[here])
            onward = [n for n in joins[here] if n != previous and n not in visited]
            previous, here = here, (onward[0] if onward else None)
        if len(outline) >= 3:
            outlines.append(outline)
    return outlines


def area_and_middle(outline):
    """Area and centroid of a flat outline given as 2D points."""
    doubled = cx = cy = 0.0
    for (x0, y0), (x1, y1) in zip(outline, outline[1:] + outline[:1]):
        k = x0 * y1 - x1 * y0
        doubled += k
        cx += (x0 + x1) * k
        cy += (y0 + y1) * k
    return abs(doubled) / 2, (cx / (3 * doubled), cy / (3 * doubled))


def holds(outline, q):
    inside = False
    for (x0, y0), (x1, y1) in zip(outline, outline[1:] + outline[:1]):
        if (y0 > q[1]) != (y1 > q[1]) and q[0] < x0 + (q[1] - y0) * (x1 - x0) / (y1 - y0):
            inside = not inside
    return inside


def upright_box(outline):
    """The smallest box round a 2D outline whose second side stays within 45 degrees of upright.

    Returns its turn in radians, its middle, and its sizes across and up.
    """
    best = None
    for half_degrees in range(-90, 91):
        turn = math.radians(half_degrees / 2)
        across, up = (math.cos(turn), -math.sin(turn)), (math.sin(turn), math.cos(turn))
        us = [p[0] * across[0] + p[1] * across[1] for p in outline]
        ws = [p[0] * up[0] + p[1] * up[1] for p in outline]
        size = (max(us) - min(us), max(ws) - min(ws))
        if best is None or size[0] * size[1] < best[0]:
            mu, mw = (max(us) + min(us)) / 2, (max(ws) + min(ws)) / 2
            middle = (mu * across[0] + mw * up[0], mu * across[1] + mw * up[1])
            best = (size[0] * size[1], turn, middle, size)
    return best[1], best[2], best[3]


def measure(points, triangles):
    """Find the vertebral body from two cuts through the bone: its frame and its sizes."""
    left = (min(p[1] for p in points) + max(p[1] for p in points)) / 2
    for _ in range(2):
        # Down the middle, the spinal canal parts the body from the arch
        # behind it, so the body is the outline furthest forward.
        outlines = [[(p[0], p[2]) for p in o] for o in cut(points, triangles, (0.0, left, 0.0), (0.0, 1.0, 0.0))]
        sized = [(area_and_middle(o), o) for o in outlines]
        largest = max(area for (area, _), _ in sized)
        body = max((item for item in sized if item[0][0] > 0.3 * largest), key=lambda item: item[0][1][0])[1]
        lean, middle, (depth, height) = upright_box(body)
        up = (math.sin(lean), 0.0, math.cos(lean))
        forward = (math.cos(lean), 0.0, -math.sin(lean))
        centre = (middle[0], left, middle[1])
        # Across the middle of the body, facing forward: its width, any
        # sideways tilt, and where its middle truly is from side to side.
        flat = [[(p[1], dot(sub(p, centre), up)) for p in o] for o in cut(points, triangles, centre, forward)]
        around = [o for o in flat if holds(o, (left, 0.0))] or [max(flat, key=lambda o: area_and_middle(o)[0])]
        tilt, middle_across, (width, _) = upright_box(around[0])
        left = middle_across[0]
    to_left = unit(add(mul((0.0, 1.0, 0.0), math.cos(tilt)), mul(up, -math.sin(tilt))))
    up = unit(cross(forward, to_left))
    return {
        "centre": (middle[0], left, middle[1]),
        "axes": (forward, to_left, up),
        "height": height,
        "halfWidth": width / 2,
        "halfDepth": depth / 2,
    }


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    archive_path = sys.argv[1]
    out = sys.argv[2] if len(sys.argv) > 2 else "app/public/bones"
    with open(archive_path, "rb") as handle:
        digest = hashlib.sha256(handle.read()).hexdigest()
    if digest != ARCHIVE_SHA256:
        sys.exit(f"This is not the archive the shapes were prepared from (sha256 {digest}).")

    bones, all_points, all_triangles = [], [], []
    with zipfile.ZipFile(archive_path) as archive:
        for level, file_id in LEVELS:
            text = archive.read(f"{FOLDER}/{file_id}.obj").decode("utf-8", errors="replace")
            points, triangles = read_obj(text)
            if not is_closed(triangles):
                sys.exit(f"{level} ({file_id}) is not a closed surface.")
            body = measure(points, triangles)
            local = [tuple(round(dot(sub(p, body["centre"]), axis), 2) for axis in body["axes"]) for p in points]
            bones.append({
                "level": level,
                "source": file_id,
                "height": round(body["height"], 2),
                "halfWidth": round(body["halfWidth"], 2),
                "halfDepth": round(body["halfDepth"], 2),
                "referenceCentre": [round(c, 2) for c in body["centre"]],
                "referenceAxes": [[round(c, 5) for c in axis] for axis in body["axes"]],
                "pointCount": len(points),
                "triangleCount": len(triangles),
            })
            all_points.append(local)
            all_triangles.append(triangles)
            print(f"{level:>3} {file_id}: {len(points):4d} points, {len(triangles):4d} triangles, body "
                  f"{body['height']:.1f} high, {2 * body['halfWidth']:.1f} wide, {2 * body['halfDepth']:.1f} deep (mm)")

    # Every bone's points first, then every bone's triangles: each block then
    # starts on a boundary its number type needs.
    data = bytearray()
    for bone, local in zip(bones, all_points):
        bone["pointsAt"] = len(data)
        for point in local:
            data += struct.pack("<3f", *point)
    for bone, triangles in zip(bones, all_triangles):
        bone["trianglesAt"] = len(data)
        for triangle in triangles:
            data += struct.pack("<3H", *triangle)

    manifest = {
        "credit": CREDIT,
        "licence": "CC BY 4.0, https://creativecommons.org/licenses/by/4.0/",
        "source": "BodyParts3D 4.0, partof_BP3D_4.0_obj_99, https://dbarchive.biosciencedbc.jp/en/bodyparts3d/",
        "changes": "Repeated points joined; each bone moved into a frame of its own. No shape was altered.",
        "frame": "Origin at the middle of the vertebral body; x forward, y to the patient's left, z up the body; millimetres.",
        "file": "vertebrae.bin: little-endian; each bone's points as float32 x, y, z from pointsAt, "
                "its triangles as uint16 corners from trianglesAt.",
        "reference": "referenceCentre and referenceAxes (the bone's x, y and z axes) put a bone back where it stood "
                     "in the source, in the same patient axes.",
        "byteLength": len(data),
        "bones": bones,
    }
    with open(f"{out}/vertebrae.bin", "wb") as handle:
        handle.write(data)
    with open(f"{out}/vertebrae.json", "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, ensure_ascii=False, indent=2)
        handle.write("\n")
    print(f"Wrote {len(bones)} bones, {len(data):,} bytes, to {out}/")


if __name__ == "__main__":
    main()

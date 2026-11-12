"""Build the typical spine the one-film fit falls back on.

    python3 bone-prep/typical_spine.py

A front film shows where each vertebra sits from side to side and how it
tilts, but not how far forward it leans, how deep its body is, how its body
is wedged from front to back, or how far it is twisted. This script writes
`engine/joint-fit/typical-spine/typical-spine.json`, which holds a typical
value and a spread for each of those, taken from published cohorts and from
the bone shapes in `app/src/spine-3d/bones/vertebrae.json`. Standard library only;
the same inputs always give the same bytes. `typical-spine.md` explains
every step.
"""

import json
import math
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BONES = os.path.join(ROOT, "app", "src", "spine-3d", "bones", "vertebrae.json")
OUT = os.path.join(ROOT, "engine", "joint-fit", "typical-spine", "typical-spine.json")

LEVELS = ["C7", "T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8", "T9", "T10", "T11", "T12",
          "L1", "L2", "L3", "L4", "L5"]

# Every number this script takes from outside the bone file. One row each:
# name, value, unit, source key (see SOURCES), and what was read: "paper"
# (the full text) or "abstract" (only the abstract).
NUMBERS = [
    # Thoracic kyphosis, T5 upper endplate to T12 lower endplate.
    ("kyphosis.straight.mean", 32.9, "deg", "burkus2018", "paper"),
    ("kyphosis.straight.sd", 15.0, "deg", "burkus2018", "paper"),
    ("kyphosis.moderate.lowestCobb", 20.0, "deg", "sullivan2020", "abstract"),
    ("kyphosis.moderate.highestCobb", 45.0, "deg", "sullivan2020", "abstract"),
    ("kyphosis.moderate.mean", 13.0, "deg", "sullivan2020", "abstract"),
    ("kyphosis.moderate.sd", 10.0, "deg", "sullivan2020", "abstract"),
    ("kyphosis.severe.mean", 5.0, "deg", "sullivan2020", "abstract"),
    ("kyphosis.severe.sd", 12.0, "deg", "sullivan2020", "abstract"),
    ("kyphosis.severe.meanCobb", 55.0, "deg", "newton2015", "abstract"),
    # Lumbar lordosis, L1 upper endplate to L5 lower endplate.
    ("lordosis.straight.mean", 46.0, "deg", "burkus2018", "paper"),
    ("lordosis.straight.sd", 9.1, "deg", "burkus2018", "paper"),
    ("lordosis.scoliosis.sd", 13.2, "deg", "burkus2018", "paper"),
    # Lean of the thoracic and lumbar spine as a whole, between people.
    ("thoracicTilt.sd", 5.2, "deg", "macthiong2007", "abstract"),
    ("lumbarTilt.sd", 5.2, "deg", "macthiong2007", "abstract"),
    # Apical twist by curve band (absolute values).
    ("twist.lowestCobb", 10.0, "deg", "jozsef2022", "paper"),
    ("twist.meanCobb", 37.7, "deg", "jozsef2022", "paper"),
    ("twist.band1.highestCobb", 20.0, "deg", "jozsef2022", "paper"),
    ("twist.band1.count", 76, "curves", "jozsef2022", "paper"),
    ("twist.band1.mean", 6.70, "deg", "jozsef2022", "paper"),
    ("twist.band1.sd", 4.55, "deg", "jozsef2022", "paper"),
    ("twist.band2.lowestCobb", 21.0, "deg", "jozsef2022", "paper"),
    ("twist.band2.highestCobb", 35.0, "deg", "jozsef2022", "paper"),
    ("twist.band2.count", 118, "curves", "jozsef2022", "paper"),
    ("twist.band2.mean", 9.34, "deg", "jozsef2022", "paper"),
    ("twist.band2.sd", 5.14, "deg", "jozsef2022", "paper"),
    ("twist.band3.lowestCobb", 36.0, "deg", "jozsef2022", "paper"),
    ("twist.band3.highestCobb", 40.0, "deg", "jozsef2022", "paper"),
    ("twist.band3.count", 19, "curves", "jozsef2022", "paper"),
    ("twist.band3.mean", 16.36, "deg", "jozsef2022", "paper"),
    ("twist.band3.sd", 7.30, "deg", "jozsef2022", "paper"),
    ("twist.band4.lowestCobb", 41.0, "deg", "jozsef2022", "paper"),
    ("twist.band4.highestCobb", 50.0, "deg", "jozsef2022", "paper"),
    ("twist.band4.count", 43, "curves", "jozsef2022", "paper"),
    ("twist.band4.mean", 15.67, "deg", "jozsef2022", "paper"),
    ("twist.band4.sd", 5.07, "deg", "jozsef2022", "paper"),
    ("twist.band5.lowestCobb", 51.0, "deg", "jozsef2022", "paper"),
    ("twist.band5.highestCobb", 55.0, "deg", "jozsef2022", "paper"),
    ("twist.band5.count", 10, "curves", "jozsef2022", "paper"),
    ("twist.band5.mean", 15.84, "deg", "jozsef2022", "paper"),
    ("twist.band5.sd", 5.77, "deg", "jozsef2022", "paper"),
    ("twist.band6.lowestCobb", 56.0, "deg", "jozsef2022", "paper"),
    ("twist.band6.count", 66, "curves", "jozsef2022", "paper"),
    ("twist.band6.mean", 21.27, "deg", "jozsef2022", "paper"),
    ("twist.band6.sd", 13.05, "deg", "jozsef2022", "paper"),
    # Endplate depth over width, in per cent, men and women, T12 to L5.
    ("depthToWidth.T12.men.mean", 75.21, "%", "pan2021", "paper"),
    ("depthToWidth.T12.men.sd", 4.23, "%", "pan2021", "paper"),
    ("depthToWidth.L1.men.mean", 73.92, "%", "pan2021", "paper"),
    ("depthToWidth.L1.men.sd", 9.43, "%", "pan2021", "paper"),
    ("depthToWidth.L2.men.mean", 73.22, "%", "pan2021", "paper"),
    ("depthToWidth.L2.men.sd", 6.45, "%", "pan2021", "paper"),
    ("depthToWidth.L3.men.mean", 74.07, "%", "pan2021", "paper"),
    ("depthToWidth.L3.men.sd", 5.67, "%", "pan2021", "paper"),
    ("depthToWidth.L4.men.mean", 71.04, "%", "pan2021", "paper"),
    ("depthToWidth.L4.men.sd", 8.66, "%", "pan2021", "paper"),
    ("depthToWidth.L5.men.mean", 70.34, "%", "pan2021", "paper"),
    ("depthToWidth.L5.men.sd", 7.53, "%", "pan2021", "paper"),
    ("depthToWidth.T12.women.mean", 74.25, "%", "pan2021", "paper"),
    ("depthToWidth.T12.women.sd", 5.67, "%", "pan2021", "paper"),
    ("depthToWidth.L1.women.mean", 73.53, "%", "pan2021", "paper"),
    ("depthToWidth.L1.women.sd", 7.87, "%", "pan2021", "paper"),
    ("depthToWidth.L2.women.mean", 73.71, "%", "pan2021", "paper"),
    ("depthToWidth.L2.women.sd", 7.44, "%", "pan2021", "paper"),
    ("depthToWidth.L3.women.mean", 73.24, "%", "pan2021", "paper"),
    ("depthToWidth.L3.women.sd", 5.23, "%", "pan2021", "paper"),
    ("depthToWidth.L4.women.mean", 71.34, "%", "pan2021", "paper"),
    ("depthToWidth.L4.women.sd", 7.54, "%", "pan2021", "paper"),
    ("depthToWidth.L5.women.mean", 70.21, "%", "pan2021", "paper"),
    ("depthToWidth.L5.women.sd", 6.45, "%", "pan2021", "paper"),
    # Spread of the front and back body heights, C7 to L5.
    ("frontHeight.C7.sd", 0.5, "mm", "busscher2010", "paper"),
    ("backHeight.C7.sd", 0.7, "mm", "busscher2010", "paper"),
    ("frontHeight.T1.sd", 0.8, "mm", "busscher2010", "paper"),
    ("backHeight.T1.sd", 1.0, "mm", "busscher2010", "paper"),
    ("frontHeight.T2.sd", 1.4, "mm", "busscher2010", "paper"),
    ("backHeight.T2.sd", 0.8, "mm", "busscher2010", "paper"),
    ("frontHeight.T3.sd", 0.8, "mm", "busscher2010", "paper"),
    ("backHeight.T3.sd", 1.0, "mm", "busscher2010", "paper"),
    ("frontHeight.T4.sd", 0.8, "mm", "busscher2010", "paper"),
    ("backHeight.T4.sd", 1.2, "mm", "busscher2010", "paper"),
    ("frontHeight.T5.sd", 3.2, "mm", "busscher2010", "paper"),
    ("backHeight.T5.sd", 1.6, "mm", "busscher2010", "paper"),
    ("frontHeight.T6.sd", 3.0, "mm", "busscher2010", "paper"),
    ("backHeight.T6.sd", 2.0, "mm", "busscher2010", "paper"),
    ("frontHeight.T7.sd", 1.1, "mm", "busscher2010", "paper"),
    ("backHeight.T7.sd", 1.7, "mm", "busscher2010", "paper"),
    ("frontHeight.T8.sd", 2.8, "mm", "busscher2010", "paper"),
    ("backHeight.T8.sd", 1.5, "mm", "busscher2010", "paper"),
    ("frontHeight.T9.sd", 1.6, "mm", "busscher2010", "paper"),
    ("backHeight.T9.sd", 2.0, "mm", "busscher2010", "paper"),
    ("frontHeight.T10.sd", 1.6, "mm", "busscher2010", "paper"),
    ("backHeight.T10.sd", 1.6, "mm", "busscher2010", "paper"),
    ("frontHeight.T11.sd", 1.9, "mm", "busscher2010", "paper"),
    ("backHeight.T11.sd", 1.9, "mm", "busscher2010", "paper"),
    ("frontHeight.T12.sd", 1.6, "mm", "busscher2010", "paper"),
    ("backHeight.T12.sd", 0.9, "mm", "busscher2010", "paper"),
    ("frontHeight.L1.sd", 2.5, "mm", "busscher2010", "paper"),
    ("backHeight.L1.sd", 2.5, "mm", "busscher2010", "paper"),
    ("frontHeight.L2.sd", 2.8, "mm", "busscher2010", "paper"),
    ("backHeight.L2.sd", 1.2, "mm", "busscher2010", "paper"),
    ("frontHeight.L3.sd", 1.9, "mm", "busscher2010", "paper"),
    ("backHeight.L3.sd", 1.0, "mm", "busscher2010", "paper"),
    ("frontHeight.L4.sd", 2.4, "mm", "busscher2010", "paper"),
    ("backHeight.L4.sd", 2.1, "mm", "busscher2010", "paper"),
    ("frontHeight.L5.sd", 1.4, "mm", "busscher2010", "paper"),
    ("backHeight.L5.sd", 3.8, "mm", "busscher2010", "paper"),
]

SOURCES = [
    ("bodyparts3d",
     "BodyParts3D 4.0, The Database Center for Life Science, licensed under CC Attribution 4.0 International. "
     "One adult man's anatomy from whole-body MRI.",
     "The per-level shape of the side profile, body widths, depth-to-width proportions, each body's own side "
     "wedge and the stacking spread."),
    ("burkus2018",
     "Burkus M, Schlégl ÁT, O'Sullivan I, Márkus I, Vermes C, Tunyogi-Csapó M. Sagittal plane assessment of "
     "spino-pelvic complex in a Central European population with adolescent idiopathic scoliosis: a case control "
     "study. Scoliosis Spinal Disord 2018;13:10. DOI 10.1186/s13013-018-0156-0. Full text read.",
     "Kyphosis T5-T12 of 69 young people without scoliosis, 32.9 (SD 15.0) degrees; lordosis L1-L5, 46.0 (9.1) "
     "without and 46.4 (13.2) with scoliosis, not different."),
    ("sullivan2020",
     "Sullivan TB, Bastrom TP, Bartley CE, Dolan LA, Weinstein SL, Newton PO. More severe thoracic idiopathic "
     "scoliosis is associated with a greater three-dimensional loss of thoracic kyphosis. Spine Deform "
     "2020;8(6):1205-1211. DOI 10.1007/s43390-020-00149-7. Abstract only.",
     "Three-dimensional kyphosis T5-T12 of 3032 patients: 13 (SD 10) degrees for curves of 20 to 45 degrees, "
     "5 (12) for curves over 45."),
    ("newton2015",
     "Newton PO, Fujimori T, Doan J, Reighard FG, Bastrom TP, Misaghi A. Defining the \"three-dimensional "
     "sagittal plane\" in thoracic adolescent idiopathic scoliosis. J Bone Joint Surg Am 2015;97(20):1694-1701. "
     "DOI 10.2106/JBJS.O.00148. Abstract only.",
     "A mean curve of 55 degrees for a group whose three-dimensional kyphosis T5-T12 was 6 (SD 14) degrees, "
     "used to place the curves over 45 degrees."),
    ("macthiong2007",
     "Mac-Thiong JM, Labelle H, Berthonnaud E, Betz RR, Roussouly P. Sagittal spinopelvic balance in normal "
     "children and adolescents. Eur Spine J 2007;16(2):227-234. DOI 10.1007/s00586-005-0013-8. Abstract only.",
     "Spread between children of the lean of the thoracic and of the lumbar spine, 5.2 degrees each."),
    ("jozsef2022",
     "József K, Schlégl ÁT, Burkus M, Márkus I, O'Sullivan I, Than P, Tunyogi-Csapó M. Maximal axial vertebral "
     "rotation in adolescent idiopathic scoliosis: is the apical vertebra the most rotated? Global Spine J "
     "2022;12(2):244-248. DOI 10.1177/2192568220948830. Full text read.",
     "Apical twist, mean and SD, in six bands of curve size, 332 patients."),
    ("pan2021",
     "Pan CL, Zhang BY, Zhu YH, Ma YH, Li MF, Wang X, Yang F, Li YQ, Zhu YH. Morphologic analysis of Chinese "
     "lumbar endplate by three-dimensional computed tomography reconstructions for helping design lumbar disc "
     "prosthesis. Medicine (Baltimore) 2021;100(6):e24583. DOI 10.1097/MD.0000000000024583. Full text read.",
     "Spread between people of endplate depth over width, T12 to L5, 150 adults."),
    ("busscher2010",
     "Busscher I, Ploegmakers JJ, Verkerke GJ, Veldhuizen AG. Comparative anatomical dimensions of the complete "
     "human and porcine spine. Eur Spine J 2010;19(7):1104-1114. DOI 10.1007/s00586-010-1326-9. Full text read.",
     "Spread of the front and back heights of each vertebral body, C7 to L5, six adult spines."),
]

# Curve sizes, in degrees, at which a side profile is written. The engine
# interpolates between them.
PROFILE_CURVES = [0.0, 20.0, 32.0, 45.0, 55.0, 60.0, 90.0]

ABOUT = (
    "A typical spine for what one front film cannot show: forward lean, front-to-back wedge and depth of each "
    "vertebral body, and twist. Typical values with their spread between people, not measurements of any "
    "patient. Side profiles are one adult's bone posture scaled to cohort kyphosis (T5 upper to T12 lower "
    "endplate) and lordosis (L1 upper to L5 lower endplate) for each curve size, and levelled so the line from "
    "T1's centre to L5's centre is upright. Angles in degrees, lengths in millimetres, levels C7 to L5. Built by "
    "bone-prep/typical_spine.py; bone-prep/typical-spine.md gives every source and step."
)

NUMBER = {row[0]: row[1] for row in NUMBERS}


def at(level):
    return LEVELS.index(level)


def read_bones():
    with open(BONES, encoding="utf-8") as handle:
        bones = json.load(handle)["bones"]
    if [bone["level"] for bone in bones] != LEVELS:
        raise SystemExit("vertebrae.json does not hold C7 to L5 in order")
    for bone in bones:
        # The bone's z axis in patient axes is the third row of referenceAxes.
        # Its forward part is the sine of the forward lean.
        bone["lean"] = math.degrees(math.asin(bone["referenceAxes"][2][0]))
    # Rise between neighbouring centres, upper level first: the two half
    # heights and the disc gap between them, as the bones stood.
    rises = [math.dist(upper["referenceCentre"], lower["referenceCentre"])
             for upper, lower in zip(bones, bones[1:])]
    return bones, rises


def side_wedges(bones):
    """Each bone's own side wedge between its two measured endplates, in
    degrees to a tenth, positive when the front is shorter."""
    return [tenth(math.degrees(bone["body"]["wedgeSide"])) for bone in bones]


def depth_to_width(bone):
    """Depth over width of the body's two measured endplates together: the
    sum of their half-depths over the sum of their half-widths."""
    top, bottom = bone["body"]["top"], bone["body"]["bottom"]
    return (top["halfDepth"] + bottom["halfDepth"]) / (top["halfWidth"] + bottom["halfWidth"])


def chord_lean(leans, rises):
    """Lean of the line from L5's centre to T1's, stacking each pair of
    neighbouring centres along the mean of their two leans."""
    forward = upward = 0.0
    for upper in range(at("T1"), at("L5")):
        mean = math.radians((leans[upper] + leans[upper + 1]) / 2)
        forward += rises[upper] * math.sin(mean)
        upward += rises[upper] * math.cos(mean)
    return math.degrees(math.atan2(forward, upward))


def kyphosis(leans, wedges):
    """T5 upper endplate to T12 lower endplate. Each endplate leans by half its
    body's side wedge."""
    return leans[at("T5")] - leans[at("T12")] + (wedges[at("T5")] + wedges[at("T12")]) / 2


def lordosis(leans, wedges):
    """L1 upper endplate to L5 lower endplate."""
    return leans[at("L5")] - leans[at("L1")] - (wedges[at("L1")] + wedges[at("L5")]) / 2


def side_profile(bones, rises, wedges, kyphosis_deg, lordosis_deg):
    source = [bone["lean"] for bone in bones]
    thoracic = (kyphosis_deg - kyphosis([0.0] * 18, wedges)) / (source[at("T5")] - source[at("T12")])
    lumbar = (lordosis_deg - lordosis([0.0] * 18, wedges)) / (source[at("L5")] - source[at("L1")])
    leans = [0.0] * 18
    for upper in range(16, -1, -1):
        step = source[upper] - source[upper + 1]
        if at("T5") <= upper < at("T12"):
            step *= thoracic
        elif at("L1") <= upper < at("L5"):
            step *= lumbar
        leans[upper] = leans[upper + 1] + step
    offset = -chord_lean(leans, rises)
    return [lean + offset for lean in leans]


def between(points, curve):
    """Straight-line interpolation through (curve, value) points, held level
    beyond the last."""
    if curve >= points[-1][0]:
        return points[-1][1]
    for (c0, v0), (c1, v1) in zip(points, points[1:]):
        if c0 <= curve <= c1:
            return v0 + (v1 - v0) * (curve - c0) / (c1 - c0)
    raise ValueError(curve)


def cohort_curves():
    moderate = (NUMBER["kyphosis.moderate.lowestCobb"] + NUMBER["kyphosis.moderate.highestCobb"]) / 2
    severe = NUMBER["kyphosis.severe.meanCobb"]
    mean = [(0.0, NUMBER["kyphosis.straight.mean"]), (moderate, NUMBER["kyphosis.moderate.mean"]),
            (severe, NUMBER["kyphosis.severe.mean"])]
    sd = [(0.0, NUMBER["kyphosis.straight.sd"]), (moderate, NUMBER["kyphosis.moderate.sd"]),
          (severe, NUMBER["kyphosis.severe.sd"])]
    return mean, sd


def side_profiles(bones, rises, wedges):
    kyphosis_mean, kyphosis_sd = cohort_curves()
    trunk_sd = max(NUMBER["thoracicTilt.sd"], NUMBER["lumbarTilt.sd"])
    rows = []
    for curve in PROFILE_CURVES:
        k = between(kyphosis_mean, curve)
        lo = NUMBER["lordosis.straight.mean"]
        k_sd = between(kyphosis_sd, curve)
        lo_sd = NUMBER["lordosis.straight.sd"] if curve == 0 else NUMBER["lordosis.scoliosis.sd"]
        leans = side_profile(bones, rises, wedges, k, lo)
        # How much each level's lean moves per degree of kyphosis and of
        # lordosis, levelling included.
        per_k = [(a - b) / 2 for a, b in zip(side_profile(bones, rises, wedges, k + 1, lo),
                                             side_profile(bones, rises, wedges, k - 1, lo))]
        per_lo = [(a - b) / 2 for a, b in zip(side_profile(bones, rises, wedges, k, lo + 1),
                                              side_profile(bones, rises, wedges, k, lo - 1))]
        sds = [math.sqrt((gk * k_sd) ** 2 + (gl * lo_sd) ** 2 + trunk_sd ** 2) for gk, gl in zip(per_k, per_lo)]
        rows.append({"curveDeg": curve, "leans": leans, "sds": sds})
    return rows


def depth_spread():
    """Spread of depth over width between people, as a fraction of the mean:
    the root mean square of SD over mean in every level and sex reported."""
    fractions = []
    for level in ["T12", "L1", "L2", "L3", "L4", "L5"]:
        for sex in ["men", "women"]:
            key = f"depthToWidth.{level}.{sex}"
            fractions.append(NUMBER[f"{key}.sd"] / NUMBER[f"{key}.mean"])
    return math.sqrt(sum(f * f for f in fractions) / len(fractions))


def wedge_spread(bones):
    """Spread of a body's side wedge: the front and back heights' spreads taken
    as independent, across the body's depth, pooled over C7 to L5."""
    angles = []
    for bone in bones:
        level = bone["level"]
        height_sd = math.hypot(NUMBER[f"frontHeight.{level}.sd"], NUMBER[f"backHeight.{level}.sd"])
        angles.append(math.degrees(height_sd / (2 * bone["halfDepth"])))
    return math.sqrt(sum(a * a for a in angles) / len(angles))


def stacking_spread(bones):
    """How far each upper centre sits forward of the line through the lower
    centre along the pair's mean lean, as a fraction of the rise; root mean
    square over the seventeen pairs."""
    fractions = []
    for upper, lower in zip(bones, bones[1:]):
        forward = upper["referenceCentre"][0] - lower["referenceCentre"][0]
        rise = upper["referenceCentre"][2] - lower["referenceCentre"][2]
        mean = math.radians((upper["lean"] + lower["lean"]) / 2)
        fractions.append((forward - rise * math.tan(mean)) / rise)
    return math.sqrt(sum(f * f for f in fractions) / len(fractions))


def twist_rows():
    """Apical twist by curve size. Each band sits at its middle; the open top
    band sits where the study's mean curve puts it. No twist at no curve."""
    bands = []
    counted = 0.0
    lowest = NUMBER["twist.lowestCobb"]
    for band in range(1, 6):
        low = NUMBER.get(f"twist.band{band}.lowestCobb", lowest)
        middle = (low + NUMBER[f"twist.band{band}.highestCobb"]) / 2
        counted += middle * NUMBER[f"twist.band{band}.count"]
        bands.append((middle, band))
    total = sum(NUMBER[f"twist.band{band}.count"] for band in range(1, 7))
    top = (NUMBER["twist.meanCobb"] * total - counted) / NUMBER["twist.band6.count"]
    bands.append((top, 6))
    rows = [(0.0, 0.0, NUMBER["twist.band1.sd"])]
    rows += [(middle, NUMBER[f"twist.band{band}.mean"], NUMBER[f"twist.band{band}.sd"]) for middle, band in bands]
    return rows


def tenth(value):
    return round(value, 1) + 0.0


def thousandth(value):
    return round(value, 3) + 0.0


def main():
    bones, rises = read_bones()
    wedges = side_wedges(bones)
    depth_fraction = depth_spread()
    wedge_sd = wedge_spread(bones)
    profiles = side_profiles(bones, rises, wedges)
    typical = {
        "about": ABOUT,
        "sources": [{"key": key, "citation": citation, "gives": gives} for key, citation, gives in SOURCES],
        "bodies": [
            {
                "level": bone["level"],
                "halfWidthMm": bone["halfWidth"],
                "depthToWidth": {
                    "mean": thousandth(depth_to_width(bone)),
                    "sd": thousandth(depth_fraction * depth_to_width(bone)),
                },
                "sideWedgeDeg": {"mean": wedge, "sd": tenth(wedge_sd)},
            }
            for bone, wedge in zip(bones, wedges)
        ],
        "sideProfiles": [
            {
                "curveDeg": tenth(row["curveDeg"]),
                "forwardLeanDeg": [tenth(lean) for lean in row["leans"]],
                "sdDeg": [tenth(sd) for sd in row["sds"]],
            }
            for row in profiles
        ],
        "stackingSpread": thousandth(stacking_spread(bones)),
        "twistAtApex": [
            {"curveDeg": tenth(curve), "twistDeg": tenth(twist), "sdDeg": tenth(sd)}
            for curve, twist, sd in twist_rows()
        ],
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as handle:
        json.dump(typical, handle, ensure_ascii=False, indent=2)
        handle.write("\n")
    for row in typical["sideProfiles"]:
        leans = row["forwardLeanDeg"]
        rounded = [body["sideWedgeDeg"]["mean"] for body in typical["bodies"]]
        print(f"curve {row['curveDeg']:5.1f}: kyphosis {kyphosis(leans, rounded):5.1f}, "
              f"lordosis {lordosis(leans, rounded):5.1f}, T1-L5 line {chord_lean(leans, rises):+.2f} degrees")
    print(f"Wrote {os.path.relpath(OUT, ROOT)}")


if __name__ == "__main__":
    main()

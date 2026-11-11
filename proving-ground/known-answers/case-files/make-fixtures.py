"""Rebuild made-up refusal fixtures without using the engine under test.

Run with Python's standard library. JSON verdicts describe one deliberate
contract violation each. Archive limits are exercised by small files with
oversized declarations, or a genuinely highly compressed stream; no large
allocation is needed to reject either one.
"""

from copy import deepcopy
from io import BytesIO
import json
from pathlib import Path
import struct
import warnings
import zipfile
import zlib


ROOT = Path(__file__).resolve().parent
MINIMAL = json.loads((ROOT / "valid/minimal.json").read_text())
FOUR_FILMS = json.loads((ROOT / "valid/four-films.json").read_text())
VERDICTS = json.loads((ROOT / "verdicts.json").read_text())


def json_refusal(name, path, edit=None, base=MINIMAL, text=None, code="case-invalid-value"):
    """Write a deliberate mutation and its independently specified verdict."""
    file = f"invalid/{name}.json"
    if text is None:
        case = deepcopy(base)
        edit(case)
        text = json.dumps(case, indent=2) + "\n"
    (ROOT / file).write_text(text)
    VERDICTS[:] = [verdict for verdict in VERDICTS if verdict["file"] != file]
    VERDICTS.append(dict(file=file, verdict="refuse", code=code, detail=path))


json_refusal("not-json", "line 2 column 0", text="{\n")
json_refusal("nan", "line 1 column 1", text="NaN\n")
json_refusal("infinity", "line 1 column 1", text="Infinity\n")
json_refusal("wrong-format", "format", lambda c: c.update(format="different-case"), code="case-wrong-format")
json_refusal("version-zero", "version", lambda c: c.update(version=0))
json_refusal("missing-null", "films[0].keptTags", lambda c: c["films"][0].pop("keptTags"))
json_refusal("wrong-type", "films[0].image.width", lambda c: c["films"][0]["image"].update(width="1200"))
json_refusal("duplicate-film", "films[1].kind", lambda c: c["films"].append(deepcopy(c["films"][0])))
json_refusal("wrong-image-path", "films[0].image.path", lambda c: c["films"][0]["image"].update(path="films/side.png"))
for field, value, suffix in [("width", 0, "zero"), ("width", 16385, "over-limit"), ("height", 0, "zero"), ("height", 16385, "over-limit"), ("bitsStored", 0, "zero"), ("bitsStored", 17, "over-limit")]:
    json_refusal(f"{'bits-stored' if field == 'bitsStored' else field}-{suffix}", f"films[0].image.{field}", lambda c, field=field, value=value: c["films"][0]["image"].update({field: value}))
json_refusal("orphan-marks", "marks[0].film", lambda c: c["films"].pop(0), FOUR_FILMS)
json_refusal("duplicate-marks", "marks[3].film", lambda c: c["marks"].append(deepcopy(c["marks"][0])), FOUR_FILMS)
json_refusal("repeated-level", "marks[0].vertebrae[1].level", lambda c: c["marks"][0]["vertebrae"][1].update(level="L4"), FOUR_FILMS)
json_refusal("level-outside-template", "marks[0].vertebrae[0].level", lambda c: c["marks"][0]["vertebrae"][0].update(level="T13"), FOUR_FILMS)
json_refusal("extension-without-flag", "marks[0].vertebrae[0].level", lambda c: c["marks"][0]["vertebrae"][0].update(level="C6"), FOUR_FILMS)
json_refusal("short-top-count", "marks[0].naming.fromTop", lambda c: c["marks"][0]["naming"].update(fromTop=["L4"]), FOUR_FILMS)
json_refusal("reversed-bottom-count", "marks[0].naming.fromBottom", lambda c: c["marks"][0]["naming"].update(fromBottom=["L5", "L4"]), FOUR_FILMS)
json_refusal("top-count-outside-template", "marks[0].naming.fromTop", lambda c: c["marks"][0]["naming"].update(fromTop=["T13", "L5"]), FOUR_FILMS)
json_refusal("wrong-l5-confirmation", "marks[0].naming.l5ConfirmedOn", lambda c: c["marks"][0]["naming"].update(l5ConfirmedOn=1), FOUR_FILMS)
json_refusal("missing-c7", "marks[2].landmarks.c7", lambda c: c["marks"][2]["landmarks"].update(c7=None), FOUR_FILMS)
json_refusal("c7-hand-placed", "marks[2].landmarks.c7.source", lambda c: c["marks"][2]["landmarks"]["c7"].update(source="hand"), FOUR_FILMS)
json_refusal("c7-wrong-inference", "marks[2].landmarks.c7.inferred", lambda c: c["marks"][2]["landmarks"]["c7"].update(inferred=True), FOUR_FILMS)
json_refusal("blank-outside-film", "films[0].blanked[0]", lambda c: c["films"][0]["blanked"].append(dict(left=-1.0, top=0.0, right=1.0, bottom=1.0)))
json_refusal("blank-crossed-edges", "films[0].blanked[0]", lambda c: c["films"][0]["blanked"].append(dict(left=2.0, top=0.0, right=1.0, bottom=1.0)))
json_refusal("scale-too-small", "films[0].geometry.mmPerPx", lambda c: c["films"][0]["geometry"].update(mmPerPx=0.009), FOUR_FILMS)
json_refusal("scale-too-large", "films[0].geometry.mmPerPx", lambda c: c["films"][0]["geometry"].update(mmPerPx=2.001), FOUR_FILMS)
json_refusal("repeated-vertebra-id", "marks[1].vertebrae[0].id", lambda c: c["marks"][1]["vertebrae"][0].update(id=1), FOUR_FILMS)
json_refusal("reused-next-id", "nextVertebraId", lambda c: c.update(nextVertebraId=7), FOUR_FILMS)
json_refusal("long-app-version", "savedWith.app", lambda c: c["savedWith"].update(app="1" * 33))
json_refusal("app-version-free-text", "savedWith.app", lambda c: c["savedWith"].update(app="not a version"))
(ROOT / "verdicts.json").write_text(json.dumps(VERDICTS, indent=2) + "\n")


def chunk(kind, payload):
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload))


def png(width, height, bits, blanked=()):
    """A real PNG containing repeated, hand-defined samples at their own depth."""
    depth = 8 if bits <= 8 else 16
    values = [0, 1, (1 << bits) // 2 - 1, (1 << bits) - 1]
    samples = [(values[x % 4] << (depth - bits)) for x in range(width)]
    row = bytes(samples) if depth == 8 else b"".join(struct.pack(">H", sample) for sample in samples)
    rows = []
    for y in range(height):
        current = bytearray(row)
        for rect in blanked:
            if rect["top"] < y + 0.5 and y - 0.5 < rect["bottom"]:
                for x in range(width):
                    if rect["left"] < x + 0.5 and x - 0.5 < rect["right"]:
                        current[x * (depth // 8):(x + 1) * (depth // 8)] = b"\0" * (depth // 8)
        rows.append(b"\0" + current)
    ihdr = struct.pack(">IIBBBBB", width, height, depth, 0, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"sBIT", bytes([bits])) + chunk(b"IDAT", zlib.compress(b"".join(rows))) + chunk(b"IEND", b"")


README = b"Spine case format version 1. See docs/contracts/case-file-v1.md.\n"
ARCHIVE_VERDICTS = []


def archive_bytes(entries):
    result = BytesIO()
    with zipfile.ZipFile(result, "w") as archive:
        for name, payload, compression in entries:
            entry = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            entry.compress_type = compression
            entry.external_attr = 0o100644 << 16
            with warnings.catch_warnings():
                warnings.simplefilter("ignore", UserWarning)
                archive.writestr(entry, payload)
    return result.getvalue()


def entries_for(case, text=None):
    if text is None:
        text = json.dumps(case, indent=2) + "\n"
    return [("case.json", text.encode(), zipfile.ZIP_DEFLATED)] + [(film["image"]["path"], png(film["image"]["width"], film["image"]["height"], film["image"]["bitsStored"], film["blanked"]), zipfile.ZIP_STORED) for film in case["films"]] + [("README.txt", README, zipfile.ZIP_DEFLATED)]


def archive_fixture(name, code, detail=None, entries=None, raw=None):
    folder = "valid" if code == "accept" else "invalid"
    file = f"archives/{folder}/{name}.spinecase"
    path = ROOT / file
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(raw if raw is not None else archive_bytes(entries))
    ARCHIVE_VERDICTS.append((file, code, detail or ""))


def declared_size(data, name, size):
    """Alter matching local and central size fields, without allocating size bytes."""
    data = bytearray(data)
    name = name.encode()
    for signature, name_offset, size_offset in [(b"PK\x03\x04", 30, 22), (b"PK\x01\x02", 46, 24)]:
        cursor = 0
        while (at := data.find(signature, cursor)) >= 0:
            if data[at + name_offset:at + name_offset + len(name)] == name:
                struct.pack_into("<I", data, at + size_offset, size)
            cursor = at + len(signature)
    return bytes(data)


def zip_flag(data, local_at, central_at, value):
    data = bytearray(data)
    for signature, field in [(b"PK\x03\x04", local_at), (b"PK\x01\x02", central_at)]:
        at = data.index(signature)
        struct.pack_into("<H", data, at + field, value)
    return bytes(data)


minimal_entries = entries_for(MINIMAL, (ROOT / "valid/minimal.json").read_text())
four_entries = entries_for(FOUR_FILMS, (ROOT / "valid/four-films.json").read_text())
archive_fixture("minimal", "accept", entries=minimal_entries)
archive_fixture("four-films", "accept", entries=four_entries)
archive_fixture("not-a-zip", "case-not-a-zip", raw=b"not a zip\n")
archive_fixture("truncated-zip", "case-not-a-zip", raw=archive_bytes(minimal_entries)[:-22])
for name, entry_name in [("unknown-entry", "unexpected.bin"), ("parent-path", "../case.json"), ("absolute-path", "/case.json"), ("backslash-path", "films\\front.png"), ("directory-entry", "films/")]:
    archive_fixture(name, "case-unknown-entry", entries=minimal_entries + [(entry_name, b"", zipfile.ZIP_STORED)])
archive_fixture("duplicate-entry", "case-unknown-entry", entries=minimal_entries + [minimal_entries[0]])
archive_fixture("unlisted-film", "case-unknown-entry", "films/side.png", entries=minimal_entries + [("films/side.png", png(1, 1, 8), zipfile.ZIP_STORED)])
archive_fixture("missing-case", "case-invalid-value", "case.json", entries=minimal_entries[1:])
archive_fixture("missing-readme", "case-invalid-value", "README.txt", entries=minimal_entries[:-1])
archive_fixture("missing-film", "case-missing-film", "films/front.png", entries=[minimal_entries[0], minimal_entries[-1]])
archive_fixture("wrong-png-size", "case-film-size-mismatch", "films/front.png", entries=[minimal_entries[0], ("films/front.png", png(1, 1, 8), zipfile.ZIP_STORED), minimal_entries[-1]])
archive_fixture("wrong-png-depth", "case-film-size-mismatch", "films/front.png", entries=[minimal_entries[0], ("films/front.png", png(1200, 3000, 7), zipfile.ZIP_STORED), minimal_entries[-1]])
archive_fixture("corrupt-png", "case-invalid-value", "films/front.png", entries=[minimal_entries[0], ("films/front.png", b"not a png", zipfile.ZIP_STORED), minimal_entries[-1]])
archive_fixture("json-size-limit", "case-too-large", "case.json", raw=declared_size(archive_bytes(minimal_entries), "case.json", 8 * 1024 * 1024 + 1))
archive_fixture("readme-size-limit", "case-too-large", "README.txt", raw=declared_size(archive_bytes(minimal_entries), "README.txt", 4097))
archive_fixture("png-size-limit", "case-too-large", "films/front.png", raw=declared_size(archive_bytes(minimal_entries), "films/front.png", 128 * 1024 * 1024 + 1))
archive_fixture("compression-ratio", "case-too-large", "case.json", entries=[("case.json", b" " * (4 * 1024 * 1024), zipfile.ZIP_DEFLATED), *minimal_entries[1:]])
archive_fixture("encrypted-entry", "case-not-a-zip", raw=zip_flag(archive_bytes(minimal_entries), 6, 8, 1))
archive_fixture("unsupported-compression", "case-not-a-zip", raw=zip_flag(archive_bytes(minimal_entries), 8, 10, 99))
archive_fixture("local-path-substitution", "case-unknown-entry", raw=archive_bytes(minimal_entries).replace(b"films/front.png", b"films/other.png", 1))
stored_readme = archive_bytes([*minimal_entries[:-1], ("README.txt", README, zipfile.ZIP_STORED)])
archive_fixture("corrupt-entry-crc", "case-not-a-zip", raw=stored_readme.replace(README, b"X" + README[1:], 1))
archive_fixture("json-not-utf8", "case-invalid-value", "case.json", entries=[("case.json", b"\xff", zipfile.ZIP_STORED), minimal_entries[-1]])

large_case = deepcopy(MINIMAL)
large_case["films"][0]["image"].update(width=10000, height=5000)
# Header alone exceeds the decoded-pixel budget; the tiny IDAT must never
# be inflated into an image of that declared size.
large_png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 10000, 5000, 8, 0, 0, 0, 0)) + chunk(b"sBIT", b"\x08") + chunk(b"IDAT", zlib.compress(b"\0")) + chunk(b"IEND", b"")
archive_fixture("png-decode-budget", "case-too-large", "films/front.png", entries=[("case.json", json.dumps(large_case).encode(), zipfile.ZIP_DEFLATED), ("films/front.png", large_png, zipfile.ZIP_STORED), minimal_entries[-1]])

unblanked_case = deepcopy(MINIMAL)
unblanked_case["films"][0]["image"].update(width=2, height=1)
unblanked_case["films"][0]["blanked"] = [dict(left=0.5, top=-0.5, right=1.5, bottom=0.5)]
archive_fixture("nonzero-blanked-pixel", "case-invalid-value", "films/front.png", entries=[("case.json", json.dumps(unblanked_case).encode(), zipfile.ZIP_DEFLATED), ("films/front.png", png(2, 1, 8), zipfile.ZIP_STORED), minimal_entries[-1]])

low_bit_case = deepcopy(MINIMAL)
low_bit_case["films"][0]["image"].update(width=1, height=1, bitsStored=1)
low_bit_case["films"][0]["blanked"] = [dict(left=-0.5, top=-0.5, right=0.5, bottom=0.5)]
low_bit_png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 0, 0, 0, 0)) + chunk(b"sBIT", b"\x01") + chunk(b"IDAT", zlib.compress(b"\0\x7f")) + chunk(b"IEND", b"")
archive_fixture("blanked-low-bits", "case-invalid-value", "films/front.png", entries=[("case.json", json.dumps(low_bit_case).encode(), zipfile.ZIP_DEFLATED), ("films/front.png", low_bit_png, zipfile.ZIP_STORED), minimal_entries[-1]])

# A sentinel offset requires ZIP64 even though the classic count looks
# small. The real count is in the inserted ZIP64 end record.
zip64_base = archive_bytes(minimal_entries)
classic_end = bytearray(zip64_base[-22:])
directory_size, directory_offset = struct.unpack_from("<II", classic_end, 12)
struct.pack_into("<I", classic_end, 16, 0xffffffff)
zip64_end = struct.pack("<4sQHHIIQQQQ", b"PK\x06\x06", 44, 45, 45, 0, 0, 1 << 32, 1 << 32, directory_size, directory_offset)
zip64_locator = struct.pack("<4sIQI", b"PK\x06\x07", 0, len(zip64_base) - 22, 1)
archive_fixture("zip64-oversized-count", "case-too-large", "archive", raw=zip64_base[:-22] + zip64_end + zip64_locator + classic_end)

# The archive reader must pass every invalid JSON through the same strict
# reader. The film data are deliberately absent: JSON fails first.
for verdict in VERDICTS:
    if verdict["verdict"] == "refuse":
        text = (ROOT / verdict["file"]).read_bytes()
        archive_fixture("json-" + Path(verdict["file"]).stem, verdict["code"], verdict["detail"], entries=[("case.json", text, zipfile.ZIP_DEFLATED), ("README.txt", README, zipfile.ZIP_DEFLATED)])

(ROOT / "archive-verdicts.tsv").write_text("".join("\t".join(verdict) + "\n" for verdict in ARCHIVE_VERDICTS))

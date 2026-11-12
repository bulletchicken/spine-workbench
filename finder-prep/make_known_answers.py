"""Write the known answers the engine's trained finder is tested against.

    spine make-up straight --seed 7 --tier t2 --out <folder>
    python finder-prep/make_known_answers.py <training-folder> \
        <folder>/straight-7-t2/front.png engine/trained-finder/tests/known-answers

Runs the training run's own Python pipeline on one MADE-UP front film and
records what every stage gave: the two pictures the network was shown,
its maps, the peaks, the frame, the chain's picks and the settled
corners. The engine's port must give the same, stage by stage.

Only a made-up film is ever used here. A real film, or anything worked
out from one, never goes into the repository.

The made-up film is a 16-bit PNG and the network reads 8-bit pictures, so
this script keeps each value's top eight bits. That rule belongs to these
fixtures alone: it is not how the engine reads a deep film.

Written out:

    manifest.json     each saved array's type, shape and SHA-256, and for
                      every block of the network its sums, range and a
                      small patch of numbers
    pipeline.json     peaks, outlines, frame, chain and corners
    chain-cases.json  the chain alone, on peaks with one taken away
    *_f32.bin         the heat maps, little-endian
    *_u8.bin          the exact pictures and patches (not committed: their
                      SHA-256 in the manifest is what the tests check)
"""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import torch
from PIL import Image

training, source, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
sys.dont_write_bytecode = True
sys.path.insert(0, str(training))

from v2.chain.select import select  # noqa: E402
from v2.common import v1  # noqa: E402
from v2.corners import refine as corner_refiner  # noqa: E402
from v2.final import predict as final  # noqa: E402
from v2.fulllength.frame import frame_from_pass  # noqa: E402

out.mkdir(parents=True, exist_ok=True)
manifest = {
    "film": "spine make-up straight --seed 7 --tier t2, the front film",
    "filmSha256": hashlib.sha256(source.read_bytes()).hexdigest(),
    "toEightBits": "each 16-bit value shifted right by eight",
    "blobs": {},
}


def blob(name, array):
    data = np.ascontiguousarray(array)
    data = data.astype(data.dtype.newbyteorder("<"))
    (out / f"{name}.bin").write_bytes(data.tobytes())
    manifest["blobs"][name] = {
        "type": str(data.dtype.name),
        "shape": list(data.shape),
        "sha256": hashlib.sha256(data.tobytes()).hexdigest(),
    }


def stats(tensor):
    a = tensor.detach().double().numpy()[0]
    channels, height, width = a.shape
    row, column = height // 2, width // 2
    return {
        "shape": [channels, height, width],
        "sum": float(a.sum()),
        "absSum": float(np.abs(a).sum()),
        "squareSum": float((a * a).sum()),
        "min": float(a.min()),
        "max": float(a.max()),
        "patchAt": [row - 2, column - 2],
        "patch": a[:4, row - 2 : row + 2, column - 2 : column + 2].tolist(),
    }


with Image.open(source) as opened:
    values = np.asarray(opened)
film = Image.fromarray((values >> 8).astype(np.uint8) if values.dtype == np.uint16 else values.astype(np.uint8), "L")

net, info = final.load_model("cpu")
captured = {}
BLOCKS = ["e1", "e2", "e3", "e4", "e5", "d4", "d3", "d2", "heat", "corners", "sub"]
for name in BLOCKS:
    getattr(net, name).register_forward_hook(lambda module, given, got, name=name: captured.__setitem__(name, got))


def reading(tag, picture):
    small = np.asarray(picture.convert("L").resize((384, 768), Image.Resampling.BILINEAR))
    blob(f"{tag}_input_u8", small)
    x = torch.from_numpy(small.astype(np.float32) / 255)[None, None] - 0.5
    with torch.inference_mode():
        heads = net(x)
        decoded = v1.decode_outputs(heads, info["threshold"])[0]
    blob(f"{tag}_heat_logits_f32", heads[0][0, 0].numpy())
    manifest[f"{tag}Stats"] = {name: stats(captured[name]) for name in BLOCKS}
    return decoded


first = reading("pass1", film)
frame = frame_from_pass(film.size, first)
second = first if frame.unchanged else reading("pass2", frame.crop(film))
chain = select(second["peaks"])
mapped = frame.map_outlines(chain)
quads = np.asarray([link["corners"] for link in mapped]) * film.size
cuts = [corner_refiner.crop(film, quad) for quad in quads]
corner_net, blend = corner_refiner.load_model(str(final.HERE / "corners.pt"), "cpu")
patches = np.stack([cut[0] for cut in cuts])
initial = np.stack([cut[1] for cut in cuts])
blob("refiner_patches_u8", patches)
with torch.inference_mode():
    deltas = corner_net(torch.from_numpy(patches.copy()).unsqueeze(1).float() / 255, torch.from_numpy(initial)).numpy()
settled = np.stack([corner_refiner.guarded_update(quad, delta, cut[3], blend) for quad, delta, cut in zip(quads, deltas, cuts)])

whole = final.run_pipeline(film, device="cpu")
mine = sorted((settled / film.size).tolist(), key=lambda corners: np.mean(corners, axis=0)[1])
assert np.array_equal(np.asarray(mine), np.asarray([o["corners"] for o in whole["outlines"]]).reshape(-1, 4, 2))

record = {
    "filmSize": list(film.size),
    "threshold": info["threshold"],
    "pass1": first,
    "frame": {"box": list(frame.box), "reason": frame.reason, "unchanged": frame.unchanged},
    "pass2": None if frame.unchanged else second,
    "chainInCropFractions": chain,
    "chainInFilmFractions": mapped,
    "corners": {
        "blend": blend,
        "initialInPatchUnits": initial.tolist(),
        "deltasInPatchUnits": deltas.tolist(),
        "patchGeometry": [{"centre": cut[3][0].tolist(), "axes": cut[3][1].tolist(), "extent": cut[3][2]} for cut in cuts],
        "settledInFilmPixels": settled.tolist(),
    },
    "final": whole,
}
(out / "pipeline.json").write_text(json.dumps(record, allow_nan=False))

# The chain alone: the seventeen peaks it picked, with one taken away, must
# come back as seventeen with that one filled in.
picked = [second["peaks"][link["peak_index"]] for link in chain]
cases = [{"name": "nothing to choose from", "peaks": [], "expected": select([])}]
for gone in (4, 8, 12):
    peaks = [peak for index, peak in enumerate(picked) if index != gone]
    cases.append({"name": f"the picked peaks without number {gone + 1}", "peaks": peaks, "expected": select(peaks)})
(out / "chain-cases.json").write_text(json.dumps(cases, allow_nan=False))

(out / "manifest.json").write_text(json.dumps(manifest, indent=1, allow_nan=False) + "\n")
print(f"frame {frame.reason} {list(frame.box)}; {len(first['peaks'])} then {len(second['peaks'])} peaks; chain of {len(chain)}")

"""Pack the trained finder's numbers into the flat file the engine compiles in.

    python finder-prep/pack_weights.py <training-folder> engine/trained-finder/weights

Reads the two checkpoints the training run froze (`v2/final/best.pt`, the
main network, and `v2/final/corners.pt`, the corner network) and writes:

    finder-weights.bin    every tensor as little-endian float32, end to end
    finder-weights.json   each tensor's name, shape and place in the .bin
    chain-priors.json     the chain's priors, byte for byte as trained
    framing.json          the framing settings, byte for byte as trained

The main network's batch norms are folded into the convolutions before
them, in float64, and rounded to float32 once: the engine then runs one
convolution where training ran a convolution and a normalisation. The
corner network uses group norm, which depends on the picture and cannot
be folded, so its tensors are written as they are.

Run by hand when the weights change. No build runs it.
"""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import torch

BATCH_NORM_EPS = 1e-5
BLOCKS = ["e1", "e2", "e3", "e4", "e5", "d4", "d3", "d2"]
HEADS = ["heat", "corners", "sub"]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def folded(net, block, conv, norm):
    """One convolution and the batch norm after it, as a weight and a bias."""
    weight = net[f"{block}.{conv}.weight"].double().numpy()
    gamma = net[f"{block}.{norm}.weight"].double().numpy()
    beta = net[f"{block}.{norm}.bias"].double().numpy()
    mean = net[f"{block}.{norm}.running_mean"].double().numpy()
    variance = net[f"{block}.{norm}.running_var"].double().numpy()
    scale = gamma / np.sqrt(variance + BATCH_NORM_EPS)
    return weight * scale[:, None, None, None], beta - mean * scale


def main():
    training, out = Path(sys.argv[1]), Path(sys.argv[2])
    final = training / "v2" / "final"
    out.mkdir(parents=True, exist_ok=True)

    spine = torch.load(final / "best.pt", map_location="cpu", weights_only=True)
    corner = torch.load(final / "corners.pt", map_location="cpu", weights_only=True)
    if corner["architecture"] != "dense":
        raise SystemExit("the corner network is not the dense one this packer knows")

    tensors = []
    for block in BLOCKS:
        for conv, norm in ((0, 1), (3, 4)):
            weight, bias = folded(spine["net"], block, conv, norm)
            tensors.append((f"spine-net/{block}.{conv}.weight", weight))
            tensors.append((f"spine-net/{block}.{conv}.bias", bias))
    for head in HEADS:
        for layer in (0, 2):
            for part in ("weight", "bias"):
                name = f"{head}.{layer}.{part}"
                tensors.append((f"spine-net/{name}", spine["net"][name].numpy()))
    for name, tensor in corner["net"].items():
        tensors.append((f"corner-net/{name}", tensor.numpy()))

    table, blob, offset = [], bytearray(), 0
    for name, values in tensors:
        data = np.ascontiguousarray(values, dtype="<f4")
        table.append({"name": name, "shape": list(data.shape), "offset": offset, "count": int(data.size)})
        blob += data.tobytes()
        offset += data.nbytes
    (out / "finder-weights.bin").write_bytes(blob)

    for source, name in (("chain/model.json", "chain-priors.json"), ("fulllength/parameters.json", "framing.json")):
        (out / name).write_bytes((training / "v2" / source).read_bytes())

    described = {
        "format": "spine-finder-weights",
        "version": 1,
        "values": "float32, little-endian, each tensor in row-major order",
        "batchNormFolded": True,
        "batchNormEps": BATCH_NORM_EPS,
        "threshold": float(spine["thr"]),
        "cornerBlend": float(corner["blend"]),
        "bytes": len(blob),
        "sha256": {
            "finder-weights.bin": hashlib.sha256(blob).hexdigest(),
            "chain-priors.json": sha256(out / "chain-priors.json"),
            "framing.json": sha256(out / "framing.json"),
        },
        "packedFrom": {"best.pt": sha256(final / "best.pt"), "corners.pt": sha256(final / "corners.pt")},
        "tensors": table,
    }
    (out / "finder-weights.json").write_text(json.dumps(described, indent=1) + "\n")
    print(f"{len(table)} tensors, {len(blob):,} bytes")


if __name__ == "__main__":
    main()

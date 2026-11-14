"""Pack one fixed side checkpoint. No build invokes this command."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import struct
import sys
import tempfile

import numpy as np
import torch

sys.dont_write_bytecode = True
CHECKPOINT = "side_lumbar_v1.pt"
CHECKPOINT_SHA256 = "a11a131c0f17eb6b7dc56c5c8d7e51b3a57db54aa576d74671d863a6dbfe0763"
SELECTED_SHA256 = "08206eb911668957b0ea1b985e4ce06865f43b1652b5274133df6419c4124444"
BATCH_NORM_EPS = 1e-5
BLOCKS = (("e1", 1, 24), ("e2", 24, 48), ("e3", 48, 96),
          ("e4", 96, 160), ("e5", 160, 224), ("d4", 384, 160),
          ("d3", 256, 96), ("d2", 144, 64))
MODEL_SCHEMA = {
    "name": "side_lumbar_heatmap_v1", "schema_version": 1,
    "input_shape": [1, 512, 384], "input_range": [0.0, 1.0],
    "internal_centering": "forward subtracts 0.5 exactly once; callers pass uncentered [0,1]",
    "input_view": "left-facing lateral lumbar radiograph",
    "levels": ["L1", "L2", "L3", "L4", "L5"],
    "corners": ["supA", "supP", "infA", "infP"],
    "output": "20 unnormalized landmark heatmap logits, level-major",
    "output_stride": 4, "heatmap_shape": [20, 128, 96],
    "heatmap_pixel_center": "(index + 0.5) * stride - 0.5",
    "decoder": "7x7 argmax-local spatial softmax centroid",
    "s1_head": False, "horizontal_flip_augmentation": False,
    "scope": "named L1-L5 landmarks; no absent-level or general whole-spine detection claim",
}
CHECKPOINT_KEYS = {"model", "model_schema", "epoch", "validation", "training_data_hashes",
                   "selected_run", "source_checkpoint_sha256", "training_seed", "selection",
                   "dataset", "data_source_url", "requires_review"}


def json_bytes(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n").encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def regular_bytes(path):
    path = Path(path)
    if not stat.S_ISREG(path.lstat().st_mode):
        raise ValueError(f"Expected a regular, non-symlink file: {path.name}")
    return path.read_bytes()


def read_json(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"Duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(regular_bytes(path), object_pairs_hook=unique,
                      parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)))


def tensor_shapes():
    """Packet order, also used by the seeded synthetic weight stream."""
    shapes = {}
    for block, inputs, outputs in BLOCKS:
        for conv, channels in ((0, inputs), (3, outputs)):
            shapes[f"{block}.{conv}.weight"] = (outputs, channels, 3, 3)
            shapes[f"{block}.{conv}.bias"] = (outputs,)
    shapes.update({"landmarks.0.weight": (64, 64, 3, 3), "landmarks.0.bias": (64,),
                   "landmarks.2.weight": (20, 64, 1, 1), "landmarks.2.bias": (20,)})
    return shapes


def state_shapes():
    shapes = {}
    for block, inputs, outputs in BLOCKS:
        for conv, norm, channels in ((0, 1, inputs), (3, 4, outputs)):
            shapes[f"{block}.{conv}.weight"] = ((outputs, channels, 3, 3), torch.float32)
            for part in ("weight", "bias", "running_mean", "running_var"):
                shapes[f"{block}.{norm}.{part}"] = ((outputs,), torch.float32)
            shapes[f"{block}.{norm}.num_batches_tracked"] = ((), torch.int64)
    for name, shape in tensor_shapes().items():
        if name.startswith("landmarks."):
            shapes[name] = (shape, torch.float32)
    return shapes


def validate_state(state):
    expected = state_shapes()
    if type(state) is not dict or state.keys() != expected.keys():
        raise ValueError("Unexpected, missing or unknown state keys")
    for name, (shape, dtype) in expected.items():
        value = state[name]
        if (type(value) is not torch.Tensor or value.device.type != "cpu"
                or value.layout != torch.strided or value.dtype != dtype
                or tuple(value.shape) != shape):
            raise ValueError(f"Wrong tensor type, shape or device: {name}")
        if not bool(torch.isfinite(value).all()):
            raise ValueError(f"Nonfinite tensor: {name}")
        if name.endswith("num_batches_tracked") and value.item() < 0:
            raise ValueError(f"Negative batch counter: {name}")
        if name.endswith("running_var") and bool((value < 0).any()):
            raise ValueError(f"Negative variance: {name}")


def validate_checkpoint(saved):
    if type(saved) is not dict or saved.keys() != CHECKPOINT_KEYS:
        raise ValueError("Unexpected checkpoint fields")
    if json_bytes(saved["model_schema"]) != json_bytes(MODEL_SCHEMA):
        raise ValueError("Checkpoint schema differs from the fixed side model")
    if saved["selected_run"] != "control" or saved["source_checkpoint_sha256"] != SELECTED_SHA256:
        raise ValueError("Checkpoint is not the selected export")
    validate_state(saved["model"])
    return saved["model"]


def load_checkpoint(model_dir):
    root = Path(model_dir).resolve(strict=True)
    selection = read_json(root / "selection.json")
    card = read_json(root / "model_card.json")
    selected, exported = selection["selected"], selection["export"]
    if (selected["name"] != "control" or selected["sha256"] != SELECTED_SHA256
            or json_bytes(selected["model_schema"]) != json_bytes(MODEL_SCHEMA)
            or exported["sha256"] != CHECKPOINT_SHA256
            or exported["state_dict_bitwise_equal_to_selected_training_checkpoint"] is not True
            or card["artifact"] != CHECKPOINT or card["artifact_sha256"] != CHECKPOINT_SHA256
            or card["training"]["selected_run"] != "control"
            or json_bytes(card["architecture"]) != json_bytes(MODEL_SCHEMA)):
        raise ValueError("Selection or model card differs from the fixed export")
    raw = regular_bytes(root / CHECKPOINT)
    if digest(raw) != CHECKPOINT_SHA256 or len(raw) != exported["bytes"]:
        raise ValueError("Exported checkpoint hash or byte count differs")
    saved = torch.load(io.BytesIO(raw), map_location="cpu", weights_only=True)
    return validate_checkpoint(saved)


def folded_tensors(state):
    validate_state(state)
    tensors = {}
    for block, _, _ in BLOCKS:
        for conv, norm in ((0, 1), (3, 4)):
            prefix = f"{block}.{norm}"
            scale = (state[f"{prefix}.weight"].double().numpy()
                     / np.sqrt(state[f"{prefix}.running_var"].double().numpy() + BATCH_NORM_EPS))
            weight = state[f"{block}.{conv}.weight"].double().numpy() * scale[:, None, None, None]
            bias = (state[f"{prefix}.bias"].double().numpy()
                    - state[f"{prefix}.running_mean"].double().numpy() * scale)
            tensors[f"{block}.{conv}.weight"] = np.ascontiguousarray(weight, dtype="<f4")
            tensors[f"{block}.{conv}.bias"] = np.ascontiguousarray(bias, dtype="<f4")
    for name in tensor_shapes():
        if name.startswith("landmarks."):
            tensors[name] = np.ascontiguousarray(state[name].numpy(), dtype="<f4")
    return tensors


def packet(tensors):
    if tensors.keys() != tensor_shapes().keys():
        raise ValueError("Unexpected packed tensor names")
    table, parts, offset = [], [], 0
    for name, shape in tensor_shapes().items():
        values = tensors[name]
        if (not isinstance(values, np.ndarray) or values.shape != shape
                or values.dtype != np.dtype("<f4") or not np.isfinite(values).all()):
            raise ValueError(f"Invalid folded tensor: {name}")
        raw = values.tobytes(order="C")
        table.append({"name": name, "shape": list(shape), "offset": offset, "count": values.size})
        parts.append(raw)
        offset += len(raw)
    if len(table) != 36 or sum(row["count"] for row in table) != 2_550_972 or offset != 10_203_888:
        raise ValueError("Calculated folded tensor count differs")
    payload = b"".join(parts)
    header = {"format": "side-finder-weights", "version": 1, "model": MODEL_SCHEMA["name"],
              "checkpointSha256": CHECKPOINT_SHA256, "batchNormEps": BATCH_NORM_EPS,
              "batchNormFolded": True, "inputShape": [1, 512, 384], "outputShape": [20, 128, 96],
              "bytes": offset, "payloadSha256": digest(payload), "tensors": table}
    encoded = json_bytes(header)
    return b"SWSIDE01" + struct.pack("<I", len(encoded)) + encoded + payload, header


def atomic_write(out, data, model_dir):
    out = Path(out).absolute()
    root = Path(model_dir).resolve(strict=True)
    for path in (out, *out.parents):
        if path.is_symlink():
            raise ValueError("Output must not traverse a symlink")
    if out.resolve().is_relative_to(root):
        raise ValueError("Output must be outside the read-only model directory")
    if out.exists() and not out.is_file():
        raise ValueError("Output must be a regular file")
    out.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(dir=out.parent, prefix=f".{out.name}.", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, out)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model-dir", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    data, header = packet(folded_tensors(load_checkpoint(args.model_dir)))
    atomic_write(args.out, data, args.model_dir)
    print(json.dumps({"tensors": len(header["tensors"]), "values": header["bytes"] // 4,
                      "payloadBytes": header["bytes"], "packetBytes": len(data),
                      "payloadSha256": header["payloadSha256"], "packetSha256": digest(data)}))


if __name__ == "__main__":
    main()

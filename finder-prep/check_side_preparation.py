"""Offline synthetic checks. No checkpoint, reference directory or film is needed."""
from __future__ import annotations

import copy
import importlib.util
import json
import math
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
import numpy as np
import torch
from torch.nn import functional as F

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
tempfile.tempdir = str(Path(tempfile.gettempdir()).resolve())


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


pack = module("checked_side_packet", HERE / "pack_side_weights.py")
answers = module("checked_side_answers", HERE / "make_side_known_answers.py")


def synthetic_state():
    state = {}
    for name, (shape, dtype) in pack.state_shapes().items():
        value = torch.zeros(shape, dtype=dtype)
        if name.endswith("running_var") or (name.endswith("weight") and len(shape) == 1):
            value.fill_(1)
        state[name] = value
    return state


class PreparationChecks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        torch.set_num_threads(3)
        torch.set_deterministic_debug_mode("error")
        cls.tensors = answers.synthetic_tensors()
        cls.fixture = json.loads((ROOT / "engine/trained-finder/tests/side-known-answers/synthetic.json").read_bytes())

    def test_bn_fold_float64_once_and_heads_unchanged(self):
        state = synthetic_state()
        state["e1.0.weight"].fill_(7)
        for key, value in (("weight", 2), ("bias", 3), ("running_mean", 5), ("running_var", 4)):
            state[f"e1.1.{key}"].fill_(value)
        state["landmarks.0.weight"].fill_(.125)
        state["landmarks.2.bias"].fill_(-.25)
        result = pack.folded_tensors(state)
        expected_weight = np.float32(14 / math.sqrt(4.00001))
        expected_bias = np.float32(3 - 10 / math.sqrt(4.00001))
        self.assertTrue(np.all(result["e1.0.weight"] == expected_weight))
        self.assertTrue(np.all(result["e1.0.bias"] == expected_bias))
        self.assertTrue(np.all(result["landmarks.0.weight"] == .125))
        self.assertTrue(np.all(result["landmarks.2.bias"] == -.25))
        self.assertEqual(len(result), 36)
        print(f"BN max absolute error: 0; weight={expected_weight}; bias={expected_bias}")

    def test_state_refusals(self):
        state = synthetic_state()
        cases = []
        bad = state.copy(); bad["unknown"] = torch.zeros(1); cases.append(bad)
        bad = state.copy(); del bad["e1.0.weight"]; cases.append(bad)
        for replacement in (torch.zeros(1), torch.zeros(24, 1, 3, 3, dtype=torch.float64),
                            np.zeros((24, 1, 3, 3), dtype=np.float32)):
            bad = state.copy(); bad["e1.0.weight"] = replacement; cases.append(bad)
        for value in (float("nan"), float("inf"), -float("inf")):
            bad = state.copy(); bad["e1.1.bias"] = torch.full((24,), value); cases.append(bad)
        for replacement in (torch.tensor(-1), torch.tensor(0.), torch.zeros(1, dtype=torch.int64)):
            bad = state.copy(); bad["e1.1.num_batches_tracked"] = replacement; cases.append(bad)
        bad = state.copy(); bad["e1.1.running_var"] = torch.full((24,), -1.); cases.append(bad)
        for bad in cases:
            with self.subTest(case=len(bad)):
                with self.assertRaises(ValueError):
                    pack.folded_tensors(bad)

    def test_checkpoint_schema_and_selection_refusals(self):
        saved = dict.fromkeys(pack.CHECKPOINT_KEYS)
        saved.update(model=synthetic_state(), model_schema=copy.deepcopy(pack.MODEL_SCHEMA),
                     selected_run="control", source_checkpoint_sha256=pack.SELECTED_SHA256)
        pack.validate_checkpoint(saved)
        for change in ({"selected_run": "context"}, {"source_checkpoint_sha256": "0" * 64},
                       {"unexpected": 0}, {"model_schema": {"name": pack.MODEL_SCHEMA["name"]}}):
            with self.assertRaises(ValueError):
                pack.validate_checkpoint(saved | change)
        for field, value in (("s1_head", 0), ("input_shape", [1, 128, 128]), ("extra", True)):
            schema = saved["model_schema"] | {field: value}
            with self.assertRaises(ValueError):
                pack.validate_checkpoint(saved | {"model_schema": schema})

    def test_hash_refusal_before_any_torch_deserialisation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw = b"synthetic corrupt bytes, never a pickle payload"
            (root / pack.CHECKPOINT).write_bytes(raw)
            selection = {"selected": {"name": "control", "sha256": pack.SELECTED_SHA256,
                                      "model_schema": pack.MODEL_SCHEMA},
                         "export": {"sha256": pack.CHECKPOINT_SHA256, "bytes": len(raw),
                                    "state_dict_bitwise_equal_to_selected_training_checkpoint": True}}
            card = {"artifact": pack.CHECKPOINT, "artifact_sha256": pack.CHECKPOINT_SHA256,
                    "architecture": pack.MODEL_SCHEMA, "training": {"selected_run": "control"}}
            (root / "selection.json").write_bytes(pack.json_bytes(selection))
            (root / "model_card.json").write_bytes(pack.json_bytes(card))
            with patch.object(torch, "load", side_effect=AssertionError("Must not deserialise")) as load:
                with self.assertRaisesRegex(ValueError, "hash or byte count"):
                    pack.load_checkpoint(root)
                load.assert_not_called()
            selection["selected"]["name"] = "context"
            (root / "selection.json").write_bytes(pack.json_bytes(selection))
            with self.assertRaisesRegex(ValueError, "Selection"):
                pack.load_checkpoint(root)

    def test_packet_order_counts_bytes_and_determinism(self):
        first, header = pack.packet(self.tensors)
        second, _ = pack.packet(answers.synthetic_tensors())
        self.assertEqual(first, second)
        self.assertEqual(first[:8], b"SWSIDE01")
        size, = struct.unpack("<I", first[8:12])
        self.assertEqual(json.loads(first[12:12+size]), header)
        self.assertEqual(set(header), {"format", "version", "model", "checkpointSha256", "batchNormEps",
                                      "batchNormFolded", "inputShape", "outputShape", "bytes", "payloadSha256", "tensors"})
        payload = first[12+size:]
        self.assertEqual(pack.digest(payload), header["payloadSha256"])
        self.assertEqual(header["payloadSha256"], self.fixture["weights"]["payloadSha256"])
        self.assertEqual(len(header["tensors"]), 36)
        self.assertEqual(len(payload), 10_203_888)
        self.assertEqual(sum(row["count"] for row in header["tensors"]), 2_550_972)
        offset = 0
        for row, (name, shape) in zip(header["tensors"], pack.tensor_shapes().items()):
            self.assertEqual(set(row), {"name", "shape", "offset", "count"})
            self.assertEqual((row["name"], row["shape"], row["offset"]), (name, list(shape), offset))
            count = math.prod(shape)
            self.assertEqual(row["count"], count)
            self.assertEqual(payload[offset:offset+count*4], self.tensors[name].astype("<f4").tobytes())
            offset += count * 4
        self.assertEqual(offset, len(payload))
        for replacement in (np.zeros(1, dtype="<f4"), np.full((24,), np.nan, dtype="<f4")):
            bad = self.tensors | {"e1.0.bias": replacement}
            with self.assertRaises(ValueError):
                pack.packet(bad)
        with self.assertRaises(ValueError):
            pack.packet(self.tensors | {"unknown": np.zeros(1, dtype="<f4")})

    def test_xorshift_and_normalisation(self):
        for seed, expected in ((17, [0, 68, 84, 1, 41, 69, 221, 222, 88, 43, 141, 115]),
                               (20261113, [33, 48, 118, 115, 221, 146, 14, 178, 119, 87, 157, 161])):
            stream = answers.xorshift32(seed)
            self.assertEqual([next(stream) >> 24 for _ in expected], expected)
        raw = answers.input_bytes(128, 128)
        self.assertEqual(pack.digest(raw.tobytes()), self.fixture["input"]["bytesSha256"])
        centred = raw.astype(np.float32) / np.float32(255) - np.float32(.5)
        self.assertEqual(pack.digest(centred.astype("<f4").tobytes()), self.fixture["input"]["centredF32Sha256"])
        self.assertEqual(self.tensors["e1.0.weight"].flat[0], -128 * 2**-14)
        self.assertTrue(all(np.all(values == 0) for name, values in self.tensors.items() if name.endswith("bias")))

    def test_entire_synthetic_network_and_independent_centroid(self):
        recorded = {}
        tensors = {name: torch.from_numpy(values) for name, values in self.tensors.items()}
        def conv(value, name, stride=1):
            kernel = tensors[f"{name}.weight"]
            value = F.conv2d(value, kernel, tensors[f"{name}.bias"], stride=stride,
                             padding=1 if kernel.shape[-1] == 3 else 0)
            if name != "landmarks.2":
                value = F.relu(value)
            recorded[name] = answers.activation(value)
            return value
        def block(value, name, stride=1):
            return conv(conv(value, f"{name}.0", stride), f"{name}.3")
        raw = answers.input_bytes(128, 128)
        x = torch.from_numpy(raw.astype(np.float32) / 255 - .5)[None, None]
        with torch.inference_mode():
            a = block(x, "e1", 2); b = block(a, "e2", 2); c = block(b, "e3", 2)
            d = block(c, "e4", 2); e = block(d, "e5", 2)
            def up(value, skip):
                return torch.cat((F.interpolate(value, scale_factor=2, mode="bilinear", align_corners=False), skip), 1)
            value = block(up(e, d), "d4"); value = block(up(value, c), "d3")
            value = block(up(value, b), "d2")
            logits = conv(conv(value, "landmarks.0"), "landmarks.2")[0].numpy()
        self.assertEqual(recorded, self.fixture["layers"])
        points = []
        for heatmap in logits:
            y, x = np.unravel_index(heatmap.argmax(), heatmap.shape)
            local = heatmap[max(0,y-3):y+4, max(0,x-3):x+4].astype(np.float64)
            weights = np.exp(local - local.max()); weights /= weights.sum()
            yy, xx = np.indices(local.shape)
            points.append([((weights*(xx+max(0,x-3))).sum()+.5)*4-.5,
                           ((weights*(yy+max(0,y-3))).sum()+.5)*4-.5])
        error = float(np.max(np.abs(np.asarray(points).reshape(5,4,2) - self.fixture["points"])))
        self.assertLessEqual(error, .01)
        print(f"Synthetic activation maximum error: 0; independent centroid maximum: {error:.9g} model px")

    def test_resize_byte_records(self):
        from PIL import Image
        import PIL
        self.assertEqual(PIL.__version__, self.fixture["resize"]["pillow"])
        for case in self.fixture["resize"]["cases"]:
            width, height = case["inputSize"]
            target_width, target_height = case["targetSize"]
            raw = np.array(case["inputBytes"], dtype=np.uint8).reshape(height, width)
            ratio = min(target_width/width, target_height/height)
            resized_width, resized_height = max(1, round(width*ratio)), max(1, round(height*ratio))
            left, top = (target_width-resized_width)//2, (target_height-resized_height)//2
            expected = np.zeros((target_height, target_width), dtype=np.uint8)
            expected[top:top+resized_height,left:left+resized_width] = np.asarray(
                Image.fromarray(raw).resize((resized_width,resized_height), Image.Resampling.BILINEAR))
            self.assertEqual(expected.reshape(-1).tolist(), case["outputBytes"])
            self.assertEqual(pack.digest(expected.tobytes()), case["outputSha256"])
            self.assertEqual([resized_width,resized_height], case["resizedSize"])
            self.assertEqual([left,top,target_width-resized_width-left,target_height-resized_height-top], case["padding"])
            affine, inverse = np.array(case["affine"]), np.array(case["inverseAffine"])
            self.assertLessEqual(float(np.max(np.abs(inverse @ affine - np.eye(3)))), 1e-9)
        self.assertEqual(self.fixture["resize"]["cases"][5]["resizedSize"][0], 2)
        self.assertEqual(self.fixture["resize"]["cases"][6]["resizedSize"][0], 4)

    def test_atomic_output_isolated_and_refuses_source_and_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); source = root/"source"; source.mkdir()
            out = root/"out"/"answer.json"
            pack.atomic_write(out, b"first", source)
            pack.atomic_write(out, b"second", source)
            self.assertEqual(out.read_bytes(), b"second")
            self.assertEqual(sorted(str(p.relative_to(root)) for p in root.rglob("*") if p.is_file()), ["out/answer.json"])
            with self.assertRaises(ValueError):
                pack.atomic_write(source/"forbidden", b"no", source)
            link = root/"link"; link.symlink_to(out)
            with self.assertRaises(ValueError):
                pack.atomic_write(link, b"no", source)
            with self.assertRaises(ValueError):
                pack.regular_bytes(link)
            with self.assertRaises(ValueError):
                pack.atomic_write(root/"out", b"no", source)

    def test_isolated_cli_imports_without_data_access_or_writes(self):
        with tempfile.TemporaryDirectory() as directory:
            for name in ("pack_side_weights.py", "make_side_known_answers.py"):
                result = subprocess.run([sys.executable, "-I", "-B", str(HERE/name), "--help"],
                                        cwd=directory, capture_output=True, text=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("--model-dir", result.stdout)
                self.assertIn("--out", result.stdout)
            # Importing either preparation module must not execute its CLI, read a
            # checkpoint or call an image/dataset loader. Audit begins after torch imports.
            script = '''
import importlib.util, pathlib, sys, torch, numpy, PIL.Image
def audit(event, args):
    if event == 'open':
        path, mode, flags = args
        if isinstance(path, (str, bytes)):
            p = pathlib.Path(path)
            if p.suffix in ('.pt', '.png', '.jpg', '.npz') or 'data' in p.parts:
                raise AssertionError('Unexpected data read')
            if isinstance(mode, str) and any(c in mode for c in 'wax+'):
                raise AssertionError('Unexpected write')
sys.addaudithook(audit)
for i, path in enumerate(sys.argv[1:]):
    spec = importlib.util.spec_from_file_location('isolated'+str(i), path)
    spec.loader.exec_module(importlib.util.module_from_spec(spec))
print('isolated imports: no data read or write')
'''
            result = subprocess.run([sys.executable, "-I", "-B", "-c", script,
                                     str(HERE/"pack_side_weights.py"), str(HERE/"make_side_known_answers.py")],
                                    cwd=directory, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(list(Path(directory).iterdir()), [])
            print(result.stdout.strip())


if __name__ == "__main__":
    unittest.main(verbosity=2)

// The vertebra shapes the 3D view draws, read from the two files in
// app/public/bones/ (written by bone-prep/prepare_bones.py; credited in
// NOTICE). This file only reads them: fetching is the caller's business.

import type { Level } from '@/shapes';

/** One bone's entry in vertebrae.json. Lengths are millimetres. */
export interface BoneEntry {
  level: Level;
  /** The element id of the source file, for tracing a shape back. */
  source: string;
  /** The vertebral body, rim to rim: what a fitted vertebra's sizes are compared with. */
  height: number;
  halfWidth: number;
  halfDepth: number;
  /** Where the bone stood in the source, in patient axes (x forward, y to the patient's left, z up). */
  referenceCentre: [number, number, number];
  /** The bone's own x, y and z axes as it stood in the source. */
  referenceAxes: [[number, number, number], [number, number, number], [number, number, number]];
  pointCount: number;
  triangleCount: number;
  /** Byte offsets into vertebrae.bin. */
  pointsAt: number;
  trianglesAt: number;
}

/** vertebrae.json. */
export interface BoneManifest {
  credit: string;
  licence: string;
  source: string;
  changes: string;
  frame: string;
  file: string;
  reference: string;
  byteLength: number;
  bones: BoneEntry[];
}

/**
 * A bone in its own frame: origin at the middle of the vertebral body,
 * x forward, y to the patient's left, z up the body. The arrays are views
 * on the bytes they were read from, not copies.
 */
export interface Bone extends BoneEntry {
  /** x, y, z for each point. */
  points: Float32Array;
  /** Three point numbers for each triangle, wound so the outside faces out. */
  triangles: Uint16Array;
}

/**
 * Pair each entry of the manifest with its points and triangles. The file
 * is little-endian, as is every machine a browser runs on. Throws when the
 * bytes are not the file the manifest describes.
 */
export function readBones(manifest: BoneManifest, bytes: ArrayBuffer): Bone[] {
  if (bytes.byteLength !== manifest.byteLength) {
    throw new Error(`The bone file is ${String(bytes.byteLength)} bytes; its list says ${String(manifest.byteLength)}.`);
  }
  return manifest.bones.map((entry) => ({
    ...entry,
    points: new Float32Array(bytes, entry.pointsAt, entry.pointCount * 3),
    triangles: new Uint16Array(bytes, entry.trianglesAt, entry.triangleCount * 3),
  }));
}

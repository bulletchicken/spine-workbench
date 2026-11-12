// The vertebra shapes the 3D view draws, read from the two files beside
// this one (written by bone-prep/prepare_bones.py; credited in NOTICE).
// This file only reads them: bundled-bones.ts is what brings them in.

import type { Level } from '@/shapes';

/** One endplate's rim, in the bone's own frame. Lengths are millimetres. */
export interface BoneEndplate {
  /** The middle of the rim. */
  centre: [number, number, number];
  /** Half the rim's span across, toward the left. */
  halfWidth: number;
  /** Half the rim's span front to back. */
  halfDepth: number;
}

/**
 * The vertebral body as the vertebra model describes one, measured on the
 * bone in its own frame. Signs as docs/conventions/signs-and-wedges.md.
 */
export interface BoneBody {
  top: BoneEndplate;
  bottom: BoneEndplate;
  /** Radians between the two endplates seen from the front: positive when the patient's left side is taller. */
  wedgeFront: number;
  /** Radians between the two endplates seen from the side: positive when the front is shorter. */
  wedgeSide: number;
}

/** One bone's entry in vertebrae.json. Lengths are millimetres. */
export interface BoneEntry {
  level: Level;
  /** The element id of the source file, for tracing a shape back. */
  source: string;
  /** The vertebral body, rim to rim: what a fitted vertebra's sizes are compared with. */
  height: number;
  halfWidth: number;
  halfDepth: number;
  /** Where the bone's own endplates are, so they can be made to sit on a fitted vertebra's. */
  body: BoneBody;
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
  body: string;
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

/** The fields of `value`, which must be an object with exactly these keys. */
function exactly(value: unknown, keys: string[], what: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new Error(`${what} is missing.`);
  const found = Object.keys(value).sort();
  if (found.join() !== [...keys].sort().join()) {
    throw new Error(`${what} has the fields ${found.join(', ')}; it should have ${keys.join(', ')}.`);
  }
  return value as Record<string, unknown>;
}

function finite(value: unknown, what: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) throw new Error(`${what} is not a number.`);
  return value;
}

function readEndplate(value: unknown, what: string): BoneEndplate {
  const plate = exactly(value, ['centre', 'halfWidth', 'halfDepth'], what);
  const centre = plate.centre;
  if (!Array.isArray(centre) || centre.length !== 3) throw new Error(`${what}'s centre is not three numbers.`);
  return {
    centre: [finite(centre[0], `${what}'s centre`), finite(centre[1], `${what}'s centre`), finite(centre[2], `${what}'s centre`)],
    halfWidth: finite(plate.halfWidth, `${what}'s halfWidth`),
    halfDepth: finite(plate.halfDepth, `${what}'s halfDepth`),
  };
}

/** A bone's body block, read strictly: every field present, nothing else, every number finite. */
function readBody(value: unknown, level: Level): BoneBody {
  const body = exactly(value, ['top', 'bottom', 'wedgeFront', 'wedgeSide'], `${level}'s body`);
  return {
    top: readEndplate(body.top, `${level}'s top endplate`),
    bottom: readEndplate(body.bottom, `${level}'s bottom endplate`),
    wedgeFront: finite(body.wedgeFront, `${level}'s wedgeFront`),
    wedgeSide: finite(body.wedgeSide, `${level}'s wedgeSide`),
  };
}

/**
 * Pair each entry of the manifest with its points and triangles. The file
 * is little-endian, as is every machine a browser runs on. Throws when the
 * bytes are not the file the manifest describes, or when a bone's body
 * block is missing or not as described.
 */
export function readBones(manifest: BoneManifest, bytes: ArrayBuffer): Bone[] {
  if (bytes.byteLength !== manifest.byteLength) {
    throw new Error(`The bone file is ${String(bytes.byteLength)} bytes; its list says ${String(manifest.byteLength)}.`);
  }
  return manifest.bones.map((entry) => ({
    ...entry,
    body: readBody(entry.body, entry.level),
    points: new Float32Array(bytes, entry.pointsAt, entry.pointCount * 3),
    triangles: new Uint16Array(bytes, entry.trianglesAt, entry.triangleCount * 3),
  }));
}

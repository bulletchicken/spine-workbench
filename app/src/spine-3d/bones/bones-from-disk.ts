// The shipped bones, read straight from the two files beside this one.
// For tests that run in Node only: the app gets them from load-bones.ts, with
// no file to read.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { type Bone, type BoneManifest, readBones } from './read-bones';

const FOLDER = fileURLToPath(new URL('./', import.meta.url));

export function bonesFromDisk(): Bone[] {
  const manifest = JSON.parse(readFileSync(`${FOLDER}vertebrae.json`, 'utf8')) as BoneManifest;
  const file = readFileSync(`${FOLDER}vertebrae.bin`);
  return readBones(manifest, file.buffer.slice(file.byteOffset, file.byteOffset + file.byteLength));
}

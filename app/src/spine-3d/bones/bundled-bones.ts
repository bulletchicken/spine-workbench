// The shipped bone shapes, brought in as part of the app's own scripts.
// The list is imported as data and the points as text (base64), so the
// bones arrive with the code that draws them and nothing is ever fetched.
// They are large: import this file only through load-bones.ts, which keeps
// it out of the first page.

import { type Bone, type BoneManifest, readBones } from './read-bones';
import packed from './vertebrae.bin?inline';
import manifest from './vertebrae.json';

/** The bytes of a `data:` address that holds them as base64. */
function bytesOf(address: string): ArrayBuffer {
  const text = atob(address.slice(address.indexOf(',') + 1));
  const bytes = new Uint8Array(text.length);
  for (let i = 0; i < text.length; i += 1) bytes[i] = text.charCodeAt(i);
  return bytes.buffer;
}

/** The eighteen bones, C7 to L5. */
export function bundledBones(): Bone[] {
  // The list is written by bone-prep in the shape `BoneManifest` describes; `readBones` checks the bytes against it.
  return readBones(manifest as unknown as BoneManifest, bytesOf(packed));
}

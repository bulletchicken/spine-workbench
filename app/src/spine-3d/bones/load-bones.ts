// The way the app gets its bone shapes: asked for when the 3D view is first
// wanted, and kept from then on. The shapes are in a script of their own,
// so they cost nothing until they are asked for.

import type { Bone } from './read-bones';

let loading: Promise<Bone[]> | null = null;

/** The eighteen bones, C7 to L5. The same answer every time. */
export function loadBones(): Promise<Bone[]> {
  loading ??= import('./bundled-bones').then(({ bundledBones }) => bundledBones());
  return loading;
}

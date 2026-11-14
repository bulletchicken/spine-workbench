import type { Level } from '@/shapes';
import type { DrawnSpine } from './drawn-spine';
import type { SceneLayer } from './scene-layer';

export type Pointed =
  | { kind: 'bone'; spine: DrawnSpine; level: Level }
  | { kind: 'part'; spine: DrawnSpine; layer: SceneLayer; id: string; readonly level?: never };

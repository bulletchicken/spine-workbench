// Development only: the whole app put in a named state, at ?scene=<name>.
//
// A file named `*.scenes.ts`, anywhere under src/, default-exports a list
// of scenes; there is no list to add it to. A scene puts the case store
// and the view state into a named state from the sample case, and then
// lets the app draw. Scene names are the design mock's deep-link names
// (`review-split`, `pass-result`, …), so the app and the mock can be
// opened side by side in the same state.
//
// Only `src/main.tsx` reaches this file, behind `import.meta.env.DEV`: a
// production build holds no scene, and none of the sample case.

import type { EngineClient } from '@/engine-room/engine-client';

export interface DevScene {
  name: string;
  /** Put the app in this state. It is handed the engine the app is running on. */
  enter(room: { engine: EngineClient }): void | Promise<void>;
}

/** Every scene of the files found, in the order of their names. Two scenes may not share a name. */
export function scenesFrom(found: Readonly<Record<string, readonly DevScene[]>>): DevScene[] {
  const scenes = Object.values(found).flat();
  const names = scenes.map((scene) => scene.name);
  const twice = names.find((name, at) => names.indexOf(name) !== at);
  if (twice !== undefined) throw new Error(`two scenes are named ${twice}`);
  return scenes.sort((a, b) => a.name.localeCompare(b.name));
}

/** Every scene in the app. */
export function findScenes(): DevScene[] {
  return scenesFrom(import.meta.glob<DevScene[]>('../**/*.scenes.ts', { eager: true, import: 'default' }));
}

/** Enter the scene with this name. A name no scene has is an error that lists the names there are. */
export async function enterScene(name: string, room: { engine: EngineClient }, scenes: readonly DevScene[] = findScenes()): Promise<void> {
  const scene = scenes.find((each) => each.name === name);
  if (scene === undefined) {
    throw new Error(`no scene is named ${name}. The scenes are: ${scenes.map((each) => each.name).join(', ') || 'none yet'}`);
  }
  await scene.enter(room);
}

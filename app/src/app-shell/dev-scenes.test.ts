import { describe, expect, it } from 'vitest';
import type { EngineClient } from '@/engine-room/engine-client';
import { enterScene, scenesFrom, type DevScene } from './dev-scenes';

// A scene is handed the engine and nothing here calls it.
const room = { engine: {} as EngineClient };

function scene(name: string, entered: string[]): DevScene {
  return {
    name,
    enter: () => {
      entered.push(name);
    },
  };
}

describe('dev scenes', () => {
  it('are gathered from every file found, by name', () => {
    const entered: string[] = [];
    const found = { '../steps/3-find/find.scenes.ts': [scene('pass-result', entered), scene('automatic-pass', entered)], '../x/x.scenes.ts': [scene('first-launch', entered)] };
    expect(scenesFrom(found).map((each) => each.name)).toEqual(['automatic-pass', 'first-launch', 'pass-result']);
  });

  it('may not share a name', () => {
    const entered: string[] = [];
    expect(() => scenesFrom({ a: [scene('saved', entered)], b: [scene('saved', entered)] })).toThrow('two scenes are named saved');
  });

  it('enter the one asked for, and wait for it', async () => {
    const entered: string[] = [];
    const slow: DevScene = {
      name: 'slow',
      enter: async () => {
        await Promise.resolve();
        entered.push('slow');
      },
    };
    await enterScene('slow', room, [scene('saved', entered), slow]);
    expect(entered).toEqual(['slow']);
  });

  it('say which scenes there are when asked for one that is not', async () => {
    const entered: string[] = [];
    await expect(enterScene('nope', room, [scene('saved', entered)])).rejects.toThrow('no scene is named nope. The scenes are: saved');
    expect(entered).toEqual([]);
  });
});

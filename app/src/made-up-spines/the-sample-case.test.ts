import { afterEach, describe, expect, it } from 'vitest';
import { startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import { SAMPLE_RECIPE, makeTheSampleCase } from './the-sample-case';

const engine = startStandInEngine({ scripts: { films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 } } });
afterEach(() => { engine.stop(); });
describe('the made-up sample case', () => {
  it('asks for the chosen recipe and gives out only four PNG films in place order', async () => {
    let asked: unknown;
    const films = await makeTheSampleCase({ makeUpSpine: (recipe) => { asked = recipe; return engine.makeUpSpine(recipe); } });
    expect(asked).toEqual(SAMPLE_RECIPE);
    expect(films.map((film) => film.kind)).toEqual(['front', 'side', 'bend-left', 'bend-right']);
    expect(films.every((film) => [...film.png.slice(0, 4)].join(',') === '137,80,78,71')).toBe(true);
    expect(films).not.toHaveProperty('answerKey');
  });
});

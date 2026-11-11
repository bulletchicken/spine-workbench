import type { AutoPassInput, Film, FilmKind, Stage, WatchFrame } from '@/shapes';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { JoinedProgress } from './auto-pass-conductor';
import { fakeCrew, filmFile, outcome, settle } from './test-rig';

const record = (kind: FilmKind) => ({ kind }) as Film;

function passOver(kinds: FilmKind[], firstVertebraId = 10): AutoPassInput {
  return { caseVersion: 7, films: kinds.map(record), options: { acquisition: 'unknown', population: 'adult', template: null }, firstVertebraId };
}

async function twoFilms(front = 'front A', side = 'side A') {
  const rig = fakeCrew();
  await rig.engine.openFilm('front', filmFile(front), null);
  await rig.engine.openFilm('side', filmFile(side), null);
  return rig;
}

afterEach(() => {
  vi.useRealTimers();
});

describe('the auto pass', () => {
  it('runs the film passes side by side, then the chain, then points film by film', async () => {
    const rig = await twoFilms();
    const frames: WatchFrame[] = [];
    const pass = rig.engine.runAutoPass(passOver(['front', 'side']), { onFrame: (frame) => frames.push(frame) });
    const [front, side] = rig.filmWorkers();
    // Both film passes are sent before either answers.
    expect(front?.jobsRun().at(-1)).toBe('film-pass');
    expect(side?.jobsRun().at(-1)).toBe('film-pass');
    const result = await pass;

    expect(rig.spine()?.jobsRun()).toEqual(['chain-pass']);
    expect(front?.jobsRun()).toEqual(['open-film', 'film-pass', 'place-points']);
    expect(side?.jobsRun()).toEqual(['open-film', 'film-pass', 'place-points']);
    expect(result.caseVersion).toBe(7);
    // Ids count up across the films: front takes 10 and 11, side 12 and 13.
    expect(result.marks.map((marks) => [marks.film, marks.vertebrae.map((vertebra) => vertebra.id)])).toEqual([
      ['front', [10, 11]],
      ['side', [12, 13]],
    ]);
    expect(result.twoFilms).toBeNull();
    // Each warning once: the chain repeats the film passes' warnings and adds its own.
    expect(result.warnings).toEqual([
      { code: 'low-evidence', film: 'front', level: null, region: null },
      { code: 'low-evidence', film: 'side', level: null, region: null },
      { code: 'tortuous', film: 'front', level: 'T5', region: null },
      { code: 'tortuous', film: 'side', level: 'T5', region: null },
    ]);
    // Each pass's frames, in the order it made them: three from each film pass, two from the chain.
    const of = (film: FilmKind, stage: Stage) => frames.filter((frame) => frame.film === film && frame.stage === stage).map((frame) => frame.kind);
    expect(of('front', 'evidence')).toEqual(['stage-started', 'warning', 'warning']);
    expect(of('side', 'evidence')).toEqual(['stage-started', 'warning', 'warning']);
    expect(of('front', 'chain')).toEqual(['stage-started', 'warning']);
    expect(frames.slice(-2).map((frame) => frame.stage)).toEqual(['chain', 'chain']);
  });

  it('places points in the order of the input, not the order the films were opened', async () => {
    const rig = await twoFilms();
    const result = await rig.engine.runAutoPass(passOver(['side', 'front'], 0));
    expect(result.marks.map((marks) => [marks.film, marks.vertebrae[0]?.id])).toEqual([
      ['side', 0],
      ['front', 2],
    ]);
  });

  it('rejects at once when its signal was already aborted, and sends nothing', async () => {
    const rig = await twoFilms();
    const controller = new AbortController();
    controller.abort();
    expect(await outcome(rig.engine.runAutoPass(passOver(['front']), { signal: controller.signal }))).toBe('pass-cancelled');
    expect(rig.filmWorkers()[0]?.jobsRun()).toEqual(['open-film']);
  });

  it('stops every film pass when aborted, and lets no frame through afterwards', async () => {
    const rig = await twoFilms('front slow', 'side slow');
    const controller = new AbortController();
    const frames: WatchFrame[] = [];
    let rejected = false;
    const pass = rig.engine.runAutoPass(passOver(['front', 'side']), { signal: controller.signal, onFrame: (frame) => frames.push(frame) });
    pass.catch(() => {
      rejected = true;
    });
    await settle(8);
    controller.abort();
    const seen = frames.length;
    expect(await outcome(pass)).toBe('pass-cancelled');
    expect(rejected).toBe(true);
    await settle();
    expect(frames).toHaveLength(seen);
    for (const worker of rig.filmWorkers()) {
      expect(worker.received.filter((message) => message.kind === 'cancel')).toHaveLength(1);
      expect(worker.said.at(-1)).toMatchObject({ kind: 'failed', error: { code: 'pass-cancelled' } });
    }
    // The workers are free again.
    expect(await rig.engine.senseGeometry('side')).toEqual({ name: 'side slow' });
  });

  it('drops an answer that was on its way when the call was stopped', async () => {
    const rig = await twoFilms();
    const controller = new AbortController();
    const front = rig.filmWorkers()[0];
    if (front === undefined) throw new Error('no film worker');
    // The worker has answered place-points, but its answer has not arrived.
    const answered = front.holdAnswerTo('place-points');
    const pass = rig.engine.runAutoPass(passOver(['front']), { signal: controller.signal });
    await answered;
    controller.abort();
    front.release();
    expect(await outcome(pass)).toBe('pass-cancelled');
    // The cancel came after the answer, so the worker says nothing more.
    await settle();
    expect(front.said.at(-1)).toMatchObject({ kind: 'done' });
  });

  it('rejects as a whole when one of its films is replaced, and stops the others', async () => {
    const rig = await twoFilms('front slow', 'side slow');
    const pass = rig.engine.runAutoPass(passOver(['front', 'side']));
    await settle(8);
    const replacing = rig.engine.openFilm('side', filmFile('side B'), null);
    expect(await outcome(pass)).toBe('pass-cancelled');
    expect((await replacing).facts.width).toBe('side B'.length);
    const [front] = rig.filmWorkers();
    expect(front?.received.filter((message) => message.kind === 'cancel')).toHaveLength(1);
    expect(await rig.engine.senseGeometry('front')).toEqual({ name: 'front slow' });
  });

  it('rejects as a whole when one of its films is retired during the chain pass', async () => {
    const rig = await twoFilms('front slow', 'side slow');
    const chaining = rig.spine()?.sentRun('chain-pass');
    const pass = rig.engine.runAutoPass(passOver(['front', 'side']));
    await chaining;
    await settle(4);
    rig.engine.retireFilm('front');
    expect(await outcome(pass)).toBe('pass-cancelled');
    await settle();
    expect(rig.spine()?.said.at(-1)).toMatchObject({ kind: 'failed', error: { code: 'pass-cancelled' } });
  });

  it('fails as a whole with the error of a film pass that fails, stopping the other', async () => {
    const rig = await twoFilms('front no spine', 'side slow');
    expect(await outcome(rig.engine.runAutoPass(passOver(['front', 'side'])))).toBe('pass-no-spine-found');
    const side = rig.filmWorkers()[1];
    await settle();
    expect(side?.received.filter((message) => message.kind === 'cancel')).toHaveLength(1);
  });

  it('fails when a film it names is not open', async () => {
    const rig = await twoFilms();
    expect(await outcome(rig.engine.runAutoPass(passOver(['front', 'bend-left'])))).toBe('engine-out-of-sync');
  });
});

describe('joined progress', () => {
  it('reports the furthest-behind stage, the films’ mean through it, at most ten times a second', () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'performance'] });
    const sent: [Stage, number][] = [];
    const progress = new JoinedProgress(['front', 'side'], (stage, fraction) => sent.push([stage, fraction]));
    progress.film('front', 'evidence', 0.5); // (0.5 + 0) / 2
    progress.film('side', 'strip', 0.2); // held: side is past evidence, so (0.5 + 1) / 2
    expect(sent).toEqual([['evidence', 0.25]]);
    vi.advanceTimersByTime(100);
    expect(sent).toEqual([
      ['evidence', 0.25],
      ['evidence', 0.75],
    ]);
    progress.film('front', 'strip', 0.6); // both in strip: (0.6 + 0.2) / 2, but only 0 ms since the last
    progress.chain('chain', 0.1); // newer, so it replaces the held one
    vi.advanceTimersByTime(100);
    expect(sent.at(-1)).toEqual(['chain', 0.1]);
    expect(sent).toHaveLength(3);
    progress.chain('naming', 0.5);
    progress.stop();
    vi.advanceTimersByTime(200);
    expect(sent).toHaveLength(3);
  });
});

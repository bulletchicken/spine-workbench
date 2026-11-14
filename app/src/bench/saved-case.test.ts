import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { saidNow } from '@/building-blocks/toast/say';
import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { DEFAULT_SCRIPTS, startStandInEngine } from '@/engine-room/stand-in/stand-in-engine';
import story from '@/sample-case/story';
import { EngineFailure } from '@/engine-room/engine-client';
import { currentStep } from '@/view-state/current-step';
import { findState } from '@/view-state/find-state';
import { filmOrigins } from '@/view-state/film-origins';
import { fitSources } from '@/view-state/fit-sources';
import { savedState } from '@/view-state/saved-state';
import { filmIntake } from '@/view-state/film-intake';
import type { CaseFileWriter } from '@/open-and-save/save-file';
import { spineBuild } from '@/view-state/spine-build';
import { filmToFitFor } from './three/fit-input';
import { readingsView } from './three/readings-view';
import { buildSpine } from './three/build-spine';
import { clearFilms, startOver } from './start-over';
import { takeIn, takeInNextSample, chooseFilms } from './film-intake';
import { reopenedSources } from './reopened-sources';
import { saveCurrentCase, hasCaseToSave } from './save-case';
import { chooseCaseDestination } from '@/open-and-save/save-file';
import { pickFilms } from '@/open-and-save/pick-files';

vi.mock('@/sample-films/sample-films', () => ({ sampleCaseFor: () => Promise.resolve(null), sampleFilmFilesFor: () => Promise.resolve([]) }));
vi.mock('@/choreography/timing', () => ({ durationOnPage: () => 0 }));
vi.mock('@/spine-3d/bones/load-bones', () => ({ loadBones: () => Promise.resolve([{ level: 'L3' }]) }));
vi.mock('@/open-and-save/save-file', () => ({ chooseCaseDestination: vi.fn() }));
vi.mock('@/open-and-save/pick-files', () => ({ pickFilms: vi.fn() }));

let engine: ReturnType<typeof startStandInEngine>;
let written: Uint8Array;
beforeEach(() => {
  engine = startStandInEngine({ story, scripts: {
    films: { openMs: 0, displayPixelsMs: 0, makeUpMs: 0 },
    find: { ...DEFAULT_SCRIPTS.find, normalMs: 0, placePointsMs: 0 },
    save: { encodeMs: 0, packMs: 0 }, reopen: { unpackMs: 0 },
    adjust: { ...DEFAULT_SCRIPTS.adjust, compareMs: 0 },
  } });
  clearFilms(engine); currentStep.getState().go('films');
  written = new Uint8Array();
  savedState.setState({ mark: null });
  vi.mocked(chooseCaseDestination).mockResolvedValue((bytes) => { written = bytes.slice(); return Promise.resolve('written'); });
});
afterEach(() => { clearFilms(engine); engine.stop(); vi.restoreAllMocks(); });

it('saves a found four-film case, starts over, reopens without searching, then builds and measures', async () => {
  expect(hasCaseToSave(caseStore.getState().case)).toBe(false);
  await takeInNextSample(engine);
  expect(hasCaseToSave(caseStore.getState().case)).toBe(false);
  const initial = caseStore.getState();
  const result = await engine.runAutoPass({ caseVersion: initial.version, films: initial.case.films,
    options: { acquisition: 'unknown', population: 'adolescent', template: null },
    firstVertebraId: initial.case.nextVertebraId, frontFinder: 'hand-built', sideFinder: 'hand-built' });
  caseStore.getState().apply(result.marks.map((marks) => ({ kind: 'replace-marks', marks, basedOn: result.caseVersion, cause: { by: 'auto-pass' } })));
  const side = caseStore.getState().case.films.find((f) => f.kind === 'side');
  if (side === undefined) throw new Error('missing Side');
  caseStore.getState().apply([{ kind: 'set-geometry', film: 'side', geometry: { ...side.geometry, imageRight: 'posterior', confirmed: { ...side.geometry.confirmed, facing: true } } }]);
  findState.getState().found(); findState.getState().lookedAt('L3');
  currentStep.getState().go('find');
  const expected = structuredClone(caseStore.getState().case);
  const expectedPixels = structuredClone(filmPixels.getState().films);
  expect(hasCaseToSave(expected)).toBe(true);
  await saveCurrentCase(engine);
  expect(saidNow()?.line).toBe('Case saved.');
  expect(savedState.getState().mark).toEqual({ version: caseStore.getState().version, how: 'saved' });
  expect(written.length).toBeGreaterThan(0);
  await startOver(engine);
  expect(caseStore.getState().case.films).toEqual([]);
  const search = vi.spyOn(engine, 'runAutoPass');
  await takeIn(engine, [new Blob([new Uint8Array(written)])], null);
  expect(caseStore.getState().case).toEqual(expected);
  expect(caseStore.getState().history.undo).toEqual([]);
  expect(currentStep.getState().slug).toBe('find');
  expect(findState.getState()).toMatchObject({ phase: 'found', looked: [], sideLooked: null });
  expect(saidNow()?.line).toBe('Opened a saved case.');
  expect(search).not.toHaveBeenCalled();
  expect(filmPixels.getState().films).toEqual(expectedPixels);
  for (const film of expected.films) expect(filmOrigins.getState().films[film.kind]).toEqual({ from: 'yours', opening: filmPixels.getState().openings[film.kind] });
  expect(reopenedSources(expected, filmPixels.getState().openings)).toEqual(fitSources.getState().films);
  expect(fitSources.getState().films.front?.finder).toBe('trained');
  expect(fitSources.getState().films.side?.finder).toBe('trained');
  expect(fitSources.getState().films['bend-left']?.finder).toBe('hand-built');
  await buildSpine(engine);
  const accepted = spineBuild.getState().accepted;
  expect(currentStep.getState().slug).toBe('three');
  if (accepted === null) throw new Error('missing spine');
  expect(accepted.fit.vertebrae.length).toBeGreaterThan(0);
  expect(accepted.input.front.markSpread).toEqual({ kind: 'share-of-height', share: 0.11, sharedShare: 0.08 });
  const c = caseStore.getState().case;
  const readings = await engine.measureSpine({ fit: accepted.fit, front: accepted.input.front, side: accepted.input.side,
    bendLeft: filmToFitFor(c, 'bend-left', fitSources.getState().films, filmPixels.getState().openings),
    bendRight: filmToFitFor(c, 'bend-right', fitSources.getState().films, filmPixels.getState().openings), versions: 32, seed: 7 });
  expect(readingsView(readings, accepted.input.front).rows.length).toBeGreaterThan(0);
  currentStep.getState().go('films');
  vi.mocked(pickFilms).mockResolvedValue([{ kind: 'case', blob: new Blob([new Uint8Array(written)]) }]);
  await chooseFilms(engine);
  expect(caseStore.getState().case).toEqual(expected);
  expect(currentStep.getState().slug).toBe('find');
  expect(spineBuild.getState().accepted).toBeNull();
});

it.each(['find', 'three'])('keeps the current case, pixels, looks and %s moment when a case is refused', async (moment) => {
  await takeInNextSample(engine);
  currentStep.getState().go(moment); findState.getState().found(); findState.getState().lookedAt('L3');
  const c = caseStore.getState(); const pixels = filmPixels.getState(); const find = findState.getState();
  vi.spyOn(engine, 'openCase').mockRejectedValue(new EngineFailure({ code: 'case-invalid-value', message: 'bad case', detail: null }));
  const intake = filmIntake.getState();
  const retire = vi.spyOn(engine, 'retireFilm');
  await takeIn(engine, [new Blob([new Uint8Array([80, 75, 3, 4])])], null);
  expect(caseStore.getState()).toBe(c); expect(filmPixels.getState()).toBe(pixels); expect(findState.getState()).toBe(find);
  expect(currentStep.getState().slug).toBe(moment); expect(retire).not.toHaveBeenCalled();
  expect(filmIntake.getState()).toBe(intake);
  expect(saidNow()?.line).toBe('This case file has a value that cannot be right');
});


it('confirms a download handoff without marking the case as saved, and words a local write failure', async () => {
  const c = { ...caseStore.getState().case, films: [filmRecordFrom(story.madeUp.films.front.facts)], marks: structuredClone(story.pass.marks.filter((marks) => marks.film === 'front')), nextVertebraId: 1000 };
  caseStore.getState().apply([{ kind: 'open-case', case: c }]);
  const saveCase = vi.fn(() => Promise.resolve(new Uint8Array([1])));
  vi.mocked(chooseCaseDestination).mockResolvedValue(() => Promise.resolve('download'));
  await saveCurrentCase({ saveCase });
  expect(saidNow()?.line).toBe('Case sent to your downloads.');
  expect(savedState.getState().mark).toBeNull();
  vi.mocked(chooseCaseDestination).mockResolvedValue(() => Promise.reject(new DOMException('no write', 'NotAllowedError')));
  await saveCurrentCase({ saveCase });
  expect(saidNow()?.line).toBe('The case could not be written there. Choose another place and try again.');
  expect(savedState.getState().mark).toBeNull();
});

it('does not write a different case if one opens while the destination dialog is pending', async () => {
  caseStore.getState().apply([{ kind: 'open-case', case: { ...caseStore.getState().case, films: [filmRecordFrom(story.madeUp.films.front.facts)], marks: structuredClone(story.pass.marks.filter((marks) => marks.film === 'front')), nextVertebraId: 1000 } }]);
  const before = caseStore.getState().case;
  expect(hasCaseToSave(before)).toBe(true);
  let choose: ((writer: CaseFileWriter) => void) | undefined;
  vi.mocked(chooseCaseDestination).mockImplementation(() => new Promise((resolve) => { choose = resolve; }));
  const saveCase = vi.fn(() => Promise.resolve(new Uint8Array([1])));
  const write = vi.fn(() => Promise.resolve('written' as const));
  const saving = saveCurrentCase({ saveCase });
  caseStore.getState().apply([{ kind: 'open-case', case: { ...before } }]);
  choose?.(write);
  await saving;
  expect(saveCase).not.toHaveBeenCalled();
  expect(write).not.toHaveBeenCalled();
  expect(savedState.getState().mark).toBeNull();
  expect(saidNow()?.line).toBe('The case changed. Save it again.');
});

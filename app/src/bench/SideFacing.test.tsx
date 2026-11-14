import { beforeEach, expect, it } from 'vitest';
import { find, pressOn, putOnThePage } from '@/building-blocks/test-rig/on-the-page';
import { caseStore } from '@/case-state/case-store';
import { emptyCase } from '@/case-state/empty-case';
import { filmPixels } from '@/case-state/film-pixels';
import { smallStory } from '@/engine-room/stand-in/small-story';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { filmOrigins } from '@/view-state/film-origins';
import { answerSideFacing, SideFacing } from './SideFacing';
import { searchFilms } from './search-films';

beforeEach(() => {
  const film = filmRecordFrom(smallStory().films.side.facts);
  film.geometry.quarterTurns = 1;
  caseStore.getState().apply([{ kind: 'open-case', case: { ...emptyCase(), films: [film] } }]);
  filmOrigins.getState().clear();
  filmOrigins.getState().opened('side', 'yours', filmPixels.getState().openings.side);
});
it('asks without a default and persists a real picture-space answer, preserving the other geometry', () => {
  const opening = filmPixels.getState().openings.side;
  putOnThePage(<SideFacing opening={opening} />);
  expect(document.querySelectorAll('input:checked')).toHaveLength(0);
  const before = caseStore.getState().case.films[0]?.geometry;
  pressOn(find('input'));
  expect(caseStore.getState().case.films[0]?.geometry).toEqual({ ...before, imageRight: 'posterior', confirmed: { ...before?.confirmed, facing: true } });
  answerSideFacing(opening - 1, 'anterior');
  expect(caseStore.getState().case.films[0]?.geometry.imageRight).toBe('posterior');
});
it('keeps Side ineligible until answered and available, without blocking an eligible Front', () => {
  const plan = (info: Parameters<typeof searchFilms>[3]) => searchFilms(caseStore.getState().case, filmOrigins.getState().films, filmPixels.getState().openings, info);
  expect(plan('waiting')).toMatchObject({ canFind: false, sideExclusion: 'facing' });
  answerSideFacing(filmPixels.getState().openings.side, 'anterior');
  expect(plan('waiting')).toMatchObject({ canFind: false, sideExclusion: 'waiting' });
  expect(plan('fault')).toMatchObject({ canFind: false, sideExclusion: 'fault' });
  const info = { engineVersion: '', shapesFingerprint: '', caseFormat: '', caseVersion: 1, trainedSideFinder: false };
  expect(plan(info)).toMatchObject({ canFind: false, sideExclusion: 'unavailable' });
  expect(plan({ ...info, trainedSideFinder: true })).toMatchObject({ canFind: true, sideFinder: 'trained' });
  const front = filmRecordFrom(smallStory().films.front.facts);
  caseStore.getState().apply([{ kind: 'set-film', film: front }]);
  filmOrigins.getState().opened('front', 'made-up-case', filmPixels.getState().openings.front);
  expect(plan(info)).toMatchObject({ canFind: true, frontFinder: 'hand-built', sideExclusion: 'unavailable' });
  expect(plan(info).films.map((f) => f.kind)).toEqual(['front']);
});

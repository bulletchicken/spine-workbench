import { useEffect, useState } from 'react';
import { openSavedCase } from './open-saved-case';
import { clearSpineBuild } from './three/build-spine';
import { filmOrigins } from '@/view-state/film-origins';
import { fitSources } from '@/view-state/fit-sources';
import { say } from '@/building-blocks/toast/say';
import { FILMS_WORDS } from '@/words/films';
import { SHELL_WORDS } from '@/words/shell';
import { faultWords } from '@/words/faults';
import { caseStore } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { FILM_KINDS } from '@/case-state/selectors';
import { EngineFailure, type EngineClient } from '@/engine-room/engine-client';
import { useEngine } from '@/engine-room/use-engine';
import { makeTheSampleCase } from '@/made-up-spines/the-sample-case';
import { filmRecordFrom } from '@/open-and-save/film-record';
import { openAFilm } from '@/open-and-save/open-a-film';
import { pickFilms } from '@/open-and-save/pick-files';
import { readBytes } from '@/open-and-save/read-bytes';
import { readDroppedFiles, type DroppedFile } from '@/open-and-save/read-dropped-files';
import { answerSideFacing } from './SideFacing';
import { sampleCaseFor, sampleFilmFilesFor } from '@/sample-films/sample-films';
import type { ErrorCode, FilmKind } from '@/shapes';
import { currentStep } from '@/view-state/current-step';
import { findState } from '@/view-state/find-state';
import { filmIntake, type FilmIntakeState, type IntakeNotice } from '@/view-state/film-intake';
import { assignPlaces } from './assign-places';
import { watchForDroppedFiles } from './drop-files';
import { clearFilms, queueFilmChange } from './start-over';
import type { Held } from './Stage';

function failureCode(error: unknown): ErrorCode {
  return error instanceof EngineFailure ? error.error.code : 'engine-crashed';
}

function backToFilms(): IntakeNotice | null {
  if (currentStep.getState().slug === 'three' || currentStep.getState().slug === 'place') { currentStep.getState().go('films'); return null; }
  if (currentStep.getState().slug !== 'find') return null;
  const phase = findState.getState().phase;
  currentStep.getState().go('films');
  return phase === 'searching' || phase === 'replaying' ? { kind: 'search-stopped' } : null;
}

/** File judging is shared by a drop and the picker; neither keeps file names. */
async function takeTold(engine: EngineClient, told: readonly DroppedFile[], onPlace: FilmKind | null, from: FilmIntakeState['from'], sample: number | null = null, slots?: readonly FilmKind[]): Promise<void> {
  if (told.length === 0) return;
  const saved = told.find((file) => file.kind === 'case');
  if (saved?.kind === 'case') {
    const leftOut = told.length - 1;
    try {
      await openSavedCase(engine, saved.blob);
      if (leftOut > 0) {
        filmIntake.getState().tell({ kind: 'left-out', count: leftOut });
        say(`${SHELL_WORDS.openedCase} ${FILMS_WORDS.leftOut(leftOut)}`);
      }
    }
    catch (error) {
      const code = failureCode(error);
      if (code !== 'pass-cancelled') {
        if (currentStep.getState().slug === 'films') filmIntake.getState().tell({ kind: 'refused', count: 1, code });
        // Keep a rejected file from changing the current moment as well as the case.
        say(leftOut > 0 ? `${FILMS_WORDS.refusedHelp(faultWords(code).title)} ${FILMS_WORDS.leftOut(leftOut)}` : faultWords(code).title, { tone: 'note' });
      }
    }
    return;
  }
  const stopped = backToFilms();
  filmIntake.getState().tell(null);
  const blobs = told.flatMap((file) => file.kind === 'film' ? [file.blob] : []);
  const refusals: ErrorCode[] = told.flatMap((file) => file.kind === 'refused' ? [file.code] : []);
  const wasFrom = filmIntake.getState().from;
  if (blobs.length > 0 && (from !== 'yours' || wasFrom === 'sample-film' || wasFrom === 'made-up-case' || wasFrom === 'real-case')) clearFilms(engine);
  const filled = caseStore.getState().case.films.map((film) => film.kind);
  const { places, leftOut } = slots === undefined ? assignPlaces(blobs.length, onPlace, filled) : { places: slots, leftOut: 0 };
  const openings = places.map((kind, index) => ({ kind, blob: blobs[index] })).sort((a, b) => FILM_KINDS.indexOf(a.kind) - FILM_KINDS.indexOf(b.kind));
  if (places.length > 0) filmIntake.getState().source(from, sample);
  filmIntake.getState().begin(openings.map(({ kind }) => kind));
  for (const { kind, blob } of openings) {
    if (blob === undefined) continue;
    const opening = { read: false };
    let kept = false;
    try {
      const bytes = await readBytes(blob);
      const film = await openAFilm({
        openFilm: async (...args) => {
          const answer = await engine.openFilm(...args);
          opening.read = true;
          return answer;
        },
        displayPixels: (place) => engine.displayPixels(place),
      }, kind, bytes, null);
      clearSpineBuild();
      fitSources.getState().forget(kind);
      caseStore.getState().apply([{ kind: 'set-film', film: filmRecordFrom(film.facts) }]);
      if (from !== null) filmOrigins.getState().opened(kind, from, filmPixels.getState().openings[kind]);
      kept = true;
    } catch (error) {
      const code = failureCode(error);
      if (code !== 'pass-cancelled') {
        if (opening.read) {
          filmOrigins.getState().forget(kind);
          engine.retireFilm(kind);
          filmPixels.getState().retired(kind);
          caseStore.getState().apply([{ kind: 'remove-film', film: kind }]);
        }
        refusals.push(code);
      }
    } finally {
      filmIntake.getState().read(kind, kept);
    }
  }
  filmIntake.getState().tell(refusals.length > 0
    ? { kind: 'refused', count: refusals.length, code: refusals[0] ?? 'film-unknown-format' }
    : leftOut > 0 ? { kind: 'left-out', count: leftOut } : stopped);
  if (caseStore.getState().case.films.length === 0) filmIntake.getState().source(null);
}

export function takeIn(engine: EngineClient, blobs: readonly Blob[], onPlace: FilmKind | null): Promise<void> {
  return queueFilmChange(async () => { await takeTold(engine, await readDroppedFiles(blobs), onPlace, 'yours'); });
}

export async function chooseFilms(engine: EngineClient, onPlace: FilmKind | null = null): Promise<void> {
  const told = await pickFilms();
  await queueFilmChange(async () => { await takeTold(engine, told, onPlace, 'yours'); });
}

/** Real films kept on this machine go round on their own; the made-up case is offered only when there are none. */
export function takeInNextSample(engine: EngineClient): Promise<void> {
  return queueFilmChange(async () => {
    try {
      const files = await sampleFilmFilesFor(engine);
      const local = await sampleCaseFor(engine);
      const offset = local === null ? 0 : 1;
      const real = files.length + offset;
      const next = real === 0 ? 0 : filmIntake.getState().nextSample % real;
      if (local !== null && next === 0) {
        const places = FILM_KINDS.filter((kind) => local.films[kind] !== undefined);
        await takeTold(engine, places.flatMap((kind) => {
          const file = local.films[kind];
          return file === undefined ? [] : [{ kind: 'film' as const, blob: new Blob([file.bytes()]) }];
        }), null, 'real-case', null, places);
        if (local.sideFaces !== undefined && caseStore.getState().case.films.some((film) => film.kind === 'side')) answerSideFacing(filmPixels.getState().openings.side, local.sideFaces === 'left' ? 'posterior' : 'anterior');
        if (caseStore.getState().case.films.length > 0) filmOrigins.getState().setCredit(local.credit ?? null);
        filmIntake.getState().next(1);
        return;
      }
      const file = files[next - offset];
      if (file !== undefined) {
        await takeTold(engine, [{ kind: 'film', blob: new Blob([file.bytes()]) }], 'front', 'sample-film', next - offset);
        filmIntake.getState().next(next + 1);
        return;
      }
      backToFilms();
      clearFilms(engine);
      filmIntake.getState().source('made-up-case');
      filmIntake.getState().make(true);
      const films = await makeTheSampleCase(engine);
      filmIntake.getState().make(false);
      await takeTold(engine, films.map(({ png }) => ({ kind: 'film', blob: new Blob([new Uint8Array(png)]) })), null, 'made-up-case');
      filmIntake.getState().next(0);
    } catch (error) {
      const code = failureCode(error);
      if (code !== 'pass-cancelled') filmIntake.getState().tell({ kind: 'refused', count: 1, code });
      if (caseStore.getState().case.films.length === 0) filmIntake.getState().source(null);
    } finally {
      filmIntake.getState().make(false);
    }
  });
}

/** The bench owns the window's one watcher for the whole visit. */
export function useFilmIntake(): { held: Held; pressPlace: (kind: FilmKind) => void } {
  const engine = useEngine();
  const [held, setHeld] = useState<Held>({ over: false, target: null });
  useEffect(() => watchForDroppedFiles({ held: setHeld, dropped: (files, place) => { void takeIn(engine, files, place); } }), [engine]);
  return { held, pressPlace: (kind) => { void chooseFilms(engine, kind); } };
}

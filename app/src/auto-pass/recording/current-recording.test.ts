import { describe, expect, it } from 'vitest';
import { createCurrentRecording } from './current-recording';
import { filmStamp } from './film-stamp';
import { filmRecording, filmStrip, recordFrames } from './record-frames';
import { testFilm, testFrames } from './test-frames';

const front = filmStamp(testFilm('front'), 1);
const side = filmStamp(testFilm('side'), 1);

function turnedFront() {
  const film = testFilm('front');
  film.geometry.quarterTurns = 1;
  return filmStamp(film, 1);
}

function bothFilms() {
  return recordFrames(['front', 'side'], [...testFrames('front'), ...testFrames('side')]);
}

function stripOf(recording: ReturnType<typeof bothFilms>, film: 'front' | 'side') {
  const part = filmRecording(recording, film);
  return part === null ? null : filmStrip(part);
}

describe('this session’s recording', () => {
  it('holds none at first', () => {
    const store = createCurrentRecording().getState();
    expect(store.recordingFor({ front, side })).toBeNull();
    expect(store.stripFor('front', front)).toBeNull();
  });

  it('gives the recording and each strip back while the films are as they were', () => {
    const current = createCurrentRecording();
    const recording = bothFilms();
    current.getState().keep(recording, { front, side });
    expect(current.getState().recordingFor({ front, side })).toBe(recording);
    expect(current.getState().stripFor('front', front)).toEqual(stripOf(recording, 'front'));
    expect(current.getState().stripFor('side', side)).toEqual(stripOf(recording, 'side'));
  });

  it('gives nothing for a film turned since the pass, and no Watch again', () => {
    const current = createCurrentRecording();
    current.getState().keep(bothFilms(), { front, side });
    expect(current.getState().stripFor('front', turnedFront())).toBeNull();
    expect(current.getState().stripFor('side', side)).not.toBeNull();
    expect(current.getState().recordingFor({ front: turnedFront(), side })).toBeNull();
  });

  it('gives nothing for a film opened again since the pass', () => {
    const current = createCurrentRecording();
    current.getState().keep(bothFilms(), { front, side });
    const reopened = filmStamp(testFilm('front'), 2);
    expect(current.getState().stripFor('front', reopened)).toBeNull();
    expect(current.getState().recordingFor({ front: reopened, side })).toBeNull();
  });

  it('gives no Watch again once a film of the pass is gone, though a film the pass never ran may come', () => {
    const current = createCurrentRecording();
    current.getState().keep(bothFilms(), { front, side });
    expect(current.getState().recordingFor({ front })).toBeNull();
    expect(current.getState().recordingFor({ front, side, 'bend-left': filmStamp(testFilm('front'), 7) })).not.toBeNull();
  });

  it('gives a rebuilt strip back under its own stamp only', () => {
    const current = createCurrentRecording();
    const strip = stripOf(bothFilms(), 'front');
    if (strip === null) throw new Error('the test pass has a whole strip');
    current.getState().keepStrip('front', strip, front);
    expect(current.getState().stripFor('front', front)).toEqual(strip);
    expect(current.getState().stripFor('front', turnedFront())).toBeNull();
    expect(current.getState().recordingFor({ front })).toBeNull();
  });

  it('keeps the strip of a film a later pass did not run, under its old stamp', () => {
    const current = createCurrentRecording();
    const first = bothFilms();
    current.getState().keep(first, { front, side });
    const sideAgain = filmStamp(testFilm('side'), 2);
    const second = recordFrames(['side'], testFrames('side'));
    current.getState().keep(second, { side: sideAgain });
    expect(current.getState().stripFor('front', front)).toEqual(stripOf(first, 'front'));
    expect(current.getState().stripFor('side', side)).toBeNull();
    expect(current.getState().stripFor('side', sideAgain)).toEqual(stripOf(second, 'side'));
    expect(current.getState().recordingFor({ front, side: sideAgain })).toBe(second);
  });

  it('lets a later pass over a film supersede the strip rebuilt for it', () => {
    const current = createCurrentRecording();
    const strip = stripOf(bothFilms(), 'front');
    if (strip === null) throw new Error('the test pass has a whole strip');
    current.getState().keepStrip('front', strip, front);
    current.getState().keep(recordFrames(['front'], testFrames('front')), { front: turnedFront() });
    expect(current.getState().stripFor('front', front)).toBeNull();
    expect(current.getState().stripFor('front', turnedFront())).not.toBeNull();
  });

  it('lets everything go for a new case, and tells whoever listens', () => {
    const current = createCurrentRecording();
    let heard = 0;
    current.subscribe(() => {
      heard += 1;
    });
    current.getState().keep(bothFilms(), { front, side });
    current.getState().forget();
    expect(heard).toBe(2);
    expect(current.getState().recordingFor({ front, side })).toBeNull();
    expect(current.getState().stripFor('front', front)).toBeNull();
  });
});

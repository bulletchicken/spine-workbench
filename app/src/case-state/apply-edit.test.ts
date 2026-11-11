// What each edit does to the case, with every expected value worked out
// by hand from the boxes sketch-case.ts draws: a box's corners are
// (100, top), (180, top), (100, top + 40) and (180, top + 40), the first
// box's top at 100 and each next one 50 lower.

import { describe, expect, it } from 'vitest';
import { TEMPLATE_LEVELS, TEMPLATE_NAMES, type CaseFileV1, type FilmGeometry, type FilmMarks, type Mark, type PedicleMark } from '@/shapes';
import { applyEdit } from './apply-edit';
import { marksOf } from './selectors';
import { auto, sketchCase, sketchFilm, sketchMarks, standardLevels } from './sketch-case';

function applied(c: CaseFileV1, ...edits: Parameters<typeof applyEdit>[1][]): CaseFileV1 {
  let result = c;
  for (const edit of edits) {
    const next = applyEdit(result, edit);
    if (next === null) throw new Error(`${edit.kind} did not apply`);
    result = next;
  }
  return result;
}

function marks(c: CaseFileV1, film: FilmMarks['film']): FilmMarks {
  const found = marksOf(c, film);
  if (found === null) throw new Error(`no marks on ${film}`);
  return found;
}

function frontPoints(filmMarks: FilmMarks, index: number) {
  const vertebra = filmMarks.vertebrae[index];
  if (vertebra?.marks.kind !== 'front') throw new Error('not a front vertebra');
  return vertebra.marks.points;
}

const frontCase = () => sketchCase(['front', 'side'], [sketchMarks('front', 1, standardLevels('C7', 3)), sketchMarks('side', 4, standardLevels('C7', 2))]);

const geometry = (change: Partial<FilmGeometry>): FilmGeometry => ({ ...sketchFilm('front').geometry, ...change });

describe('films', () => {
  it('adds a film in kind order, whatever order it came in', () => {
    const c = applied(sketchCase(['bend-right']), { kind: 'set-film', film: sketchFilm('side') });
    expect(c.films.map((film) => film.kind)).toEqual(['side', 'bend-right']);
  });

  it("replaces a film of the same kind and drops that film's marks, and only that film's", () => {
    const c = applied(frontCase(), { kind: 'set-film', film: { ...sketchFilm('front'), blanked: [{ left: 0, top: 0, right: 9, bottom: 9 }] } });
    expect(c.films[0]?.blanked).toHaveLength(1);
    expect(c.marks.map((m) => m.film)).toEqual(['side']);
  });

  it('removes a film with its marks, and refuses one the case does not have', () => {
    const c = applied(frontCase(), { kind: 'remove-film', film: 'front' });
    expect(c.films.map((film) => film.kind)).toEqual(['side']);
    expect(c.marks.map((m) => m.film)).toEqual(['side']);
    expect(applyEdit(c, { kind: 'remove-film', film: 'front' })).toBeNull();
  });

  it('flips the brightness and who flipped it, and flips them back', () => {
    const once = applied(frontCase(), { kind: 'flip-brightness', film: 'side' });
    expect(once.films[1]?.values).toMatchObject({ inverted: true, invertedByUser: true });
    expect(applied(once, { kind: 'flip-brightness', film: 'side' }).films[1]?.values).toMatchObject({ inverted: false, invertedByUser: false });
  });

  it('sets the blanked boxes', () => {
    const boxes = [{ left: 1, top: 2, right: 30, bottom: 40 }];
    expect(applied(frontCase(), { kind: 'set-blanked', film: 'side', boxes }).films[1]?.blanked).toEqual(boxes);
  });
});

describe('set-geometry', () => {
  it('drops the marks when the quarter turns change, whatever else changes', () => {
    const c = applied(frontCase(), { kind: 'set-geometry', film: 'front', geometry: geometry({ quarterTurns: 2, imageRight: 'patient-right' }) });
    expect(c.films[0]?.geometry.quarterTurns).toBe(2);
    expect(marksOf(c, 'front')).toBeNull();
    expect(marksOf(c, 'side')).not.toBeNull();
  });

  it('swaps every left and right name for a new facing, each point keeping its place', () => {
    const before = frontCase();
    const c = applied(before, { kind: 'set-geometry', film: 'front', geometry: geometry({ imageRight: 'patient-right' }) });
    const after = marks(c, 'front');
    // supL held (100, 100) and supR (180, 100): now the names change hands.
    expect(frontPoints(after, 0).supL.at).toEqual([180, 100]);
    expect(frontPoints(after, 0).supR.at).toEqual([100, 100]);
    expect(frontPoints(after, 0).pedL).toEqual(frontPoints(marks(before, 'front'), 0).pedR);
    expect(after.namedFor).toBe('patient-right');
    // C7's centre: x (100 + 180 + 180 + 100) / 4 = 140, y (100 + 100 + 140 + 140) / 4 = 120.
    expect(after.landmarks.c7?.at).toEqual([140, 120]);
  });

  it('leaves the marks as they are when the facing given is the one they are named for', () => {
    const before = frontCase();
    const c = applied(before, { kind: 'set-geometry', film: 'front', geometry: geometry({ imageRight: 'patient-left' }) });
    expect(marksOf(c, 'front')).toBe(marksOf(before, 'front'));
  });

  it('leaves the marks untouched when only the scale or the confirmations change, or the facing becomes unknown', () => {
    const named = applied(frontCase(), { kind: 'set-geometry', film: 'front', geometry: geometry({ imageRight: 'patient-right' }) });
    const scaled = applied(named, {
      kind: 'set-geometry',
      film: 'front',
      geometry: geometry({
        imageRight: 'patient-right',
        mmPerPx: 0.2,
        scale: { source: 'ruler', rulers: [{ a: [0, 0], b: [100, 0], mm: 20 }] },
        confirmed: { upright: true, leftRight: true, facing: false, scale: true, polarity: false, privacy: false },
      }),
    });
    expect(marksOf(scaled, 'front')).toBe(marksOf(named, 'front'));
    const unknown = applied(scaled, { kind: 'set-geometry', film: 'front', geometry: geometry({ imageRight: null }) });
    expect(marksOf(unknown, 'front')).toBe(marksOf(named, 'front'));
    expect(marks(unknown, 'front').namedFor).toBe('patient-right');
  });

  it('never renames on a bare confirmation, even when the marks were named for the other facing', () => {
    // Marks named for patient-right on a film whose image right is
    // patient-left, as a file from elsewhere might hold. Confirming the
    // facing as it stands changes the geometry but not the facing.
    const sketched = sketchCase(['front'], [{ ...sketchMarks('front', 1, standardLevels('C7', 3)), namedFor: 'patient-right' }]);
    const c: CaseFileV1 = { ...sketched, films: sketched.films.map((film) => ({ ...film, geometry: geometry({ imageRight: 'patient-left' }) })) };
    const confirmed = applied(c, {
      kind: 'set-geometry',
      film: 'front',
      geometry: geometry({ imageRight: 'patient-left', confirmed: { upright: false, leftRight: true, facing: false, scale: false, polarity: false, privacy: false } }),
    });
    expect(confirmed.films[0]?.geometry.confirmed.leftRight).toBe(true);
    expect(marksOf(confirmed, 'front')).toBe(marksOf(c, 'front'));
  });
});

describe('replace-marks', () => {
  it("puts the engine's marks in whole, with c7 worked out again from the C7 corners", () => {
    const answer = sketchMarks('side', 20, standardLevels('C7', 1));
    const wrongC7 = { ...answer, landmarks: { ...answer.landmarks, c7: auto(0, 0) } };
    const c = applied(frontCase(), { kind: 'replace-marks', marks: wrongC7, basedOn: 0, cause: { by: 'auto-pass' } });
    expect(marks(c, 'side').vertebrae.map((v) => v.id)).toEqual([20]);
    expect(marks(c, 'side').landmarks.c7?.at).toEqual([140, 120]);
  });

  it('refuses marks for a film the case does not have', () => {
    expect(applyEdit(frontCase(), { kind: 'replace-marks', marks: sketchMarks('bend-left', 9, []), basedOn: 0, cause: { by: 'auto-pass' } })).toBeNull();
  });
});

describe('nextVertebraId', () => {
  it('never goes down: it stays when every id is below it', () => {
    const c = { ...frontCase(), nextVertebraId: 10 };
    expect(applied(c, { kind: 'flip-brightness', film: 'front' }).nextVertebraId).toBe(10);
  });

  it('goes to one above the highest id when an answer brings a higher one', () => {
    const c = applied(frontCase(), { kind: 'replace-marks', marks: sketchMarks('side', 14, standardLevels('T1', 1)), basedOn: 0, cause: { by: 'auto-pass' } });
    expect(c.nextVertebraId).toBe(15);
  });

  it("takes an opened case's own, even a lower one, and keeps its films and marks in kind order", () => {
    const opened: CaseFileV1 = {
      ...sketchCase(['side', 'front']),
      marks: [sketchMarks('side', 1, []), sketchMarks('front', 2, [])],
      nextVertebraId: 3,
    };
    const c = applied({ ...frontCase(), nextVertebraId: 50 }, { kind: 'open-case', case: opened });
    expect(c.nextVertebraId).toBe(3);
    expect(c.films.map((film) => film.kind)).toEqual(['front', 'side']);
    expect(c.marks.map((m) => m.film)).toEqual(['front', 'side']);
  });
});

describe('move-mark', () => {
  it('puts a corner exactly there, by hand, still pinned, no longer inferred', () => {
    const pinnedInferred: Mark = { at: [100, 150], source: 'auto', pinned: true, inferred: true };
    const start = frontCase();
    const front = marks(start, 'front');
    const vertebra = front.vertebrae[1];
    if (vertebra?.marks.kind !== 'front') throw new Error('front vertebra expected');
    const c0 = applied(start, {
      kind: 'replace-marks',
      marks: { ...front, vertebrae: front.vertebrae.with(1, { ...vertebra, marks: { kind: 'front', points: { ...vertebra.marks.points, supL: pinnedInferred } } }) },
      basedOn: 0,
      cause: { by: 'auto-pass' },
    });
    const c = applied(c0, { kind: 'move-mark', film: 'front', target: { on: 'vertebra', vertebra: 2, mark: 'supL' }, to: [95.5, 148], gesture: null });
    expect(frontPoints(marks(c, 'front'), 1).supL).toStrictEqual({ at: [95.5, 148], source: 'hand', pinned: true, inferred: false });
  });

  it('works out c7 again when a C7 corner moves', () => {
    const c = applied(frontCase(), { kind: 'move-mark', film: 'front', target: { on: 'vertebra', vertebra: 1, mark: 'infR' }, to: [184, 144], gesture: 3 });
    // x (100 + 180 + 184 + 100) / 4 = 141, y (100 + 100 + 144 + 140) / 4 = 121.
    expect(marks(c, 'front').landmarks.c7).toStrictEqual({ at: [141, 121], source: 'auto', pinned: false, inferred: false });
  });

  it('places a pedicle that could not be seen, unpinned and without a ring score', () => {
    const hidden = applied(frontCase(), { kind: 'hide-pedicle', film: 'front', vertebra: 3, mark: 'pedR' });
    const c = applied(hidden, { kind: 'move-mark', film: 'front', target: { on: 'vertebra', vertebra: 3, mark: 'pedR' }, to: [170, 215], gesture: null });
    const placed: PedicleMark = { seen: 'visible', at: [170, 215], source: 'hand', pinned: false, inferred: false, ringScore: null };
    expect(frontPoints(marks(c, 'front'), 2).pedR).toStrictEqual(placed);
  });

  it('moves one end of a bending film endplate line', () => {
    const c0 = sketchCase(['bend-left'], [sketchMarks('bend-left', 1, standardLevels('T5', 2))]);
    const c = applied(c0, { kind: 'move-mark', film: 'bend-left', target: { on: 'endplate', vertebra: 2, line: 'inf', end: 'patient-right' }, to: [190, 195], gesture: null });
    const vertebra = marks(c, 'bend-left').vertebrae[1];
    expect(vertebra?.marks).toStrictEqual({
      kind: 'bending',
      lines: { sup: [auto(100, 150), auto(180, 150)], inf: [auto(100, 190), { at: [190, 195], source: 'hand', pinned: false, inferred: false }] },
    });
  });

  it('moves a sacrum end and a side film hip centre, which loses its fitted radius', () => {
    const side = sketchMarks('side', 1, []);
    const withLandmarks: FilmMarks = {
      ...side,
      landmarks: { c7: null, sacrumTop: { view: 'side', sacA: auto(100, 900), sacP: auto(160, 920) }, hips: [{ side: 'unpaired', centre: auto(200, 1000), radiusPx: 25 }] },
    };
    const c0 = sketchCase(['side'], [withLandmarks]);
    const c = applied(
      c0,
      { kind: 'move-mark', film: 'side', target: { on: 'landmark', landmark: 'sacP' }, to: [162, 921], gesture: null },
      { kind: 'move-mark', film: 'side', target: { on: 'landmark', landmark: 'hipFirst' }, to: [205, 1003], gesture: null },
    );
    expect(marks(c, 'side').landmarks).toStrictEqual({
      c7: null,
      sacrumTop: { view: 'side', sacA: auto(100, 900), sacP: { at: [162, 921], source: 'hand', pinned: false, inferred: false } },
      hips: [{ side: 'unpaired', centre: { at: [205, 1003], source: 'hand', pinned: false, inferred: false }, radiusPx: null }],
    });
  });

  it('refuses a point the film does not have', () => {
    const c = frontCase();
    const move = (target: Parameters<typeof applyEdit>[1] & { kind: 'move-mark' }) => applyEdit(c, target);
    expect(move({ kind: 'move-mark', film: 'front', target: { on: 'vertebra', vertebra: 99, mark: 'supL' }, to: [1, 1], gesture: null })).toBeNull();
    expect(move({ kind: 'move-mark', film: 'front', target: { on: 'vertebra', vertebra: 1, mark: 'supA' }, to: [1, 1], gesture: null })).toBeNull();
    expect(move({ kind: 'move-mark', film: 'front', target: { on: 'endplate', vertebra: 1, line: 'sup', end: 'patient-left' }, to: [1, 1], gesture: null })).toBeNull();
    expect(move({ kind: 'move-mark', film: 'front', target: { on: 'landmark', landmark: 'sacL' }, to: [1, 1], gesture: null })).toBeNull();
    expect(move({ kind: 'move-mark', film: 'bend-left', target: { on: 'vertebra', vertebra: 1, mark: 'supL' }, to: [1, 1], gesture: null })).toBeNull();
  });
});

describe('pin-mark and hide-pedicle', () => {
  it('pins and unpins a point where it is', () => {
    const pinned = applied(frontCase(), { kind: 'pin-mark', film: 'side', target: { on: 'vertebra', vertebra: 5, mark: 'infP' }, pinned: true });
    const vertebra = marks(pinned, 'side').vertebrae[1];
    if (vertebra?.marks.kind !== 'side') throw new Error('side vertebra expected');
    expect(vertebra.marks.points.infP).toStrictEqual({ at: [180, 190], source: 'auto', pinned: true, inferred: false });
    const unpinned = applied(pinned, { kind: 'pin-mark', film: 'side', target: { on: 'vertebra', vertebra: 5, mark: 'infP' }, pinned: false });
    expect(marks(unpinned, 'side')).toStrictEqual(marks(frontCase(), 'side'));
  });

  it('says a pedicle cannot be seen, decided by hand, and then has nothing to pin', () => {
    const c = applied(frontCase(), { kind: 'hide-pedicle', film: 'front', vertebra: 2, mark: 'pedL' });
    expect(frontPoints(marks(c, 'front'), 1).pedL).toStrictEqual({ seen: 'not-visible', source: 'hand' });
    expect(applyEdit(c, { kind: 'pin-mark', film: 'front', target: { on: 'vertebra', vertebra: 2, mark: 'pedL' }, pinned: true })).toBeNull();
  });
});

describe('confirm-l5', () => {
  // TEMPLATE_NAMES.standard runs C1 … C7, T1 … T12, L1 … L7.
  const threeOnSide = () => sketchCase(['side'], [sketchMarks('side', 1, standardLevels('C7', 3))]);

  it('names the chosen vertebra L5 and the others along the count, and records it', () => {
    const c = applied(threeOnSide(), { kind: 'confirm-l5', film: 'side', vertebra: 3 });
    const side = marks(c, 'side');
    expect(side.vertebrae.map((v) => v.level)).toEqual(['L3', 'L4', 'L5']);
    expect(side.naming.l5ConfirmedOn).toBe(3);
    expect(side.naming.flags).toEqual([]);
    // No vertebra is C7 any more.
    expect(side.landmarks.c7).toBeNull();
  });

  it('flags a non-standard count when a name falls past the count', () => {
    const c = applied(threeOnSide(), { kind: 'confirm-l5', film: 'side', vertebra: 1 });
    expect(marks(c, 'side').vertebrae.map((v) => v.level)).toEqual(['L5', 'L6', 'L7']);
    expect(marks(c, 'side').naming.flags).toEqual(['non-standard-count']);
  });

  it('refuses an L5 that leaves a vertebra without a name', () => {
    const four = sketchCase(['side'], [sketchMarks('side', 1, standardLevels('C7', 4))]);
    expect(applyEdit(four, { kind: 'confirm-l5', film: 'side', vertebra: 1 })).toBeNull();
  });

  it('moves C7 onto the vertebra the new names make C7', () => {
    // Eighteen boxes named T1 … L6, one too low: L5 on the last puts C7 on the first.
    const levels = TEMPLATE_NAMES.standard.slice(7, 25);
    const c0 = sketchCase(['side'], [sketchMarks('side', 1, levels)]);
    expect(marks(c0, 'side').landmarks.c7).toBeNull();
    const c = applied(c0, { kind: 'confirm-l5', film: 'side', vertebra: 18 });
    expect(marks(c, 'side').vertebrae.map((v) => v.level)).toEqual(TEMPLATE_LEVELS.standard);
    // The first box: x (100 + 180 + 180 + 100) / 4 = 140, y (100 + 100 + 140 + 140) / 4 = 120.
    expect(marks(c, 'side').landmarks.c7?.at).toEqual([140, 120]);
  });
});

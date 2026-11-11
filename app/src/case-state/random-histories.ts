// Random histories of an open case, for the property tests: four films,
// edits from every screen, engine answers that are asked for at one
// version and arrive later in any order, undo, redo, a fresh history and
// opened cases. Region wand requests are the exception to "any order":
// a film's worker answers them in the order asked, as the engine does,
// and some get no answer at all, which the player tells the store, as
// the tool must, before the person clicks the same vertebra again.
// Tests only; nothing in the app imports this file.
//
// A history is a list of steps. `play` runs one through the reducer and
// tells its watcher what each step did, with the state before and after,
// so a test can hold every step to the rules. The engine's answers are
// made in made-up-answers.ts the way the engine makes them: from the case
// as it stood when they were asked for, new ids counting up from the
// case's `nextVertebraId` then, each film's above the film's before it.

import fc from 'fast-check';
import type { CaseFileV1, FilmGeometry, FilmKind, FilmMarks, ImageRight, PinTarget, Px, VertebraOnFilm, WandClick } from '@/shapes';
import type { ActionRecord, CalledAction } from './action-record';
import type { CaseEdit, MarksCause } from './case-edits';
import { cornersOf } from './c7-of';
import { applyAction, redo, startAfresh, stateOf, undo, wandClicksWith, wandUnanswered, type ActionReport, type CaseState } from './case-reducer';
import { emptyCase } from './empty-case';
import { answerOf, madeUpMarks, mergedMarks, nudged, passAnswers, pinned, splitMarks, withVertebra } from './made-up-answers';
import { between, onFilm, pick, randomNumbers } from './seeded-numbers';
import { FILM_KINDS, filmOf, marksOf } from './selectors';
import { isFrontView, sketchFilm, usualFacing } from './sketch-case';

const OPS = {
  // Edits from the screens, applied at once.
  film: 2,
  remove: 1,
  turn: 1,
  face: 2,
  scale: 1,
  flip: 1,
  blank: 1,
  drag: 4,
  pin: 2,
  hide: 1,
  'confirm-l5': 2,
  'confirm-l5-everywhere': 1,
  'looks-right': 1,
  // Engine answers, asked for now and arriving later.
  'ask-pass': 3,
  'ask-region-wand': 3,
  'ask-split': 2,
  'ask-split-on-two': 2,
  'ask-merge': 2,
  'ask-pin': 2,
  arrive: 10,
  // The history itself.
  undo: 5,
  redo: 3,
  afresh: 1,
  open: 1,
} as const;

export type Op = keyof typeof OPS;

/** One step: what happens, and two numbers that choose its film, vertebra, point and so on. */
export type Step = readonly [op: Op, a: number, b: number];

export interface RandomHistory {
  /** Chooses the case the history opens with. */
  opened: number;
  steps: readonly Step[];
}

export const randomHistories: fc.Arbitrary<RandomHistory> = fc.record({
  opened: fc.nat(),
  steps: fc.array(
    fc.tuple(
      fc.oneof(...(Object.entries(OPS) as [Op, number][]).map(([op, weight]) => ({ arbitrary: fc.constant(op), weight }))),
      fc.nat(),
      fc.nat(),
    ),
    { minLength: 20, maxLength: 100, size: 'large' },
  ),
});

/** What one step did. */
export type Happened =
  | {
      did: 'action';
      edits: CaseEdit[];
      late: boolean;
      /** For a region wand answer, the clicks its request was asked with; otherwise `null`. */
      wandClicks: readonly WandClick[] | null;
      report: ActionReport;
      before: CaseState;
      after: CaseState;
    }
  | { did: 'undo' | 'redo'; record: ActionRecord | null; before: CaseState; after: CaseState }
  | { did: 'afresh' | 'ask' | 'unanswered' | 'nothing'; before: CaseState; after: CaseState };

/**
 * Play `runs` random histories under fast-check, each told to a fresh
 * watcher from `watch`. A watcher that throws fails the run, and
 * fast-check then looks for the shortest history that still fails.
 */
export function eachRandomHistory(runs: number, watch: () => (happened: Happened) => void): void {
  fc.assert(
    fc.property(randomHistories, (history) => {
      play(history, watch());
    }),
    { numRuns: runs },
  );
}

/** Play a history from an empty case, telling `see` what each step did, the opening included. */
export function play(history: RandomHistory, see: (happened: Happened) => void): CaseState {
  const player = new Player(see);
  player.act([{ kind: 'open-case', case: madeUpCase(history.opened) }], false);
  for (const step of history.steps) player.step(step);
  return player.state;
}

interface Asked {
  edits: CaseEdit[];
  /** For a region wand request: its film, its number, the clicks it was asked with, and whether the engine refuses it. */
  wand?: { film: FilmKind; vertebra: number; ask: number; clicks: readonly WandClick[]; refused: boolean };
}

class Player {
  state: CaseState = stateOf(emptyCase());
  private readonly asked: Asked[] = [];
  private gestures = 0;

  constructor(private readonly see: (happened: Happened) => void) {}

  act(edits: CaseEdit[], late: boolean, as?: CalledAction, wandClicks: readonly WandClick[] | null = null): void {
    const before = this.state;
    const { state, report } = applyAction(before, edits, as === undefined ? {} : { as });
    this.state = state;
    this.see({ did: 'action', edits, late, wandClicks, report, before, after: state });
  }

  private quiet(did: 'afresh' | 'ask' | 'unanswered' | 'nothing', after: CaseState): void {
    const before = this.state;
    this.state = after;
    this.see({ did, before, after });
  }

  step([op, a, b]: Step): void {
    const c = this.state.case;
    const rng = randomNumbers(a * 7919 + b);
    switch (op) {
      case 'undo':
      case 'redo': {
        const before = this.state;
        const { state, record } = op === 'undo' ? undo(before) : redo(before);
        this.state = state;
        this.see({ did: op, record, before, after: state });
        return;
      }
      case 'afresh':
        this.quiet('afresh', startAfresh(this.state));
        return;
      case 'open':
        this.act([{ kind: 'open-case', case: madeUpCase(a) }], false);
        return;
      case 'arrive':
        this.arrive(a, b);
        return;
      case 'film':
        this.act([{ kind: 'set-film', film: sketchFilm(pick(FILM_KINDS, a) ?? 'front') }], false);
        return;
      default:
        break;
    }

    const film = pick(
      c.films.map((present) => present.kind),
      a,
    );
    if (film === undefined) {
      this.quiet('nothing', this.state);
      return;
    }
    const record = filmOf(c, film);
    const marks = marksOf(c, film);
    if (record === null) throw new Error('a film just picked is in the case');
    const geometry = record.geometry;

    switch (op) {
      case 'remove':
        this.act([{ kind: 'remove-film', film }], false);
        return;
      case 'turn':
        this.act([{ kind: 'set-geometry', film, geometry: { ...geometry, quarterTurns: turned(geometry, b) } }], false);
        return;
      case 'face':
        this.act([{ kind: 'set-geometry', film, geometry: faced(film, geometry, b) }], false);
        return;
      case 'scale':
        this.act([{ kind: 'set-geometry', film, geometry: scaled(geometry, b) }], false);
        return;
      case 'flip':
        this.act([{ kind: 'flip-brightness', film }], false);
        return;
      case 'blank':
        this.act([{ kind: 'set-blanked', film, boxes: b % 3 === 0 ? [] : [{ left: 0, top: 0, right: 10 + (b % 200), bottom: 40 }] }], false);
        return;
      case 'ask-pass':
        this.ask(passAnswers(c, this.state.version, rng));
        return;
      default:
        break;
    }

    if (marks === null || marks.vertebrae.length === 0) {
      this.quiet('nothing', this.state);
      return;
    }
    const index = b % marks.vertebrae.length;
    const vertebra = marks.vertebrae[index];
    if (vertebra === undefined) throw new Error('an index within the vertebrae');
    const target = pointOf(film, vertebra, marks, b);

    switch (op) {
      case 'drag': {
        // A drag is a few moves of one gesture, each its own action; a move with no gesture stands alone.
        const gesture = b % 5 === 0 ? null : (this.gestures += 1);
        const moves = gesture === null ? 1 : 1 + (b % 4);
        for (let move = 0; move < moves; move += 1) this.act([{ kind: 'move-mark', film, target, to: onFilm(rng), gesture }], false);
        return;
      }
      case 'pin':
        this.act([{ kind: 'pin-mark', film, target, pinned: b % 2 === 0 }], false);
        return;
      case 'hide':
        this.act([{ kind: 'hide-pedicle', film, vertebra: vertebra.id, mark: b % 2 === 0 ? 'pedL' : 'pedR' }], false);
        return;
      case 'confirm-l5':
        this.act([{ kind: 'confirm-l5', film, vertebra: vertebra.id }], false);
        return;
      case 'confirm-l5-everywhere':
        this.act(
          c.marks.flatMap((each) => {
            const chosen = each.vertebrae[Math.min(index, each.vertebrae.length - 1)];
            return chosen === undefined ? [] : [{ kind: 'confirm-l5' as const, film: each.film, vertebra: chosen.id }];
          }),
          false,
        );
        return;
      case 'looks-right':
        this.act(
          cornerTargets(film, vertebra).map(([corner, at]) => ({ kind: 'move-mark' as const, film, target: corner, to: at, gesture: null })),
          false,
          'confirm-vertebra',
        );
        return;
      case 'ask-region-wand': {
        if (film !== 'front' && film !== 'side') break;
        // Some refusals come back at once, before anything else changes the case.
        const atOnce = b % 6 === 0 && !this.asked.some((out) => out.wand?.film === film);
        this.askWand(film, vertebra.id, rng, b, b % 3 === 0);
        if (atOnce) this.arrive(this.asked.length - 1, b);
        return;
      }
      case 'ask-split': {
        if (film !== 'front' && film !== 'side') break;
        const split = splitMarks(marks, index, c.nextVertebraId);
        if (split === null) break;
        this.ask([answerOf(split, this.state.version, { by: 'boundary-wand', action: 'split', vertebra: vertebra.id })]);
        return;
      }
      case 'ask-split-on-two': {
        const front = marksOf(c, 'front');
        const side = marksOf(c, 'side');
        const onFront = front === null ? null : splitMarks(front, Math.min(index, front.vertebrae.length - 1), c.nextVertebraId);
        const onSide = side === null ? null : splitMarks(side, Math.min(index, side.vertebrae.length - 1), c.nextVertebraId + 1);
        if (front === null || side === null || onFront === null || onSide === null) break;
        const cause = (on: FilmMarks): MarksCause => ({ by: 'boundary-wand', action: 'split', vertebra: on.vertebrae[Math.min(index, on.vertebrae.length - 1)]?.id ?? 0 });
        this.ask([answerOf(onFront, this.state.version, cause(front)), answerOf(onSide, this.state.version, cause(side))]);
        return;
      }
      case 'ask-merge': {
        if (film !== 'front' && film !== 'side') break;
        const merged = mergedMarks(marks, Math.min(index, marks.vertebrae.length - 2));
        if (merged === null) break;
        this.ask([answerOf(merged, this.state.version, { by: 'boundary-wand', action: 'merge', vertebra: vertebra.id })]);
        return;
      }
      case 'ask-pin':
        this.ask([answerOf(pinned(marks, target, onFilm(rng)), this.state.version, { by: 'pin', target })]);
        return;
      default:
        break;
    }
    this.quiet('nothing', this.state);
  }

  /** The next answer arrives, chosen by `a`; a refusal is told to the store, and the person clicks the vertebra again. */
  private arrive(a: number, b: number): void {
    const answer = this.nextAnswer(a);
    if (answer === undefined) this.quiet('nothing', this.state);
    else if (answer.wand?.refused === true) {
      this.quiet('unanswered', wandUnanswered(this.state, answer.wand.ask));
      this.askWand(answer.wand.film, answer.wand.vertebra, randomNumbers(a + 1), b + 1, false);
    } else this.act(answer.edits, true, undefined, answer.wand?.clicks ?? null);
  }

  /**
   * The answer that arrives next, chosen by `a` from those asked. A
   * region wand answer, or its refusal, is instead the oldest region wand
   * request still out on the same film, since the film's worker answers
   * those in order.
   */
  private nextAnswer(a: number): Asked | undefined {
    const chosen = this.asked[a % Math.max(1, this.asked.length)];
    const film = chosen?.wand?.film;
    const index = film === undefined ? a % Math.max(1, this.asked.length) : this.asked.findIndex((asked) => asked.wand?.film === film);
    return this.asked.splice(index, 1)[0];
  }

  /** A region wand click on a vertebra, when its film still holds it; the engine will refuse it when `refused`. */
  private askWand(film: FilmKind, vertebra: number, rng: () => number, b: number, refused: boolean): void {
    const marks = marksOf(this.state.case, film);
    const index = marks?.vertebrae.findIndex((each) => each.id === vertebra) ?? -1;
    const target = marks?.vertebrae[index];
    const asked = wandClicksWith(this.state, film, vertebra, { at: onFilm(rng), polarity: b % 2 === 0 ? 'add' : 'subtract' });
    if (marks === null || target === undefined || asked === null) return;
    this.quiet('ask', asked.state);
    const edits = [answerOf(withVertebra(marks, index, nudged(target, rng)), this.state.version, { by: 'region-wand', vertebra })];
    this.asked.push({ edits, wand: { film, vertebra, ask: asked.ask, clicks: asked.clicks, refused } });
  }

  private ask(edits: CaseEdit[]): void {
    if (edits.length > 0) this.asked.push({ edits });
    this.quiet('ask', this.state);
  }
}

/** A case to open: each film there or not, some with marks, ids from 1 and the next id at or above the highest. */
export function madeUpCase(seed: number): CaseFileV1 {
  const rng = randomNumbers(seed);
  const c = emptyCase();
  let next = 1;
  for (const film of FILM_KINDS) {
    if (rng() < 0.2) continue;
    const record = sketchFilm(film);
    const facings: (ImageRight | null)[] = isFrontView(film) ? [null, 'patient-left', 'patient-right'] : [null, 'anterior', 'posterior'];
    const imageRight = pick(facings, between(rng, 0, 2)) ?? null;
    c.films.push({ ...record, geometry: { ...record.geometry, imageRight, quarterTurns: rng() < 0.2 ? 2 : 0 } });
    if (rng() < 0.3) continue;
    const ids = Array.from({ length: between(rng, 0, 5) }, (_, index) => next + index);
    next += ids.length;
    c.marks.push(madeUpMarks(film, ids, imageRight ?? usualFacing(film), rng));
  }
  return { ...c, nextVertebraId: next + between(rng, 0, 3) };
}

// ---- Edits from the screens.

function turned(geometry: FilmGeometry, b: number): FilmGeometry['quarterTurns'] {
  return ((geometry.quarterTurns + 1 + (b % 3)) % 4) as FilmGeometry['quarterTurns'];
}

/** The film turned to face the other way, or to a facing at all when it had none. */
function faced(film: FilmKind, geometry: FilmGeometry, b: number): FilmGeometry {
  const facings: ImageRight[] = isFrontView(film) ? ['patient-left', 'patient-right'] : ['anterior', 'posterior'];
  const imageRight = geometry.imageRight === null ? (pick(facings, b) ?? null) : (facings.find((facing) => facing !== geometry.imageRight) ?? null);
  const confirmed = isFrontView(film) ? { ...geometry.confirmed, leftRight: b % 2 === 0 } : { ...geometry.confirmed, facing: b % 2 === 0 };
  return { ...geometry, imageRight, confirmed };
}

/** A geometry change that leaves the marks alone: a scale, a confirmation, or the facing made unknown. */
function scaled(geometry: FilmGeometry, b: number): FilmGeometry {
  switch (b % 3) {
    case 0:
      return geometry.mmPerPx === null
        ? { ...geometry, mmPerPx: 0.15, scale: { source: 'ruler', rulers: [{ a: [10, 10], b: [210, 10], mm: 30 }] }, confirmed: { ...geometry.confirmed, scale: true } }
        : { ...geometry, mmPerPx: null, scale: null, confirmed: { ...geometry.confirmed, scale: false } };
    case 1:
      return { ...geometry, confirmed: { ...geometry.confirmed, upright: !geometry.confirmed.upright } };
    default:
      return { ...geometry, imageRight: null, confirmed: { ...geometry.confirmed, leftRight: false, facing: false } };
  }
}

/** One point of a vertebra, chosen by `b`, or a landmark when the film has one and `b` says so. */
function pointOf(film: FilmKind, vertebra: VertebraOnFilm, marks: FilmMarks, b: number): PinTarget {
  const top = marks.landmarks.sacrumTop;
  if (b % 7 === 0 && top !== null) return { on: 'landmark', landmark: top.view === 'front' ? 'sacL' : 'sacP' };
  if (b % 11 === 0 && marks.landmarks.hips.length > 0) {
    const hip = marks.landmarks.hips[0]?.side;
    return { on: 'landmark', landmark: hip === 'left' ? 'hipLeft' : hip === 'right' ? 'hipRight' : 'hipFirst' };
  }
  if (film === 'front') return { on: 'vertebra', vertebra: vertebra.id, mark: pick(['supL', 'supR', 'infL', 'infR', 'pedL', 'pedR'] as const, b) ?? 'supL' };
  if (film === 'side') return { on: 'vertebra', vertebra: vertebra.id, mark: pick(['supA', 'supP', 'infA', 'infP'] as const, b) ?? 'supA' };
  return { on: 'endplate', vertebra: vertebra.id, line: b % 2 === 0 ? 'sup' : 'inf', end: b % 4 < 2 ? 'patient-left' : 'patient-right' };
}

/** Each corner a person can see (not inferred), with where it is now: "Looks right". */
function cornerTargets(film: FilmKind, vertebra: VertebraOnFilm): [PinTarget, Px][] {
  const names: PinTarget[] =
    film === 'front'
      ? (['supL', 'supR', 'infR', 'infL'] as const).map((name) => ({ on: 'vertebra', vertebra: vertebra.id, mark: name }))
      : film === 'side'
        ? (['supA', 'supP', 'infP', 'infA'] as const).map((name) => ({ on: 'vertebra', vertebra: vertebra.id, mark: name }))
        : [
            { on: 'endplate', vertebra: vertebra.id, line: 'sup', end: 'patient-left' },
            { on: 'endplate', vertebra: vertebra.id, line: 'sup', end: 'patient-right' },
            { on: 'endplate', vertebra: vertebra.id, line: 'inf', end: 'patient-right' },
            { on: 'endplate', vertebra: vertebra.id, line: 'inf', end: 'patient-left' },
          ];
  return cornersOf(vertebra.marks).flatMap((corner, index): [PinTarget, Px][] => {
    const target = names[index];
    return corner.inferred || target === undefined ? [] : [[target, corner.at]];
  });
}

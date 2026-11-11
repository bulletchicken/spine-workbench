// A recording → what stands on each film at time t. Pure: the same
// recording and the same t always give the same picture, so playing,
// scrubbing both ways and replaying need no state.
//
// During the search the dashed guesses are the whole chain of the newest
// sweep frame at or before t: each sweep frame holds the best chain
// ending at its level, so earlier guesses may move from frame to frame,
// and the picture follows them. A sweep frame arrives on screen when the
// sweep line reaches its lowest body: 5.4 s plus 3.6 s times how far down
// the strip that body sits. Those arrival times are given out
// (`sweepTimes`), so the lane and the marks keep the same beat.
//
// When the search ends, the vertebrae are the bodies of the locked chain.
// Sweep frames come only from the standard count, and the locked chain
// may use another and is named afterwards, so a locked body is matched to
// the last sweep frame's guesses by where it sits on the strip, never by
// name: the guess whose middle is on the strip row nearest its own, if
// that is within half its height. A matched body stands as that guess
// until it locks; one with no match appears at lock-in; a guess no body
// took leaves when the lock-in reaches its row. A body moved when either
// endplate sits more than a quarter of its height from its guess's.
//
// Each body locks at 9.15 s plus 36 ms for each place from the bottom,
// is named at 9.8 s plus 38 ms for each place from the nearer end, and is
// coloured at 10.4 s plus 34 ms for each place from the top, when its
// corners become the final outline's. Each time is held no earlier than
// the one before it, so the phases come in order for every body. As with
// a moved body's guess, the corners it was locked on stay given from its
// colouring on, so the outline can ease from them without the picture
// depending on the moment before.

import type { Phase } from '@/look-and-feel/data-attributes';
import type { ChainBody, ConfidenceBand, FilmKind, Level, Px } from '@/shapes';
import { PASS_TIMES } from './beats';
import type { FilmRecording, PassRecording, StripShape } from './recording/pass-recording';
import { nearestRow } from './recording/strip-position';

/** Four corners round an outline: upper left, upper right, lower right, lower left. */
export type Quad = [Px, Px, Px, Px];

/** One dashed guess of the search. */
export interface Guess {
  /** Its place in its chain, top to bottom. */
  index: number;
  /** The level the standard count gives it; not a name the pass has settled. */
  level: Level;
  corners: Quad;
}

/** One body of the locked chain at time t. */
export interface StandingVertebra {
  /** Its place in the locked chain, top to bottom; the same for the whole pass. */
  index: number;
  phase: Phase;
  /** Where its outline stands; null while hidden. */
  corners: Quad | null;
  /** Its name, once named. */
  level: Level | null;
  /** How sure the pass is of it, once coloured. */
  band: ConfidenceBand | null;
  /** The search's last guess for it, from its lock on, when lock-in moved it. */
  movedFrom: Quad | null;
  /** The corners it was locked on, from its colouring on, for its final corners to ease from. */
  colouredFrom: Quad | null;
}

/** What stands on one film at time t. */
export interface FilmPicture {
  guesses: Guess[];
  vertebrae: StandingVertebra[];
  /** Sweep frames the search has reached, of all it sent. */
  searched: number;
  searchTotal: number;
  /** Bodies locked so far. */
  locked: number;
}

/** When one body of the locked chain reaches each phase; null for a phase it never reaches. */
export interface VertebraTimes {
  index: number;
  /**
   * When it first stands, as the guess it was matched to: the end of the
   * search for every matched body, not when the sweep reached its level
   * (that is in `sweepTimes`). Null when no guess was within half its
   * height, so it first stands at its lock.
   */
  ghostAt: number | null;
  lockAt: number;
  nameAt: number | null;
  colourAt: number | null;
}

export interface PassTimeline {
  /** What stands on a film at time t, or null when the pass did not run that film. */
  at(film: FilmKind, t: number): FilmPicture | null;
  /** Each locked body's times on a film, top to bottom. */
  times(film: FilmKind): VertebraTimes[];
  /**
   * When each sweep frame of a film arrives on screen, in the order sent,
   * never earlier than the one before; `searched` at time t counts those at
   * or before t.
   */
  sweepTimes(film: FilmKind): number[];
  /** A whole number that changes whenever the picture on any film changes. */
  changeIndex(t: number): number;
}

interface VertebraPlan extends VertebraTimes {
  locked: Quad;
  final: Quad;
  ghost: Quad | null;
  movedFrom: Quad | null;
  level: Level | null;
  band: ConfidenceBand | null;
}

interface FilmPlan {
  sweeps: { at: number; guesses: Guess[] }[];
  leftovers: { guess: Guess; leaveAt: number }[];
  vertebrae: VertebraPlan[];
  events: number[];
}

const SEARCH_END = PASS_TIMES.sweepUntil;

export function passTimeline(recording: PassRecording): PassTimeline {
  const plans = new Map(recording.films.map((part) => [part.film, planFilm(part)]));
  const events = [...plans.values()].flatMap((plan) => plan.events).sort((a, b) => a - b);
  return {
    at(film, t) {
      const plan = plans.get(film);
      return plan === undefined ? null : pictureAt(plan, t);
    },
    times(film) {
      return (plans.get(film)?.vertebrae ?? []).map(({ index, ghostAt, lockAt, nameAt, colourAt }) => ({ index, ghostAt, lockAt, nameAt, colourAt }));
    },
    sweepTimes(film) {
      return (plans.get(film)?.sweeps ?? []).map((sweep) => sweep.at);
    },
    changeIndex(t) {
      return countAtOrBefore(events, t);
    },
  };
}

function pictureAt(plan: FilmPlan, t: number): FilmPicture {
  const reached = plan.sweeps.filter((sweep) => sweep.at <= t);
  const guesses = t < SEARCH_END ? (reached.at(-1)?.guesses ?? []) : plan.leftovers.filter((left) => t < left.leaveAt).map((left) => left.guess);
  const vertebrae = plan.vertebrae.map((v): StandingVertebra => {
    const phase = phaseAt(v, t);
    const corners = phase === 'hidden' ? null : phase === 'ghost' ? v.ghost : phase === 'coloured' ? v.final : v.locked;
    return {
      index: v.index,
      phase,
      corners,
      level: phase === 'named' || phase === 'coloured' ? v.level : null,
      band: phase === 'coloured' ? v.band : null,
      movedFrom: t >= v.lockAt ? v.movedFrom : null,
      colouredFrom: phase === 'coloured' ? v.locked : null,
    };
  });
  return {
    guesses,
    vertebrae,
    searched: reached.length,
    searchTotal: plan.sweeps.length,
    locked: plan.vertebrae.filter((v) => t >= v.lockAt).length,
  };
}

function phaseAt(v: VertebraPlan, t: number): Phase {
  if (v.colourAt !== null && t >= v.colourAt) return 'coloured';
  if (v.nameAt !== null && t >= v.nameAt) return 'named';
  if (t >= v.lockAt) return 'locked';
  if (v.ghostAt !== null && t >= v.ghostAt) return 'ghost';
  return 'hidden';
}

function planFilm(part: FilmRecording): FilmPlan {
  const shape = part.stripShape !== null && part.stripShape.centres.length > 0 ? part.stripShape : null;
  const sweeps = shape === null ? [] : planSweeps(part, shape);
  const lastGuesses = sweeps.at(-1)?.guesses ?? [];

  const bottomUp = part.locked ?? [];
  const n = bottomUp.length;
  const levels = part.named !== null && part.named.levels.length === n ? part.named.levels : null;
  const matchedGuesses = new Set<Guess>();

  const vertebrae = bottomUp
    .slice()
    .reverse()
    .map((body, index): VertebraPlan => {
      const locked = quadOf(body);
      const guess = shape === null ? null : matchingGuess(shape, body, lastGuesses);
      if (guess !== null) matchedGuesses.add(guess);
      const level = levels?.[index] ?? null;
      const lockAt = PASS_TIMES.lockFrom + PASS_TIMES.lockGap * (n - 1 - index);
      const nameAt = level === null ? null : Math.max(lockAt, PASS_TIMES.namesFrom + PASS_TIMES.nameGap * Math.min(index, n - 1 - index));
      const band = level === null ? null : (part.confidence?.find((c) => c.level === level)?.band ?? null);
      const colourAt = band === null || nameAt === null ? null : Math.max(nameAt, PASS_TIMES.colourFrom + PASS_TIMES.colourGap * index);
      const outline = level === null ? undefined : part.outlines?.find((quad) => quad.level === level);
      return {
        index,
        ghostAt: guess === null ? null : SEARCH_END,
        lockAt,
        nameAt,
        colourAt,
        locked,
        final: outline === undefined ? locked : outline.corners,
        ghost: guess === null ? null : guess.corners,
        movedFrom: guess !== null && moved(body, guess) ? guess.corners : null,
        level,
        band,
      };
    });

  const leftovers = lastGuesses
    .filter((guess) => !matchedGuesses.has(guess))
    .map((guess) => ({ guess, leaveAt: shape === null ? PASS_TIMES.lockFrom : leaveTime(shape, guess, vertebrae) }));

  const events = [
    ...sweeps.map((sweep) => sweep.at),
    ...(sweeps.length > 0 || vertebrae.length > 0 ? [SEARCH_END] : []),
    ...vertebrae.flatMap((v) => [v.lockAt, ...(v.nameAt === null ? [] : [v.nameAt]), ...(v.colourAt === null ? [] : [v.colourAt])]),
    ...leftovers.map((left) => left.leaveAt),
  ];
  return { sweeps, leftovers, vertebrae, events };
}

/** Each sweep frame's guesses, top to bottom, and when it arrives on screen. */
function planSweeps(part: FilmRecording, shape: StripShape): FilmPlan['sweeps'] {
  let latest: number = PASS_TIMES.sweepFrom;
  return part.sweeps.map((sweep) => {
    const rows = sweep.tentative.map((body) => rowOf(shape, middle(body)));
    const guesses = sweep.tentative
      .map((body, k) => ({ body, row: rows[k] ?? 0 }))
      .sort((p, q) => p.row - q.row)
      .map(({ body }, index): Guess => ({ index, level: body.level, corners: quadOf(body) }));
    if (rows.length > 0) {
      const lowest = Math.max(...rows);
      latest = Math.max(latest, PASS_TIMES.sweepFrom + (PASS_TIMES.sweepUntil - PASS_TIMES.sweepFrom) * downTheStrip(shape, lowest));
    }
    return { at: latest, guesses };
  });
}

/** The guess on the row nearest the body's middle, if within half the body's height in rows. */
function matchingGuess(shape: StripShape, body: ChainBody, guesses: readonly Guess[]): Guess | null {
  const row = rowOf(shape, middle(body));
  const height = Math.abs(rowOf(shape, centre(body.sup)) - rowOf(shape, centre(body.inf)));
  let best: Guess | null = null;
  let bestDistance = Infinity;
  for (const guess of guesses) {
    const distance = Math.abs(rowOf(shape, mean(guess.corners)) - row);
    if (distance < bestDistance) {
      best = guess;
      bestDistance = distance;
    }
  }
  return bestDistance <= height / 2 ? best : null;
}

/** Whether either endplate of the locked body sits more than a quarter of its height from the guess's. */
function moved(body: ChainBody, guess: Guess): boolean {
  const [supA, supB, infB, infA] = guess.corners;
  const height = distance(centre(body.sup), centre(body.inf));
  const supMoved = distance(centre(body.sup), centre({ a: supA, b: supB }));
  const infMoved = distance(centre(body.inf), centre({ a: infA, b: infB }));
  return supMoved > height / 4 || infMoved > height / 4;
}

/** A leftover guess leaves when the body nearest it on the strip locks. */
function leaveTime(shape: StripShape, guess: Guess, vertebrae: readonly VertebraPlan[]): number {
  const row = rowOf(shape, mean(guess.corners));
  let leaveAt: number = PASS_TIMES.lockFrom;
  let bestDistance = Infinity;
  for (const v of vertebrae) {
    const distance = Math.abs(rowOf(shape, mean(v.locked)) - row);
    if (distance < bestDistance) {
      leaveAt = v.lockAt;
      bestDistance = distance;
    }
  }
  return leaveAt;
}

function quadOf(body: ChainBody): Quad {
  return [body.sup.a, body.sup.b, body.inf.b, body.inf.a];
}

function middle(body: ChainBody): Px {
  return mean(quadOf(body));
}

function centre(line: { a: Px; b: Px }): Px {
  return [(line.a[0] + line.b[0]) / 2, (line.a[1] + line.b[1]) / 2];
}

function mean(corners: Quad): Px {
  return [(corners[0][0] + corners[1][0] + corners[2][0] + corners[3][0]) / 4, (corners[0][1] + corners[1][1] + corners[2][1] + corners[3][1]) / 4];
}

function distance(p: Px, q: Px): number {
  return Math.hypot(p[0] - q[0], p[1] - q[1]);
}

function rowOf(shape: StripShape, point: Px): number {
  return nearestRow(shape, point) ?? 0;
}

/** 0 at the strip's first row, 1 at its last. */
function downTheStrip(shape: StripShape, row: number): number {
  const last = shape.centres.length - 1;
  return last > 0 ? row / last : 0;
}

function countAtOrBefore(sorted: readonly number[], t: number): number {
  let low = 0;
  let high = sorted.length;
  while (low < high) {
    const mid = (low + high) >> 1;
    if ((sorted[mid] ?? Infinity) <= t) low = mid + 1;
    else high = mid;
  }
  return low;
}

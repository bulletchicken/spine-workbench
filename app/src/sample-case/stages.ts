// The sample case at each stage of the design's story, as the case store
// would hold it, with each film's pixels for the session; and the findings
// a check of the case gives there.
//
//   four-films-up   the four films hung, nothing known about them
//   films-all-set   sizes and facings in: the front film's left and right
//                   answered, bending right measured with the ruler,
//                   bending left turned upright, the rest read from files
//   pass-result     "Find the spine" pressed (confirming upright, facing
//                   and size for the facts shown) and the pass landed
//   l5-set          L5 confirmed on every film
//   after-split     the tall outline split on Front and Side
//   t3-fixed        the region wand has added T3's missed bone on Front
//   all-clear       T1 confirmed with "Looks right" on Front and Side
//
// Each stage is the one before with the edits of the story applied by the
// frozen rules in app/src/case-state/case-edits.ts and the naming rules
// in engine/shapes (docs/conventions/mark-and-level-names.md):
// - `confirm-l5` makes the chosen vertebra L5 and gives every other the
//   name before or after it in `TEMPLATE_NAMES`, so the C7 body becomes
//   T1, the sacrum's outline L6 (flagged `non-standard-count`), and C7,
//   whose vertebra is gone, leaves `landmarks.c7` empty.
// - A boundary wand split keeps the parent's id on the upper piece and
//   gives the lower piece the case's next id, Front before Side. The
//   names on one side of the edit shift by one step: the side away from
//   the confirmed L5, so L5 keeps its name and its confirmation, and the
//   lower piece takes the parent's name. The counts are made again and
//   now agree.
// - The vertebrae a tool was aimed at carry `edited-by-user`.
// - "Looks right" is a `move-mark` of each corner that is not inferred to
//   where it already is: its source becomes `hand`.

import type {
  CaseFileV1,
  CheckFinding,
  CheckKind,
  Film,
  FilmGeometry,
  FilmKind,
  FilmMarks,
  Level,
  Mark,
  Px,
  Severity,
  VertebraOnFilm,
} from '@/shapes';
import { TEMPLATE_LEVELS, TEMPLATE_NAMES } from '@/shapes';
import { BALL, boneOf, FILM_KINDS, MM_PER_PX } from './anatomy';
import { SAMPLE_FILMS } from './films';
import { c7Of, defaultPedicles, filmPoint, inferredPedicle, passMarks, withIdsFrom } from './marks';

export const STORY_STAGES = ['four-films-up', 'films-all-set', 'pass-result', 'l5-set', 'after-split', 't3-fixed', 'all-clear'] as const;
export type StoryStage = (typeof STORY_STAGES)[number];

/** The app's version, the only thing a case records about what saved it. */
export const APP_VERSION = '0.1.0';

/** The first id the pass gives out: the case's `nextVertebraId` before it. */
export const FIRST_VERTEBRA_ID = 1;

export interface StageCase {
  case: CaseFileV1;
  /** Each film's pixels and thumbnail, which the session keeps beside the case. */
  films: Record<FilmKind, { pixels(): Uint16Array; thumbnail: Uint8Array }>;
}

const NOTHING_CONFIRMED = { upright: false, leftRight: false, facing: false, scale: false, polarity: false, privacy: false };

const NOTHING_KNOWN: FilmGeometry = {
  mmPerPx: null,
  mmPerPxVertical: null,
  scale: null,
  quarterTurns: 0,
  imageRight: null,
  confirmed: NOTHING_CONFIRMED,
};

/** The two clicks across the marker ball on bending right, end to end through its centre. */
export function rulerAcrossTheBall(): { a: Px; b: Px; mm: number } {
  const left = filmPoint('bend-right', { x: BALL.x - BALL.r, y: BALL.y });
  const right = filmPoint('bend-right', { x: BALL.x + BALL.r, y: BALL.y });
  return { a: left, b: right, mm: 25 };
}

/** Each film's geometry once sizes and facings are in, before "Find the spine" confirms them. */
function geometryAllSet(kind: FilmKind): FilmGeometry {
  const fromFile = { mmPerPx: MM_PER_PX, mmPerPxVertical: null, scale: { source: 'file', rule: 'geometry' } } as const;
  switch (kind) {
    case 'front':
      return { ...fromFile, quarterTurns: 0, imageRight: 'patient-left', confirmed: { ...NOTHING_CONFIRMED, leftRight: true } };
    case 'side':
      return { ...fromFile, quarterTurns: 0, imageRight: 'anterior', confirmed: NOTHING_CONFIRMED };
    case 'bend-left':
      return { ...fromFile, quarterTurns: 2, imageRight: 'patient-left', confirmed: NOTHING_CONFIRMED };
    case 'bend-right': {
      const ruler = rulerAcrossTheBall();
      const length = Math.hypot(ruler.b[0] - ruler.a[0], ruler.b[1] - ruler.a[1]);
      return {
        mmPerPx: ruler.mm / length,
        mmPerPxVertical: null,
        scale: { source: 'ruler', rulers: [ruler] },
        quarterTurns: 0,
        imageRight: 'patient-left',
        confirmed: { ...NOTHING_CONFIRMED, scale: true },
      };
    }
  }
}

/** After "Find the spine": upright, facing and size confirmed for the facts shown; left and right only where answered. */
function geometryConfirmed(kind: FilmKind): FilmGeometry {
  const geometry = geometryAllSet(kind);
  return {
    ...geometry,
    confirmed: { ...geometry.confirmed, upright: true, facing: kind === 'side', scale: true },
  };
}

function filmRecord(kind: FilmKind, geometry: FilmGeometry): Film {
  const facts = SAMPLE_FILMS[kind].facts;
  return {
    kind,
    image: { path: `films/${kind}.png`, width: facts.width, height: facts.height, bitsStored: facts.bitsStored },
    source: facts.source,
    keptTags: facts.keptTags,
    values: facts.values,
    flags: facts.flags,
    blanked: [],
    geometry,
  };
}

/** The `confirm-l5` edit on one film. */
export function confirmL5(marks: FilmMarks, vertebraId: number): FilmMarks {
  const names = TEMPLATE_NAMES[marks.naming.template];
  const levels = TEMPLATE_LEVELS[marks.naming.template];
  const chosen = marks.vertebrae.findIndex((vertebra) => vertebra.id === vertebraId);
  const l5 = names.indexOf('L5');
  const vertebrae = marks.vertebrae.map((vertebra, k) => ({ ...vertebra, level: names[l5 + k - chosen] ?? vertebra.level }));
  const outside = vertebrae.some((vertebra) => !levels.includes(vertebra.level));
  const flags = marks.naming.flags.filter((flag) => flag !== 'non-standard-count');
  return {
    ...marks,
    vertebrae,
    landmarks: { ...marks.landmarks, c7: c7Of(vertebrae) },
    naming: { ...marks.naming, flags: outside ? [...flags, 'non-standard-count'] : flags, l5ConfirmedOn: vertebraId },
  };
}

const moved = (to: Px, source: Mark['source']): Mark => ({ at: to, source, pinned: false, inferred: false });

/** The boundary wand's split of T11 and T12, taken as one, at the disc between them. */
export function splitTheTallOne(marks: FilmMarks, newId: number): FilmMarks {
  const kind = marks.film;
  const at = marks.vertebrae.findIndex((vertebra) => vertebra.confidence.reasons.includes('odd-height'));
  const parent = marks.vertebrae[at];
  if (parent === undefined) return marks;
  const upperBone = boneOf(kind, 11);
  const lowerBone = boneOf(kind, 12);
  // The new endplates, patient left first: image right, as on every film here.
  const upperInf = { left: filmPoint(kind, upperBone.q[2]), right: filmPoint(kind, upperBone.q[3]) };
  const lowerSup = { left: filmPoint(kind, lowerBone.q[1]), right: filmPoint(kind, lowerBone.q[0]) };
  const confidence = { ...parent.confidence, reasons: ['edited-by-user' as const], endplateSpreadPx: null, margin: null };
  let upper: VertebraOnFilm;
  let lower: VertebraOnFilm;
  if (parent.marks.kind === 'front') {
    const p = parent.marks.points;
    const lowerSupL = moved(lowerSup.left, 'boundary-wand');
    const lowerSupR = moved(lowerSup.right, 'boundary-wand');
    const pedicles = defaultPedicles(lowerSupL.at, lowerSupR.at, p.infL.at, p.infR.at);
    upper = {
      ...parent,
      confidence,
      marks: { kind: 'front', points: { ...p, infL: moved(upperInf.left, 'boundary-wand'), infR: moved(upperInf.right, 'boundary-wand') } },
    };
    lower = {
      ...parent,
      id: newId,
      confidence,
      marks: {
        kind: 'front',
        points: {
          supL: lowerSupL,
          supR: lowerSupR,
          infL: p.infL,
          infR: p.infR,
          pedL: inferredPedicle(pedicles.pedL, 'boundary-wand'),
          pedR: inferredPedicle(pedicles.pedR, 'boundary-wand'),
        },
      },
    };
  } else if (parent.marks.kind === 'side') {
    const p = parent.marks.points;
    upper = { ...parent, confidence, marks: { kind: 'side', points: { ...p, infA: moved(upperInf.left, 'boundary-wand'), infP: moved(upperInf.right, 'boundary-wand') } } };
    lower = {
      ...parent,
      id: newId,
      confidence,
      marks: { kind: 'side', points: { supA: moved(lowerSup.left, 'boundary-wand'), supP: moved(lowerSup.right, 'boundary-wand'), infA: p.infA, infP: p.infP } },
    };
  } else {
    return marks;
  }

  // Names: from the lower piece down they stay; above it, each takes the name one step up.
  const names = TEMPLATE_NAMES[marks.naming.template];
  const stepUp = (level: Level) => names[names.indexOf(level) - 1] ?? level;
  const vertebrae = [
    ...marks.vertebrae.slice(0, at).map((vertebra) => ({ ...vertebra, level: stepUp(vertebra.level) })),
    { ...upper, level: stepUp(parent.level) },
    { ...lower, level: parent.level },
    ...marks.vertebrae.slice(at + 1),
  ];
  const counted = vertebrae.map((vertebra) => vertebra.level);
  return {
    ...marks,
    vertebrae,
    landmarks: { ...marks.landmarks, c7: c7Of(vertebrae) },
    naming: {
      ...marks.naming,
      fromTop: counted,
      fromBottom: [...counted],
      flags: marks.naming.flags.filter((flag) => flag !== 'ends-disagree'),
    },
  };
}

/** The region wand's click on the bone T3's outline missed, on the patient's left of the front film. */
export function addT3sMissedBone(marks: FilmMarks): FilmMarks {
  const t3 = boneOf('front', 3);
  const vertebrae = marks.vertebrae.map((vertebra): VertebraOnFilm => {
    if (vertebra.level !== 'T3' || vertebra.marks.kind !== 'front') return vertebra;
    const points = vertebra.marks.points;
    const supL = moved(filmPoint('front', t3.q[1]), 'region-wand');
    const infL = moved(filmPoint('front', t3.q[2]), 'region-wand');
    const corners = [points.supR.at, supL.at, infL.at, points.infR.at];
    // The region found: the outline's corners and the middle of each side.
    const outline = corners.flatMap((corner, k): Px[] => {
      const next = corners[(k + 1) % 4] ?? corner;
      return [corner, [(corner[0] + next[0]) / 2, (corner[1] + next[1]) / 2]];
    });
    return {
      ...vertebra,
      marks: { kind: 'front', points: { ...points, supL, infL } },
      confidence: { ...vertebra.confidence, reasons: [...vertebra.confidence.reasons, 'edited-by-user'] },
      outline,
    };
  });
  return { ...marks, vertebrae };
}

/** "Looks right" on T1: each corner that is not inferred, moved by hand to where it is. */
export function confirmT1(marks: FilmMarks): FilmMarks {
  const byHand = (mark: Mark): Mark => (mark.inferred ? mark : { ...mark, source: 'hand' });
  const vertebrae = marks.vertebrae.map((vertebra): VertebraOnFilm => {
    if (vertebra.level !== 'T1') return vertebra;
    const m = vertebra.marks;
    if (m.kind === 'front') {
      const p = m.points;
      return { ...vertebra, marks: { kind: 'front', points: { ...p, supL: byHand(p.supL), supR: byHand(p.supR), infL: byHand(p.infL), infR: byHand(p.infR) } } };
    }
    if (m.kind === 'side') {
      const p = m.points;
      return { ...vertebra, marks: { kind: 'side', points: { supA: byHand(p.supA), supP: byHand(p.supP), infA: byHand(p.infA), infP: byHand(p.infP) } } };
    }
    return vertebra;
  });
  return { ...marks, vertebrae };
}

/** The vertebra each film confirms as L5: the one above the sacrum, which Front and Side count from it. */
function l5Of(marks: FilmMarks): number {
  const fromSacrum = marks.naming.fromBottom ?? marks.vertebrae.map((vertebra) => vertebra.level);
  const k = fromSacrum.indexOf('L5');
  return marks.vertebrae[k]?.id ?? -1;
}

const onFrontAndSide = (marks: FilmMarks[], edit: (film: FilmMarks) => FilmMarks) =>
  marks.map((film) => (film.film === 'front' || film.film === 'side' ? edit(film) : film));

function marksAt(stage: StoryStage): { marks: FilmMarks[]; nextVertebraId: number } {
  const pass = withIdsFrom(passMarks(), FIRST_VERTEBRA_ID);
  const afterPass = FIRST_VERTEBRA_ID + pass.reduce((sum, film) => sum + film.vertebrae.length, 0);
  if (stage === 'four-films-up' || stage === 'films-all-set') return { marks: [], nextVertebraId: FIRST_VERTEBRA_ID };
  if (stage === 'pass-result') return { marks: pass, nextVertebraId: afterPass };
  const l5Set = pass.map((film) => confirmL5(film, l5Of(film)));
  if (stage === 'l5-set') return { marks: l5Set, nextVertebraId: afterPass };
  // Front's lower piece takes the next id, then Side's.
  const split = l5Set.map((film) => (film.film === 'front' ? splitTheTallOne(film, afterPass) : film.film === 'side' ? splitTheTallOne(film, afterPass + 1) : film));
  if (stage === 'after-split') return { marks: split, nextVertebraId: afterPass + 2 };
  const fixed = split.map((film) => (film.film === 'front' ? addT3sMissedBone(film) : film));
  if (stage === 't3-fixed') return { marks: fixed, nextVertebraId: afterPass + 2 };
  return { marks: onFrontAndSide(fixed, confirmT1), nextVertebraId: afterPass + 2 };
}

/** A film's pixels and thumbnail as the session keeps them. */
function sessionFilm(kind: FilmKind): StageCase['films'][FilmKind] {
  return {
    pixels: () => SAMPLE_FILMS[kind].pixels(),
    get thumbnail() {
      return SAMPLE_FILMS[kind].thumbnail;
    },
  };
}

/** The case at a stage of the story, made afresh on every call. */
export function caseAt(stage: StoryStage): StageCase {
  const geometry = stage === 'four-films-up' ? () => NOTHING_KNOWN : stage === 'films-all-set' ? geometryAllSet : geometryConfirmed;
  const { marks, nextVertebraId } = marksAt(stage);
  const c: CaseFileV1 = {
    format: 'spine-case',
    version: 1,
    savedWith: { app: APP_VERSION },
    films: FILM_KINDS.map((kind) => filmRecord(kind, geometry(kind))),
    marks,
    nextVertebraId,
  };
  return {
    case: structuredClone(c),
    films: { front: sessionFilm('front'), side: sessionFilm('side'), 'bend-left': sessionFilm('bend-left'), 'bend-right': sessionFilm('bend-right') },
  };
}

// ---- What a check of the case finds at each stage.

const finding = (kind: CheckKind, severity: Severity, film: FilmKind | null, vertebra?: VertebraOnFilm): CheckFinding => ({
  kind,
  severity,
  film,
  vertebraId: vertebra?.id ?? null,
  level: vertebra?.level ?? null,
});

/**
 * The findings a check of the case gives at a stage, worked out by these
 * rules, film by film in the case's order:
 * - a vertebra-missing finding (blocking) for each outline still carrying
 *   `odd-height`, which stands in for the vertebra it swallowed;
 * - likely-off-by-one (warning) on a film whose two counts disagree and
 *   whose L5 is not confirmed, pointing at the vertebra its names call L5;
 * - non-standard-count (warning) on a film flagged so, pointing at the
 *   vertebra outside its count;
 * - l5-unconfirmed (warning) on every film with marks and no L5 confirmed;
 * - upright, left-right (front and bending films), facing (the side film)
 *   and scale unconfirmed, for each fact nobody confirmed, left and right
 *   blocking on the front film and warnings elsewhere; polarity and
 *   privacy, which the design never asks, on every film;
 * - pedicle-not-visible (warning) for each pedicle marked not visible.
 */
export function findingsAt(stage: StoryStage): CheckFinding[] {
  const { case: c } = caseAt(stage);
  const findings: CheckFinding[] = [];
  for (const film of c.films) {
    const kind = film.kind;
    const marks = c.marks.find((m) => m.film === kind);
    if (marks !== undefined) {
      for (const vertebra of marks.vertebrae) {
        if (vertebra.confidence.reasons.includes('odd-height')) findings.push(finding('vertebra-missing', 'blocking', kind, vertebra));
      }
      const naming = marks.naming;
      if (naming.l5ConfirmedOn === null && naming.flags.includes('ends-disagree')) {
        findings.push(finding('likely-off-by-one', 'warning', kind, marks.vertebrae.find((vertebra) => vertebra.level === 'L5')));
      }
      if (naming.flags.includes('non-standard-count')) {
        const levels = TEMPLATE_LEVELS[naming.template];
        findings.push(finding('non-standard-count', 'warning', kind, marks.vertebrae.find((vertebra) => !levels.includes(vertebra.level))));
      }
      if (naming.l5ConfirmedOn === null) findings.push(finding('l5-unconfirmed', 'warning', kind));
    }
    const confirmed = film.geometry.confirmed;
    const frontView = kind !== 'side';
    if (!confirmed.upright) findings.push(finding('upright-unconfirmed', 'warning', kind));
    if (frontView && !confirmed.leftRight) findings.push(finding('left-right-unconfirmed', kind === 'front' ? 'blocking' : 'warning', kind));
    if (!frontView && !confirmed.facing) findings.push(finding('facing-unconfirmed', 'warning', kind));
    if (!confirmed.scale) findings.push(finding('scale-unconfirmed', 'warning', kind));
    if (!confirmed.polarity) findings.push(finding('polarity-unconfirmed', 'warning', kind));
    if (!confirmed.privacy) findings.push(finding('privacy-unchecked', 'warning', kind));
    for (const vertebra of marks?.vertebrae ?? []) {
      if (vertebra.marks.kind !== 'front') continue;
      for (const pedicle of [vertebra.marks.points.pedL, vertebra.marks.points.pedR]) {
        if (pedicle.seen === 'not-visible') findings.push(finding('pedicle-not-visible', 'warning', kind, vertebra));
      }
    }
  }
  return findings;
}

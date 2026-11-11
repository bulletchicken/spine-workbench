// The stand-in's pins: a pin puts one point exactly where it is dragged
// and holds it there, and the facing corner across the disc gives a
// little. Unpinning leaves the point where it is.

import type { FilmMarks, Mark, PedicleMark, PinInput, PinOutput, PinTarget, VertebraOnFilm } from '@/shapes';
import { EngineFailure } from '../engine-client';
import { add, c7Of, cornersOf, sub, times, withCorners, type Corners } from './quads';
import type { AdjustScript } from './scripts/adjust';
import { markedEdited, outsideFilm, type ToolFilm } from './tool-film';

export function resolvePins(film: ToolFilm, input: PinInput, script: AdjustScript): PinOutput {
  const { marks, caseVersion, action } = input;
  const pinned = action.action === 'move';
  if (pinned && outsideFilm(film, action.to)) throw new EngineFailure({ code: 'tool-click-outside-film', message: 'the pin is outside the film', detail: null });
  const target = action.target;

  if (target.on === 'landmark') {
    return { caseVersion, marks: { ...marks, landmarks: movedLandmark(marks, target.landmark, action) }, changed: [], warnings: [] };
  }

  const index = marks.vertebrae.findIndex((vertebra) => vertebra.id === target.vertebra);
  const vertebra = marks.vertebrae[index];
  if (vertebra === undefined) throw new EngineFailure({ code: 'tool-unknown-vertebra', message: 'no vertebra on this film has that id', detail: String(target.vertebra) });
  const slot = slotOf(vertebra, target);
  const changed = [vertebra.id];
  let vertebrae = [...marks.vertebrae];

  if (slot === 'pedL' || slot === 'pedR') {
    if (vertebra.marks.kind !== 'front') throw pinRefused(target);
    const before = vertebra.marks.points[slot];
    const pedicle: PedicleMark =
      action.action === 'move'
        ? { seen: 'visible', at: action.to, source: 'pin', pinned: true, inferred: false, ringScore: null }
        : before.seen === 'visible'
          ? { ...before, pinned: false }
          : before;
    vertebrae[index] = markedEdited({ ...vertebra, marks: { kind: 'front', points: { ...vertebra.marks.points, [slot]: pedicle } } });
    return { caseVersion, marks: { ...marks, vertebrae }, changed, warnings: [] };
  }

  const corners = [...cornersOf(vertebra)] as Corners;
  const before = corners[slot];
  corners[slot] = action.action === 'move' ? { at: action.to, source: 'pin', pinned: true, inferred: false } : { ...before, pinned: false };
  vertebrae[index] = markedEdited(withCorners(vertebra, corners));

  if (action.action === 'move') {
    const upperEnd = slot === 0 || slot === 1;
    const neighbourIndex = upperEnd ? index - 1 : index + 1;
    const neighbour = vertebrae[neighbourIndex];
    // The facing corner across the disc: an upper corner faces the lower corner of the vertebra above on the same side.
    const facing = upperEnd ? (slot === 0 ? 3 : 2) : slot === 3 ? 0 : 1;
    if (neighbour !== undefined) {
      const theirs = [...cornersOf(neighbour)] as Corners;
      const corner = theirs[facing];
      if (!corner.pinned) {
        theirs[facing] = { ...corner, at: add(corner.at, times(sub(action.to, before.at), script.neighbourGive)), source: 'pin' };
        vertebrae[neighbourIndex] = withCorners(neighbour, theirs);
        changed.push(neighbour.id);
      }
    }
  }
  vertebrae = vertebrae.map((each) => ({ ...each, outline: changed.includes(each.id) ? null : each.outline }));
  return { caseVersion, marks: { ...marks, vertebrae, landmarks: { ...marks.landmarks, c7: c7Of(vertebrae) } }, changed, warnings: [] };
}

function slotOf(vertebra: VertebraOnFilm, target: Exclude<PinTarget, { on: 'landmark' }>): 0 | 1 | 2 | 3 | 'pedL' | 'pedR' {
  const marks = vertebra.marks;
  if (target.on === 'endplate') {
    if (marks.kind !== 'bending') throw pinRefused(target);
    const left = target.end === 'patient-left';
    return target.line === 'sup' ? (left ? 0 : 1) : left ? 3 : 2;
  }
  const slots = marks.kind === 'front' ? FRONT_SLOTS : marks.kind === 'side' ? SIDE_SLOTS : null;
  const slot = slots?.[target.mark];
  if (slot === undefined) throw pinRefused(target);
  return slot;
}

const FRONT_SLOTS: Partial<Record<string, 0 | 1 | 2 | 3 | 'pedL' | 'pedR'>> = { supL: 0, supR: 1, infR: 2, infL: 3, pedL: 'pedL', pedR: 'pedR' };
const SIDE_SLOTS: Partial<Record<string, 0 | 1 | 2 | 3>> = { supA: 0, supP: 1, infP: 2, infA: 3 };

function movedLandmark(marks: FilmMarks, name: Extract<PinTarget, { on: 'landmark' }>['landmark'], action: PinInput['action']): FilmMarks['landmarks'] {
  const landmarks = marks.landmarks;
  const moved = (mark: Mark): Mark => (action.action === 'move' ? { at: action.to, source: 'pin', pinned: true, inferred: false } : { ...mark, pinned: false });
  const sacrum = landmarks.sacrumTop;
  if (sacrum?.view === 'front' && (name === 'sacL' || name === 'sacR')) return { ...landmarks, sacrumTop: { ...sacrum, [name]: moved(sacrum[name]) } };
  if (sacrum?.view === 'side' && (name === 'sacA' || name === 'sacP')) return { ...landmarks, sacrumTop: { ...sacrum, [name]: moved(sacrum[name]) } };
  const hips = [...landmarks.hips];
  const at =
    name === 'hipLeft'
      ? hips.findIndex((hip) => hip.side === 'left')
      : name === 'hipRight'
        ? hips.findIndex((hip) => hip.side === 'right')
        : name === 'hipFirst'
          ? 0
          : name === 'hipSecond'
            ? 1
            : -1;
  const hip = hips[at];
  if (hip === undefined) throw pinRefused({ on: 'landmark', landmark: name });
  hips[at] = { ...hip, centre: moved(hip.centre) };
  return { ...landmarks, hips };
}

function pinRefused(target: PinTarget): EngineFailure {
  return new EngineFailure({ code: 'tool-pin-invalid', message: 'that point is not on this film', detail: JSON.stringify(target) });
}

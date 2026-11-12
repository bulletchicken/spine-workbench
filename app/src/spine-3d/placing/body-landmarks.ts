// The points of a vertebral body that can be checked against a film: each
// endplate's centre and the four ends of its rim's oval. For a fitted
// vertebra they come from its thirteen numbers; for a bone, from the body
// bone-prep measured on it.

import type { VertebraPose } from '@/shapes';
import type { BoneBody } from '../bones/read-bones';
import { bodyFrameOf, outOfBody } from './bone-body';
import { type BodyNumbers, type Endplate, endplatesOf } from './endplates';
import { placingOf } from './place-vertebra';
import { type Vec3, along } from './vectors';

/** One endplate's rim: its centre and its ends across and front to back. */
export interface RimLandmarks {
  centre: Vec3;
  left: Vec3;
  right: Vec3;
  front: Vec3;
  back: Vec3;
}

export interface BodyLandmarks {
  top: RimLandmarks;
  bottom: RimLandmarks;
}

export const RIM_POINTS = ['centre', 'left', 'right', 'front', 'back'] as const;

function rimOf(plate: Endplate): RimLandmarks {
  return {
    centre: plate.centre,
    left: along(plate.centre, plate.widthAxis, plate.halfWidth),
    right: along(plate.centre, plate.widthAxis, -plate.halfWidth),
    front: along(plate.centre, plate.depthAxis, -plate.halfDepth),
    back: along(plate.centre, plate.depthAxis, plate.halfDepth),
  };
}

function each(landmarks: BodyLandmarks, move: (point: Vec3) => Vec3): BodyLandmarks {
  const rim = (from: RimLandmarks): RimLandmarks => ({
    centre: move(from.centre),
    left: move(from.left),
    right: move(from.right),
    front: move(from.front),
    back: move(from.back),
  });
  return { top: rim(landmarks.top), bottom: rim(landmarks.bottom) };
}

/** The landmarks of a body in its own frame. */
export function landmarksOf(body: BodyNumbers): BodyLandmarks {
  const plates = endplatesOf(body);
  return { top: rimOf(plates.top), bottom: rimOf(plates.bottom) };
}

/** A bone's landmarks, in the bone's own frame. */
export function boneLandmarks(body: BoneBody): BodyLandmarks {
  const frame = bodyFrameOf(body);
  return each(landmarksOf(frame.numbers), (point) => outOfBody(frame, point));
}

/** A fitted vertebra's landmarks, in the patient frame. */
export function fittedLandmarks(pose: VertebraPose): BodyLandmarks {
  const placing = placingOf(pose);
  return each(landmarksOf(pose), (point) => placing.toPatient(point));
}

/** Every landmark moved by the same rule. */
export const moveLandmarks = each;

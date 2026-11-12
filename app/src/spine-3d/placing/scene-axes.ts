// Patient axes and the scene's axes. The patient frame is x forward, y to
// the patient's left, z up. The scene is drawn with X to the patient's
// left, Y up and Z forward, so that a camera looking back along Z from in
// front sees what a front film shows: the patient's left on the right of
// the picture.
//
// Going from one to the other only renames the axes in turn (x → Z, y → X,
// z → Y). That is a rotation, never a mirror: a left vertebra stays a left
// vertebra.

import type { Mat3, Vec3 } from './vectors';

/** The rotation from patient axes to scene axes. */
export const PATIENT_TO_SCENE: Mat3 = [
  [0, 1, 0],
  [0, 0, 1],
  [1, 0, 0],
];

/** A patient point or direction, in scene axes. */
export const toScene = (patient: Vec3): Vec3 => [patient[1], patient[2], patient[0]];

/** A scene point or direction, in patient axes. */
export const toPatient = (scene: Vec3): Vec3 => [scene[2], scene[0], scene[1]];

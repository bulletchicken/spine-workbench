import { describe, expect, it } from 'vitest';
import { viewAxes } from '../looking/view';
import { PATIENT_TO_SCENE, toPatient, toScene } from './scene-axes';
import { type Vec3, apply, determinant, dot } from './vectors';

const LEFT: Vec3 = [0, 1, 0];
const FORWARD: Vec3 = [1, 0, 0];
const UP: Vec3 = [0, 0, 1];

describe('patient axes in the scene', () => {
  it('are turned, never mirrored', () => {
    expect(determinant(PATIENT_TO_SCENE)).toBe(1);
    for (const axis of [LEFT, FORWARD, UP]) expect(apply(PATIENT_TO_SCENE, axis)).toEqual(toScene(axis));
    expect(toPatient(toScene([3, -5, 7]))).toEqual([3, -5, 7]);
  });

  it('show the patient\'s left on the right of the picture in the front view, as a front film does', () => {
    const { right, up, toViewer } = viewAxes(0, 0);
    expect(dot(toScene(LEFT), right)).toBe(1);
    expect(dot(toScene(UP), up)).toBe(1);
    // The one looking stands in front of the patient.
    expect(dot(toScene(FORWARD), toViewer)).toBe(1);
  });

  it('show the front of the body on the right of the picture in the side view, as a side film named for anterior does', () => {
    const { right, up, toViewer } = viewAxes(-90, 0);
    expect(dot(toScene(FORWARD), right)).toBeCloseTo(1, 15);
    expect(dot(toScene(UP), up)).toBe(1);
    // The one looking stands at the patient's right.
    expect(dot(toScene(LEFT), toViewer)).toBeCloseTo(-1, 15);
  });

  it('show the front of the body toward the bottom of the picture from above, the patient\'s left still on the right', () => {
    const { right, up, toViewer } = viewAxes(0, 90);
    expect(dot(toScene(LEFT), right)).toBe(1);
    expect(dot(toScene(FORWARD), up)).toBeCloseTo(-1, 15);
    expect(dot(toScene(UP), toViewer)).toBe(1);
  });

  it('keep every view right-handed: right, up and toward the viewer', () => {
    for (const [azimuth, elevation] of [[0, 0], [34, 14], [-90, 0], [180, -20], [77, 89]] as const) {
      const { right, up, toViewer } = viewAxes(azimuth, elevation);
      expect(determinant([right, up, toViewer])).toBeCloseTo(1, 14);
    }
  });
});

import { describe, expect, it } from 'vitest';
import { type BodyNumbers, endplatesOf } from './endplates';
import { cross, dot, length, sub } from './vectors';

const straight: BodyNumbers = {
  height: 20,
  wedgeFront: 0,
  wedgeSide: 0,
  topHalfWidth: 15,
  topHalfDepth: 10,
  bottomHalfWidth: 16,
  bottomHalfDepth: 11,
};

describe('the two endplates of a body', () => {
  it('are level and the height apart when the body is not wedged', () => {
    const { top, bottom } = endplatesOf(straight);
    expect(top.centre).toEqual([0, 0, 10]);
    expect(bottom.centre).toEqual([0, 0, -10]);
    for (const plate of [top, bottom]) {
      expect(length(sub(plate.normal, [0, 0, 1]))).toBe(0);
      expect(length(sub(plate.widthAxis, [0, 1, 0]))).toBe(0);
      // ẑ × ŷ = −x̂: front to back.
      expect(length(sub(plate.depthAxis, [-1, 0, 0]))).toBe(0);
    }
    expect([top.halfWidth, top.halfDepth, bottom.halfWidth, bottom.halfDepth]).toEqual([15, 10, 16, 11]);
  });

  it('each take half of each wedge, the front wedge after the side one', () => {
    // Worked by hand with ωf/2 = ωs/2 = 30°: rotY(30°)ẑ = (½, 0, √3/2), and rotX(30°) of that is
    // (½, −√3/4, ¾). The other order would give (√3/4, −½, ¾).
    const { top, bottom } = endplatesOf({ ...straight, wedgeFront: Math.PI / 3, wedgeSide: Math.PI / 3 });
    expect(length(sub(top.normal, [0.5, -Math.sqrt(3) / 4, 0.75]))).toBeLessThan(1e-15);
    expect(length(sub(bottom.normal, [-0.5, Math.sqrt(3) / 4, 0.75]))).toBeLessThan(1e-15);
  });

  it('a positive front wedge makes the left side taller, a positive side wedge the front shorter', () => {
    const front = endplatesOf({ ...straight, wedgeFront: 0.2 });
    // The top plate's left end is above its centre, the bottom plate's left end below its own.
    expect(front.top.widthAxis[2]).toBeGreaterThan(0);
    expect(front.bottom.widthAxis[2]).toBeLessThan(0);
    const side = endplatesOf({ ...straight, wedgeSide: 0.2 });
    // The depth axis points back, so the front end is the centre minus it: lower on top, higher below.
    expect(-side.top.depthAxis[2]).toBeLessThan(0);
    expect(-side.bottom.depthAxis[2]).toBeGreaterThan(0);
  });

  it('have right-handed unit frames', () => {
    const plates = endplatesOf({ ...straight, wedgeFront: 0.4, wedgeSide: -0.3 });
    for (const plate of [plates.top, plates.bottom]) {
      for (const axis of [plate.normal, plate.widthAxis, plate.depthAxis]) expect(length(axis)).toBeCloseTo(1, 14);
      expect(Math.abs(dot(plate.normal, plate.widthAxis))).toBeLessThan(1e-15);
      expect(length(sub(cross(plate.normal, plate.widthAxis), plate.depthAxis))).toBeLessThan(1e-15);
    }
  });
});

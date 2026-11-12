import { describe, expect, it } from 'vitest';
import { fitZoom, travelTo, type CameraFrame } from '@/film-viewer/camera/camera';
import { filmSpace } from '@/film-viewer/film-space';
import { spineFraming, vertebraFraming } from './film-framing';

const camera: CameraFrame = { size: { width: 1000, height: 2000 }, view: { quarterTurns: 0, mirrored: false }, viewport: { width: 400, height: 800 }, margins: { top: 0, right: 0, bottom: 0, left: 0 } };

describe('framing the film', () => {
  it('leaves a box that fills the film at the whole-film fit', () => {
    expect(spineFraming(camera, { left: 0, top: 0, right: 1000, bottom: 2000 }, 24, 1.3).over).toBe(1);
  });
  it('limits a small spine’s push-in to the most requested', () => {
    expect(spineFraming(camera, { left: 400, top: 500, right: 600, bottom: 900 }, 0, 1.3).over).toBe(1.3);
  });
  it('makes room for names by narrowing the fit and moving the look point right', () => {
    const box = { left: 250, right: 750, top: 400, bottom: 1000 };
    const bare = spineFraming(camera, box, 0, 2);
    const names = spineFraming(camera, box, 100, 2);
    expect(names.over).toBeLessThan(bare.over);
    expect(names.point[0] - bare.point[0]).toBe(100);
  });
  it('gives the made-up Front film the system’s 1.3 push-in and 24-pixel name room', () => {
    const front = { ...camera, size: { width: 586, height: 1300 }, viewport: { width: 322, height: 644 } };
    const box = { left: 220, right: 366, top: 200, bottom: 1100 };
    const frame = spineFraming(front, box, 24, 1.3);
    expect(frame.over).toBe(1.3);
    expect(frame.point[0]).toBe(293 + 24);
    expect(frame.point[1]).toBeCloseTo(650 - 0.03 * 644 / (fitZoom(front) * 1.3));
  });
  it('centres a reviewed vertebra vertically and leaves room for its name', () => {
    const framing = vertebraFraming(camera, [500, 1000], 24, 2);
    expect(framing.over).toBe(2);
    expect(framing.point[0]).toBe(524);
    const position = filmSpace(travelTo(camera, framing.point, framing.over)).toScreen([500, 1000]);
    // Film coordinates address pixel centres, so the half-pixel is kept.
    expect(position[1]).toBeCloseTo(camera.viewport.height / 2);
  });
});

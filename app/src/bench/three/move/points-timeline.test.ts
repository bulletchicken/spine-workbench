import { expect, it } from 'vitest';
import { LENGTH, ORDER, sample, scriptFor } from './points-timeline';
import { cropPixels, cropWall, flyWall, screenPoint } from './film-flight';
import { filmCamera, toFilm } from '@/spine-3d/placing/film-camera';
import { toScene } from '@/spine-3d/placing/scene-axes';
import { toCanvas } from '@/spine-3d/looking/lens';

it('finds and fills every bone in the fixed scattered order and leaves only the settled spine', () => {
  const anatomical = ['T1', 'T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'T8', 'T9', 'T10', 'T11', 'T12', 'L1', 'L2', 'L3', 'L4', 'L5'];
  expect(new Set(ORDER).size).toBe(17);
  ORDER.forEach((level, i) => {
    const previous = ORDER[i - 1]; if (previous !== undefined) expect(Math.abs(anatomical.indexOf(level) - anatomical.indexOf(previous))).toBeGreaterThanOrEqual(3);
    expect(scriptFor(level).begins).toBe(660 + i * 66); expect(scriptFor(level).fills).toBe(2520 + i * 24);
    expect(sample(scriptFor(level).begins / LENGTH).bone(level).threads).toBe(0);
    expect(sample((scriptFor(level).begins + 840) / LENGTH).bone(level).lines).toBe(1);
    expect(sample(1).bone(level)).toMatchObject({ fill: 1, revealed: 1, lines: 1, drawing: 0, threads: 0, lift: 0 });
    // On Front alone a bone forms on its outline and has risen to the fit before its faces arrive.
    const { first, spread, rise, begins, fills } = scriptFor(level);
    expect(rise[0]).toBeGreaterThanOrEqual(spread[1]); expect(rise[0]).toBeGreaterThan(first);
    expect(begins + rise[0] + rise[1]).toBeLessThanOrEqual(fills);
  });
  expect(sample(1)).toMatchObject({ ownParts: false, hand: 1, shadow: 1, settle: 1, trips: { front: 1, side: 1, 'bend-left': 1, 'bend-right': 1 } });
  expect(sample(3440 / LENGTH).shadow).toBe(0);
  for (const p of [.8, .2, 1, 0]) expect(sample(p).bone('T9')).toEqual(sample(p).bone('T9'));
});

it('keeps the crop on film pixels and lands travelling wall corners exactly on the rail', () => {
  const placement = { size: { width: 100, height: 200 }, view: { quarterTurns: 0 as const, mirrored: false }, zoom: 2, pan: [-20, -40] as const };
  const frame = { x: 200, y: 50, width: 80, height: 160, placement };
  const camera = filmCamera({ mmPerPx: 1, mmPerPxVertical: null, quarterTurns: 0, imageRight: 'patient-left' }, [100, 200], [50, 100]);
  if (camera === null) throw new Error('no-camera');
  const pixels = cropPixels(frame), corners = cropWall(camera, pixels, -100);
  corners.forEach((corner, i) => { expect(toFilm(camera, corner)).toEqual(pixels[i]); });
  const view = { azimuth: 36, elevation: 12, perspective: .6, pxPerMm: [1.2, 1.2] as const, aim: [0, 0, 0] as const, aimAt: [400, 300] as const }, size = { width: 800, height: 600 };
  const projected = toCanvas(view, size, toScene(screenPoint(view, size, [33, 77], -380)));
  expect(projected.x).toBeCloseTo(33, 10); expect(projected.y).toBeCloseTo(77, 10);
  flyWall(corners, frame, view, size, 1).forEach((corner, i) => { const at = toCanvas(view, size, toScene(corner)); expect(at.x).toBeCloseTo(frame.x + (i === 1 || i === 2 ? frame.width : 0), 10); expect(at.y).toBeCloseTo(frame.y + (i >= 2 ? frame.height : 0), 10); });
});

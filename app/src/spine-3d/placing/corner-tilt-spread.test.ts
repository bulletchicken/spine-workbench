import { expect, it } from 'vitest';
import { cornerTiltSpread } from './corner-tilt-spread';
import { filmCamera } from './film-camera';

it('propagates a corner spread through the two side midpoints in millimetres', () => {
  const camera = filmCamera({ imageRight: 'patient-left', quarterTurns: 0, mmPerPx: 0.5, mmPerPxVertical: 0.25 }, [200, 400], [100, 300]);
  if (camera === null) throw new Error('the film has no camera');
  // The side midpoints are 30 mm across and 40 mm apart vertically: a 50 mm line.
  const corners = { supL: [100, 0], infL: [100, 40], supR: [40, 160], infR: [40, 200] } as const;
  expect(cornerTiltSpread(camera, corners, 1)).toBeCloseTo(1 / 50, 12);
});

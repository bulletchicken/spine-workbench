// The whole claim, on a made-up spine: the engine fits the marks of a front
// and a side film, each bone is shaped to its fitted vertebra and stood in
// the patient, and, seen along a film's beam, the bone's body is on the
// outline the marks drew.
//
// Nothing here is fitted to the bones. The film's camera comes from the
// film's own geometry and the pixel where the engine says the patient
// origin lands; the corners compared with are the marks the fit was given.

import { describe, expect, it } from 'vitest';
import type { SpineFit } from '@/shapes';
import { bonesFromDisk } from '../bones/bones-from-disk';
import { toCanvas } from '../looking/lens';
import { viewAlongFilm } from '../looking/framing';
import { FRONT_ONLY_CASE, MADE_UP_CASE } from '../made-up-case/made-up-case';
import { boneLandmarks, moveLandmarks } from './body-landmarks';
import { type FilmCamera, type Px, toFilm, toUpright } from './film-camera';
import { fittedFilmCamera } from './fitted-film-camera';
import { filmPlane } from './film-plane';
import { moved, pressedOnto } from './flat-on-film';
import { placingOf } from './place-vertebra';
import { rimEnds, rimThrough } from './rim-on-film';
import { toScene } from './scene-axes';
import { archJoinOf, shaperFor } from './shape-bone';
import type { Vec3 } from './vectors';

const bones = bonesFromDisk();

/** The body's four corners on a film, found from the placed bone's own rims: first-end and second-end of the top, then of the bottom. */
function cornersInSpace(fit: SpineFit, level: string, camera: FilmCamera, axis: 'across' | 'front-to-back'): [Vec3, Vec3, Vec3, Vec3] {
  const vertebra = fit.vertebrae.find((each) => each.level === level);
  const bone = bones.find((each) => each.level === level);
  if (!vertebra || !bone) throw new Error(`no ${level}`);
  const shape = shaperFor(bone.body, vertebra.pose, archJoinOf(bone).height);
  const placing = placingOf(vertebra.pose);
  const placed = moveLandmarks(boneLandmarks(bone.body), (point) => placing.toPatient(shape(point)));
  const top = rimEnds(rimThrough(placed.top.centre, placed.top.left, placed.top.back), camera, axis);
  const bottom = rimEnds(rimThrough(placed.bottom.centre, placed.bottom.left, placed.bottom.back), camera, axis);
  return [top[0], top[1], bottom[0], bottom[1]];
}

const apart = (a: Px, b: Px) => Math.hypot(a[0] - b[0], a[1] - b[1]);

describe.each([['two films', MADE_UP_CASE], ['front only', FRONT_ONLY_CASE]] as const)('the bones fitted from %s', (_, fixture) => {
  const { fit, front, side } = fixture;
  const frontCamera = fittedFilmCamera(front.geometry, front.size, fit.frontFilm);
  if (frontCamera === null) throw new Error('the front film has no camera');
  const sideCamera = side === null || fit.sideFilm === null ? null : fittedFilmCamera(side.geometry, side.size, fit.sideFilm);
  it('are the seventeen from T1 to L5, each with a bone to draw', () => {
    expect(fit.vertebrae.map((each) => each.level)).toEqual(front.vertebrae.map((each) => each.level));
    expect(fit.vertebrae.length).toBe(17);
    for (const { level } of fit.vertebrae) expect(bones.some((bone) => bone.level === level), level).toBe(true);
  });

  it('land on the fit\'s own corners on the front film, to a tenth of a pixel', () => {
    let worst = 0;
    for (const { level, fitted } of front.vertebrae) {
      const [supL, supR, infL, infR] = cornersInSpace(fit, level, frontCamera, 'across').map((point) => toFilm(frontCamera, point));
      if (!supL || !supR || !infL || !infR) throw new Error('no corners');
      for (const off of [apart(supL, fitted.supL), apart(supR, fitted.supR), apart(infL, fitted.infL), apart(infR, fitted.infR)]) {
        worst = Math.max(worst, off);
        expect(off, level).toBeLessThan(0.1);
      }
    }
    // Measured: rounding only.
    expect(worst).toBeLessThan(1e-8);
  });

  it('land on the fit\'s own corners on the side film too, when present', () => {
    if (side === null || sideCamera === null) {
      expect(fit.sideFilm).toBeNull();
      return;
    }
    for (const { level, fitted } of side.vertebrae) {
      const [supA, supP, infA, infP] = cornersInSpace(fit, level, sideCamera, 'front-to-back').map((point) => toFilm(sideCamera, point));
      if (!supA || !supP || !infA || !infP) throw new Error('no corners');
      for (const off of [apart(supA, fitted.supA), apart(supP, fitted.supP), apart(infA, fitted.infA), apart(infP, fitted.infP)]) {
        expect(off, level).toBeLessThan(0.1);
      }
    }
  });

  it('sit on the marks the fit was given, to within the fit\'s own residual', () => {
    const [su, sv] = frontCamera.uprightMmPerPx;
    let worstMm = 0;
    for (const { level, marked } of front.vertebrae) {
      const vertebra = fit.vertebrae.find((each) => each.level === level);
      const corners = cornersInSpace(fit, level, frontCamera, 'across').map((point) => toFilm(frontCamera, point));
      const marks = [marked.supL, marked.supR, marked.infL, marked.infR];
      let squared = 0;
      corners.forEach((corner, i) => {
        const mark = marks[i] ?? [NaN, NaN];
        const mm = Math.hypot((corner[0] - mark[0]) * su, (corner[1] - mark[1]) * sv);
        worstMm = Math.max(worstMm, mm);
        squared += mm * mm;
      });
      // The residual includes four corners, plus two pedicles on the two-film fixture.
      // The corners alone can hold no more than all the observed marks' squares.
      const residual = vertebra?.rmsMm.front ?? NaN;
      expect(squared, level).toBeLessThanOrEqual((side === null ? 4 : 6) * residual * residual * (1 + 1e-9));
    }
    // The one-film fit also has to balance the typical profile against the marks.
    expect(worstMm).toBeLessThan(side === null ? 0.4 : 0.035);
    expect(worstMm).toBeGreaterThan(0);
  });

  it('are drawn on those pixels when the view looks along the film with no perspective', () => {
    // The film shown at 0.4 screen pixels a film pixel, its first pixel 100 right and 20 down in the canvas.
    const shown = { pxPerFilmPx: 0.4, firstPixelAt: [100, 20] as const };
    const canvas = { width: 900, height: 1200 };
    for (const [film, axis, name] of [[frontCamera, 'across', 'front'], [sideCamera, 'front-to-back', 'side']] as const) {
      if (film === null) continue;
      const view = viewAlongFilm(film, shown);
      expect(view.perspective).toBe(0);
      for (const { level } of fit.vertebrae) {
        for (const point of cornersInSpace(fit, level, film, axis)) {
          const [u, v] = toUpright(film, point);
          const drawn = toCanvas(view, canvas, toScene(point));
          expect(Math.hypot(drawn.x - (100 + 0.4 * u), drawn.y - (20 + 0.4 * v)), `${level} on the ${name} film`).toBeLessThan(1e-9);
        }
      }
    }
    // And the patient's left is on the right of the picture, as on the front film.
    const view = viewAlongFilm(frontCamera, shown);
    expect(toCanvas(view, canvas, toScene([0, 10, 0])).x).toBeGreaterThan(toCanvas(view, canvas, toScene([0, -10, 0])).x);
  });

  it('stay on their outlines while pressed flat onto the film and all the way out to where the fit stands them', () => {
    const plane = filmPlane(frontCamera, -140);
    for (const amount of [0, 0.3, 1]) {
      const press = pressedOnto(plane, amount);
      for (const { level, fitted } of front.vertebrae) {
        const [supL] = cornersInSpace(fit, level, frontCamera, 'across');
        expect(apart(toFilm(frontCamera, moved(press, supL)), fitted.supL), level).toBeLessThan(1e-8);
      }
    }
  });
});

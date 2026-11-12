// The stage's camera: a view (looking/view.ts) handed to the GPU. It is
// set whole from a view each frame and never moves by itself. The three
// lights ride with it, so the spine is lit the same from every side.

import { Camera, DirectionalLight } from 'three';
import { lensMatrices } from '../looking/lens';
import type { CanvasSize, View } from '../looking/view';
import { FILL_LIGHT, KEY_LIGHT, RIM_LIGHT } from './look';

export class LensCamera extends Camera {
  /** Read by the renderer: with no perspective, light is worked out as for a parallel view. */
  isOrthographicCamera = false;

  constructor() {
    super();
    this.matrixAutoUpdate = false;
    for (const { colour, strength, from } of [KEY_LIGHT, FILL_LIGHT, RIM_LIGHT]) {
      const light = new DirectionalLight(colour, strength);
      light.position.set(from[0], from[1], from[2]);
      // The light and what it points at both ride with the camera.
      this.add(light, light.target);
    }
  }

  /** Look as `view` says. `reach` is how far from the aim, in mm, anything drawn can be. */
  look(view: View, canvas: CanvasSize, reach: number): void {
    const { world, projection, flat } = lensMatrices(view, canvas, reach);
    this.matrix.fromArray(world);
    this.matrixWorldNeedsUpdate = true;
    this.projectionMatrix.fromArray(projection);
    this.projectionMatrixInverse.copy(this.projectionMatrix).invert();
    this.isOrthographicCamera = flat;
  }
}

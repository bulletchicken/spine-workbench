// The one soft round shadow under the spine: dark at its centre and gone
// at its edge. It is its own thing, switched and faded by itself, so it can
// arrive last. It turns with the view, and when the spine is seen level it
// is tipped a little toward the viewer so it never goes edge-on.

import { Color, Group, Mesh, PlaneGeometry, ShaderMaterial } from 'three';
import type { Bounds } from '../looking/framing';
import { SHADOW } from './look';

const VERTEX = /* glsl */ `
varying vec2 vAt;
void main() {
  vAt = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const FRAGMENT = /* glsl */ `
uniform vec3 uColour;
uniform float uShown;
varying vec2 vAt;
void main() {
  float out_ = 2.0 * length(vAt - 0.5);
  float midway = ${SHADOW.midway.at.toFixed(3)};
  float strength = out_ < midway
    ? mix(${SHADOW.centre.toFixed(3)}, ${SHADOW.midway.strength.toFixed(3)}, out_ / midway)
    : mix(${SHADOW.midway.strength.toFixed(3)}, 0.0, clamp((out_ - midway) / (1.0 - midway), 0.0, 1.0));
  gl_FragColor = vec4(uColour, strength * uShown);
}
`;

export interface ContactShadow {
  readonly pivot: Group;
  /** Lay the shadow under a box of the scene, as seen from these two angles, shown this strongly (0 to 1). */
  lay(under: Bounds, azimuth: number, elevation: number, amount: number): void;
  dispose(): void;
}

export function contactShadow(): ContactShadow {
  const geometry = new PlaneGeometry(1, 1);
  const uniforms = {
    // The design gives the colour as it looks on screen; it is written as it is.
    uColour: { value: new Color().setRGB(SHADOW.colour[0] / 255, SHADOW.colour[1] / 255, SHADOW.colour[2] / 255) },
    uShown: { value: 0 },
  };
  const material = new ShaderMaterial({
    vertexShader: VERTEX,
    fragmentShader: FRAGMENT,
    uniforms,
    transparent: true,
    depthWrite: false,
  });
  const disc = new Mesh(geometry, material);
  disc.renderOrder = -5;
  const pivot = new Group();
  pivot.add(disc);
  pivot.visible = false;

  return {
    pivot,
    lay(under, azimuth, elevation, amount) {
      pivot.visible = amount > 0;
      if (amount <= 0) return;
      const width = SHADOW.widthOverFootprint * Math.max(under.high[0] - under.low[0], under.high[2] - under.low[2]);
      pivot.position.set((under.low[0] + under.high[0]) / 2, under.low[1] - SHADOW.below, (under.low[2] + under.high[2]) / 2);
      pivot.rotation.set(0, (azimuth * Math.PI) / 180, 0);
      disc.rotation.set(-Math.PI / 2 + (Math.max(0, SHADOW.tipWhenLevel - elevation) * Math.PI) / 180, 0, 0);
      disc.scale.set(width, width, 1);
      uniforms.uShown.value = SHADOW.shown * Math.min(1, amount);
    },
    dispose() {
      geometry.dispose();
      material.dispose();
    },
  };
}

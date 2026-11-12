// What a bone is drawn with: the matte bone material, with three things
// added that the stock material cannot do.
//
// - A fill. A bone can be filled from one end to the other, with a soft
//   edge; past the edge nothing is drawn. The whole spine can be filled the
//   same way, by height in the patient. Which end fills first is the
//   caller's to say: it hands over the height of that end and the signed
//   distance to the other.
// - A strength. Fading a solid bone by ordinary blending shows its far side
//   through its near side. So a faded bone is drawn twice: first to the
//   depth buffer only, then in colour where it is nearest. It fades as one
//   solid, and what is behind it (a film, the page) shows through evenly.
// - A push back by a hair, so the lines of the see-through version lie on
//   the bone and not half inside it.

import { Color, MeshBasicMaterial, MeshStandardMaterial, Vector2, Vector4, type WebGLProgramParametersWithUniforms } from 'three';
import { ACCENT, BONE, BONE_METALNESS, BONE_ROUGHNESS, FILL_EDGE, TINT_SHARE } from './look';

export interface BoneUniforms {
  /** How strongly the bone is drawn, 0 to 1. */
  strength: { value: number };
  /** How much of the bone is filled, 0 to 1, from the end that fills first. */
  fill: { value: number };
  /** Along the bone's own up axis: the height of the end that fills first, and the signed distance from there to the other end, mm. */
  ownHeights: { value: Vector2 };
  /** The patient's up axis in the bone's own frame, and the bone's own origin's height in the patient. */
  patientUp: { value: Vector4 };
  /** Along the patient's up axis: the height of the spine's end that fills first, the signed distance from there to its other end, how much of the spine is filled, and the soft edge's share of its height. */
  spineFill: { value: Vector4 };
}

const DECLARATIONS = /* glsl */ `
uniform float uStrength;
uniform float uFill;
uniform vec2 uOwnHeights;
uniform vec4 uSpineFill;
varying float vOwnHeight;
varying float vPatientHeight;
`;

const SHOWN = /* glsl */ `
float ownShare = (vOwnHeight - uOwnHeights.x) / uOwnHeights.y;
float ownEdge = uFill * (1.0 + ${FILL_EDGE.toFixed(4)});
float spineShare = (vPatientHeight - uSpineFill.x) / uSpineFill.y;
float spineEdge = uSpineFill.z * (1.0 + uSpineFill.w);
float shown = uStrength
  * (1.0 - smoothstep(ownEdge - ${FILL_EDGE.toFixed(4)}, ownEdge, ownShare))
  * (1.0 - smoothstep(spineEdge - uSpineFill.w, spineEdge, spineShare));
if (shown < 0.004) discard;
diffuseColor.a *= shown;
`;

function withFill(uniforms: BoneUniforms) {
  return (shader: WebGLProgramParametersWithUniforms) => {
    shader.uniforms.uStrength = uniforms.strength;
    shader.uniforms.uFill = uniforms.fill;
    shader.uniforms.uOwnHeights = uniforms.ownHeights;
    shader.uniforms.uPatientUp = uniforms.patientUp;
    shader.uniforms.uSpineFill = uniforms.spineFill;
    shader.vertexShader = shader.vertexShader
      .replace('#include <common>', `#include <common>\nuniform vec4 uPatientUp;\nvarying float vOwnHeight;\nvarying float vPatientHeight;`)
      .replace('#include <begin_vertex>', `#include <begin_vertex>\nvOwnHeight = position.z;\nvPatientHeight = dot(uPatientUp.xyz, position) + uPatientUp.w;`);
    shader.fragmentShader = shader.fragmentShader
      .replace('#include <common>', `#include <common>\n${DECLARATIONS}`)
      .replace('#include <color_fragment>', `#include <color_fragment>\n${SHOWN}`);
  };
}

export interface BoneMaterials {
  /** The bone in colour. */
  solid: MeshStandardMaterial;
  /** The same shape drawn to the depth buffer only, for a bone that is faded or part filled. */
  depthOnly: MeshBasicMaterial;
  uniforms: BoneUniforms;
  /** Tint the bone this share of the way to its highlight: 0 is plain bone, 1 the whole highlight. */
  tint(amount: number): void;
  dispose(): void;
}

const PLAIN = new Color(BONE);
const HIGHLIGHT = new Color(BONE).lerp(new Color(ACCENT), TINT_SHARE);

export function boneMaterials(): BoneMaterials {
  const uniforms: BoneUniforms = {
    strength: { value: 1 },
    fill: { value: 1 },
    ownHeights: { value: new Vector2(0, 1) },
    patientUp: { value: new Vector4(0, 0, 1, 0) },
    spineFill: { value: new Vector4(0, 1, 1, FILL_EDGE) },
  };
  const solid = new MeshStandardMaterial({
    color: PLAIN.clone(),
    roughness: BONE_ROUGHNESS,
    metalness: BONE_METALNESS,
    vertexColors: true,
    // Always blended, so its strength can change from frame to frame without building a new program.
    transparent: true,
    polygonOffset: true,
    polygonOffsetFactor: 1,
    polygonOffsetUnits: 1,
  });
  solid.onBeforeCompile = withFill(uniforms);
  solid.customProgramCacheKey = () => 'spine-3d-bone';

  // Drawn among the blended things, in its turn: after the films, which must already be behind a
  // faded bone for it to fade over them, and before any bone's colour.
  const depthOnly = new MeshBasicMaterial({ colorWrite: false, transparent: true, polygonOffset: true, polygonOffsetFactor: 1, polygonOffsetUnits: 1 });
  depthOnly.onBeforeCompile = withFill(uniforms);
  depthOnly.customProgramCacheKey = () => 'spine-3d-bone-depth';

  return {
    solid,
    depthOnly,
    uniforms,
    tint(amount) {
      solid.color.copy(PLAIN).lerp(HIGHLIGHT, Math.min(1, Math.max(0, amount)));
    },
    dispose() {
      solid.dispose();
      depthOnly.dispose();
    },
  };
}

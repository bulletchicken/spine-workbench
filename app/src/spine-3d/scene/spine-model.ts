// A fitted spine as shapes ready to draw: each fitted vertebra's bone,
// shaped to it, with its smooth normals, the shade of its hollows and its
// see-through line version. Made once for a fit; any number of drawn spines
// share it.

import { BufferAttribute, BufferGeometry, Color } from 'three';
import type { FittedVertebra, Level, SpineFit } from '@/shapes';
import type { Bone } from '../bones/read-bones';
import type { Bounds } from '../looking/framing';
import { type BodyLandmarks, landmarksOf } from '../placing/body-landmarks';
import { type Placing, placingOf } from '../placing/place-vertebra';
import { toScene } from '../placing/scene-axes';
import { shapeBone } from '../placing/shape-bone';
import { cageLines } from './cage';
import { formationMesh, type FormationMesh } from './formation-mesh';
import { hollowShade } from './hollow-shade';
import { HOLLOW_KEEPS, HOLLOW_TINT } from './look';

export interface ModelBone {
  level: Level;
  /** What the fit says of this vertebra: its pose, its angles, what was inferred and how sure each number is. */
  vertebra: FittedVertebra;
  /** Where the vertebra stands in the patient. */
  placing: Placing;
  /** The shaped bone in the vertebra's own frame, with normals and the shade of its hollows. */
  geometry: BufferGeometry;
  /** The lines of its see-through version. */
  cage: BufferGeometry;
  formation: FormationMesh;
  /** The lowest and highest of the shaped bone along its own up axis, mm. */
  ownHeights: readonly [number, number];
  /** The fitted body's landmarks in the vertebra's own frame. */
  landmarks: BodyLandmarks;
}

export interface SpineModel {
  bones: readonly ModelBone[];
  /** Fitted levels that have no bone shape to draw. */
  missing: readonly Level[];
  /** A box round the whole spine as fitted, in the scene's axes, mm. */
  bounds: Bounds;
  /** The lowest and highest point of the spine as fitted, along the patient's up axis, mm. */
  heights: readonly [number, number];
  /** Frees the shapes. Call it once no drawn spine uses the model. */
  dispose(): void;
}

/** The shade of a bone's hollows, kept so two models of the same bones work it out once. */
const shades = new WeakMap<Bone, Float32Array>();

function shadeOf(bone: Bone): Float32Array {
  let shade = shades.get(bone);
  if (shade === undefined) {
    shade = hollowShade(bone.points, bone.triangles);
    shades.set(bone, shade);
  }
  return shade;
}

/** The colour each point's light is multiplied by: white where it is open, a warm grey in a hollow. */
function shadeColours(open: Float32Array): Float32Array {
  const deepest = new Color(HOLLOW_TINT);
  const scale = HOLLOW_KEEPS / Math.max(deepest.r, deepest.g, deepest.b);
  const colours = new Float32Array(open.length * 3);
  open.forEach((share, i) => {
    colours[i * 3] = deepest.r * scale + (1 - deepest.r * scale) * share;
    colours[i * 3 + 1] = deepest.g * scale + (1 - deepest.g * scale) * share;
    colours[i * 3 + 2] = deepest.b * scale + (1 - deepest.b * scale) * share;
  });
  return colours;
}

/** The fitted spine as shapes. A fitted level with no bone among `bones` is left out and named in `missing`. */
export function modelSpine(fit: SpineFit, bones: readonly Bone[]): SpineModel {
  const made: ModelBone[] = [];
  const missing: Level[] = [];
  const low: [number, number, number] = [Infinity, Infinity, Infinity];
  const high: [number, number, number] = [-Infinity, -Infinity, -Infinity];
  const heights: [number, number] = [Infinity, -Infinity];

  for (const vertebra of fit.vertebrae) {
    const bone = bones.find((each) => each.level === vertebra.level);
    if (bone === undefined) {
      missing.push(vertebra.level);
      continue;
    }
    const shaped = shapeBone(bone, vertebra.pose);
    const placing = placingOf(vertebra.pose);
    const ownHeights: [number, number] = [Infinity, -Infinity];
    for (let i = 0; i + 2 < shaped.length; i += 3) {
      const own = [shaped[i] ?? 0, shaped[i + 1] ?? 0, shaped[i + 2] ?? 0] as const;
      ownHeights[0] = Math.min(ownHeights[0], own[2]);
      ownHeights[1] = Math.max(ownHeights[1], own[2]);
      const patient = placing.toPatient(own);
      heights[0] = Math.min(heights[0], patient[2]);
      heights[1] = Math.max(heights[1], patient[2]);
      const scene = toScene(patient);
      for (const axis of [0, 1, 2] as const) {
        low[axis] = Math.min(low[axis], scene[axis]);
        high[axis] = Math.max(high[axis], scene[axis]);
      }
    }

    const geometry = new BufferGeometry();
    geometry.setAttribute('position', new BufferAttribute(shaped, 3));
    geometry.setAttribute('color', new BufferAttribute(shadeColours(shadeOf(bone)), 3));
    // The file's own triangles, copied: the bytes they are a view on are not this geometry's to keep alive.
    geometry.setIndex(new BufferAttribute(new Uint16Array(bone.triangles), 1));
    // Normals are worked out after shaping, so a stretched bone is still lit as its new shape.
    geometry.computeVertexNormals();
    geometry.computeBoundingSphere();

    const cage = new BufferGeometry();
    cage.setAttribute('position', new BufferAttribute(cageLines(shaped, bone.triangles), 3));

    const formation = formationMesh(shaped, bone.triangles, vertebra.pose.height);
    geometry.setAttribute('aCell', new BufferAttribute(formation.cell, 1));
    geometry.setAttribute('aTurn', new BufferAttribute(formation.turn, 1));

    made.push({
      level: vertebra.level,
      vertebra,
      placing,
      geometry,
      cage,
      formation,
      ownHeights,
      landmarks: landmarksOf(vertebra.pose),
    });
  }

  return {
    bones: made,
    missing,
    bounds: { low, high },
    heights,
    dispose() {
      for (const bone of made) {
        bone.geometry.dispose();
        bone.cage.dispose();
      }
    },
  };
}

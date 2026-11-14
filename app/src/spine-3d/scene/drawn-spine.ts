// One spine on the stage: a bone for every fitted vertebra, each with its
// own values, and the spine's own. Nothing here moves by itself. Every
// value is set from outside and drawn on the next frame, so a transition
// is whatever its caller makes of one progress number, and can be stopped,
// scrubbed or jumped to its end.
//
// Each bone has
//   pose     0 pressed flat onto the film it was found on … 1 where the fit stands it
//   reveal   0 not there … 1 there (it fades in, rising a little)
//   fill     0 empty … 1 filled, with a soft edge, from the end the spine fills from
//   strength 0 clear … 1 solid
//   tint     0 plain bone … 1 highlighted
//   cage     0 none … 1 its see-through line version
// and the spine has a depth (0 flat against the film … 1 its true depth),
// a fill of its own by height, the end both fills start from (the top, T1
// first, unless told otherwise), a place and a turn of its own, and a shadow.

import { type ColorRepresentation, type Texture, Color, Group, LineBasicMaterial, LineSegments, Matrix4, Mesh } from 'three';
import type { FittedVertebra, Level, VertebraPose } from '@/shapes';
import type { Bounds } from '../looking/framing';
import { viewAxes } from '../looking/view';
import { type BodyLandmarks, moveLandmarks } from '../placing/body-landmarks';
import type { FilmPlane } from '../placing/film-plane';
import { moved, pressedOnto } from '../placing/flat-on-film';
import { toScene } from '../placing/scene-axes';
import { type Vec3, add, apply, scale as scaled, sub } from '../placing/vectors';
import { type BoneMaterials, boneMaterials } from './bone-material';
import { BONE, CAGE, FILL_EDGE, REVEAL_RISE } from './look';
import { formationDrawing, type FormationDrawing } from './formation-drawing';
import type { ModelBone, SpineModel } from './spine-model';
import { placingOf, type Placing } from '../placing/place-vertebra';
import { poseBetween, sameShape } from '../placing/pose-between';
import { sceneLayer, type SceneLayer, type SceneLayerInside } from './scene-layer';

/** A film's sheet, as far as pressing a bone onto it needs: which way it faces and where it stands. */
export type FilmSheet = Pick<FilmPlane, 'towardReader' | 'out'>;

/** What can be set on a bone. Each runs from 0 to 1. */
export interface BoneValues {
  pose: number;
  reveal: number;
  fill: number;
  strength: number;
  tint: number;
  cage: number;
  /** -1 uses the ordinary height fill; 0 to 1 fills from the net inward. */
  edgeFill: number;
}

/** What can be set on a whole spine. */
export interface SpineValues {
  strength: number;
  flatColour: ColorRepresentation;
  /** 0 flat against the film … 1 its true depth. Multiplies every bone's pose. */
  depth: number;
  /** How much of the spine is filled, by height in the patient, 0 to 1. */
  fill: number;
  /** The soft edge of that fill, as a share of the spine's height. */
  fillEdge: number;
  /** The end that fills first, for the spine's own fill and for each bone's: the top, as the vertebrae are found from T1 down, or the bottom. */
  fillFrom: 'top' | 'bottom';
  /** The see-through lines' colour: 0 the ink line that reads on the page … 1 the white line that reads over a film. */
  cageOnFilm: number;
  /** How far a bone rises as it is revealed, mm. Set 0 for fades only. */
  revealRise: number;
  /** The spine moved as one object, in the scene's millimetres (X the patient's left, Y up, Z forward). */
  offset: Vec3;
  /** The spine turned as one object about its middle, as if seen from this much further round and above, degrees. */
  turned: readonly [number, number];
  /** The spine's size as one object, about its middle. */
  scale: number;
  /** The round shadow under the spine, 0 to 1. */
  shadow: number;
  visible: boolean;
}

/** The turn each kind of thing is drawn in. Films are -10 and shadows -5. */
const DRAWN = { depth: 0, colour: 1, lines: 2 } as const;

const BONE_DEFAULTS: BoneValues = { pose: 1, reveal: 1, fill: 1, strength: 1, tint: 0, cage: 0, edgeFill: -1 };

const SPINE_DEFAULTS: SpineValues = {
  strength: 1,
  flatColour: BONE,
  depth: 1,
  fill: 1,
  fillEdge: FILL_EDGE / 4,
  fillFrom: 'top',
  cageOnFilm: 0,
  revealRise: REVEAL_RISE,
  offset: [0, 0, 0],
  turned: [0, 0],
  scale: 1,
  shadow: 0,
  visible: true,
};

export interface DrawnSpine {
  addLayer(): SceneLayer;
  betweenPoses(from: readonly VertebraPose[], to: readonly VertebraPose[], p: number): void;
  /** The externally timed points, edges and threads. */
  readonly formation: FormationDrawing;
  /** The levels drawn, top to bottom. */
  readonly levels: readonly Level[];
  /** Set values on one bone, some bones, or (with no levels) every bone. */
  set(values: Partial<BoneValues>, levels?: Level | readonly Level[]): void;
  /** Set values of the whole spine. */
  setSpine(values: Partial<SpineValues>): void;
  /**
   * The film whose sheet "flat" means: bones at pose 0 lie on it. Give the
   * film itself and the bones follow its sheet wherever it is stood; or
   * give a sheet. `null` leaves bones where the fit stands them.
   */
  pressOnto(film: FilmSheet | { plane(): FilmSheet } | null): void;
  /** Every value at rest: each bone solid, whole, plain and in its fitted place. */
  settle(): void;
  /** A bone's values as they stand. */
  valuesOf(level: Level): BoneValues | null;
  spineValues(): SpineValues;
  /** What the fit says of a vertebra: its pose and angles, which of them were inferred, and how sure each is. */
  about(level: Level): FittedVertebra | null;
  /** A vertebra's endplate centres and rim ends as drawn now, in the scene's axes, mm. */
  landmarks(level: Level): BodyLandmarks | null;
  /** A vertebra's centre as drawn now, in the scene's axes, mm. */
  centre(level: Level): Vec3 | null;
  /** A patient point of the fitted spine, where this spine's own place and turn put it in the scene. */
  inScene(patient: Vec3): Vec3;
  /** A box round the spine as drawn at its own place, turn and size, in the scene's axes. */
  bounds(): Bounds;
  /** Takes the spine off the stage and frees what it alone holds. */
  remove(): void;
}

interface DrawnBone {
  of: ModelBone;
  placing: Placing;
  values: BoneValues;
  holder: Group;
  solid: Mesh;
  depthOnly: Mesh;
  lines: LineSegments;
  materials: BoneMaterials;
  lineMaterial: LineBasicMaterial;
}

/** What a drawn spine needs of the stage it stands on. */
export interface SpineHost {
  /** The group whose axes are the scene's. */
  scene: Group;
  /** Something changed: draw a frame. */
  changed(): void;
  /** The spine has gone. */
  removed(spine: DrawnSpine): void;
}

/** The inside of a drawn spine, for the stage: what it must look at each frame. */
export interface DrawnSpineInside {
  spine: DrawnSpine;
  root: Group;
  model: SpineModel;
  flat: boolean;
  layers: readonly SceneLayerInside[];
  pass(which: 'base' | 'opaque-depth' | 'parts' | 'rest', texture?: Texture | null, width?: number, height?: number): void;
  /** The solid meshes that can be pointed at, each tagged with its level. */
  pickable(): Mesh[];
  /** True when the bone of `level` is filled in at this point of its own frame. */
  filledAt(level: Level, own: Vec3): boolean;
  /** The film the spine is pressed onto may have been stood somewhere new: press onto where it is now. */
  followFilm(): void;
}

export function drawSpine(model: SpineModel, host: SpineHost, flat = false): DrawnSpineInside {
  const releaseModel = model.retain();
  const layers: SceneLayerInside[] = [];
  let removed = false, travelled = false;
  const root = new Group();
  root.matrixAutoUpdate = false;
  host.scene.add(root);

  let liveHeights = model.heights;
  let spineValues: SpineValues = { ...SPINE_DEFAULTS };
  /** Where the sheet that "flat" means is now, and the sheet the bones were last pressed onto. */
  let sheetNow: () => FilmSheet | null = () => null;
  let plane: FilmSheet | null = null;
  const middle: Vec3 = [
    (model.bounds.low[0] + model.bounds.high[0]) / 2,
    (model.bounds.low[1] + model.bounds.high[1]) / 2,
    (model.bounds.low[2] + model.bounds.high[2]) / 2,
  ];
  const cageOnPage = new Color(CAGE.onPage);
  const cageOnFilm = new Color(CAGE.onFilm);

  const bones: DrawnBone[] = model.bones.map((of) => {
    const materials = boneMaterials(flat);
    // The patient's up axis in the bone's own frame is the bottom row of its rotation.
    const up = of.placing.rotation[2];
    materials.uniforms.patientUp.value.set(up[0], up[1], up[2], of.placing.centre[2]);
    const lineMaterial = new LineBasicMaterial({ color: cageOnPage, transparent: true, opacity: 0, depthWrite: false });
    const holder = new Group();
    holder.matrixAutoUpdate = false;
    const depthOnly = new Mesh(of.geometry, materials.depthOnly);
    const solid = new Mesh(of.geometry, materials.solid);
    const lines = new LineSegments(of.cage, lineMaterial);
    solid.userData.level = of.level;
    // Shared shapes are never moved themselves, so their own matrices stay as they are.
    for (const part of [depthOnly, solid, lines]) part.matrixAutoUpdate = false;
    // The order things are drawn in: films, shadows, every bone's depth, every bone's colour, then lines.
    depthOnly.renderOrder = DRAWN.depth;
    solid.renderOrder = DRAWN.colour;
    lines.renderOrder = DRAWN.lines;
    lines.visible = false;
    depthOnly.visible = false;
    holder.add(depthOnly, solid, lines);
    root.add(holder);
    return { of, placing: of.placing, values: { ...BONE_DEFAULTS }, holder, solid, depthOnly, lines, materials, lineMaterial };
  });
  const byLevel = new Map(bones.map((bone) => [bone.of.level, bone]));

  const placeRoot = () => {
    // The scene's axes, then the spine's own size and turn about its middle, then its own move.
    const { right, up, toViewer } = viewAxes(spineValues.turned[0], spineValues.turned[1]);
    const turn = [right, up, toViewer] as const;
    const size = spineValues.scale;
    const at = add(sub(middle, scaled(apply(turn, middle), size)), spineValues.offset);
    // Rows of (turn × size) applied to the patient axes renamed as the scene's: x → Z, y → X, z → Y.
    root.matrix.set(
      turn[0][2] * size, turn[0][0] * size, turn[0][1] * size, at[0],
      turn[1][2] * size, turn[1][0] * size, turn[1][1] * size, at[1],
      turn[2][2] * size, turn[2][0] * size, turn[2][1] * size, at[2],
      0, 0, 0, 1,
    );
    root.matrixWorldNeedsUpdate = true;
    root.visible = spineValues.visible && spineValues.strength > 0;
  };

  /** How much of its distance from the film's sheet a bone keeps. */
  const keptBy = (bone: DrawnBone) => (plane === null ? 1 : spineValues.depth * bone.values.pose);

  /** Where the bone's own frame is put in the patient: stood by the fit, lowered while it is revealed, pressed toward the film. */
  const placingOfBone = (bone: DrawnBone): { linear: readonly [Vec3, Vec3, Vec3]; offset: Vec3 } => {
    const { rotation, centre } = bone.placing;
    const rising: Vec3 = [centre[0], centre[1], centre[2] - (1 - bone.values.reveal) * spineValues.revealRise];
    const kept = keptBy(bone);
    if (plane === null || kept >= 1) return { linear: rotation, offset: rising };
    const press = pressedOnto(plane, kept);
    const columns = [0, 1, 2].map((column) => apply(press.linear, [rotation[0][column] ?? 0, rotation[1][column] ?? 0, rotation[2][column] ?? 0]));
    const [x, y, z] = [columns[0] ?? [0, 0, 0], columns[1] ?? [0, 0, 0], columns[2] ?? [0, 0, 0]];
    return {
      linear: [
        [x[0], y[0], z[0]],
        [x[1], y[1], z[1]],
        [x[2], y[2], z[2]],
      ],
      offset: moved(press, rising),
    };
  };

  const placeBone = (bone: DrawnBone) => {
    const { linear, offset } = placingOfBone(bone);
    bone.holder.matrix.set(
      linear[0][0], linear[0][1], linear[0][2], offset[0],
      linear[1][0], linear[1][1], linear[1][2], offset[1],
      linear[2][0], linear[2][1], linear[2][2], offset[2],
      0, 0, 0, 1,
    );
    bone.holder.matrixWorldNeedsUpdate = true;
  };

  const paintBone = (bone: DrawnBone) => {
    const { values, materials } = bone;
    const strength = values.strength * values.reveal * spineValues.strength;
    const up = bone.placing.rotation[2];
    materials.uniforms.patientUp.value.set(up[0], up[1], up[2], bone.placing.centre[2]);
    materials.uniforms.strength.value = strength;
    materials.uniforms.fill.value = values.fill;
    materials.uniforms.edgeFill.value = values.edgeFill;
    // A share along a fill runs from 0 at the end that fills first to 1 at the other.
    const fromTop = spineValues.fillFrom === 'top';
    const [ownLow, ownHigh] = bone.of.ownHeights;
    const [low, high] = liveHeights;
    materials.uniforms.ownHeights.value.set(fromTop ? ownHigh : ownLow, fromTop ? ownLow - ownHigh : ownHigh - ownLow);
    materials.uniforms.spineFill.value.set(fromTop ? high : low, fromTop ? low - high : high - low, spineValues.fill, spineValues.fillEdge);
    materials.tint(values.tint);
    materials.flatColour(spineValues.flatColour);
    const whole = values.fill >= 1 && spineValues.fill >= 1;
    bone.solid.visible = strength > 0 && values.fill > 0 && spineValues.fill > 0;
    // Drawn to the depth buffer first whenever anything could show through it.
    bone.depthOnly.visible = bone.solid.visible && (strength < 1 || !whole);
    bone.lineMaterial.color.copy(cageOnPage).lerp(cageOnFilm, spineValues.cageOnFilm);
    bone.lineMaterial.opacity = flat ? 0 : CAGE.strength * values.cage * spineValues.strength;
    bone.lines.visible = !flat && bone.lineMaterial.opacity > 0;
  };

  const clamp = (value: number) => Math.min(1, Math.max(0, value));

  const formation = formationDrawing(model, root, () => { host.changed(); });
  const formationGroup = root.children[bones.length];
  let formationVisible = false, passing = false;
  const spine: DrawnSpine = {
    formation,
    addLayer() {
      if (removed) throw new RangeError('The spine has been removed');
      const made = sceneLayer(root, new Map(bones.map((bone) => [bone.of.level, bone.holder])), () => { host.changed(); }, () => {
        const at = layers.indexOf(made); if (at >= 0) layers.splice(at, 1);
      });
      layers.push(made); host.changed(); return made.layer;
    },
    betweenPoses(from, to, p) {
      const complete = (poses: readonly VertebraPose[]) => poses.length === bones.length && new Set(poses.map((pose) => pose.level)).size === bones.length && bones.every((bone) => {
        const pose = poses.find((pose) => pose.level === bone.of.level);
        return pose !== undefined && sameShape(pose, bone.of.vertebra.pose) && [...pose.centre, ...pose.rotation].every(Number.isFinite);
      });
      if (!Number.isFinite(p) || !complete(from) || !complete(to)) throw new RangeError('A complete compatible pose set is required');
      const next = bones.map((bone) => {
        const a = from.find((pose) => pose.level === bone.of.level), b = to.find((pose) => pose.level === bone.of.level);
        if (a === undefined || b === undefined) throw new RangeError('Missing pose');
        return placingOf(poseBetween(a, b, p));
      });
      travelled = true;
      const heights: [number, number] = [Infinity, -Infinity];
      bones.forEach((bone, i) => {
        bone.placing = next[i] ?? bone.placing;
        const points = bone.of.geometry.getAttribute('position');
        for (let j = 0; j < points.count; j++) {
          const z = bone.placing.toPatient([points.getX(j), points.getY(j), points.getZ(j)])[2];
          heights[0] = Math.min(heights[0], z); heights[1] = Math.max(heights[1], z);
        }
      });
      liveHeights = heights;
      bones.forEach((bone, i) => { bone.placing = next[i] ?? bone.placing; placeBone(bone); paintBone(bone); });
      host.changed();
    },
    levels: bones.map((bone) => bone.of.level),
    set(values, levels) {
      const chosen = levels === undefined ? bones : (typeof levels === 'string' ? [levels] : levels).flatMap((level) => byLevel.get(level) ?? []);
      for (const bone of chosen) {
        const moved = (values.pose !== undefined && values.pose !== bone.values.pose) || (values.reveal !== undefined && values.reveal !== bone.values.reveal);
        for (const [name, value] of Object.entries(values) as [keyof BoneValues, number | undefined][]) {
          if (value !== undefined) bone.values[name] = name === 'edgeFill' && value < 0 ? -1 : clamp(value);
        }
        if (moved) placeBone(bone);
        paintBone(bone);
      }
      host.changed();
    },
    setSpine(values) {
      spineValues = { ...spineValues, ...values, strength: clamp(values.strength ?? spineValues.strength), depth: clamp(values.depth ?? spineValues.depth), fill: clamp(values.fill ?? spineValues.fill), cageOnFilm: clamp(values.cageOnFilm ?? spineValues.cageOnFilm) };
      placeRoot();
      for (const bone of bones) {
        placeBone(bone);
        paintBone(bone);
      }
      for (const layer of layers) layer.prepare(spineValues.strength, null, 1, 1);
      host.changed();
    },
    pressOnto(film) {
      sheetNow = film === null ? () => null : 'plane' in film ? () => film.plane() : () => film;
      plane = sheetNow();
      for (const bone of bones) placeBone(bone);
      host.changed();
    },
    settle() {
      travelled = false; liveHeights = model.heights;
      for (const bone of bones) { bone.values = { ...BONE_DEFAULTS }; bone.placing = bone.of.placing; }
      spine.setSpine({ depth: 1, fill: 1 });
    },
    valuesOf: (level) => {
      const bone = byLevel.get(level);
      return bone ? { ...bone.values } : null;
    },
    spineValues: () => ({ ...spineValues }),
    about: (level) => byLevel.get(level)?.of.vertebra ?? null,
    landmarks(level) {
      const bone = byLevel.get(level);
      if (!bone) return null;
      const { linear, offset } = placingOfBone(bone);
      return moveLandmarks(bone.of.landmarks, (own) => spine.inScene(add(offset, apply(linear, own))));
    },
    centre(level) {
      const bone = byLevel.get(level);
      return bone ? spine.inScene(placingOfBone(bone).offset) : null;
    },
    inScene(patient) {
      const { right, up, toViewer } = viewAxes(spineValues.turned[0], spineValues.turned[1]);
      const from = sub(toScene(patient), middle);
      const turned = apply([right, up, toViewer], from);
      return add(add(middle, scaled(turned, spineValues.scale)), spineValues.offset);
    },
    bounds() {
      if (travelled) {
        const low: [number, number, number] = [Infinity, Infinity, Infinity], high: [number, number, number] = [-Infinity, -Infinity, -Infinity];
        for (const bone of bones) {
          const { linear, offset } = placingOfBone(bone), points = bone.of.geometry.getAttribute('position');
          for (let i = 0; i < points.count; i++) {
            const point = spine.inScene(add(offset, apply(linear, [points.getX(i), points.getY(i), points.getZ(i)])));
            for (const axis of [0, 1, 2] as const) { low[axis] = Math.min(low[axis], point[axis]); high[axis] = Math.max(high[axis], point[axis]); }
          }
        }
        return { low, high };
      }
      const { low, high } = model.bounds;
      const reach: [number, number, number] = [Infinity, Infinity, Infinity];
      const top: [number, number, number] = [-Infinity, -Infinity, -Infinity];
      const { right, up, toViewer } = viewAxes(spineValues.turned[0], spineValues.turned[1]);
      for (const x of [low[0], high[0]]) {
        for (const y of [low[1], high[1]]) {
          for (const z of [low[2], high[2]]) {
            const turned = apply([right, up, toViewer], sub([x, y, z], middle));
            const at = add(add(middle, scaled(turned, spineValues.scale)), spineValues.offset);
            for (const axis of [0, 1, 2] as const) {
              reach[axis] = Math.min(reach[axis], at[axis]);
              top[axis] = Math.max(top[axis], at[axis]);
            }
          }
        }
      }
      return { low: reach, high: top };
    },
    remove() {
      if (removed) return; removed = true;
      for (const layer of [...layers]) layer.layer.remove();
      formation.dispose();
      host.scene.remove(root);
      for (const bone of bones) {
        bone.materials.dispose();
        bone.lineMaterial.dispose();
      }
      releaseModel();
      host.removed(spine);
    },
  };

  placeRoot();
  for (const bone of bones) {
    placeBone(bone);
    paintBone(bone);
  }

  return {
    spine,
    root,
    model,
    flat,
    layers,
    pass(which, texture = null, width = 1, height = 1) {
      if (formationGroup !== undefined) {
        if (which === 'opaque-depth') { formationVisible = formationGroup.visible; passing = true; formationGroup.visible = false; }
        else if (which === 'parts') formationGroup.visible = false;
        else if (passing) { formationGroup.visible = formationVisible; if (which === 'rest') passing = false; }
      }
      for (const bone of bones) {
        paintBone(bone);
        if (which === 'opaque-depth') {
          bone.depthOnly.visible = !flat && bone.solid.visible && bone.materials.uniforms.strength.value >= 1;
          bone.solid.visible = false; bone.lines.visible = false;
        } else if (which === 'parts') {
          bone.depthOnly.visible = false; bone.solid.visible = false; bone.lines.visible = false;
        }
      }
      for (const layer of layers) { layer.prepare(spineValues.strength, texture, width, height); layer.show(which === 'parts' || which === 'rest'); }
    },
    pickable: () => (!flat && !removed && spineValues.visible ? bones.filter((bone) => bone.solid.visible && bone.materials.uniforms.strength.value > 0).map((bone) => bone.solid) : []),
    followFilm() {
      const now = sheetNow();
      const same = now === plane || (now !== null && plane !== null && now.out === plane.out && now.towardReader.every((part, i) => part === plane?.towardReader[i]));
      if (same) return;
      plane = now;
      for (const bone of bones) placeBone(bone);
    },
    filledAt(level, own) {
      const bone = byLevel.get(level);
      if (!bone) return false;
      const upOwn = (own[2] - bone.of.ownHeights[0]) / (bone.of.ownHeights[1] - bone.of.ownHeights[0]);
      const upSpine = (bone.placing.toPatient(own)[2] - liveHeights[0]) / (liveHeights[1] - liveHeights[0]);
      const fromTop = spineValues.fillFrom === 'top';
      const ownShare = fromTop ? 1 - upOwn : upOwn;
      const spineShare = fromTop ? 1 - upSpine : upSpine;
      return ownShare <= bone.values.fill * (1 + FILL_EDGE) - FILL_EDGE / 2 && spineShare <= spineValues.fill * (1 + spineValues.fillEdge) - spineValues.fillEdge / 2;
    },
  };
}

/** The matrix that renames patient axes as the scene's. A group with it holds anything placed in patient millimetres. */
export function patientAxes(): Matrix4 {
  return new Matrix4().set(0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1);
}

// The 3D stage: a transparent canvas with the spine floating on it, lit
// softly from the upper front left, with nothing else. No grid, no axes,
// no floor, no box.
//
// The stage draws only when something has changed, on the app's one
// animation loop. It never animates anything itself: the view, every bone
// and every film are set from outside, each frame if need be.

import { Group, HemisphereLight, Raycaster, SRGBColorSpace, Scene, Vector3, WebGLRenderer } from 'three';
import type { Level } from '@/shapes';
import { joinLoop } from '@/choreography/one-loop';
import { type Bounds, type FilmOnPage, centreOf, namedView, viewAlongFilm } from '../looking/framing';
import { type OnCanvas, toCanvas } from '../looking/lens';
import type { ViewName } from '../looking/named-views';
import { rayThrough } from '../looking/ray';
import type { CanvasSize, View } from '../looking/view';
import type { FilmCamera } from '../placing/film-camera';
import { toPatient, toScene } from '../placing/scene-axes';
import { type Vec3, dot, length, sub } from '../placing/vectors';
import { type ContactShadow, contactShadow } from './contact-shadow';
import { type DrawnSpine, type DrawnSpineInside, drawSpine, patientAxes } from './drawn-spine';
import { type FilmPixels, type FilmWall, type FilmWallInside, hangFilm } from './film-wall';
import { LensCamera } from './lens-camera';
import { AMBIENT_STRENGTH, GROUND, SKY } from './look';
import type { SpineModel } from './spine-model';

/** A view, or a rule that gives the view for the canvas's size (asked again whenever the canvas changes size). */
export type ViewOrRule = View | ((canvas: CanvasSize) => View);

/** The two films a spine is fitted from. */
export type FilmName = 'front' | 'side';

/** How far behind the spine a film stands when nobody says, mm. */
const BEHIND_THE_SPINE = 24;

/** A screen is never drawn at more than this many device pixels to a CSS pixel. */
const MOST_DEVICE_PIXELS = 2;

export interface Stage {
  /** Stand a spine on the stage. Any number may share one model. */
  addSpine(model: SpineModel): DrawnSpine;
  /** Stand a film in the space of the bones, from its own camera, with its picture if there is one yet. */
  hangFilm(name: FilmName, camera: FilmCamera, pixels?: FilmPixels | null): FilmWall;
  /** The film standing under this name, if any. */
  film(name: FilmName): FilmWall | null;

  /** Look as this view says, from now until another is set. */
  setView(view: ViewOrRule): void;
  /** The view as it stands. */
  view(): View;
  /** How many times a view has been set. Whoever is moving the view frame by frame stops when someone else sets one. */
  viewsSet(): number;
  /** One of the named views, fitted to the spines on the stage, for the canvas as it is. */
  namedView(name: ViewName): View;
  /** The view along a film's beam with no perspective, drawing the spine at the film's own scale and height. */
  viewAlong(name: FilmName, shown: FilmOnPage): View | null;
  /** The canvas's size in CSS pixels. */
  size(): CanvasSize;
  /** A box round every spine on the stage, in the scene's axes, mm. */
  bounds(): Bounds;

  /** The spine and level drawn at a place on the canvas, in CSS pixels from its top-left corner. */
  pick(x: number, y: number): { spine: DrawnSpine; level: Level } | null;
  /** Where a point of the scene is on the canvas, in CSS pixels. */
  toCanvas(scenePoint: Vec3): OnCanvas;
  /** Where a patient point is on the canvas: of the fitted spine as `spine` is placed, or of the patient frame itself. */
  patientToCanvas(patient: Vec3, spine?: DrawnSpine): OnCanvas;

  /** Draw a frame soon. Every setter asks for this itself. */
  changed(): void;
  /** Draw now, without waiting for the next frame. */
  drawNow(): void;
  /** Called after each frame is drawn: the moment to move anything laid over the canvas. */
  onDrawn(listener: () => void): () => void;
  /** Stops drawing and frees everything the stage holds on the GPU. Models are the caller's to free. */
  dispose(): void;
}

export function makeStage(canvas: HTMLCanvasElement): Stage {
  const renderer = new WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'high-performance' });
  renderer.setClearColor(0x000000, 0);
  renderer.outputColorSpace = SRGBColorSpace;

  const scene = new Scene();
  const camera = new LensCamera();
  scene.add(camera);
  scene.add(new HemisphereLight(SKY, GROUND, AMBIENT_STRENGTH));

  /** Everything placed in the scene's own axes: spines and their shadows. */
  const inScene = new Group();
  /** Everything placed in patient millimetres: the films. */
  const inPatient = new Group();
  inPatient.matrixAutoUpdate = false;
  inPatient.matrix.copy(patientAxes());
  scene.add(inScene, inPatient);

  const spines: { inside: DrawnSpineInside; shadow: ContactShadow }[] = [];
  const walls = new Map<FilmName, FilmWallInside>();
  const listeners = new Set<() => void>();
  let size: CanvasSize = { width: Math.max(1, canvas.clientWidth), height: Math.max(1, canvas.clientHeight) };
  let rule: ViewOrRule | null = null;
  let viewsSet = 0;
  let leave: (() => void) | null = null;
  let disposed = false;

  const bounds = (): Bounds => {
    const low: [number, number, number] = [Infinity, Infinity, Infinity];
    const high: [number, number, number] = [-Infinity, -Infinity, -Infinity];
    for (const { inside } of spines) {
      const each = inside.spine.bounds();
      for (const axis of [0, 1, 2] as const) {
        low[axis] = Math.min(low[axis], each.low[axis]);
        high[axis] = Math.max(high[axis], each.high[axis]);
      }
    }
    // An empty stage still has somewhere to look.
    return spines.length === 0 ? { low: [-100, 0, -100], high: [100, 400, 100] } : { low, high };
  };

  const viewNow = (): View => {
    if (rule === null) return namedView(bounds(), size, 'three-quarter');
    return typeof rule === 'function' ? rule(size) : rule;
  };

  /** Where each film stands: behind the spine as its reader sees it, and meeting the other film in a corner when both are up. */
  const standWalls = () => {
    const front = walls.get('front');
    const side = walls.get('side');
    const box = bounds();
    const corners: Vec3[] = [];
    for (const x of [box.low[0], box.high[0]]) for (const y of [box.low[1], box.high[1]]) for (const z of [box.low[2], box.high[2]]) corners.push(toPatient([x, y, z]));
    const behind = (wall: FilmWallInside) => Math.min(...corners.map((corner) => dot(corner, wall.wall.plane().towardReader))) - BEHIND_THE_SPINE;

    let frontOut = front ? (front.out() ?? behind(front)) : 0;
    let sideOut = side ? (side.out() ?? behind(side)) : 0;
    if (front && side) {
      const toFrontReader = front.wall.plane().towardReader;
      const toSideReader = side.wall.plane().towardReader;
      // Each film ends where the other stands, if that is still behind the spine: the two meet in a corner.
      if (front.out() === null) frontOut = Math.min(frontOut, Math.min(...side.wall.plane().corners.map((corner) => dot(corner, toFrontReader))));
      if (side.out() === null) sideOut = Math.min(sideOut, Math.min(...front.wall.plane().corners.map((corner) => dot(corner, toSideReader))));
      // The side film is hinged on the upright edge it shares with the front film, and swings flat beside it.
      const on: Vec3 = [toFrontReader[0] * frontOut + toSideReader[0] * sideOut, toFrontReader[1] * frontOut + toSideReader[1] * sideOut, 0];
      front.stand(frontOut, { on, flatToward: null });
      side.stand(sideOut, { on, flatToward: [-toSideReader[0], -toSideReader[1], 0] });
      return;
    }
    front?.stand(frontOut, null);
    side?.stand(sideOut, null);
  };

  /** How far from the aim anything drawn can be, mm. */
  const reachFrom = (aim: Vec3): number => {
    const box = bounds();
    let reach = length(sub(centreOf(box), aim)) + length(sub(box.high, box.low));
    for (const wall of walls.values()) {
      for (const corner of wall.corners()) reach = Math.max(reach, length(sub(toScene(corner), aim)));
    }
    return reach * 1.05 + 50;
  };

  const draw = () => {
    if (disposed) return;
    const view = viewNow();
    standWalls();
    for (const { inside, shadow } of spines) {
      inside.followFilm();
      const values = inside.spine.spineValues();
      shadow.lay(inside.spine.bounds(), view.azimuth, view.elevation, values.visible ? values.shadow : 0);
    }
    camera.look(view, size, reachFrom(view.aim));
    renderer.render(scene, camera);
    for (const listener of listeners) listener();
  };

  const changed = () => {
    if (disposed || leave !== null) return;
    leave = joinLoop(() => {
      leave?.();
      leave = null;
      draw();
    });
  };

  const fit = () => {
    const next = { width: Math.max(1, canvas.clientWidth), height: Math.max(1, canvas.clientHeight) };
    const ratio = Math.min(window.devicePixelRatio || 1, MOST_DEVICE_PIXELS);
    if (next.width === size.width && next.height === size.height && ratio === renderer.getPixelRatio()) return;
    size = next;
    renderer.setPixelRatio(ratio);
    // The canvas's own size on the page is the stylesheet's to say.
    renderer.setSize(size.width, size.height, false);
    // Drawn at once, so a canvas that is being resized never shows a stretched frame.
    draw();
  };
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, MOST_DEVICE_PIXELS));
  renderer.setSize(size.width, size.height, false);
  const watch = new ResizeObserver(fit);
  watch.observe(canvas);

  const raycaster = new Raycaster();

  const stage: Stage = {
    addSpine(model) {
      const shadow = contactShadow();
      inScene.add(shadow.pivot);
      const inside = drawSpine(model, {
        scene: inScene,
        changed,
        removed: (spine) => {
          const at = spines.findIndex((each) => each.inside.spine === spine);
          const [gone] = at < 0 ? [] : spines.splice(at, 1);
          if (gone) {
            inScene.remove(gone.shadow.pivot);
            gone.shadow.dispose();
          }
          changed();
        },
      });
      spines.push({ inside, shadow });
      standWalls();
      changed();
      return inside.spine;
    },
    hangFilm(name, filmCamera, pixels = null) {
      walls.get(name)?.wall.remove();
      const inside = hangFilm(filmCamera, changed, (wall) => {
        if (walls.get(name)?.wall === wall) walls.delete(name);
        changed();
      });
      walls.set(name, inside);
      inPatient.add(inside.mesh);
      if (pixels !== null) inside.wall.show(pixels);
      standWalls();
      changed();
      return inside.wall;
    },
    film: (name) => walls.get(name)?.wall ?? null,

    setView(view) {
      rule = view;
      viewsSet += 1;
      changed();
    },
    view: viewNow,
    viewsSet: () => viewsSet,
    namedView: (name) => namedView(bounds(), size, name),
    viewAlong(name, shown) {
      const wall = walls.get(name);
      return wall ? viewAlongFilm(wall.wall.camera, shown) : null;
    },
    size: () => size,
    bounds,

    pick(x, y) {
      const view = viewNow();
      const ray = rayThrough(view, size, x, y, reachFrom(view.aim));
      raycaster.ray.origin.set(ray.from[0], ray.from[1], ray.from[2]);
      raycaster.ray.direction.set(ray.toward[0], ray.toward[1], ray.toward[2]);
      scene.updateMatrixWorld();
      const meshes = spines.flatMap(({ inside }) => inside.pickable());
      const own = new Vector3();
      for (const hit of raycaster.intersectObjects(meshes, false)) {
        const found = spines.find(({ inside }) => hit.object.parent?.parent === inside.root);
        const level = hit.object.userData.level as Level | undefined;
        if (!found || level === undefined) continue;
        // A part of a bone that is not filled in yet is not there to be pointed at.
        own.copy(hit.point);
        hit.object.worldToLocal(own);
        if (found.inside.filledAt(level, [own.x, own.y, own.z])) return { spine: found.inside.spine, level };
      }
      return null;
    },
    toCanvas: (scenePoint) => toCanvas(viewNow(), size, scenePoint),
    patientToCanvas: (patient, spine) => toCanvas(viewNow(), size, spine ? spine.inScene(patient) : toScene(patient)),

    changed,
    drawNow: draw,
    onDrawn(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      leave?.();
      leave = null;
      watch.disconnect();
      for (const { inside } of [...spines]) inside.spine.remove();
      for (const wall of [...walls.values()]) wall.wall.remove();
      listeners.clear();
      renderer.dispose();
    },
  };
  return stage;
}

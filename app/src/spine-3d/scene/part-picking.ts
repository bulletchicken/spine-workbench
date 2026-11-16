import { BackSide, Box3, DoubleSide, Line, LineLoop, LineSegments, Mesh, Vector3, type BufferAttribute, type BufferGeometry, type InterleavedBufferAttribute, type Material, type Object3D } from 'three';
import type { OnCanvas } from '../looking/lens';
import type { Vec3 } from '../placing/vectors';
import type { PartRegistration } from './scene-layer';

export const PART_RADIUS_PX = 12;
const disposed = new WeakSet<BufferGeometry | Material>();
/** Disposal listeners live only as long as their layer. */
export function partDisposals() {
  const watched = new Map<BufferGeometry | Material, () => void>();
  return {
    watch(object: Object3D) {
      object.traverse((child) => {
        if (!(child instanceof Mesh || child instanceof Line)) return;
        const material = child.material as Material | Material[];
        const resources: (BufferGeometry | Material)[] = [child.geometry as BufferGeometry, ...Array.isArray(material) ? material : [material]];
        for (const resource of resources) {
          if (watched.has(resource) || disposed.has(resource)) continue;
          const onDispose = () => { disposed.add(resource); resource.removeEventListener('dispose',onDispose); watched.delete(resource); };
          watched.set(resource, onDispose); resource.addEventListener('dispose', onDispose);
        }
      });
    },
    remove() { for (const [resource, listener] of watched) resource.removeEventListener('dispose', listener); watched.clear(); },
  };
}
export function visibleObject(object: Object3D): boolean {
  for (let at: Object3D | null = object; at !== null; at = at.parent) if (!at.visible) return false;
  return true;
}
interface Candidate { registration: PartRegistration; distance: number; nearer: number }

/** Each shape's own box, kept until its points are next changed. */
const boxes = new WeakMap<BufferAttribute | InterleavedBufferAttribute, { version: number; count: number; box: Box3 }>();
const boxOf = (position: BufferAttribute | InterleavedBufferAttribute): Box3 => {
  const version = 'version' in position ? position.version : position.data.version, known = boxes.get(position);
  if (known?.version === version && known.count === position.count) return known.box;
  const box = new Box3().setFromBufferAttribute(position as BufferAttribute);
  boxes.set(position, { version, count: position.count, box });
  return box;
};
/** A share of a CSS pixel by which a pointer may be outside a part's box and still be tried, for rounding. */
const BOX_SLACK_PX = 1e-6;
const corner = new Vector3(), scratch = new Vector3(), at: [number, number, number] = [0, 0, 0];
/** Each point of the object being tried, as projected: kept between picks and grown as needed, so no point needs a new object. */
let xs = new Float64Array(0), ys = new Float64Array(0), nears = new Float64Array(0), seen = new Uint32Array(0), stamp = 0;

/** The closest rendered triangle or line, never the object's centre or its box. */
export function pickParts(registrations: readonly PartRegistration[], project: (point: Vec3) => OnCanvas, x: number, y: number, eye: number | null = null): Candidate | null {
  const registered = new Set(registrations.map(({ object }) => object));
  let best: Candidate | null = null;
  const consider = (registration: PartRegistration, distance: number, nearer: number) => {
    if (!Number.isFinite(distance) || distance > PART_RADIUS_PX) return;
    if (best === null || distance < best.distance - 1e-7 || (Math.abs(distance - best.distance) <= 1e-7 && (nearer > best.nearer + 1e-7 || (Math.abs(nearer - best.nearer) <= 1e-7 && registration.order < best.registration.order)))) best = { registration, distance, nearer };
  };
  /** The depth at a weighted mix of two or three projected points: in step with the lens when there is one. Sums in the order of the points. */
  const depthAt = (a: number, wa: number, b: number, wb: number, c = -1, wc = 0) => {
    const na = nears[a] ?? NaN, nb = nears[b] ?? NaN, nc = c < 0 ? 0 : nears[c] ?? NaN;
    if (eye === null) return c < 0 ? 0 + na * wa + nb * wb : 0 + na * wa + nb * wb + nc * wc;
    return eye - 1 / (c < 0 ? 0 + wa / (eye - na) + wb / (eye - nb) : 0 + wa / (eye - na) + wb / (eye - nb) + wc / (eye - nc));
  };
  const segment = (registration: PartRegistration, a: number, b: number) => {
    const ax = xs[a] ?? NaN, ay = ys[a] ?? NaN, dx = (xs[b] ?? NaN) - ax, dy = (ys[b] ?? NaN) - ay, square = dx * dx + dy * dy;
    const t = square === 0 ? 0 : Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / square));
    consider(registration, Math.hypot(x - ax - t * dx, y - ay - t * dy), depthAt(a, 1 - t, b, t));
  };
  /** Where a point of an object's own frame lands on the canvas, through the reused `at`. */
  const projected = (point: Vector3, matrix: Object3D['matrixWorld']) => {
    point.applyMatrix4(matrix); at[0] = point.x; at[1] = point.y; at[2] = point.z;
    return project(at);
  };
  /** True when the pointer is surely further than the radius from everything the object draws: outside its box on the canvas. */
  const outside = (position: BufferAttribute | InterleavedBufferAttribute, object: Object3D) => {
    const box = boxOf(position);
    if (box.isEmpty()) return false;
    let left = Infinity, right = -Infinity, top = Infinity, bottom = -Infinity;
    for (let i = 0; i < 8; i++) {
      const p = projected(corner.set(i & 1 ? box.max.x : box.min.x, i & 2 ? box.max.y : box.min.y, i & 4 ? box.max.z : box.min.z), object.matrixWorld);
      // A corner at or behind the eye is not drawn where the lens puts it: try the object point by point.
      if (eye !== null && !(p.nearer < eye)) return false;
      left = Math.min(left, p.x); right = Math.max(right, p.x); top = Math.min(top, p.y); bottom = Math.max(bottom, p.y);
    }
    const reach = PART_RADIUS_PX + BOX_SLACK_PX;
    return x < left - reach || x > right + reach || y < top - reach || y > bottom + reach;
  };
  for (const registration of registrations) {
    const walk = (object: Object3D) => {
      if (!visibleObject(object) || (object !== registration.object && registered.has(object))) return;
      if (object instanceof Mesh || object instanceof Line) {
        const geometry = object.geometry as BufferGeometry;
        const paints = object.material as Material | Material[];
        const position = geometry.getAttribute('position'), index = geometry.index;
        if (!disposed.has(geometry) && geometry.hasAttribute('position') && !outside(position, object)) {
          if (seen.length < position.count) { const room = Math.max(position.count, 2 * seen.length); xs = new Float64Array(room); ys = new Float64Array(room); nears = new Float64Array(room); seen = new Uint32Array(room); stamp = 0; }
          if (++stamp > 0xffffffff) { seen.fill(0); stamp = 1; }
          /** The point the i-th index names, projected once however many triangles share it. */
          const vertex = (i: number) => {
            const k = index?.getX(i) ?? i;
            if (seen[k] !== stamp) {
              const p = projected(scratch.fromBufferAttribute(position, k), object.matrixWorld);
              seen[k] = stamp; xs[k] = p.x; ys[k] = p.y; nears[k] = p.nearer;
            }
            return k;
          };
          const start = geometry.drawRange.start, end = Math.min(index?.count ?? position.count, start + geometry.drawRange.count);
          const groups = Array.isArray(paints) ? geometry.groups : [{ start, count: end - start, materialIndex: 0 }];
          for (const group of groups) {
            const material = Array.isArray(paints) ? paints[group.materialIndex ?? 0] : paints;
            if (material === undefined || !material.visible || material.opacity <= 0 || disposed.has(material)) continue;
            const first = Math.max(start, group.start), last = Math.min(end, group.start + group.count);
            if (object instanceof Mesh) {
              for (let i = first; i + 2 < last; i += 3) {
                const a = vertex(i), b = vertex(i + 1), c = vertex(i + 2);
                const ax = xs[a] ?? NaN, ay = ys[a] ?? NaN, bx = xs[b] ?? NaN, by = ys[b] ?? NaN, cx = xs[c] ?? NaN, cy = ys[c] ?? NaN;
                const area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
                if (Math.abs(area) < 1e-10) continue;
                if (material.side !== DoubleSide && (material.side === BackSide ? area < 0 : area > 0)) continue;
                const u = ((bx - x) * (cy - y) - (by - y) * (cx - x)) / area;
                const v = ((cx - x) * (ay - y) - (cy - y) * (ax - x)) / area;
                const w = 1 - u - v;
                if (u >= 0 && v >= 0 && w >= 0) consider(registration, 0, depthAt(a, u, b, v, c, w));
                else { segment(registration, a, b); segment(registration, b, c); segment(registration, c, a); }
              }
            } else {
              const step = object instanceof LineSegments ? 2 : 1;
              for (let i = first; i + 1 < last; i += step) segment(registration, vertex(i), vertex(i + 1));
              if (object instanceof LineLoop && last > first) segment(registration, vertex(last - 1), vertex(first));
            }
          }
        }
      }
      for (const child of object.children) walk(child);
    };
    walk(registration.object);
  }
  return best;
}

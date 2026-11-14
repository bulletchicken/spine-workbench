import { BackSide, DoubleSide, Line, LineLoop, LineSegments, Mesh, Vector3, type BufferGeometry, type Material, type Object3D } from 'three';
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

/** The closest rendered triangle or line, never the object's centre or its box. */
export function pickParts(registrations: readonly PartRegistration[], project: (point: Vec3) => OnCanvas, x: number, y: number, eye: number | null = null): Candidate | null {
  const registered = new Set(registrations.map(({ object }) => object));
  let best: Candidate | null = null;
  const consider = (registration: PartRegistration, distance: number, nearer: number) => {
    if (!Number.isFinite(distance) || distance > PART_RADIUS_PX) return;
    if (best === null || distance < best.distance - 1e-7 || (Math.abs(distance - best.distance) <= 1e-7 && (nearer > best.nearer + 1e-7 || (Math.abs(nearer - best.nearer) <= 1e-7 && registration.order < best.registration.order)))) best = { registration, distance, nearer };
  };
  const depthAt = (points: readonly OnCanvas[], weights: readonly number[]) => eye === null
    ? points.reduce((sum, point, i) => sum + point.nearer * (weights[i] ?? 0), 0)
    : eye - 1 / points.reduce((sum, point, i) => sum + (weights[i] ?? 0) / (eye - point.nearer), 0);
  const segment = (a: OnCanvas, b: OnCanvas) => {
    const dx = b.x - a.x, dy = b.y - a.y, square = dx * dx + dy * dy;
    const t = square === 0 ? 0 : Math.max(0, Math.min(1, ((x - a.x) * dx + (y - a.y) * dy) / square));
    return { distance: Math.hypot(x - a.x - t * dx, y - a.y - t * dy), nearer: depthAt([a, b], [1 - t, t]) };
  };
  for (const registration of registrations) {
    const walk = (object: Object3D) => {
      if (!visibleObject(object) || (object !== registration.object && registered.has(object))) return;
      if (object instanceof Mesh || object instanceof Line) {
        const geometry = object.geometry as BufferGeometry;
        const paints = object.material as Material | Material[];
        const position = geometry.getAttribute('position'), index = geometry.index;
        if (!disposed.has(geometry) && geometry.hasAttribute('position')) {
          const vertex = (i: number) => {
            const v = new Vector3().fromBufferAttribute(position, index?.getX(i) ?? i).applyMatrix4(object.matrixWorld);
            return project([v.x, v.y, v.z]);
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
                const area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
                if (Math.abs(area) < 1e-10) continue;
                if (material.side !== DoubleSide && (material.side === BackSide ? area < 0 : area > 0)) continue;
                const u = ((b.x - x) * (c.y - y) - (b.y - y) * (c.x - x)) / area;
                const v = ((c.x - x) * (a.y - y) - (c.y - y) * (a.x - x)) / area;
                const w = 1 - u - v;
                if (u >= 0 && v >= 0 && w >= 0) consider(registration, 0, depthAt([a, b, c], [u, v, w]));
                else for (const [from, to] of [[a, b], [b, c], [c, a]] as const) { const hit = segment(from, to); consider(registration, hit.distance, hit.nearer); }
              }
            } else {
              const step = object instanceof LineSegments ? 2 : 1;
              for (let i = first; i + 1 < last; i += step) { const hit = segment(vertex(i), vertex(i + 1)); consider(registration, hit.distance, hit.nearer); }
              if (object instanceof LineLoop && last > first) { const hit = segment(vertex(last - 1), vertex(first)); consider(registration, hit.distance, hit.nearer); }
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

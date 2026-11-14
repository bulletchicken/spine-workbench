import { Group, Line, Mesh, type Object3D, type ColorRepresentation, type MeshBasicMaterial, type MeshStandardMaterial, type Texture } from 'three';
import type { Level } from '@/shapes';
import { layerMaterials, type LayerMaterial } from './layer-materials';
import { partDisposals } from './part-picking';

export interface SceneLayer {
  readonly onSpine: Group;
  onBone(level: Level): Group;
  pointable(object: Object3D, id: string): void;
  inFront(object: Object3D, on: boolean): void;
  lit(colour: ColorRepresentation): MeshStandardMaterial;
  flat(colour: ColorRepresentation, strength: number): MeshBasicMaterial;
  changed(): void;
  remove(): void;
}

export interface PartRegistration { object: Object3D; id: string; order: number; layer: SceneLayer }
let registrationOrder = 0;

export function sceneLayer(root: Group, holders: ReadonlyMap<Level, Group>, changed: () => void, removed: () => void) {
  const onSpine = new Group(), spineGate = new Group(); spineGate.add(onSpine); root.add(spineGate);
  const bones = new Map<Level, Group>(), gates = new Map<Level, Group>(), registrations = new Map<Object3D, PartRegistration>(), front = new Set<Object3D>();
  const materials = layerMaterials(), disposal = partDisposals();
  let gone = false;
  const contains = (object: Object3D) => {
    let own = false;
    for (let at: Object3D | null = object; at !== null; at = at.parent) {
      if (at === onSpine || [...bones.values()].includes(at as Group)) own = true;
      if (at === root) return own;
    }
    return false;
  };
  const layer: SceneLayer = {
    onSpine,
    onBone(level) {
      const holder = holders.get(level);
      if (holder === undefined || gone) throw new RangeError(`Missing bone: ${level}`);
      let group = bones.get(level);
      if (group === undefined) { group = new Group(); const gate = new Group(); gate.add(group); bones.set(level, group); gates.set(level, gate); holder.add(gate); }
      return group;
    },
    pointable(object, id) {
      if (gone || !contains(object)) throw new RangeError('Object is outside the layer');
      disposal.watch(object);
      const order = registrations.get(object)?.order ?? registrationOrder++;
      registrations.set(object, { object, id, order, layer }); changed();
    },
    inFront(object, on) {
      if (gone || !contains(object)) throw new RangeError('Object is outside the layer');
      if (on) front.add(object); else front.delete(object);
      layer.changed();
    },
    lit: (colour) => { if (gone) throw new RangeError('The layer has been removed'); return materials.lit(colour); },
    flat: (colour, strength) => { if (gone) throw new RangeError('The layer has been removed'); return materials.flat(colour, strength); },
    changed() { if (!gone) { for (const group of [onSpine, ...bones.values()]) disposal.watch(group); changed(); } },
    remove() {
      if (gone) return; gone = true;
      spineGate.removeFromParent(); for (const group of gates.values()) group.removeFromParent();
      registrations.clear(); front.clear(); materials.dispose(); disposal.remove(); removed(); changed();
    },
  };
  return {
    layer,
    registrations: () => [...registrations.values()].filter(({ object }) => contains(object)),
    show(on: boolean) { spineGate.visible = on; for (const group of gates.values()) group.visible = on; },
    prepare(strength: number, texture: Texture | null, width: number, height: number) {
      materials.strength(strength); materials.mask(texture, width, height);
      for (const group of [onSpine, ...bones.values()]) group.traverse((object) => {
        if (!(object instanceof Mesh || object instanceof Line)) return;
        let inFront = false;
        for (let at: Object3D | null = object; at !== null && at !== root; at = at.parent) if (front.has(at)) inFront = true;
        const paint = (material: LayerMaterial) => materials.forObject(material, inFront);
        object.material = Array.isArray(object.material) ? (object.material as LayerMaterial[]).map(paint) : paint(object.material as LayerMaterial);
      });
      for (const group of [onSpine, ...bones.values()]) disposal.watch(group);
    },
  };
}
export type SceneLayerInside = ReturnType<typeof sceneLayer>;

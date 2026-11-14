import { MeshBasicMaterial, MeshStandardMaterial, Vector2, type ColorRepresentation, type Texture } from 'three';
import { BONE_ROUGHNESS } from './look';

export type LayerMaterial = MeshBasicMaterial | MeshStandardMaterial;

/** A separate bone depth image leaves the hardware's own depth buffer free. */
export function layerMaterials() {
  const owned = new Map<LayerMaterial, { strength: number; applied: number; front: LayerMaterial | null }>();
  let previousWhole = 1;
  const depth = { value: null as Texture | null }, veil = { value: null as Texture | null }, hasVeil = { value: false }, size = { value: new Vector2(1, 1) }, enabled = { value: false };
  const prepare = <T extends LayerMaterial>(material: T, inFront: boolean): T => {
    material.transparent = true;
    material.depthTest = true;
    material.depthWrite = true;
    material.onBeforeCompile = (shader) => {
      shader.uniforms.uBoneDepth = depth; shader.uniforms.uBoneVeil = veil; shader.uniforms.uHasBoneVeil = hasVeil; shader.uniforms.uDepthSize = size; shader.uniforms.uBoneMask = enabled;
      shader.fragmentShader = shader.fragmentShader.replace('#include <common>', '#include <common>\nuniform sampler2D uBoneDepth;\nuniform sampler2D uBoneVeil;\nuniform bool uHasBoneVeil;\nuniform vec2 uDepthSize;\nuniform bool uBoneMask;')
        .replace('#include <clipping_planes_fragment>', `#include <clipping_planes_fragment>\n${inFront ? '' : 'if (uBoneMask && gl_FragCoord.z > texture2D(uBoneDepth, gl_FragCoord.xy / uDepthSize).x + 0.000002) { float bone = uHasBoneVeil ? texture2D(uBoneVeil, gl_FragCoord.xy / uDepthSize).r : 1.0; if (bone >= 1.0) discard; diffuseColor.a *= 1.0 - bone; }'}`);
    };
    material.customProgramCacheKey = () => `spine-layer-${String(inFront)}`;
    return material;
  };
  return {
    lit(colour: ColorRepresentation) {
      const material = prepare(new MeshStandardMaterial({ color: colour, roughness: BONE_ROUGHNESS, metalness: 0 }), false);
      owned.set(material, { strength: 1, applied: 1, front: null }); return material;
    },
    flat(colour: ColorRepresentation, strength: number) {
      const local = Math.max(0, Math.min(1, strength));
      const material = prepare(new MeshBasicMaterial({ color: colour, opacity: local }), false);
      owned.set(material, { strength: local, applied: local, front: null }); return material;
    },
    forObject(material: LayerMaterial, inFront: boolean): LayerMaterial {
      const entry = owned.get(material) ?? [...owned.values()].find((each) => each.front === material);
      if (entry === undefined) return material;
      const base = [...owned].find(([, value]) => value === entry)?.[0];
      if (base === undefined || !inFront) return base ?? material;
      entry.front ??= prepare(base.clone(), true);
      entry.front.color.copy(base.color); entry.front.opacity = base.opacity; entry.front.visible = base.visible;
      return entry.front;
    },
    strength(whole: number) {
      for (const [material, entry] of owned) {
        if (material.opacity !== entry.applied) entry.strength = Math.max(0, Math.min(1, previousWhole > 0 ? material.opacity / previousWhole : material.opacity));
        material.opacity = entry.strength * whole;
        entry.applied = material.opacity;
        if (entry.front !== null) entry.front.opacity = material.opacity;
      }
      previousWhole = whole;
    },
    mask(texture: Texture | null, width: number, height: number, strength: Texture | null) { depth.value = texture; veil.value = strength; hasVeil.value = strength !== null; size.value.set(width, height); enabled.value = texture !== null; },
    dispose() { for (const [material, entry] of owned) { material.dispose(); entry.front?.dispose(); } owned.clear(); },
  };
}

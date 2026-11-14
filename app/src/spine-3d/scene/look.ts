// How the spine looks: every colour and strength of the 3D stage in one
// place. These are the design's values for bone, light and the shadow under
// the spine. The page's stylesheet cannot reach a GPU, so the stage keeps
// its own copy; keep it equal to the design's.

/** Bone: a warm off-white, matte. */
export const BONE = '#ece7df';
export const BONE_ROUGHNESS = 0.72;
export const BONE_METALNESS = 0;

/** The vertebra pointed at or chosen is tinted this share of the way to the accent. */
export const ACCENT = '#0a7cff';
export const TINT_SHARE = 0.7;

/** Soft light from above: white over warm grey. */
export const SKY = '#ffffff';
export const GROUND = '#d8d2c8';
export const AMBIENT_STRENGTH = 0.9;

/** The three lights ride with the camera, so the spine is lit the same from every side. `from` is right, up, toward the viewer. */
export const KEY_LIGHT = { colour: '#ffffff', strength: 1.9, from: [-1, 1.3, 1.1] } as const;
export const FILL_LIGHT = { colour: '#f2f5ff', strength: 0.55, from: [1.2, 0.1, 0.8] } as const;
export const RIM_LIGHT = { colour: '#ffffff', strength: 1.1, from: [0.6, 0.9, -1.6] } as const;

/**
 * The shade a bone carries in its hollows, worked out once from its own
 * shape: the canal, the notches and the roots of the processes are a
 * little darker and warmer than the open faces of the body. The deepest
 * hollow keeps this share of its light, tinted toward the warm grey.
 */
export const HOLLOW_KEEPS = 0.5;
export const HOLLOW_TINT = '#cfc6b8';

/** The one soft round shadow under the spine. */
export const SHADOW = {
  /** Its centre, fading to nothing at its edge. */
  colour: [20, 24, 32] as const,
  centre: 0.55,
  /** Its strength 45% of the way out. */
  midway: { at: 0.45, strength: 0.22 },
  /** How strongly the whole of it is shown. */
  shown: 0.55,
  /** Its width over the spine's footprint. */
  widthOverFootprint: 1.1,
  /** How far under the lowest bone it lies, mm. */
  below: 8,
  /** Seen level, it is tipped this many degrees toward the viewer so it never goes edge-on. */
  tipWhenLevel: 22,
} as const;

/** The see-through line version of a bone: a quiet ink line on the page's white, a white line over a film. */
export const CAGE = { onPage: '#5d5d5d', onFilm: '#ffffff', strength: 0.55 } as const;

/** Behind every film. */
export const FILM_BLACK = '#000000';

/** A bone's fill ends in a soft edge this share of the bone's height. */
export const FILL_EDGE = 0.14;

/** A bone arriving rises this far, mm, as it fades in. */
export const REVEAL_RISE = 8;

/** The drawing that grows into a bone. Widths are CSS pixels. */
export const FORMATION = {
  dot: 2.6, dotAtScale: 1.2, leastDotScale: .7, first: 1.3, node: 1.5, from: 1.7, line: 0.75, lift: 1.5, far: 0.32,
  ink: '#0d0d0d', dotStrength: 0.78, lightDot: 0.9, lightLine: 0.6,
  outlineLook: '#f5a524', mark: .9, casing: .45, casingLook: .65,
  threadStrength: 0.45, lightThread: 0.5, cellSoft: 0.3, fillReach: 1.35, outward: 0.6,
} as const;

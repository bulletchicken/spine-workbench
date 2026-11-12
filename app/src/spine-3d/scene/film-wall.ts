// A film standing in the same space as the bones: a flat sheet across its
// beam, black, with the film's picture on it. Where it stands across and up
// comes from the film's own camera, so every pixel is straight behind the
// anatomy that lands on it; it is never moved to suit the bones.
//
// The picture is whatever the caller already holds: a bitmap, a canvas, or
// plain numbers. Nothing is fetched and nothing is decoded here.
//
// A film is drawn only on the side its reader sees. From behind, its
// picture would be mirrored, and a mirrored film is a wrong film: so from
// behind there is nothing.

import {
  BufferAttribute,
  BufferGeometry,
  CanvasTexture,
  ClampToEdgeWrapping,
  Color,
  DataTexture,
  HalfFloatType,
  LinearFilter,
  LinearMipmapLinearFilter,
  Mesh,
  NoColorSpace,
  RGBAFormat,
  RedFormat,
  ShaderMaterial,
  type Texture,
  UnsignedByteType,
  Vector2,
} from 'three';
import type { FilmCamera } from '../placing/film-camera';
import { type FilmPlane, filmPlane } from '../placing/film-plane';
import { type Vec3, add, scale, sub } from '../placing/vectors';
import { FILM_BLACK } from './look';

/**
 * A film's picture. Numbers are one value a pixel, row by row from the top:
 * bytes from 0 to 255, or half-float bits from 0 to 1 (what the film viewer
 * holds). Four bytes a pixel are read as red, green, blue and alpha.
 */
export type FilmPixels =
  | ImageBitmap
  | HTMLCanvasElement
  | OffscreenCanvas
  | { data: Uint8Array | Uint8ClampedArray | Uint16Array; width: number; height: number };

export interface FilmWallValues {
  /** How strongly the film is shown, 0 to 1. */
  strength: number;
  /** How far the sheet stands from the patient origin toward the film's reader, mm. `null` lets the stage stand it just behind the spine. */
  out: number | null;
  /** The radius of the sheet's corners, mm. */
  corner: number;
  /**
   * A sheet hinged on one upright edge, swung flat. 1 is where the film
   * truly stands; 0 is swung a quarter turn about its hinge. Only a side
   * film beside a front one is hinged.
   */
  hinge: number;
}

export interface FilmWall {
  readonly camera: FilmCamera;
  set(values: Partial<FilmWallValues>): void;
  values(): FilmWallValues;
  /** Give the film a picture, or a new one; `null` leaves the sheet black. */
  show(pixels: FilmPixels | null): void;
  /** The sheet as it stands, hinge aside: its direction, its distance and its corners in the patient. */
  plane(): FilmPlane;
  /** Takes the film down and frees its picture. */
  remove(): void;
}

/** What the stage looks at each frame. */
export interface FilmWallInside {
  wall: FilmWall;
  mesh: Mesh;
  /**
   * Stand the sheet `out` mm toward its reader. `corner` is the upright
   * line where it meets another film, if it does: the edge there is left
   * square, and a sheet given a direction to lie flat in is hinged on it.
   */
  stand(out: number, corner: { on: Vec3; flatToward: Vec3 | null } | null): void;
  /** The sheet's corners as drawn, in the patient. */
  corners(): readonly Vec3[];
  out(): number | null;
}

const VERTEX = /* glsl */ `
varying vec2 vAt;
void main() {
  vAt = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

// The film's own values are written as they are: a film is shown as its greys, not lit.
const FRAGMENT = /* glsl */ `
uniform sampler2D uPicture;
uniform bool uHasPicture;
uniform bool uGrey;
uniform float uStrength;
uniform vec3 uBlack;
uniform vec2 uSize;
uniform float uCorner;
uniform vec2 uJoined;
varying vec2 vAt;
void main() {
  vec3 colour = uBlack;
  if (uHasPicture) {
    vec4 read = texture2D(uPicture, vAt);
    colour = uGrey ? vec3(read.r) : mix(uBlack, read.rgb, read.a);
  }
  // Rounded corners: how far inside the rounded outline this place is, in millimetres.
  // An edge that meets another film in a corner is left square: on that side the place is taken as
  // the sheet's middle, which is inside every rounding.
  vec2 at = vAt - 0.5;
  if (uJoined.x * at.x > 0.0) at.x = 0.0;
  if (uJoined.y * at.y > 0.0) at.y = 0.0;
  vec2 fromCentre = abs(at) * uSize;
  vec2 pastCorner = fromCentre - (0.5 * uSize - uCorner);
  float outside = length(max(pastCorner, 0.0)) + min(max(pastCorner.x, pastCorner.y), 0.0) - uCorner;
  float edge = max(fwidth(outside), 1e-5);
  float inside = 1.0 - smoothstep(-edge, 0.0, outside);
  gl_FragColor = vec4(colour, uStrength * inside);
}
`;

function textureOf(pixels: FilmPixels): { texture: Texture; grey: boolean } {
  if ('data' in pixels) {
    const { data, width, height } = pixels;
    const four = data.length === width * height * 4 && !(data instanceof Uint16Array);
    const texture = new DataTexture(
      data instanceof Uint8ClampedArray ? new Uint8Array(data.buffer, data.byteOffset, data.byteLength) : data,
      width,
      height,
      four ? RGBAFormat : RedFormat,
      data instanceof Uint16Array ? HalfFloatType : UnsignedByteType,
    );
    texture.unpackAlignment = 1;
    texture.magFilter = LinearFilter;
    // Half-float pictures are smoothed only between neighbouring pixels: smaller copies of them cannot be made everywhere.
    texture.minFilter = data instanceof Uint16Array ? LinearFilter : LinearMipmapLinearFilter;
    texture.generateMipmaps = !(data instanceof Uint16Array);
    texture.colorSpace = NoColorSpace;
    texture.needsUpdate = true;
    return { texture, grey: !four };
  }
  const texture = new CanvasTexture(pixels);
  texture.flipY = false;
  texture.colorSpace = NoColorSpace;
  texture.wrapS = ClampToEdgeWrapping;
  texture.wrapT = ClampToEdgeWrapping;
  return { texture, grey: false };
}

export function hangFilm(camera: FilmCamera, changed: () => void, removed: (wall: FilmWall) => void): FilmWallInside {
  let values: FilmWallValues = { strength: 1, out: null, corner: 8, hinge: 1 };
  let standing = filmPlane(camera, 0);
  let drawn: readonly Vec3[] = standing.corners;
  let picture: Texture | null = null;

  // The stored picture's own width and height, mm. Its spacings along a row and down a column are
  // the upright pixel's width and height, the other way about when the quarter turns are odd.
  const [su, sv] = camera.uprightMmPerPx;
  const [alongRow, downColumn] = camera.quarterTurns % 2 === 0 ? [su, sv] : [sv, su];
  const stored = [camera.size[0] * alongRow, camera.size[1] * downColumn] as const;

  const geometry = new BufferGeometry();
  const positions = new BufferAttribute(new Float32Array(12), 3);
  geometry.setAttribute('position', positions);
  // The stored picture's top-left, top-right, bottom-right and bottom-left, with its first row at the top.
  geometry.setAttribute('uv', new BufferAttribute(new Float32Array([0, 0, 1, 0, 1, 1, 0, 1]), 2));
  // Wound so that the side its reader sees is the front: from behind, a film is not drawn at all,
  // because from behind its picture would be mirrored.
  geometry.setIndex([0, 2, 1, 0, 3, 2]);

  const uniforms = {
    uPicture: { value: null as Texture | null },
    uHasPicture: { value: false },
    uGrey: { value: true },
    uStrength: { value: 1 },
    uBlack: { value: new Color(FILM_BLACK) },
    uSize: { value: new Vector2(stored[0], stored[1]) },
    uCorner: { value: values.corner },
    // Which edge of the stored picture meets another film: -1 or 1 along a row, and the same down a column; 0 for none.
    uJoined: { value: new Vector2(0, 0) },
  };
  const material = new ShaderMaterial({
    vertexShader: VERTEX,
    fragmentShader: FRAGMENT,
    uniforms,
    transparent: true,
    // A bone pressed flat lies on this sheet: the sheet is drawn a hair further away, so the bone wins.
    polygonOffset: true,
    polygonOffsetFactor: 2,
    polygonOffsetUnits: 4,
  });
  const mesh = new Mesh(geometry, material);
  mesh.matrixAutoUpdate = false;
  mesh.frustumCulled = false;
  mesh.renderOrder = -10;

  const inside: FilmWallInside = {
    wall: {
      camera,
      set(next) {
        values = { ...values, ...next };
        uniforms.uStrength.value = Math.min(1, Math.max(0, values.strength));
        uniforms.uCorner.value = Math.max(0, Math.min(values.corner, 0.5 * Math.min(stored[0], stored[1])));
        mesh.visible = values.strength > 0;
        changed();
      },
      values: () => ({ ...values }),
      show(pixels) {
        picture?.dispose();
        picture = null;
        if (pixels !== null) {
          const made = textureOf(pixels);
          picture = made.texture;
          uniforms.uGrey.value = made.grey;
        }
        uniforms.uPicture.value = picture;
        uniforms.uHasPicture.value = picture !== null;
        changed();
      },
      plane: () => standing,
      remove() {
        picture?.dispose();
        geometry.dispose();
        material.dispose();
        mesh.removeFromParent();
        removed(inside.wall);
      },
    },
    mesh,
    stand(out, hinge) {
      standing = filmPlane(camera, out);
      const flatToward = hinge?.flatToward ?? null;
      const swing = flatToward === null ? 0 : (1 - Math.min(1, Math.max(0, values.hinge))) * (Math.PI / 2);
      drawn =
        hinge === null || flatToward === null || swing === 0
          ? standing.corners
          : standing.corners.map((corner) => {
              // The corner's distance from the hinge, along the sheet. Swinging turns that reach from the
              // sheet's own direction toward the flat one, about the upright line through the hinge.
              const from = sub(corner, hinge.on);
              const reach = Math.hypot(from[0], from[1]);
              if (reach === 0) return corner;
              const along: Vec3 = [from[0] / reach, from[1] / reach, 0];
              const swung = add(scale(along, Math.cos(swing)), scale(flatToward, Math.sin(swing)));
              return [hinge.on[0] + swung[0] * reach, hinge.on[1] + swung[1] * reach, corner[2]];
            });
      drawn.forEach((corner, i) => {
        positions.setXYZ(i, corner[0], corner[1], corner[2]);
      });
      positions.needsUpdate = true;
      // The edge on the hinge: the two corners nearest it, as a side of the stored picture.
      uniforms.uJoined.value.set(0, 0);
      if (hinge !== null) {
        const near = standing.corners.map((corner) => Math.hypot(corner[0] - hinge.on[0], corner[1] - hinge.on[1]));
        const [topLeft = 0, topRight = 0, bottomRight = 0, bottomLeft = 0] = near;
        const sides = [
          { reach: topLeft + bottomLeft, joined: [-1, 0] },
          { reach: topRight + bottomRight, joined: [1, 0] },
          { reach: topLeft + topRight, joined: [0, -1] },
          { reach: bottomLeft + bottomRight, joined: [0, 1] },
        ] as const;
        const nearest = sides.reduce((best, side) => (side.reach < best.reach ? side : best));
        uniforms.uJoined.value.set(nearest.joined[0], nearest.joined[1]);
      }
    },
    corners: () => drawn,
    out: () => values.out,
  };
  return inside;
}

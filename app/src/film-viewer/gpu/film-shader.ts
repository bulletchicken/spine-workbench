// The shader that draws a film. The canvas is the size of the viewport,
// never the size of the film, and one triangle covers it: each device
// pixel asks the frozen film-space matrix (inverted) which film point it
// shows, and samples the film there. So turning, mirroring, zoom and pan
// all arrive as one matrix, exactly the one the layers over the film use.
//
// Sampling depends on how many texels one device pixel spans:
// - zoomed out, the texels it covers are averaged, so a 7000-pixel film
//   shown 700 pixels tall does not shimmer as it moves. The average is a
//   grid of at most 8 by 8 bilinear taps, each covering about 2 texels, so
//   it is complete up to 16 texels a device pixel; further out than that
//   (a whole film in a thumbnail) the grid samples the footprint instead;
// - past 1:1, each film pixel is drawn as a flat square, so single pixels
//   stay visible, with its edges blended across one device pixel so the
//   squares stay even at any zoom.
// Window and level and the brightness flip are applied last, and the
// film's outline is rounded and smoothed over one device pixel.
//
// Where half-float textures cannot be filtered, the same shader is built
// with BY_HAND, and blends texels itself from four exact texel reads, so
// both ways sample the same points with the same weights.

import { invert, screenMatrix, uprightSize, type FilmPlacement, type Matrix } from '../film-space';

/**
 * How a film is shown: window and level, and the brightness flip. The
 * window is in the display pixels' own units: the values from
 * `centre − width / 2` (black) to `centre + width / 2` (white).
 */
export interface FilmLook {
  centre: number;
  width: number;
  /** Bone dark instead of bright. */
  flipped: boolean;
}

/** The display pixels as the engine made them: 0 black, 1 white. */
export const PLAIN_LOOK: FilmLook = { centre: 0.5, width: 1, flipped: false };

/** One triangle that covers the whole canvas, made from the vertex index alone. */
export const COVER_VIEW = `#version 300 es
void main() {
  vec2 corner = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);
}
`;

/** The most taps along each side of the box a zoomed-out device pixel averages. */
const MOST_TAPS = 8;

export function filmFragmentShader(byHand: boolean): string {
  return `#version 300 es
${byHand ? '#define BY_HAND' : ''}
precision highp float;
precision highp int;
precision highp sampler2D;

uniform sampler2D u_film;
uniform vec2 u_texels;
uniform float u_step;
uniform mat3 u_toFilm;
uniform vec2 u_cssPerDevice;
uniform float u_deviceHeight;
uniform float u_texelsPerDevice;
uniform vec4 u_edges;
uniform float u_corner;
uniform vec2 u_window;
uniform bool u_flipped;

out vec4 colour;

const int MOST_TAPS = ${String(MOST_TAPS)};

#ifdef BY_HAND
float texel(ivec2 at) {
  return texelFetch(u_film, clamp(at, ivec2(0), ivec2(u_texels) - 1), 0).r;
}
// Bilinear, from four exact reads. t is in texels from the outer corner.
float blend(vec2 t) {
  vec2 p = t - 0.5;
  vec2 below = floor(p);
  vec2 f = p - below;
  ivec2 i = ivec2(below);
  float top = mix(texel(i), texel(i + ivec2(1, 0)), f.x);
  float bottom = mix(texel(i + ivec2(0, 1)), texel(i + ivec2(1, 1)), f.x);
  return mix(top, bottom, f.y);
}
#else
float blend(vec2 t) {
  return texture(u_film, t / u_texels).r;
}
#endif

// Each tap is bilinear, so it already averages up to a 2 x 2 block.
const float TAP_REACH = 2.0;

// Past 1:1: each texel flat, its edge blended across one device pixel.
float sharp(vec2 t) {
  vec2 p = t - 0.5;
  vec2 below = floor(p);
  vec2 f = p - below;
  f = clamp((f - 0.5) / u_texelsPerDevice + 0.5, 0.0, 1.0);
  return blend(below + f + 0.5);
}

// Zoomed out: the mean of a grid of taps across the device pixel's footprint.
float box(vec2 t) {
  float span = u_texelsPerDevice;
  int taps = int(min(ceil(span / TAP_REACH), float(MOST_TAPS)));
  float sum = 0.0;
  for (int y = 0; y < MOST_TAPS; ++y) {
    if (y >= taps) break;
    for (int x = 0; x < MOST_TAPS; ++x) {
      if (x >= taps) break;
      vec2 offset = (vec2(float(x), float(y)) + 0.5) / float(taps) - 0.5;
      sum += blend(t + offset * span);
    }
  }
  return sum / float(taps * taps);
}

void main() {
  vec2 screen = vec2(gl_FragCoord.x, u_deviceHeight - gl_FragCoord.y) * u_cssPerDevice;

  // How much of this device pixel the film's rounded outline covers.
  vec2 middle = (u_edges.xy + u_edges.zw) * 0.5;
  vec2 halfSize = (u_edges.zw - u_edges.xy) * 0.5;
  float radius = min(u_corner, min(halfSize.x, halfSize.y));
  vec2 q = abs(screen - middle) - halfSize + radius;
  float outside = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
  float cover = clamp(0.5 - outside / u_cssPerDevice.x, 0.0, 1.0);
  if (cover <= 0.0) {
    colour = vec4(0.0);
    return;
  }

  vec2 film = (u_toFilm * vec3(screen, 1.0)).xy;
  vec2 t = (film + 0.5) / u_step;
  float value = u_texelsPerDevice <= 1.0 ? sharp(t) : box(t);
  float shade = clamp((value - u_window.x) / (u_window.y - u_window.x), 0.0, 1.0);
  if (u_flipped) shade = 1.0 - shade;
  colour = vec4(vec3(shade) * cover, cover);
}
`;
}

/** Compile and link a program, or throw with the driver's own words: a broken shader is a bug, not a user's problem. */
export function compileProgram(gl: WebGL2RenderingContext, vertexSource: string, fragmentSource: string): WebGLProgram {
  const program = gl.createProgram();
  const shaders = [compileShader(gl, gl.VERTEX_SHADER, vertexSource), compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource)];
  for (const shader of shaders) gl.attachShader(program, shader);
  gl.linkProgram(program);
  for (const shader of shaders) {
    gl.detachShader(program, shader);
    gl.deleteShader(shader);
  }
  if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true && !gl.isContextLost()) {
    const log = gl.getProgramInfoLog(program) ?? '';
    gl.deleteProgram(program);
    throw new Error(`The film shader did not link: ${log}`);
  }
  return program;
}

function compileShader(gl: WebGL2RenderingContext, kind: GLenum, source: string): WebGLShader {
  const shader = gl.createShader(kind);
  if (shader === null) throw new Error('No shader could be made');
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (gl.getShaderParameter(shader, gl.COMPILE_STATUS) !== true && !gl.isContextLost()) {
    throw new Error(`A film shader did not compile: ${gl.getShaderInfoLog(shader) ?? ''}`);
  }
  return shader;
}

const UNIFORMS = [
  'u_film',
  'u_texels',
  'u_step',
  'u_toFilm',
  'u_cssPerDevice',
  'u_deviceHeight',
  'u_texelsPerDevice',
  'u_edges',
  'u_corner',
  'u_window',
  'u_flipped',
] as const;

type UniformName = (typeof UNIFORMS)[number];

export interface FilmProgram {
  program: WebGLProgram;
  byHand: boolean;
  uniforms: Record<UniformName, WebGLUniformLocation | null>;
}

export function makeFilmProgram(gl: WebGL2RenderingContext, byHand: boolean): FilmProgram {
  const program = compileProgram(gl, COVER_VIEW, filmFragmentShader(byHand));
  const uniforms = Object.fromEntries(UNIFORMS.map((name) => [name, gl.getUniformLocation(program, name)])) as Record<
    UniformName,
    WebGLUniformLocation | null
  >;
  return { program, byHand, uniforms };
}

/** The canvas: its size in device pixels and in CSS pixels. */
export interface CanvasSize {
  deviceWidth: number;
  deviceHeight: number;
  cssWidth: number;
  cssHeight: number;
}

/** What one draw needs. */
export interface FilmScene {
  placement: FilmPlacement;
  look: FilmLook;
  canvas: CanvasSize;
  /** The film's corner radius on screen, CSS pixels. */
  corner: number;
  texels: { width: number; height: number };
  /** Film pixels per texel. */
  step: number;
}

/** The uniform values for a scene, worked out apart from any GPU so they can be checked. */
export function filmUniforms(scene: FilmScene) {
  const { placement, canvas, look } = scene;
  const toFilm: Matrix = invert(screenMatrix(placement));
  const cssPerDevice: [number, number] = [canvas.cssWidth / canvas.deviceWidth, canvas.cssHeight / canvas.deviceHeight];
  const upright = uprightSize(placement.size, placement.view.quarterTurns);
  const [left, top] = placement.pan;
  return {
    texels: [scene.texels.width, scene.texels.height] as const,
    step: scene.step,
    // Column by column, as GLSL's mat3 wants it.
    toFilm: [toFilm[0], toFilm[1], 0, toFilm[2], toFilm[3], 0, toFilm[4], toFilm[5], 1] as const,
    cssPerDevice,
    deviceHeight: canvas.deviceHeight,
    texelsPerDevice: cssPerDevice[0] / (placement.zoom * scene.step),
    edges: [left, top, left + upright.width * placement.zoom, top + upright.height * placement.zoom] as const,
    corner: scene.corner,
    window: [look.centre - look.width / 2, look.centre + look.width / 2] as const,
    flipped: look.flipped,
  };
}

/** Draw a film over whatever the canvas holds; the caller sets the viewport and clears. */
export function drawFilm(gl: WebGL2RenderingContext, film: FilmProgram, texture: WebGLTexture, scene: FilmScene): void {
  const values = filmUniforms(scene);
  const u = film.uniforms;
  gl.useProgram(film.program);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.uniform1i(u.u_film, 0);
  gl.uniform2f(u.u_texels, ...values.texels);
  gl.uniform1f(u.u_step, values.step);
  gl.uniformMatrix3fv(u.u_toFilm, false, values.toFilm);
  gl.uniform2f(u.u_cssPerDevice, ...values.cssPerDevice);
  gl.uniform1f(u.u_deviceHeight, values.deviceHeight);
  gl.uniform1f(u.u_texelsPerDevice, values.texelsPerDevice);
  gl.uniform4f(u.u_edges, ...values.edges);
  gl.uniform1f(u.u_corner, values.corner);
  gl.uniform2f(u.u_window, ...values.window);
  gl.uniform1i(u.u_flipped, values.flipped ? 1 : 0);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
}

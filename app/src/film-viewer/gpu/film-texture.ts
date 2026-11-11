// A film's half-float pixels as an R16F texture (docs/decisions/
// half-float-film-texture.md). WebGL2 promises that R16F can be filtered,
// but a browser that breaks the promise would show a blocky film, so
// `canFilterHalfFloat` asks the GPU itself: it reads a two-texel texture
// halfway between its texels and sees whether it got the blend. Where it
// did not, the texture is read texel by texel and the shader blends by
// hand (film-shader.ts, BY_HAND).
//
// A film larger than the GPU's largest texture is shown from a copy made
// smaller by a whole number, each texel the mean of a square of pixels.
// The film itself is never changed, and the shader still works in film
// pixels, so marks stay exactly where they are.

import type { FilmSize } from '../film-space';
import { COVER_VIEW, compileProgram } from './film-shader';
import { floatToHalf, halfFloatValues } from './half-float';

export interface FilmTexture {
  texture: WebGLTexture;
  texels: { width: number; height: number };
  /** Film pixels per texel along each side: 1 unless the film had to be made smaller. */
  step: number;
}

const READ_HALFWAY = `#version 300 es
precision highp float;
uniform highp sampler2D u_film;
out vec4 colour;
void main() {
  colour = vec4(texture(u_film, vec2(0.5, 0.5)).r);
}
`;

/** True when this GPU blends between half-float texels as WebGL2 says it must. */
export function canFilterHalfFloat(gl: WebGL2RenderingContext): boolean {
  const source = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, source);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 2);
  // 0.0 and 1.0: read halfway between, a filtered read gives 0.5, an unfiltered one 0 or 1.
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.R16F, 2, 1, 0, gl.RED, gl.HALF_FLOAT, new Uint16Array([0x0000, 0x3c00]));
  setFiltering(gl, gl.LINEAR);
  const target = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, target);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
  const frame = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, frame);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, target, 0);
  const program = compileProgram(gl, COVER_VIEW, READ_HALFWAY);
  gl.useProgram(program);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, source);
  gl.uniform1i(gl.getUniformLocation(program, 'u_film'), 0);
  gl.viewport(0, 0, 1, 1);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
  const read = new Uint8Array(4);
  gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, read);
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  gl.useProgram(null);
  gl.deleteProgram(program);
  gl.deleteFramebuffer(frame);
  gl.deleteTexture(target);
  gl.deleteTexture(source);
  return Math.abs((read[0] ?? 0) - 128) <= 32;
}

/** Upload a film's pixels; `filterable` from `canFilterHalfFloat`. */
export function uploadFilm(gl: WebGL2RenderingContext, pixels: Uint16Array, size: FilmSize, filterable: boolean): FilmTexture {
  const largest = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
  const fitted = shrinkToFit(pixels, size, largest);
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 2);
  gl.texStorage2D(gl.TEXTURE_2D, 1, gl.R16F, fitted.width, fitted.height);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, fitted.width, fitted.height, gl.RED, gl.HALF_FLOAT, fitted.pixels);
  setFiltering(gl, filterable ? gl.LINEAR : gl.NEAREST);
  return { texture, texels: { width: fitted.width, height: fitted.height }, step: fitted.step };
}

function setFiltering(gl: WebGL2RenderingContext, filter: GLenum) {
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}

export interface FittedPixels {
  pixels: Uint16Array;
  width: number;
  height: number;
  step: number;
}

/**
 * The film as it is when it fits within `largest` texels a side, or else
 * a copy whose texel (i, j) is the mean of film pixels i·step to
 * (i + 1)·step − 1 across and j·step to (j + 1)·step − 1 down, cut short
 * at the film's edge.
 */
export function shrinkToFit(pixels: Uint16Array, size: FilmSize, largest: number): FittedPixels {
  const step = Math.ceil(Math.max(size.width, size.height) / largest);
  if (step <= 1) return { pixels, width: size.width, height: size.height, step: 1 };
  const width = Math.ceil(size.width / step);
  const height = Math.ceil(size.height / step);
  const values = halfFloatValues();
  const out = new Uint16Array(width * height);
  for (let j = 0; j < height; j += 1) {
    const rowEnd = Math.min((j + 1) * step, size.height);
    for (let i = 0; i < width; i += 1) {
      const columnEnd = Math.min((i + 1) * step, size.width);
      let sum = 0;
      let count = 0;
      for (let y = j * step; y < rowEnd; y += 1) {
        for (let x = i * step; x < columnEnd; x += 1) {
          sum += values[pixels[y * size.width + x] ?? 0] ?? 0;
          count += 1;
        }
      }
      out[j * width + i] = floatToHalf(sum / count);
    }
  }
  return { pixels: out, width, height, step };
}

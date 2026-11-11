import { inflateSync } from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { FILM_HEIGHT, FILM_WIDTH } from './anatomy';
import { paintedFilm } from './film-painter';
import { percentileWindow, SAMPLE_FILMS, storedGrey, thumbnailGrey } from './films';

const bytes = (buffer: ArrayBuffer) => new Uint8Array(buffer);
const textOf = (data: Uint8Array) => String.fromCharCode(...data);

/** The pixels of an 8-bit greyscale PNG with one IDAT, read back with zlib. */
function pngPixels(png: Uint8Array) {
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  let width = 0;
  let height = 0;
  let rows = new Uint8Array(0);
  const texts: string[] = [];
  for (let at = 8; at < png.length; ) {
    const length = view.getUint32(at);
    const type = textOf(png.subarray(at + 4, at + 8));
    const data = png.subarray(at + 8, at + 8 + length);
    if (type === 'IHDR') [width, height] = [view.getUint32(at + 8), view.getUint32(at + 12)];
    if (type === 'IDAT') rows = new Uint8Array(inflateSync(data));
    if (type === 'tEXt') texts.push(textOf(data));
    at += 12 + length;
  }
  const grey = new Uint8Array(width * height);
  for (let row = 0; row < height; row++) grey.set(rows.subarray(row * (width + 1) + 1, (row + 1) * (width + 1)), row * width);
  return { width, height, grey, texts };
}

describe('the sample films', () => {
  it('are DICOM files with a size, except bending right, a PNG without one', () => {
    for (const kind of ['front', 'side', 'bend-left'] as const) {
      const file = bytes(SAMPLE_FILMS[kind].file);
      expect(textOf(file.subarray(128, 132))).toBe('DICM');
      const facts = SAMPLE_FILMS[kind].facts;
      expect(facts.source).toBe('dicom');
      expect(facts.keptTags?.pixelSpacing).toEqual([0.45, 0.45]);
      expect(facts.keptTags?.pixelSpacingCalibration).toBe('geometry');
      expect(facts.defaultWindow.from).toBe('file');
      // The pixels sit at the end of the file, as stored.
      expect(file.subarray(file.length - FILM_WIDTH * FILM_HEIGHT)).toEqual(storedGrey(kind));
    }
    const png = pngPixels(bytes(SAMPLE_FILMS['bend-right'].file));
    expect(png.texts).toEqual(['Title\0Spine Workbench sample case: bend-right film']);
    expect(png.grey).toEqual(storedGrey('bend-right'));
    const facts = SAMPLE_FILMS['bend-right'].facts;
    expect(facts.source).toBe('png');
    expect(facts.keptTags).toBeNull();
    expect(facts.defaultWindow.from).toBe('percentile');
  });

  it('give every film the mock size and eight bits', () => {
    for (const film of Object.values(SAMPLE_FILMS)) {
      expect([film.facts.width, film.facts.height, film.facts.bitsStored]).toEqual([720, 1720, 8]);
      expect(film.pixels()).toHaveLength(720 * 1720);
    }
  });

  it('store bending left upside down', () => {
    const upright = paintedFilm('bend-left').film;
    const stored = storedGrey('bend-left');
    expect(stored[0]).toBe(upright[upright.length - 1]);
    expect(stored[FILM_WIDTH * 10 + 3]).toBe(upright[FILM_WIDTH * (FILM_HEIGHT - 11) + FILM_WIDTH - 4]);
  });

  it('hand out half floats of the stored values, 0 to 1', () => {
    const pixels = SAMPLE_FILMS.side.pixels();
    const grey = storedGrey('side');
    const index = grey.findIndex((value) => value === 255);
    if (index >= 0) expect(pixels[index]).toBe(0x3c00);
    const zero = grey.findIndex((value) => value === 0);
    if (zero >= 0) expect(pixels[zero]).toBe(0);
    expect(grey.some((value) => value === 0 || value === 255)).toBe(true);
  });

  it('hand out fresh copies, so emptying one leaves the next whole', () => {
    const first = SAMPLE_FILMS.front.file;
    structuredClone(first, { transfer: [first] });
    expect(first.byteLength).toBe(0);
    expect(SAMPLE_FILMS.front.file.byteLength).toBeGreaterThan(FILM_WIDTH * FILM_HEIGHT);
  });

  it('make thumbnails at most 256 pixels on the longest side', () => {
    const thumb = pngPixels(SAMPLE_FILMS.front.thumbnail);
    expect([thumb.width, thumb.height]).toEqual([107, 256]);
  });
});

describe('thumbnails and windows', () => {
  it('average a flat picture to the same value and keep a split one split', () => {
    const flat = thumbnailGrey(new Uint8Array(512 * 512).fill(77), 512, 512);
    expect([flat.width, flat.height]).toEqual([256, 256]);
    expect(flat.grey.every((value) => value === 77)).toBe(true);
    const halves = Uint8Array.from({ length: 512 * 512 }, (_, k) => (k % 512 < 256 ? 0 : 200));
    const small = thumbnailGrey(halves, 512, 512).grey;
    expect([small[0], small[127], small[128], small[255]]).toEqual([0, 0, 200, 200]);
  });

  it('keep a film no bigger than a thumbnail at its own size, pixel for pixel', () => {
    const grey = Uint8Array.from({ length: 100 * 60 }, (_, k) => k % 251);
    const kept = thumbnailGrey(grey, 100, 60);
    expect([kept.width, kept.height]).toEqual([100, 60]);
    expect([...kept.grey]).toEqual([...grey]);
  });

  it('take the 1st and 99th percentiles', () => {
    // One pixel of each value from 0 to 99: 1% of them is value 0, 99% is value 98.
    expect(percentileWindow(Uint8Array.from({ length: 100 }, (_, k) => k))).toEqual({ centre: 49, width: 98 });
  });
});

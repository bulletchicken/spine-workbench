// A real DICOM file, written by hand: a digital X-ray for presentation,
// 8 bits, MONOCHROME2, uncompressed, explicit VR little endian (DICOM
// PS3.10 for the file, PS3.5 for the encoding). It carries only what a
// film reader needs and nothing that names anyone: the identifiers it
// must have are made-up UIDs under 2.25.
//
// The pixel spacing says how big a pixel is, with its calibration type
// GEOMETRY, which is how three of the sample films have a size "read from
// the file".

const EXPLICIT_LITTLE_ENDIAN = '1.2.840.10008.1.2.1';
/** Digital X-Ray Image Storage, For Presentation. */
export const DX_FOR_PRESENTATION = '1.2.840.10008.5.1.4.1.1.1.1';

interface Element {
  group: number;
  element: number;
  vr: string;
  value: Uint8Array;
}

const ascii = (text: string) => Uint8Array.from(text, (character) => character.charCodeAt(0) & 0x7f);

/** Text values pad to an even length: UIDs with a zero byte, the rest with a space. */
function text(group: number, element: number, vr: string, value: string): Element {
  const bytes = ascii(value);
  if (bytes.length % 2 === 0) return { group, element, vr, value: bytes };
  const padded = new Uint8Array(bytes.length + 1);
  padded.set(bytes);
  padded[bytes.length] = vr === 'UI' ? 0 : 0x20;
  return { group, element, vr, value: padded };
}

function us(group: number, element: number, value: number): Element {
  const bytes = new Uint8Array(2);
  new DataView(bytes.buffer).setUint16(0, value, true);
  return { group, element, vr: 'US', value: bytes };
}

function ul(group: number, element: number, value: number): Element {
  const bytes = new Uint8Array(4);
  new DataView(bytes.buffer).setUint32(0, value, true);
  return { group, element, vr: 'UL', value: bytes };
}

/** Pixel data, padded with a zero byte to an even length as every value must be (PS3.5 7.1.1). */
function pixelData(grey: Uint8Array): Element {
  if (grey.length % 2 === 0) return { group: 0x7fe0, element: 0x0010, vr: 'OB', value: grey };
  const padded = new Uint8Array(grey.length + 1);
  padded.set(grey);
  return { group: 0x7fe0, element: 0x0010, vr: 'OB', value: padded };
}

/** VRs whose length takes four bytes after two reserved ones (PS3.5 7.1.2). */
const LONG_FORM = new Set(['OB', 'OW', 'OF', 'SQ', 'UT', 'UN']);

function encode(elements: Element[]): Uint8Array {
  const size = elements.reduce((sum, e) => sum + (LONG_FORM.has(e.vr) ? 12 : 8) + e.value.length, 0);
  const out = new Uint8Array(size);
  const view = new DataView(out.buffer);
  let at = 0;
  for (const { group, element, vr, value } of elements) {
    view.setUint16(at, group, true);
    view.setUint16(at + 2, element, true);
    out[at + 4] = vr.charCodeAt(0);
    out[at + 5] = vr.charCodeAt(1);
    if (LONG_FORM.has(vr)) {
      view.setUint32(at + 8, value.length, true);
      at += 12;
    } else {
      view.setUint16(at + 6, value.length, true);
      at += 8;
    }
    out.set(value, at);
    at += value.length;
  }
  return out;
}

export interface DicomFilm {
  /** Row-major, `columns` × `rows`, bone bright. */
  grey: Uint8Array;
  columns: number;
  rows: number;
  /** Millimetres per pixel, the same down a column and along a row. */
  mmPerPx: number;
  /** AP for a front-view film, LL for the side film. */
  viewPosition: 'AP' | 'LL';
  /** The window the film opens with. */
  windowing: { centre: number; width: number };
  /** Made-up numbers that make this file's UIDs its own. */
  uid: string;
}

/** The DICOM file of an 8-bit greyscale film. */
export function writeDicom(film: DicomFilm): Uint8Array {
  const instance = `2.25.${film.uid}`;
  const meta = encode([
    { group: 0x0002, element: 0x0001, vr: 'OB', value: Uint8Array.of(0, 1) },
    text(0x0002, 0x0002, 'UI', DX_FOR_PRESENTATION),
    text(0x0002, 0x0003, 'UI', instance),
    text(0x0002, 0x0010, 'UI', EXPLICIT_LITTLE_ENDIAN),
    text(0x0002, 0x0012, 'UI', '2.25.302114620447265412061402551023'),
  ]);
  const spacing = String(film.mmPerPx);
  const data = encode([
    text(0x0008, 0x0016, 'UI', DX_FOR_PRESENTATION),
    text(0x0008, 0x0018, 'UI', instance),
    text(0x0008, 0x0060, 'CS', 'DX'),
    text(0x0008, 0x0068, 'CS', 'FOR PRESENTATION'),
    text(0x0018, 0x5101, 'CS', film.viewPosition),
    us(0x0028, 0x0002, 1),
    text(0x0028, 0x0004, 'CS', 'MONOCHROME2'),
    us(0x0028, 0x0010, film.rows),
    us(0x0028, 0x0011, film.columns),
    text(0x0028, 0x0030, 'DS', `${spacing}\\${spacing}`),
    us(0x0028, 0x0100, 8),
    us(0x0028, 0x0101, 8),
    us(0x0028, 0x0102, 7),
    us(0x0028, 0x0103, 0),
    text(0x0028, 0x0a02, 'CS', 'GEOMETRY'),
    text(0x0028, 0x1050, 'DS', String(film.windowing.centre)),
    text(0x0028, 0x1051, 'DS', String(film.windowing.width)),
    pixelData(film.grey),
  ]);
  const groupLength = encode([ul(0x0002, 0x0000, meta.length)]);
  const out = new Uint8Array(128 + 4 + groupLength.length + meta.length + data.length);
  out.set(ascii('DICM'), 128);
  out.set(groupLength, 132);
  out.set(meta, 132 + groupLength.length);
  out.set(data, 132 + groupLength.length + meta.length);
  return out;
}

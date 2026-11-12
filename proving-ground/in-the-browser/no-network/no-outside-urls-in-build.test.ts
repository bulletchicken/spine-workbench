// Every http(s) and ws(s) address written anywhere in the production
// build, the .wasm included, must be on the short reviewed list below.
// None of them is ever requested: each is a name or a piece of text, not a
// place the app goes. A new address in the build fails this test until
// someone reviews it and adds it here with the reason.

import { describe, expect, it } from 'vitest';
import { builtFiles, builtText } from '../test-rig/built-app';

const REVIEWED: Record<string, string> = {
  'http://www.w3.org/2000/svg': 'the SVG namespace: React creates SVG elements in it; the favicon declares it',
  'http://www.w3.org/1999/xlink': 'the XLink namespace for SVG attributes, in React DOM',
  'http://www.w3.org/1998/Math/MathML': 'the MathML namespace, in React DOM',
  'http://www.w3.org/XML/1998/namespace': 'the XML namespace for xml: attributes, in React DOM',
  'https://react.dev/errors/': 'the start of the text of React’s production error messages',
  'https://creativecommons.org/licenses/by/4.0/':
    'the licence of the bone shapes, as text in their list (spine-3d/bones/vertebrae.json), which is built into the 3D view: the licence asks to be named by its address wherever the shapes go',
  'https://dbarchive.biosciencedbc.jp/en/bodyparts3d/': 'where the bone shapes come from, as text in the same list',
  'http://www.w3.org/1999/xhtml': 'the XHTML namespace: three.js makes its canvases with createElementNS',
  'https://jcgt.org/published/0007/04/01/':
    'a paper cited in a comment inside the text of one of three.js’s shaders (sampling for rough reflections); the text is kept whole because it is a string',
};

const ADDRESS = /(?:https?|wss?):\/\/[A-Za-z0-9\-._~:/?#[\]@!$&*+,;=%]*/g;

describe('the production build', () => {
  const found = new Map<string, Set<string>>();
  for (const file of builtFiles()) {
    for (const [address] of builtText(file).matchAll(ADDRESS)) {
      const files = found.get(address) ?? new Set<string>();
      files.add(file);
      found.set(address, files);
    }
  }

  it('names only reviewed addresses', () => {
    const unreviewed = [...found].filter(([address]) => !(address in REVIEWED)).map(([address, files]) => `${address} in ${[...files].join(', ')}`);
    expect(unreviewed).toEqual([]);
  });

  it('is actually scanned: the SVG namespace is found', () => {
    expect(found.has('http://www.w3.org/2000/svg')).toBe(true);
  });
});

// Every http(s) and ws(s) address written anywhere in the production
// build, the .wasm included, must be on the short reviewed list below.
// None of them is ever requested: each is a name or a piece of text, not a
// place the app goes. A new address in the build fails this test until
// someone reviews it and adds it here with the reason.

import { describe, expect, it } from 'vitest';
import { builtFiles, builtText } from '../test-rig/built-app';

// Where the sources behind the placement proposals and the load bench can
// be read. Each is a citation: text in the engine's evidence, built into
// the .wasm, never fetched. A few end in a full stop or a comma because the
// address closes a sentence in a source's note.
const citedIn = (evidence: string, addresses: readonly string[]): Record<string, string> =>
  Object.fromEntries(
    addresses.map((address) => [address, `where a source cited in the engine’s evidence (${evidence}) can be read, as text, never fetched`]),
  );

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
  'https://pmc.ncbi.nlm.nih.gov/articles/PMC5795289/':
    'where the 2016 SOSORT guidelines can be read: the source the spine readings give for calling a curve a scoliosis from 10 degrees, as text in the engine’s evidence (engine/spine-readings/evidence), never fetched',  ...citedIn('engine/implant-layout/evidence', [
    'https://doi.org/10.1186/1748-7161-9-11',
    'https://europepmc.org/abstract/MED/11507125',
    'https://europepmc.org/abstract/MED/20127493',
    'https://europepmc.org/abstract/MED/21739245',
    'https://europepmc.org/abstract/MED/24275616',
    'https://europepmc.org/abstract/MED/24480964',
    'https://europepmc.org/abstract/MED/25147977',
    'https://europepmc.org/abstract/MED/26124077',
    'https://europepmc.org/abstract/MED/2718038',
    'https://europepmc.org/abstract/MED/27750078',
    'https://europepmc.org/abstract/MED/27927354',
    'https://europepmc.org/abstract/MED/28296814',
    'https://europepmc.org/abstract/MED/30348348',
    'https://europepmc.org/abstract/MED/30739071',
    'https://europepmc.org/abstract/MED/34096362',
    'https://europepmc.org/abstract/MED/36138576',
    'https://europepmc.org/abstract/MED/37134132',
    'https://europepmc.org/abstract/MED/37973031',
    'https://europepmc.org/abstract/MED/38255344',
    'https://europepmc.org/abstract/MED/38408520',
    'https://europepmc.org/abstract/MED/39658742',
    'https://europepmc.org/abstract/MED/40020691',
    'https://europepmc.org/abstract/MED/40103372',
    'https://europepmc.org/abstract/MED/40299013',
    'https://europepmc.org/abstract/MED/40299013.',
    'https://europepmc.org/abstract/MED/40856839',
    'https://europepmc.org/abstract/MED/40924388.',
    'https://europepmc.org/abstract/MED/42811138',
    'https://europepmc.org/article/PMC/PMC10455094',
    'https://europepmc.org/article/PMC/PMC11022599',
    'https://europepmc.org/article/PMC/PMC11350560',
    'https://europepmc.org/article/PMC/PMC11875695',
    'https://europepmc.org/article/PMC/PMC11875695.',
    'https://europepmc.org/article/PMC/PMC12361177',
    'https://europepmc.org/article/PMC/PMC12870182',
    'https://europepmc.org/article/PMC/PMC13295182',
    'https://europepmc.org/article/PMC/PMC2856392',
    'https://europepmc.org/article/PMC/PMC2899159',
    'https://europepmc.org/article/PMC/PMC3675381',
    'https://europepmc.org/article/PMC/PMC4331738',
    'https://europepmc.org/article/PMC/PMC4417731',
    'https://europepmc.org/article/PMC/PMC5353768',
    'https://europepmc.org/article/PMC/PMC5638737',
    'https://europepmc.org/article/PMC/PMC6459034',
    'https://europepmc.org/article/PMC/PMC6773991',
    'https://europepmc.org/article/PMC/PMC6773991,',
    'https://europepmc.org/article/PMC/PMC7788412',
    'https://europepmc.org/article/PMC/PMC8273784',
    'https://europepmc.org/article/PMC/PMC9450434',
    'https://europepmc.org/article/PMC/PMC9636753',
    'https://pubmed.ncbi.nlm.nih.gov/21959835/',
    'https://stg.edit.surgeryreference.aofoundation.org/spine/deformities/adolescent-idiopathic-scoliosis/further-reading/lenke-classification',
    'https://www.ebi.ac.uk/europepmc/webservices/rest/PMC12006242/fullTextXML',
    'https://www.frontiersin.org/articles/10.3389/fsurg.2023.1219816/full',
  ]),
  ...citedIn('engine/load-bench/evidence', [
    'https://europepmc.org/abstract/MED/10222525',
    'https://europepmc.org/abstract/MED/10934658',
    'https://europepmc.org/abstract/MED/11383294',
    'https://europepmc.org/abstract/MED/11784277',
    'https://europepmc.org/abstract/MED/13680185',
    'https://europepmc.org/abstract/MED/14706323',
    'https://europepmc.org/abstract/MED/14977223',
    'https://europepmc.org/abstract/MED/15928553',
    'https://europepmc.org/abstract/MED/1750009',
    'https://europepmc.org/abstract/MED/17983694',
    'https://europepmc.org/abstract/MED/18469689',
    'https://europepmc.org/abstract/MED/18562971',
    'https://europepmc.org/abstract/MED/18790684',
    'https://europepmc.org/abstract/MED/19402917',
    'https://europepmc.org/abstract/MED/19829250',
    'https://europepmc.org/abstract/MED/21798014',
    'https://europepmc.org/abstract/MED/23141957',
    'https://europepmc.org/abstract/MED/23623494',
    'https://europepmc.org/abstract/MED/23680391',
    'https://europepmc.org/abstract/MED/24201510',
    'https://europepmc.org/abstract/MED/24211089',
    'https://europepmc.org/abstract/MED/24559120',
    'https://europepmc.org/abstract/MED/24590340',
    'https://europepmc.org/abstract/MED/24743577',
    'https://europepmc.org/abstract/MED/24866883',
    'https://europepmc.org/abstract/MED/24973826',
    'https://europepmc.org/abstract/MED/25082759',
    'https://europepmc.org/abstract/MED/25310387',
    'https://europepmc.org/abstract/MED/25640900',
    'https://europepmc.org/abstract/MED/25810754',
    'https://europepmc.org/abstract/MED/26451947',
    'https://europepmc.org/abstract/MED/26681978',
    'https://europepmc.org/abstract/MED/26900035',
    'https://europepmc.org/abstract/MED/26952022',
    'https://europepmc.org/abstract/MED/27914627',
    'https://europepmc.org/abstract/MED/27927364',
    'https://europepmc.org/abstract/MED/27991528',
    'https://europepmc.org/abstract/MED/28391382',
    'https://europepmc.org/abstract/MED/28428982',
    'https://europepmc.org/abstract/MED/28520819',
    'https://europepmc.org/abstract/MED/28773240',
    'https://europepmc.org/abstract/MED/29852313',
    'https://europepmc.org/abstract/MED/29991241',
    'https://europepmc.org/abstract/MED/32117927',
    'https://europepmc.org/abstract/MED/32195239',
    'https://europepmc.org/abstract/MED/32453125',
    'https://europepmc.org/abstract/MED/34155224',
    'https://europepmc.org/abstract/MED/34712654',
    'https://europepmc.org/abstract/MED/36685708',
    'https://europepmc.org/abstract/MED/41383715',
    'https://europepmc.org/abstract/MED/8678352',
    'https://europepmc.org/abstract/MED/8970923',
    'https://europepmc.org/abstract/MED/9588471',
  ]),
  ...citedIn('engine/implant-layout/evidence and engine/load-bench/evidence', [
    'https://europepmc.org/abstract/MED/14501914',
    'https://europepmc.org/abstract/MED/22426445',
  ]),
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

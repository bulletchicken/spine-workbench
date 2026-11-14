import { readdirSync, readFileSync } from 'node:fs';
import type { LevelRule } from '@/shapes';
import { describe, expect, it } from 'vitest';
import {
  EVIDENCE_LEDGER, PLACE_SOURCES_WITHOUT_ENTRY, type EvidenceEntry, type EvidenceSource,
} from './evidence-ledger';

interface SourceRecord {
  key: string;
  citation: {
    authors: string | null;
    year: number | null;
    title: string;
    journal: string | null;
    volume: string | null;
    issue: string | null;
    pages: string | null;
  };
  pmid: string | null;
  doi: string | null;
  read: 'full-text' | 'abstract' | 'secondary';
  n: string | null;
  label: string;
}

interface LayoutCell {
  coordinate: { key: string; group: string };
  source: string;
  usage?: string;
}

interface BenchRow {
  key: string;
  values: { source: string }[];
}

interface TableFile {
  cells?: LayoutCell[];
  rows?: BenchRow[];
  sources?: SourceRecord[];
}

function readJson(url: URL): unknown {
  return JSON.parse(readFileSync(url, 'utf8')) as unknown;
}

const catalogues = (['implant-layout', 'load-bench', 'spine-readings'] as const).map((crate) => {
  const folder = new URL(`../../../engine/${crate}/evidence/`, import.meta.url);
  const files = readdirSync(folder).filter((name) => name.endsWith('.json')).sort();
  const tables = files.filter((name) => name !== 'sources.json')
    .map((name) => ({ name: name.replace(/\.json$/, ''), data: readJson(new URL(name, folder)) as TableFile }));
  return {
    crate,
    // Some crates keep their source records inside the numerical evidence file.
    sources: files.includes('sources.json')
      ? (readJson(new URL('sources.json', folder)) as { sources: SourceRecord[] }).sources
      : tables.flatMap(({ data }) => data.sources ?? []),
    tables,
  };
});
const floor = readJson(
  new URL('../../../engine/spine-readings/evidence/curve-floor.json', import.meta.url),
) as { value: { value: number }; unit: string; source: string; sources: SourceRecord[] };
const placeEntries: readonly EvidenceEntry[] = EVIDENCE_LEDGER.filter((added) => added.id.startsWith('place-'));

function recordsFor(added: EvidenceEntry): SourceRecord[] {
  const crate = added.id.startsWith('place-bench-') ? 'load-bench'
    : added.id.startsWith('place-readings-') ? 'spine-readings' : 'implant-layout';
  const catalogue = catalogues.find((candidate) => candidate.crate === crate);
  if (catalogue === undefined) throw new Error(`no source catalogue for ${crate}`);
  return catalogue.sources;
}

function entry(id: string): EvidenceEntry {
  const found = placeEntries.find((candidate) => candidate.id === id);
  expect(found, id).toBeDefined();
  if (!found) throw new Error(`no placement ledger entry ${id}`);
  return found;
}

function matches(source: EvidenceSource, record: SourceRecord): boolean {
  return (source.key === undefined ? isPublication(record) : source.key === record.key)
    && source.pmid === record.pmid && source.doi === record.doi;
}

function isPublication(source: SourceRecord): boolean {
  return source.pmid !== null || source.doi !== null;
}

/** Includes layout definitions and sources nested in individual bench values. */
function sourceKeys(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap(sourceKeys);
  if (typeof value !== 'object' || value === null) return [];
  return Object.entries(value).flatMap(([key, child]) =>
    key === 'source' && typeof child === 'string' ? [child] : sourceKeys(child),
  );
}

function slug(value: string): string {
  return value.replace('predictive', 'test').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

describe('the Place evidence ledger', () => {
  it('keeps every addition unchecked and experimental', () => {
    expect(placeEntries.length).toBeGreaterThan(0);
    for (const added of placeEntries) {
      expect(added.id).toMatch(/^place-/);
      expect(added.checked, added.id).toBe(false);
      expect(added.status, added.id).toBe('experimental');
      expect(added.feature.trim(), added.id).not.toBe('');
      expect(added.publishedNumber.trim(), added.id).not.toBe('');
      expect(added.population.trim(), added.id).not.toBe('');
    }
  });

  it('gives every named level rule an entry', () => {
    const rules = {
      'end-plus-one': 'place-rule-end-plus-one',
      'suk-neutral': 'place-rule-suk-neutral',
      'last-touched': 'place-rule-last-touched',
      'stable-minus-one': 'place-rule-stable-minus-one',
      'substantially-touched': 'place-rule-substantially-touched',
      'lenke-l3-l4': 'place-rule-lenke-l3-l4',
      'lev-plus-one-5c': 'place-layout-level-rule-thresholds-bmc-level-review-lev-plus-one-5c',
      'liv-index': 'place-layout-level-rule-thresholds-chen-2025-liv-index',
      'deviated-vertebra': 'place-layout-level-rule-thresholds-liu-2020-deviated-vertebra',
      'upper-end-vertebra': 'place-rule-upper-end-vertebra',
      'uev-5c': 'place-layout-level-rule-thresholds-bmc-level-review-uev-5c',
      'lenke-shoulder': 'place-rule-lenke-shoulder',
      'selective-thoracic': 'place-layout-level-rule-thresholds-korean-level-review-selective-thoracic',
      'selective-lumbar': 'place-layout-level-rule-thresholds-korean-level-review-selective-lumbar',
    } satisfies Record<LevelRule, string>;
    for (const id of Object.values(rules)) expect(entry(id).sources.length, id).toBeGreaterThan(0);
  });

  it('gives every place entry a source key and retains its recorded reading and sample size', () => {
    for (const added of placeEntries) {
      const records = recordsFor(added);
      expect(added.sources.length, added.id).toBeGreaterThan(0);
      for (const source of added.sources) {
        expect(source.key, added.id).toBeDefined();
        const record = records.find((candidate) => candidate.key === source.key);
        expect(record, added.id).toBeDefined();
        expect(source.pmid, added.id).toBe(record?.pmid);
        expect(source.doi, added.id).toBe(record?.doi);
        expect(source.reading, added.id).toBe(record?.read);
      }
      const first = records.find((record) => record.key === added.sources[0]?.key);
      expect(added.sampleSize, added.id).toBe(first?.n ?? undefined);
    }
  });

  it('distinguishes the secondary Winter attribution from the direct Keenan model', () => {
    for (const id of ['head-neck', 'each-arm-hand']) {
      expect(entry(`place-bench-daily-loads-${id}`).sources.map((source) => source.key))
        .toEqual(['winter-via-keenan-2015']);
    }
    expect(entry('place-bench-daily-loads-tissue-density').sources.map((source) => source.key))
      .toEqual(['keenan-2015']);
  });

  for (const { crate, sources, tables } of catalogues) {
    it(`covers every ${crate} evidence source with an entry or a stated kind`, () => {
      expect(sources.length).toBeGreaterThan(0);
      const keys = new Set(sources.map((source) => source.key));
      const used = new Set(tables.flatMap(({ data }) => sourceKeys(data)));
      const entries = placeEntries.filter((added) => recordsFor(added) === sources);
      const covered = new Set(placeEntries.flatMap((added) => added.sources.map((source) => source.key)));
      const withoutEntry = new Set(Object.keys(PLACE_SOURCES_WITHOUT_ENTRY));
      expect(used.size).toBeGreaterThan(0);
      for (const key of used) {
        expect(keys.has(key), `${crate}: ${key}`).toBe(true);
        expect(covered.has(key) || withoutEntry.has(key), `${crate}: ${key}`).toBe(true);
      }
      // Registry-wide coverage also includes indirect and definition-only citations.
      for (const source of sources.filter(isPublication)) {
        expect(
          entries.some((added) => added.sources.some((candidate) => matches(candidate, source))),
          `${crate}: ${source.key}`,
        ).toBe(true);
      }
    });

    if (crate === 'spine-readings') continue;
    it(`keeps every published ${crate} table measurement or row tied to its own source`, () => {
      expect(tables.length).toBeGreaterThan(0);
      for (const { name, data } of tables) {
        for (const cell of data.cells ?? []) {
          const group = ['level-rule-thresholds', 'channel-grades', 'stop-short-odds', 'screw-use'].includes(name)
            ? cell.coordinate.group
            : cell.coordinate.key;
          const source = sources.find((record) => record.key === cell.source);
          expect(source, cell.source).toBeDefined();
          const id = `place-layout-${name}-${cell.source}-${slug(group)}`;
          if (source === undefined || !isPublication(source)) {
            expect(placeEntries.some((added) => added.id === id), id).toBe(false);
            continue;
          }
          const added = entry(id);
          expect(added.sources.some((candidate) => matches(candidate, source)), added.id).toBe(true);
          if (cell.usage === 'printed-only') expect(added.publishedNumber, added.id).toContain('excluded from use');
        }
        for (const row of data.rows ?? []) {
          const publications = sourceKeys(row)
            .flatMap((key) => sources.filter((source) => source.key === key && isPublication(source)));
          const id = `place-bench-${name}-${slug(row.key)}`;
          if (publications.length === 0) {
            expect(placeEntries.some((added) => added.id === id), id).toBe(false);
            continue;
          }
          const added = entry(id);
          for (const source of publications) {
            expect(added.sources.some((candidate) => matches(candidate, source)), `${added.id}: ${source.key}`).toBe(true);
          }
        }
      }
    });
  }

  it('takes every new citation and identifier from an embedded source record', () => {
    const records = catalogues.flatMap(({ sources }) => sources);
    for (const added of placeEntries) {
      expect(added.sources.length, added.id).toBeGreaterThan(0);
      for (const source of added.sources) {
        const record = records.find((candidate) => matches(source, candidate));
        expect(record, added.id).toBeDefined();
        if (record === undefined) continue;
        // Some references omit a title or abbreviate a collaboration's name.
        if (record.citation.authors !== null) {
          const firstAuthor = record.citation.authors.split(',')[0];
          expect(source.citation, added.id).toContain(firstAuthor);
        }
        for (const field of ['year', 'journal', 'volume', 'issue', 'pages'] as const) {
          const value = record.citation[field];
          if (value !== null) expect(source.citation, added.id).toContain(String(value));
        }
      }
    }
  });

  it('shows uncitable sources by their recorded kind and never hides a publication', () => {
    const records = catalogues.flatMap(({ sources }) => sources);
    const kindsByLabel: Readonly<Record<string, string>> = {
      'own-inference': 'project-assumption',
      guideline: 'guideline-page',
      'secondary-summary': 'summary-of-papers',
    };
    const uncitableKeys = new Set(records.filter((source) => !isPublication(source)).map((source) => source.key));
    expect(Object.keys(PLACE_SOURCES_WITHOUT_ENTRY).sort()).toEqual([...uncitableKeys].sort());
    for (const [key, kind] of Object.entries(PLACE_SOURCES_WITHOUT_ENTRY)) {
      const matchingRecords = records.filter((source) => source.key === key);
      expect(matchingRecords.length, key).toBeGreaterThan(0);
      for (const record of matchingRecords) {
        expect(record.pmid, key).toBeNull();
        expect(record.doi, key).toBeNull();
        expect(kind, key).toBe(kindsByLabel[record.label]);
      }
      expect(placeEntries.some((added) => added.sources.some((source) => source.key === key)), key).toBe(false);
    }
    expect(placeEntries.flatMap((added) => added.sources)
      .filter((source) => source.pmid === null && source.doi === null)).toEqual([]);
  });

  it('keeps the displayed curve floor separate from the placement rules', () => {
    const added = entry('place-readings-curve-floor');
    expect(floor.value.value).toBe(10);
    expect(added.publishedNumber).toContain('10 degrees');
    expect(added.publishedNumber).toContain('placement rules retain smaller curves');
    const source = floor.sources.find((record) => record.key === floor.source);
    expect(added.sources.some((candidate) => source !== undefined && matches(candidate, source))).toBe(true);
    expect(added.sources[0]?.key).toBe(floor.source);
  });

  it('keeps the stiffness reference assumptions beside the published force range', () => {
    const added = entry('place-bench-joint-springs-stiffness-scale');
    expect(added.sources.map((source) => source.pmid)).toEqual(['14977223', '23273729', '22321374']);
    expect(added.publishedNumber).toContain('reference rod diameter: 6 mm [own-inference; salmingo-2012]');
    expect(added.publishedNumber).toContain('three-patient report and is assumed for the six patients');
    expect(added.publishedNumber).toContain('largest screw force, 192 to 439 N across six patients');
    expect(added.publishedNumber).toContain('as if all of it came from stiffness');
    expect(added.publishedNumber).toContain('six patients are few');
    expect(added.publishedNumber).toContain('summed forces spread by a factor of 3.1');
    expect(added.publishedNumber).toContain('held rigid, before springback, so the rod chosen does not change them');
    expect(added.publishedNumber).toContain('published forces come from rods after they flexed');
    expect(added.publishedNumber).toContain('3.4 at scale 1, more than the published 2.29');
    expect(added.publishedNumber).toContain('can still pass 439 N');
    expect(added.publishedNumber).not.toMatch(/widest|per patient/);
    expect(added.population).toContain('Sample: 6');
    expect(added.population).toContain('Sample: 3');
  });

  it('keeps the corrected axis, strength and exclusion distinctions beside the copied numbers', () => {
    const springs = entry('place-bench-joint-springs-t1-2-to-l4-5-intact').publishedNumber;
    expect(springs).toContain('moment 1: 448 N·mm/deg');
    expect(springs).toContain('moment 2: 644 N·mm/deg');
    expect(springs).toContain('moment 3: 738 N·mm/deg');
    expect(springs).toContain('AP (x; lateral bending)');
    const strength = entry('place-bench-rod-materials-titanium-alloy-body-text').publishedNumber;
    for (const value of ['896', '1034', '965', '1103']) expect(strength).toContain(`${value} MPa`);
    expect(entry('place-layout-stop-short-odds-liu-2020-sv-minus-one').publishedNumber).toContain('2.6 ratio');
    expect(entry('place-layout-diameter-rules-sato-2022-outer-width-ratio-max').publishedNumber).toContain('1.25 ratio');
    const walls = entry('place-layout-inner-walls-chiu-2024-medial-wall-thickness').publishedNumber;
    expect(walls).toContain('chiu-right-t4-t10: 0.75 ± 0.23');
    expect(walls).toContain('chiu-left-t4-t7: 0.76 ± 0.24');
    const angles = entry('place-layout-medial-angles-upendra-2010-transverse-angle').publishedNumber;
    expect(angles).toContain('retained only, excluded from use');
  });

  it('leaves choices supported only by project assumptions out of the published ledger', () => {
    for (const id of ['place-model-first-rod-contour', 'place-model-target-joint-rotations', 'place-model-film-scale-and-tier', 'place-bench-fatigue-missing']) {
      expect(placeEntries.some((added) => added.id === id), id).toBe(false);
    }
  });
});

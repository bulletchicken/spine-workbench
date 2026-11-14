import { readdirSync, readFileSync } from 'node:fs';
import type { LevelRule } from '@/shapes';
import { describe, expect, it } from 'vitest';
import { EVIDENCE_LEDGER, type EvidenceEntry, type EvidenceSource } from './evidence-ledger';

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
}

function readJson(url: URL): unknown {
  return JSON.parse(readFileSync(url, 'utf8')) as unknown;
}

const catalogues = (['implant-layout', 'load-bench'] as const).map((crate) => {
  const folder = new URL(`../../../engine/${crate}/evidence/`, import.meta.url);
  return {
    crate,
    sources: (readJson(new URL('sources.json', folder)) as { sources: SourceRecord[] }).sources,
    tables: readdirSync(folder)
      .filter((name) => name.endsWith('.json') && name !== 'sources.json')
      .sort()
      .map((name) => ({ name: name.replace(/\.json$/, ''), data: readJson(new URL(name, folder)) as TableFile })),
  };
});
const floor = readJson(
  new URL('../../../engine/spine-readings/evidence/curve-floor.json', import.meta.url),
) as { value: { value: number }; unit: string; source: string; sources: SourceRecord[] };
const placeEntries: readonly EvidenceEntry[] = EVIDENCE_LEDGER.slice(2);

function entry(id: string): EvidenceEntry {
  const found = placeEntries.find((candidate) => candidate.id === id);
  expect(found, id).toBeDefined();
  if (!found) throw new Error(`no placement ledger entry ${id}`);
  return found;
}

function matches(source: EvidenceSource, record: SourceRecord): boolean {
  return isPublication(record) && source.pmid === record.pmid && source.doi === record.doi;
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

  for (const { crate, sources, tables } of catalogues) {
    it(`covers every ${crate} publication with its recorded identifiers`, () => {
      expect(sources.length).toBeGreaterThan(0);
      const keys = new Set(sources.map((source) => source.key));
      const used = new Set(tables.flatMap(({ data }) => sourceKeys(data)));
      expect(used.size).toBeGreaterThan(0);
      for (const key of used) expect(keys.has(key), `${crate}: ${key}`).toBe(true);
      // Registry-wide coverage also includes indirect and definition-only citations.
      for (const source of sources.filter(isPublication)) {
        expect(
          placeEntries.some((added) => added.sources.some((candidate) => matches(candidate, source))),
          `${crate}: ${source.key}`,
        ).toBe(true);
      }
    });

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
    const records = [...catalogues.flatMap(({ sources }) => sources), ...floor.sources];
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

  it('excludes the five non-publication source records', () => {
    const records = catalogues.flatMap(({ sources }) => sources).filter((source) => source.pmid === null && source.doi === null);
    expect(records.map((source) => source.key).sort()).toEqual([
      'ao-lenke', 'fusion-level-definitions', 'project-assumptions', 'screw-p1-hard-floor', 'screw-p4-summary',
    ]);
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

import { describe, expect, it } from 'vitest';
import { EVIDENCE_LEDGER, type EvidenceEntry, evidenceEntry } from './evidence-ledger';

const OUTCOME_WORDS: readonly RegExp[] = [
  /\bpredict/i,
  /\boutcomes?\b/i,
  /\bprognos/i,
  /\bsafe(ly|ty|r|st)?\b/i,
  /\boptimal/i,
  /\bvalidated\b/i,
  /\baccura(te|tely|cy)\b/i,
  /\bdiagnos/i,
  /\brecommend/i,
];

describe('the evidence ledger', () => {
  it('has unique ids, each kebab-case', () => {
    const ids = EVIDENCE_LEDGER.map((entry) => entry.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) expect(id).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/);
  });

  it('gives every entry a source with a PubMed id or a DOI', () => {
    for (const entry of EVIDENCE_LEDGER) {
      expect(entry.sources.length, entry.id).toBeGreaterThan(0);
      for (const source of entry.sources) {
        expect(source.pmid !== null || source.doi !== null, entry.id).toBe(true);
        if (source.pmid !== null) expect(source.pmid, entry.id).toMatch(/^\d+$/);
        if (source.doi !== null) expect(source.doi, entry.id).toMatch(/^10\.\d{4,9}\/\S+$/);
      }
    }
  });

  it('resolves every id to its own entry, and refuses an id that is not in the ledger', () => {
    for (const entry of EVIDENCE_LEDGER) {
      expect(evidenceEntry(entry.id)).toBe(entry);
    }
    // @ts-expect-error: an id outside the ledger must not compile
    expect(() => evidenceEntry('not-in-the-ledger')).toThrow('no ledger entry not-in-the-ledger');
  });

  it('hands out entries that cannot be changed through the lookup', () => {
    const relabel = (): void => {
      // @ts-expect-error: a ledger entry is read-only
      evidenceEntry('side-tags-behind-confirm-left-right').status = 'supported';
    };
    expect(relabel).toBeTypeOf('function');
    expect(evidenceEntry('side-tags-behind-confirm-left-right').status).toBe('experimental');
  });

  it('marks an entry supported only when its source was read and checked', () => {
    const entries: readonly EvidenceEntry[] = EVIDENCE_LEDGER;
    for (const entry of entries) {
      if (entry.status === 'supported') {
        expect(entry.checked, entry.id).toBe(true);
        expect(entry.sources.length, entry.id).toBeGreaterThan(0);
      }
      if (!entry.checked) expect(entry.status, entry.id).toBe('experimental');
    }
  });

  it('never promises an outcome in what a feature does or what the number says', () => {
    for (const entry of EVIDENCE_LEDGER) {
      const text = `${entry.feature} ${entry.publishedNumber} ${entry.population}`;
      for (const pattern of OUTCOME_WORDS) expect(text, entry.id).not.toMatch(pattern);
    }
  });

  it('gives the transitional-anatomy percentages that follow from the counts', () => {
    const entry = evidenceEntry('transitional-anatomy-behind-confirm-l5');
    expect(Math.round((22 / 147) * 1000) / 10).toBe(15);
    expect(Math.round((12 / 147) * 1000) / 10).toBe(8.2);
    expect(entry.publishedNumber).toContain('22 of 147');
    expect(entry.publishedNumber).toContain('15.0%');
    expect(entry.publishedNumber).toContain('8.2%');
    expect(entry.population).toContain('n = 147');
  });

  it('keeps the side-tag entry experimental, because its paper studied limb films and not the app', () => {
    const entry = evidenceEntry('side-tags-behind-confirm-left-right');
    expect(entry.checked).toBe(true);
    expect(entry.status).toBe('experimental');
    expect(entry.population).toContain('limb');
    expect(entry.population).toContain('no spine films');
  });

  it('gives the side-tag counts as the paper states them', () => {
    const entry = evidenceEntry('side-tags-behind-confirm-left-right');
    expect(entry.publishedNumber).toContain('694');
    expect(entry.publishedNumber).toContain('4,619');
    expect(entry.publishedNumber).toContain('67 images from 23 exams');
    expect(entry.publishedNumber).toContain('237 images (1.5%)');
    expect(Math.round((694 / 4619) * 100)).toBe(15);
    expect(Math.round((237 / 15405) * 1000) / 10).toBe(1.5);
  });
});


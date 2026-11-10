// Every sentence the user reads lives in words/. This test reads every
// string in those files and fails on any claim about outcomes, accuracy or
// safety. The approved preview notice is the one place such words appear,
// and only to deny them.

import ts from 'typescript';
import { describe, expect, it } from 'vitest';
import { PREVIEW_NOTICE } from './preview-notice';

const CLAIMS: readonly RegExp[] = [
  /\bdiagnos/i,
  /\btreat(s|ed|ing|ment|ments)?\b/i,
  /\bcur(e|es|ed|ing)\b/i,
  /\boutcomes?\b/i,
  /\bpredict/i,
  /\brecommend/i,
  /\bprognos/i,
  /\baccura(te|tely|cy)\b/i,
  /\bprecis(e|ely|ion)\b/i,
  /\bperfect(ly)?\b/i,
  /\breliabl/i,
  /\bguarantee/i,
  /\bsafe(ly|ty|r|st)?\b/i,
  /\brisk-free\b/i,
  /\bclinical(ly)?\b/i,
  /\bmedical device\b/i,
  /\b(validated|proven|approved|certified)\b/i,
  /\b(FDA|Health Canada|CE mark)/i,
  /\bsurgical (plan|decision)/i,
  /\b100\s?%/,
  /\berror[- ]free\b/i,
  /\b(never|cannot be) wrong\b/i,
];

const APPROVED = new Set<string>(Object.values(PREVIEW_NOTICE));

/** Every string literal and template text in a TypeScript source. */
function stringsIn(source: string): string[] {
  const file = ts.createSourceFile('words.ts', source, ts.ScriptTarget.Latest, false, ts.ScriptKind.TS);
  const found: string[] = [];
  const visit = (node: ts.Node) => {
    if (ts.isStringLiteralLike(node) || ts.isTemplateLiteralToken(node)) found.push(node.text);
    ts.forEachChild(node, visit);
  };
  visit(file);
  return found;
}

/** The claims in a words file, each as `pattern: "sentence"`. */
function claimsIn(source: string): string[] {
  return stringsIn(source)
    .filter((text) => !APPROVED.has(text))
    .flatMap((text) => CLAIMS.filter((claim) => claim.test(text)).map((claim) => `${String(claim)}: "${text}"`));
}

const wordsFiles = import.meta.glob<string>(['./*.ts', '!./*.test.ts'], { query: '?raw', import: 'default', eager: true });

describe('the words never claim', () => {
  it('reads every words file', () => {
    expect(Object.keys(wordsFiles)).toContain('./faults.ts');
    expect(Object.keys(wordsFiles)).toContain('./preview-notice.ts');
  });

  for (const [path, source] of Object.entries(wordsFiles)) {
    it(`finds no outcome or safety claim in ${path}`, () => {
      expect(claimsIn(source)).toEqual([]);
    });
  }

  it('catches a claim in a string, and ignores one in a comment', () => {
    expect(claimsIn("// accurate to the millimetre\nexport const a = 'Clinically accurate angles';")).toHaveLength(2);
    expect(claimsIn('export const t = `Safe to ${"use"}`;')).toHaveLength(1);
    expect(claimsIn("// precise\nexport const ok = 'Corners placed';")).toEqual([]);
  });

  it('allows the approved notice word for word, and nothing near it', () => {
    expect(claimsIn(`export const n = '${PREVIEW_NOTICE.short}';`)).toEqual([]);
    expect(claimsIn("export const n = 'A preview, and a medical device.';")).not.toEqual([]);
  });
});

// base.css is the one stylesheet the app imports, and the order of what it
// pulls in matters: tokens before everything that reads them, the design
// system's parts with their own reduced-motion answers, and
// the four shared keyframes present by name for every component's CSS. The
// page, its reset and the focus ring are the design system's (components.css),
// so base.css says none of them again.

import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const css = readFileSync(fileURLToPath(new URL('./base.css', import.meta.url)), 'utf8');
const imports = [...css.matchAll(/@import\s+'([^']+)';/g)].map(([, path = '']) => path);

describe('base.css', () => {
  it('imports the shared stylesheets in the agreed order', () => {
    expect(imports).toEqual([
      './tokens.css',
      './earlier-tokens.css',
      './named-timings.css',
      './fonts.css',
      './components.css',
      './confidence.css',
      '../choreography/breathe.css',
      '../choreography/settle.css',
      '../choreography/roll.css',
      '../choreography/flash.css',
    ]);
  });

  it('imports only files that exist', () => {
    for (const path of imports) expect(existsSync(fileURLToPath(new URL(path, import.meta.url))), path).toBe(true);
  });

  it('puts every import before the first rule, where CSS still honours it', () => {
    const lastImport = css.lastIndexOf('@import');
    const firstRule = css.search(/^[^@/\s][^{]*\{/m);
    expect(firstRule).toBeGreaterThan(lastImport);
  });

  it('defines each shared animation once, under the name components use', () => {
    for (const name of ['breathe', 'settle', 'roll', 'flash']) {
      const file = readFileSync(fileURLToPath(new URL(`../choreography/${name}.css`, import.meta.url)), 'utf8');
      expect(file.match(new RegExp(`@keyframes ${name}\\b`, 'g')), name).toHaveLength(1);
    }
  });

  it('leaves the page and the focus ring to the design system: no body rule, no focus ring of its own', () => {
    const rules = css.replace(/\/\*[\s\S]*?\*\//g, '').replace(/@import[^;]*;/g, '');
    const selectors = [...rules.matchAll(/([^{}]+)\{[^{}]*\}/g)].flatMap(([, selector = '']) => selector.split(',').map((each) => each.trim()));
    expect(selectors.filter((selector) => /^(html|body)\b/.test(selector))).toEqual([]);
    expect(selectors.filter((selector) => selector.includes(':focus-visible') && !selector.includes(':not(:focus-visible)'))).toEqual([]);
  });
});

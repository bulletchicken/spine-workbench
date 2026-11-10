// load-engine.ts is the only file in the app allowed to make a network
// call. These tests hold the lint rule to that: it bites on every network
// API in app code, it exempts exactly one file, and the exemption lets
// that file call `fetch` and nothing else.

import { ESLint, type Linter } from 'eslint';
import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';
import { describe, expect, it } from 'vitest';

const REPOSITORY = fileURLToPath(new URL('../../..', import.meta.url));
const APP_SOURCES = join(REPOSITORY, 'app/src');
const LOADER = 'app/src/engine-room/load-engine.ts';
const NETWORK_RULES = ['no-restricted-globals', 'no-restricted-syntax', 'no-eval', 'no-new-func'] as const;
const WORKER = 'app/src/engine-room/workers/boot.ts';

const eslint = new ESLint({ cwd: REPOSITORY });

function sourceFiles(folder: string): string[] {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) return entry.name === 'wasm-build' || entry.name === 'generated' ? [] : sourceFiles(path);
    return /\.tsx?$/.test(entry.name) ? [path] : [];
  });
}

async function networkRules(path: string): Promise<string> {
  const config = (await eslint.calculateConfigForFile(path)) as Linter.Config;
  return JSON.stringify(NETWORK_RULES.map((rule) => config.rules?.[rule]));
}

/** The network messages ESLint reports for `code` placed at `path`. */
async function networkMessages(code: string, path: string): Promise<string[]> {
  const [result] = await eslint.lintText(code, { filePath: join(REPOSITORY, path) });
  const fatal = result?.messages.find((message) => message.fatal === true);
  if (result === undefined || fatal !== undefined) throw new Error(`ESLint could not lint the probe: ${fatal?.message ?? 'no result'}`);
  return result.messages
    .filter((message) => message.ruleId !== null && (NETWORK_RULES as readonly string[]).includes(message.ruleId))
    .map((message) => message.message);
}

describe('the network ban', () => {
  it('lists exactly one exempt file, the loader', () => {
    const config = ts.createSourceFile('eslint.config.js', readFileSync(join(REPOSITORY, 'eslint.config.js'), 'utf8'), ts.ScriptTarget.Latest);
    const lists: string[][] = [];
    const visit = (node: ts.Node) => {
      if (ts.isVariableDeclaration(node) && node.name.getText(config) === 'NETWORK_EXEMPT' && node.initializer !== undefined) {
        const list = node.initializer;
        lists.push(ts.isArrayLiteralExpression(list) ? list.elements.map((element) => element.getText(config)) : ['not a list']);
      }
      ts.forEachChild(node, visit);
    };
    visit(config);
    expect(lists).toEqual([[`'${LOADER}'`]]);
  });

  it('applies to every app source file alike, except the loader', async () => {
    const files = sourceFiles(APP_SOURCES);
    expect(files.length).toBeGreaterThan(10);
    const usual = await networkRules(join(REPOSITORY, 'app/src/main.tsx'));
    const different: string[] = [];
    for (const file of files) {
      if ((await networkRules(file)) !== usual) different.push(relative(REPOSITORY, file));
    }
    expect(different).toEqual([LOADER]);
  });

  it.each([
    ["fetch('/x');", 'the fetch API'],
    ["globalThis.fetch('/x');", 'fetch as a property'],
    ["globalThis['fetch']('/x');", 'fetch as a computed property'],
    ["Reflect.get(globalThis, 'fetch');", 'fetch named in a string'],
    ['new XMLHttpRequest();', 'the XMLHttpRequest API'],
    ["new WebSocket('wss://example.org');", 'the WebSocket API'],
    ["new EventSource('/events');", 'the EventSource API'],
    ["new WebTransport('https://example.org');", 'the WebTransport API'],
    ['new RTCPeerConnection();', 'the RTCPeerConnection API'],
    ["new Request('/x');", 'the Request API'],
    ["navigator.sendBeacon('/x', '');", 'the sendBeacon API'],
    ["void navigator.serviceWorker.register('/sw.js');", 'the serviceWorker API'],
    ["importScripts('/x.js');", 'the importScripts API'],
    ["void import('https://example.org/x.js');", 'import() of a URL'],
    ['const where = "/x"; void import(where);', 'import() of a computed path'],
    ["const { fetch: go } = globalThis; void go('/x');", 'fetch taken out by destructuring'],
    ["const { sendBeacon } = navigator; sendBeacon('/x');", 'sendBeacon taken out by destructuring'],
    ["const go = globalThis['fe' + 'tch'] as unknown;", 'fetch named by a string built at run time'],
    ["const g = globalThis; const go = g['fe' + 'tch'] as unknown;", 'fetch through an alias of the global object'],
    ["const go = Reflect.get(globalThis, 'fe' + 'tch') as unknown;", 'fetch through Reflect with a built name'],
    ['const name = "x"; const got = self[name] as unknown;', 'a global property named by a variable'],
    ["eval('1');", 'direct eval'],
    ["(0, eval)('1');", 'indirect eval'],
    ["const run = new Function('return 1');", 'the Function constructor'],
    ["open('https://example.org');", 'opening a window'],
    ["window.open('https://example.org');", 'opening a window through window'],
    ["location.href = 'https://example.org';", 'navigating by assigning to location.href'],
    ["window.location = 'https://example.org' as unknown as Location;", 'navigating by assigning to window.location'],
    ["window.location.href = 'https://example.org';", 'navigating by assigning to window.location.href'],
    ["document.location.href = 'https://example.org';", 'navigating by assigning to document.location.href'],
    ["location.assign('https://example.org');", 'navigating with location.assign'],
  ])('refuses %s (%s) in app code and in worker code', async (code) => {
    for (const path of ['app/src/main.tsx', WORKER]) {
      expect(await networkMessages(`${code}\nexport {};\n`, path), path).not.toEqual([]);
    }
  });

  it('lets app code read where it was loaded from', async () => {
    expect(await networkMessages('const here = location.origin;\nexport { here };\n', 'app/src/main.tsx')).toEqual([]);
  });

  it('allows a relative import()', async () => {
    expect(await networkMessages("void import('./messages');\nexport {};\n", 'app/src/engine-room/engine-client.ts')).toEqual([]);
  });

  it('lets the loader call fetch, and nothing else', async () => {
    expect(await networkMessages("void fetch('/x');\nexport {};\n", LOADER)).toEqual([]);
    expect(await networkMessages("new WebSocket('wss://example.org');\nexport {};\n", LOADER)).not.toEqual([]);
    expect(await networkMessages("navigator.sendBeacon('/x', '');\nexport {};\n", LOADER)).not.toEqual([]);
  });
}, 120_000);

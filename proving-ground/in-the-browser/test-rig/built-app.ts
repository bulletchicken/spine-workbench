// The production build the browser tests run against: app/dist, as
// written by `pnpm build`.

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

export const DIST = fileURLToPath(new URL('../../../app/dist', import.meta.url));

/** Every file in the build, as paths relative to dist/ with forward slashes. */
export function builtFiles(): string[] {
  if (!existsSync(join(DIST, 'index.html'))) {
    throw new Error(`there is no production build in ${DIST}; run \`pnpm build\` first`);
  }
  const walk = (folder: string): string[] =>
    readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
      const path = join(folder, entry.name);
      return entry.isDirectory() ? walk(path) : [relative(DIST, path).split(sep).join('/')];
    });
  return walk(DIST);
}

/** Every URL path the preview server can answer with a file from the build. */
export function servedPaths(): Set<string> {
  return new Set(['/', ...builtFiles().map((file) => `/${file}`)]);
}

/** A built file's bytes, read as Latin-1 so binary files scan as text too. */
export function builtText(file: string): string {
  return readFileSync(join(DIST, file), 'latin1');
}

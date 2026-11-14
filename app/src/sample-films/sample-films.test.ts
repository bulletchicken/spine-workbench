import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build, type ConfigEnv } from 'vite';
import { describe, expect, it } from 'vitest';
import appConfig from '../../vite.config';
import { sampleFilmFiles, sampleFilmFilesFor, sampleFilmFilesFrom } from './sample-films';

const configFor = (command: ConfigEnv['command'], mode: string, isPreview = false) => {
  if (typeof appConfig !== 'function') throw new Error('the build needs its command and mode');
  return appConfig({ command, mode, isPreview });
};

describe('sample films kept on this machine', () => {
  it('decodes fresh byte buffers, orders numbers, caps at six and gives out no path', () => {
    const files = sampleFilmFilesFrom({ './sample-film-10.jpg': 'data:image/jpeg;base64,AwQ=', './sample-film-2.png': 'data:image/png;base64,AAEC' });
    expect(files.map((film) => [...new Uint8Array(film.bytes())])).toEqual([[0, 1, 2], [3, 4]]);
    expect(files.map(Object.keys)).toEqual([['bytes'], ['bytes']]);
    expect(files[0]?.bytes()).not.toBe(files[0]?.bytes());
    expect(sampleFilmFilesFrom({})).toEqual([]);
    expect(sampleFilmFilesFrom(Object.fromEntries(Array.from({ length: 9 }, (_, n) => [String(n), 'data:image/png;base64,AA=='])))).toHaveLength(6);
  });
  it('offers no local film to the stand-in, and the found list to the real engine', async () => {
    const info = (engineVersion: string) => () => Promise.resolve({ engineVersion, shapesFingerprint: '', caseFormat: 'spine-case', caseVersion: 1, trainedSideFinder: false });
    expect(await sampleFilmFilesFor({ info: info('stand-in') })).toEqual([]);
    expect(await sampleFilmFilesFor({ info: info('0.1.0') })).toBe(sampleFilmFiles());
  });
  it('turns the switch off for every ordinary build, and keeps the local output separate', () => {
    for (const mode of ['production', 'development', 'test']) {
      const config = configFor('build', mode);
      expect(config.define?.['import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE']).toBe('false');
      expect(config.build?.outDir).toBe('dist');
    }
    expect((configFor('serve', 'development')).define?.['import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE']).toBe('true');
    const deliberate = configFor('build', 'films-on-this-machine');
    expect(deliberate.define?.['import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE']).toBe('true');
    expect(deliberate.build?.outDir).toBe('films-on-this-machine/dist');
    expect((configFor('serve', 'films-on-this-machine', true)).build?.outDir).toBe('films-on-this-machine/dist');
  });
  it('leaves a made-up PNG out of production code even when the folder holds it', async () => {
    const folder = await mkdtemp(join(tmpdir(), 'sample-films-'));
    const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=';
    try {
      await mkdir(join(folder, 'on-this-machine'));
      await writeFile(join(folder, 'on-this-machine/sample-film-1.png'), Buffer.from(png, 'base64'));
      await writeFile(join(folder, 'sample-films.ts'), await readFile(new URL('./sample-films.ts', import.meta.url), 'utf8'));
      for (const mode of ['production', 'films-on-this-machine']) {
        const config = configFor('build', mode);
        const result = await build({ configFile: false, root: folder, logLevel: 'silent', define: config.define ?? {},
          build: { write: false, minify: false, lib: { entry: join(folder, 'sample-films.ts'), formats: ['es'] } } });
        const output = (Array.isArray(result) ? result : [result]).flatMap((built) => 'output' in built ? built.output : []);
        const code = output.flatMap((file) => file.type === 'chunk' ? [file.code] : []).join('\n');
        expect(code.includes(png)).toBe(mode === 'films-on-this-machine');
        expect(output.filter((file) => /\.(png|jpe?g)$/.test(file.fileName))).toEqual([]);
      }
    } finally { await rm(folder, { recursive: true, force: true }); }
  });
});

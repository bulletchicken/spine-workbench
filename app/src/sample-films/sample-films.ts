import type { EngineClient } from '@/engine-room/engine-client';

export interface SampleFilmFile { bytes(): ArrayBuffer }

export function sampleFilmFilesFrom(found: Readonly<Record<string, string>>): SampleFilmFile[] {
  return Object.keys(found).sort((a, b) => a.localeCompare(b, 'en', { numeric: true })).slice(0, 6).map((path) => ({
    bytes() {
      const address = found[path] ?? '';
      const text = atob(address.slice(address.indexOf(',') + 1));
      return Uint8Array.from(text, (letter) => letter.charCodeAt(0)).buffer;
    },
  }));
}

// The build replaces the switch before unused inline films are removed.
const found = import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE
  ? import.meta.glob<string>('./on-this-machine/*.{jpg,jpeg,png}', { query: '?inline', import: 'default', eager: true })
  : {};
const files = sampleFilmFilesFrom(found);
export function sampleFilmFiles(): SampleFilmFile[] { return files; }

/**
 * The sample films the engine in use can open. In development the engine
 * may be the one that answers from a story, which opens its own story's
 * films only, so it is offered none. That check is dropped from a
 * production build, which always runs the real engine.
 */
export async function sampleFilmFilesFor(engine: Pick<EngineClient, 'info'>): Promise<SampleFilmFile[]> {
  if (import.meta.env.DEV && (await engine.info()).engineVersion === 'stand-in') return [];
  return sampleFilmFiles();
}

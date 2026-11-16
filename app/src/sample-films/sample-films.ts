import type { FilmKind } from '@/shapes';
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

export interface SampleCase {
  films: Partial<Record<FilmKind, SampleFilmFile>> & { front: SampleFilmFile };
  sideFaces?: 'left' | 'right';
  credit?: string;
}

/** Explicit places keep a missing side film from moving a bending film into its slot. */
export function sampleCaseFrom(found: Readonly<Record<string, string>>, metadata: unknown = {}): SampleCase | null {
  const films: Partial<Record<FilmKind, SampleFilmFile>> = {};
  for (const kind of ['front', 'side', 'bend-left', 'bend-right'] as const) {
    const path = Object.keys(found).sort().find((p) => new RegExp(`/${kind}\\.(png|jpe?g)$`).test(p));
    if (path !== undefined) {
      const file = sampleFilmFilesFrom({ [path]: found[path] ?? '' })[0];
      if (file !== undefined) films[kind] = file;
    }
  }
  if (films.front === undefined) return null;
  const fields = typeof metadata === 'object' && metadata !== null ? metadata as Record<string, unknown> : {};
  return { films: { ...films, front: films.front },
    ...(fields.sideFaces === 'left' || fields.sideFaces === 'right' ? { sideFaces: fields.sideFaces } : {}),
    ...(typeof fields.credit === 'string' && fields.credit.length > 0 && !/[\r\n]/.test(fields.credit) ? { credit: fields.credit } : {}) };
}
const caseFilms = import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE
  ? import.meta.glob<string>('./on-this-machine/case/{front,side,bend-left,bend-right}.{jpg,jpeg,png}', { query: '?inline', import: 'default', eager: true }) : {};
const caseMetadata = import.meta.env.SAMPLE_FILMS_ON_THIS_MACHINE
  ? import.meta.glob<unknown>('./on-this-machine/case/case.json', { import: 'default', eager: true }) : {};
const localCase = sampleCaseFrom(caseFilms, Object.values(caseMetadata)[0]);
/** Whether this machine holds real films for the sample control: numbered front films or a case. */
export function hasRealSamples(): boolean { return files.length > 0 || localCase !== null; }
export async function sampleCaseFor(engine: Pick<EngineClient, 'info'>): Promise<SampleCase | null> {
  if (import.meta.env.DEV && (await engine.info()).engineVersion === 'stand-in') return null;
  return localCase;
}

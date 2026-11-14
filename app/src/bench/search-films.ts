import { useEffect, useState } from 'react';
import { useStore } from 'zustand';
import { useCase } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import type { EngineClient } from '@/engine-room/engine-client';
import { useEngine } from '@/engine-room/use-engine';
import type { CaseFileV1, EngineInfo, Film, FilmKind, FrontFinder, SideFinder } from '@/shapes';
import { filmOrigins, type FilmOrigins } from '@/view-state/film-origins';
import { SIDE_WORDS as W } from '@/words/side';

export type SideExclusion = 'facing' | 'unavailable' | 'waiting' | 'fault' | 'origin';
export type FinderCapability = EngineInfo | 'waiting' | 'fault';
export const exclusionWords = (reason: SideExclusion): string => ({ facing: W.chooseFacing, unavailable: W.unavailable, waiting: W.waiting, fault: W.fault, origin: W.origin })[reason];

/** No film is eligible by inference from another film's source. */
export function searchFilms(c: CaseFileV1, origins: FilmOrigins, openings: Readonly<Record<FilmKind, number>>, info: FinderCapability): { films: Film[]; frontFinder: FrontFinder; sideFinder: SideFinder; sideExclusion: SideExclusion | null; canFind: boolean } {
  let sideExclusion: SideExclusion | null = null;
  const route = (kind: FilmKind) => origins[kind]?.opening === openings[kind] ? origins[kind].from : undefined;
  const frontFinder: FrontFinder = route('front') === 'made-up-case' ? 'hand-built' : 'trained';
  const sideFinder: SideFinder = route('side') === 'made-up-case' ? 'hand-built' : 'trained';
  const films = c.films.filter((film) => {
    if (film.kind !== 'side') return film.kind !== 'front' || route('front') !== undefined;
    if (route('side') === undefined) sideExclusion = 'origin';
    else if (sideFinder === 'hand-built') return true;
    else if (!film.geometry.confirmed.facing || !['anterior', 'posterior'].includes(film.geometry.imageRight ?? '')) sideExclusion = 'facing';
    else if (info === 'waiting' || info === 'fault') sideExclusion = info;
    else if (!info.trainedSideFinder) sideExclusion = 'unavailable';
    return sideExclusion === null;
  });
  return { films, frontFinder, sideFinder, sideExclusion, canFind: films.some((f) => f.kind === 'front' || f.kind === 'side') };
}

export async function finderCapability(engine: Pick<EngineClient, 'info'>): Promise<FinderCapability> {
  try { return await engine.info(); } catch { return 'fault'; }
}

export function useSearchFilms() {
  const engine = useEngine();
  const c = useCase((s) => s.case);
  const origins = useStore(filmOrigins, (s) => s.films);
  const openings = useStore(filmPixels, (s) => s.openings);
  const [info, setInfo] = useState<FinderCapability>('waiting');
  useEffect(() => {
    let here = true;
    void finderCapability(engine).then((value) => { if (here) setInfo(value); });
    return () => { here = false; };
  }, [engine]);
  return searchFilms(c, origins, openings, info);
}

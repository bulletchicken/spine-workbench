import { useMemo, type CSSProperties, type ReactNode } from 'react';
import type { FilmKind } from '@/shapes';
import { useLayer } from '@/film-viewer/layer-slot';
import type { FilmLayerProps } from '@/film-viewer/film-space';
import { MarksOverlay } from '@/marks/MarksOverlay';
import { TravellingLight, type ReplaySignal } from '@/marks/TravellingLight';
import { THREE_WORDS } from '@/words/three';
import { MARK_WORDS } from '@/words/marks';
import { HungFilm } from './HungFilm';
import type { StageFilm } from './Stage';
import type { Spot } from './stage-layout';
import './FilmTile.css';

interface FilmTileProps {
  controls?: ReactNode;
  onPress?: () => void;
  selected?: boolean;
  kind: FilmKind;
  spot: Spot;
  index: number;
  film: StageFilm;
  replay: ReplaySignal | null;
  glide?: boolean;
  target: boolean;
  onPoint?: (key: string | null) => void;
}

function Light({ light, replay, width, ...given }: { light: StageFilm['light']; replay: ReplaySignal | null; width: number } & FilmLayerProps) {
  return light === null || replay === null ? null : <TravellingLight {...given} {...light} width={width} signal={replay} />;
}

export function FilmTile({ kind, spot, index, film, replay, target, onPoint, glide = true, onPress, selected = false, controls }: FilmTileProps) {
  const light = useLayer('light', Light, { light: film.light, replay, width: film.size.width });
  const outlines = useLayer('outlines', MarksOverlay, { label: MARK_WORDS.outlinesOn(kind), marks: film.outlines, on: film.on, only: film.only, ...(onPoint === undefined ? {} : { onPoint }) });
  const layers = useMemo(() => [light, outlines], [light, outlines]);
  return (
    <div className="tile in" role={onPress === undefined ? undefined : 'button'} tabIndex={onPress === undefined ? undefined : 0} aria-label={onPress === undefined ? undefined : THREE_WORDS.railFilm(kind, selected)} onClick={onPress} onKeyDown={onPress === undefined ? undefined : (event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onPress(); } }} data-place={kind} data-names={film.names ? 'on' : 'off'} data-drop-target={target || undefined}
      style={{ '--i': index, '--bw': spot.width, '--bh': spot.height, '--x': spot.x, '--y': spot.y, '--s': 1, '--r': `var(--r-${spot.radius})` } as CSSProperties}>
      <div className="film"><HungFilm kind={kind} size={film.size} pixels={film.pixels} thumbnail={film.thumbnail} layers={layers} frame={film.frame} glide={glide} view={film.view} /></div>
      {controls}
    </div>
  );
}

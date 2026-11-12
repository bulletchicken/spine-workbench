import type { CSSProperties } from 'react';
import type { FilmKind } from '@/shapes';
import { PlaceGlyph } from '@/look-and-feel/figures/PlaceGlyph';
import { STAGE_WORDS } from '@/words/stage';
import { CAPTION, FILM_ORDER, type Spot } from './stage-layout';

interface FilmPlacesProps {
  spots: Record<FilmKind, Spot>;
  faces: Record<FilmKind, 'empty' | 'reading' | 'filled'>;
  target: FilmKind | null;
  onPress?: (kind: FilmKind) => void;
}

export function FilmPlaces({ spots, faces, target, onPress }: FilmPlacesProps) {
  const first = spots.front;
  const last = spots['bend-right'];
  const reading = FILM_ORDER.some((kind) => faces[kind] === 'reading');
  const style = { '--zx': first.x, '--zy': first.y, '--zw': last.x + last.width - first.x, '--zh': first.height + CAPTION, '--tw': first.width, '--th': first.height } as CSSProperties;
  return (
    <div className="dropzone" style={style}>
      {FILM_ORDER.map((kind, index) => (
        <button
          key={kind}
          type="button"
          className={faces[kind] === 'filled' ? 'place filled' : 'place'}
          data-place={kind}
          data-reading={faces[kind] === 'reading' || undefined}
          data-drop-target={target === kind || undefined}
          aria-busy={faces[kind] === 'reading' || undefined}
          aria-label={faces[kind] === 'reading' ? STAGE_WORDS.reading(kind) : faces[kind] === 'filled' ? STAGE_WORDS.replace(kind) : STAGE_WORDS.choose(kind)}
          disabled={reading || onPress === undefined}
          style={{ '--i': index, '--px': spots[kind].x - first.x } as CSSProperties}
          onClick={() => onPress?.(kind)}
        >
          <PlaceGlyph kind={kind} />
        </button>
      ))}
    </div>
  );
}

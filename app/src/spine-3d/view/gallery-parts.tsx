// What the 3D view's gallery pages share: the made-up case with its bones
// and films, a room to show a spine in, and a dial for a value from 0 to 1.

import { useEffect, useState, type ReactNode } from 'react';
import { Slider } from '@/building-blocks/Slider';
import { loadBones } from '../bones/load-bones';
import type { Bone } from '../bones/read-bones';
import { MADE_UP_CASE } from '../made-up-case/made-up-case';
import { type MadeUpFilms, madeUpFilms } from '../made-up-case/made-up-films';
import type { DrawnSpine } from '../scene/drawn-spine';
import type { Stage } from '../scene/stage';
import './gallery-parts.css';

export const CASE = MADE_UP_CASE;

/** The stage and spine a `Spine3D` hands out once it is ready. */
export interface Parts {
  stage: Stage;
  spine: DrawnSpine;
}

/** The bone shapes, once they have arrived. */
export function useBones(): readonly Bone[] | null {
  const [bones, setBones] = useState<readonly Bone[] | null>(null);
  useEffect(() => {
    let wanted = true;
    void loadBones().then((loaded) => {
      if (wanted) setBones(loaded);
    });
    return () => {
      wanted = false;
    };
  }, []);
  return bones;
}

/** The made-up case's two films, once they are decoded. */
export function useFilms(): MadeUpFilms | null {
  const [films, setFilms] = useState<MadeUpFilms | null>(null);
  useEffect(() => {
    let wanted = true;
    void madeUpFilms().then((decoded) => {
      if (wanted) setFilms(decoded);
    });
    return () => {
      wanted = false;
    };
  }, []);
  return films;
}

interface RoomProps {
  /** `tall` for one spine, `wide` for a spine with films beside it, `small` for one of a row. */
  size?: 'tall' | 'wide' | 'small';
  /** Anything laid over the canvas, in the same box. */
  over?: ReactNode;
  /** Dials and notes, under the room. */
  under?: ReactNode;
  children: ReactNode;
}

/** A quiet room with the page's soft glow, as the spine has behind it in the app. */
export function Room({ size = 'tall', over, under, children }: RoomProps) {
  return (
    <figure className="spine-room" data-size={size}>
      <div className="spine-room-box">
        <div className="spine-room-glow" />
        {children}
        {over}
      </div>
      {under !== undefined && <figcaption className="spine-room-under">{under}</figcaption>}
    </figure>
  );
}

const WHOLE = [{ from: 0, to: 1 }] as const;

/** A value from 0 to 1 on a slider, with its name and its number. */
export function Dial({ name, value, onChange }: { name: string; value: number; onChange: (value: number) => void }) {
  return (
    <label className="spine-dial">
      <span className="spine-dial-name">{name}</span>
      <Slider label={name} segments={WHOLE} value={value} step={0.02} valueText={(now) => `${String(Math.round(now * 100))} in 100`} onChange={onChange} />
      <span className="spine-dial-value">{value.toFixed(2)}</span>
    </label>
  );
}

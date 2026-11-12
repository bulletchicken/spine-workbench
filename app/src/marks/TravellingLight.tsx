import { useId, useLayoutEffect, useRef } from 'react';
import { numberOf } from '@/choreography/timing';
import type { FilmLayerProps } from '@/film-viewer/film-space';
import './marks.css';

/** Where the light is on its journey, and word whenever that changes. */
export interface ReplaySignal {
  /** The share of the journey, 0 to 1; `null` before it starts and after it ends. */
  share(): number | null;
  subscribe(listener: () => void): () => void;
}

export interface TravellingLightProps {
  /** Where its middle starts and ends on the film, in film pixels down it. */
  from: number;
  to: number;
  /** How tall it is, and how wide the film, in film pixels. */
  height: number;
  width: number;
  signal: ReplaySignal;
}

/**
 * The stops of the design's --find-light, down the light: white at these
 * alphas. An SVG paint cannot take a CSS gradient, so they are written out;
 * the test holds them to the token.
 */
export const LIGHT_STOPS: readonly (readonly [offset: number, alpha: number])[] = [
  [0, 0],
  [0.34, 0.08],
  [0.5, 0.22],
  [0.66, 0.08],
  [1, 0],
];

/** The stops of the design's --find-light-mask, across the film: clear at both sides, solid across the middle. */
export const MASK_STOPS: readonly (readonly [offset: number, alpha: number])[] = [
  [0, 0],
  [0.26, 1],
  [0.74, 1],
  [1, 0],
];

/**
 * How strong the light is at a share of its journey: 0 at the ends, 1 from
 * `fadedIn` (the design's --light-in) to `fadingFrom` (--light-out), straight
 * in between.
 */
function lightOpacity(share: number, fadedIn: number, fadingFrom: number): number {
  if (share <= 0 || share >= 1) return 0;
  if (share < fadedIn) return share / fadedIn;
  if (share > fadingFrom) return (1 - share) / (1 - fadingFrom);
  return 1;
}

/**
 * The band of light that goes down a film while a search's answer is
 * replayed. One rect in the film's own layer, so it moves with the film
 * through a glide. It is moved by writing `y` and `opacity` straight onto
 * the rect whenever the signal says, so React draws nothing per frame.
 *
 * White at a given alpha laid over a film is the same sum as the design's
 * screen blend, so no blend is asked for. It assumes the film is shown as
 * stored, not turned.
 */
export function TravellingLight({ from, to, height, width, signal, space }: TravellingLightProps & FilmLayerProps) {
  const band = useRef<SVGRectElement>(null);
  // An id that is safe inside url(#…).
  const id = useId().replace(/[^A-Za-z0-9_-]/g, '');
  const fill = `travelling-light-fill-${id}`;
  const fade = `travelling-light-fade-${id}`;
  const mask = `travelling-light-mask-${id}`;

  useLayoutEffect(() => {
    const drawn = band.current;
    if (drawn === null) return;
    // Read once, where the light stands, when it is put on the film.
    const fadedIn = numberOf(drawn, '--light-in');
    const fadingFrom = numberOf(drawn, '--light-out');
    const place = () => {
      const rect = band.current;
      if (rect === null) return;
      const share = signal.share();
      if (share === null) {
        rect.setAttribute('display', 'none');
        return;
      }
      rect.removeAttribute('display');
      rect.setAttribute('y', String(from + (to - from) * share - height / 2));
      rect.setAttribute('opacity', String(lightOpacity(share, fadedIn, fadingFrom)));
    };
    place();
    return signal.subscribe(place);
  }, [from, to, height, signal]);

  // This viewport is in film pixels inside the layer's camera transform.
  const filmHeight = space.placement.size.height;
  return (
    <svg
      className="light"
      viewBox={`0 0 ${String(width)} ${String(filmHeight)}`}
      preserveAspectRatio="none"
      aria-hidden="true"
      style={{ width, height: filmHeight }}
    >
      <defs>
        <linearGradient id={fill} x1={0} y1={0} x2={0} y2={1}>
          {LIGHT_STOPS.map(([offset, alpha]) => (
            <stop key={offset} offset={offset} stopColor="#fff" stopOpacity={alpha} />
          ))}
        </linearGradient>
        <linearGradient id={fade} x1={0} y1={0} x2={1} y2={0}>
          {/* A mask reads light as solid: white at the token's alphas is its black at those alphas. */}
          {MASK_STOPS.map(([offset, alpha]) => (
            <stop key={offset} offset={offset} stopColor="#fff" stopOpacity={alpha} />
          ))}
        </linearGradient>
        <mask id={mask} maskContentUnits="objectBoundingBox">
          <rect width={1} height={1} fill={`url(#${fade})`} />
        </mask>
      </defs>
      <rect ref={band} className="travelling-light" x={0} width={width} height={height} fill={`url(#${fill})`} mask={`url(#${mask})`} />
    </svg>
  );
}

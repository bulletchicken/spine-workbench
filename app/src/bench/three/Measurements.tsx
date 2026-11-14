import { useEffect, useLayoutEffect, useRef, useSyncExternalStore, type CSSProperties } from 'react';
import { useStore } from 'zustand';
import { Button } from '@/building-blocks/Button';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { stopKeysAt } from '@/building-blocks/keyboard/keyboard';
import { useEngine } from '@/engine-room/use-engine';
import { popoverOpen } from '@/on-this-device/popover-open';
import { threeState } from '@/view-state/three-state';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { GENERIC_BONES_NOTE } from '@/words/spine-3d';
import { ASSUMED_FACING_NOTE, SAME_SPINE_NOTE, THREE_WORDS } from '@/words/three';
import { FILMS_WORDS } from '@/words/films';
import { FIND_WORDS } from '@/words/find';
import { READINGS_WORDS as W } from '@/words/spine-readings';
import { startOver } from '../start-over';
import { useSpineReadings } from './use-spine-readings';
import './measurements.css';
const subscribeWidth = (listen: () => void) => { window.addEventListener('resize', listen); return () => { window.removeEventListener('resize', listen); }; };
export function useNarrowThree() { return useSyncExternalStore(subscribeWidth, () => window.innerWidth <= (parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--break-narrow-w')) || 900)); }
export function Measurements({ accepted }: { accepted: AcceptedSpine }) {
  const state = useSpineReadings(accepted);
  const engine = useEngine();
  const sheet = useStore(threeState, (s) => s.sheet);
  const compared = useStore(threeState, (s) => s.compared);
  const moving = useStore(threeState, (s) => s.moving);
  const trust = useStore(popoverOpen, (s) => s.open);
  const narrow = useNarrowThree();
  const panel = useRef<HTMLElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const hidden = compared !== null || narrow && !sheet;
  useKey('sheet', ['Escape'], () => { threeState.getState().showSheet(false); }, () => narrow && sheet);
  useEffect(() => { if (!narrow || trust) threeState.getState().showSheet(false); }, [narrow, trust]);
  useLayoutEffect(() => {
    if (!narrow || !sheet) return;
    const surface = panel.current;
    heading.current?.focus();
    const unstop = stopKeysAt('sheet');
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Element && (panel.current?.contains(event.target) || event.target.closest('[aria-controls="measurements"]') !== null)) return;
      threeState.getState().showSheet(false);
      if (!(event.target instanceof Element) || event.target.closest('.bar') === null) { event.preventDefault(); event.stopPropagation(); }
    };
    document.addEventListener('pointerdown', outside, true);
    return () => {
      unstop(); document.removeEventListener('pointerdown', outside, true);
      if (surface?.contains(document.activeElement)) document.querySelector<HTMLElement>('[aria-controls="measurements"]')?.focus();
    };
  }, [narrow, sheet]);
  const focused = useRef(false);
  return <aside ref={panel} id="measurements" className={`panel glass measurements${narrow ? ' sheet' : ''}`} aria-labelledby="measurements-title" aria-hidden={hidden || undefined} inert={hidden || moving} data-hidden={hidden || undefined}>
    <h2 ref={heading} id="measurements-title" tabIndex={-1}>{W.title}</h2>
    {state.phase === 'loading' && <p className="m-about">{W.loading}</p>}
    {state.phase === 'failed' && <p className="m-about">{W.failed} <Button look="text" onPress={state.retry}>{THREE_WORDS.retry}</Button></p>}
    {state.phase === 'ready' && <><ul className="measures">{state.view.rows.map((row, i) => <li key={row.key} tabIndex={0} style={{ '--i': i } as CSSProperties}
      onFocus={() => { focused.current = true; threeState.getState().light(row.levels); }} onBlur={() => { focused.current = false; threeState.getState().light([]); }}
      onPointerEnter={() => { if (!focused.current) threeState.getState().light(row.levels); }} onPointerLeave={() => { if (!focused.current) threeState.getState().light([]); }}>
      <span className="m-name">{row.name}</span><span className="m-value num">{row.value}<span className="m-band">{row.band}</span></span><span className="m-about">{row.about}</span>
    </li>)}</ul>{state.view.reasons.map((reason) => <p className="m-about" key={reason}>{reason}</p>)}</>}
    <div className="panel-note">
      {state.phase === 'ready' && state.view.notes.map((note) => <p key={note}>{note}</p>)}
      <p>{GENERIC_BONES_NOTE}</p>
      {accepted.fit.sideFilm !== null && <p>{SAME_SPINE_NOTE}</p>}
      {(accepted.fit.frontFilm.facingAssumed || accepted.fit.sideFilm?.facingAssumed === true) && <p>{ASSUMED_FACING_NOTE}</p>}
      {accepted.input.front.marks.naming.flags.includes('estimated') && <p>{FIND_WORDS.estimated}</p>}
    </div>
    <div className="panel-acts"><Button look="text" onPress={() => { void startOver(engine); }}>{FILMS_WORDS.startOver}</Button></div>
  </aside>;
}

import { handState, startFirstPlacement } from './hand-state';
import { WhyHere } from './WhyHere';
import { useEffect, useLayoutEffect, useRef } from 'react';
import { useStore } from 'zustand';
import { Button } from '@/building-blocks/Button';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { stopKeysAt } from '@/building-blocks/keyboard/keyboard';
import type { AcceptedSpine } from '@/view-state/spine-build';
import { CANNOT_TELL_WORDS, PLACE_WORDS as W, PLACE_EDIT_WORDS as E } from '@/words/place';
import { placeState } from './place-state';
import { LevelWords } from '@/building-blocks/LevelWords';
import { loadState } from './load-state';
import { LoadEstimates } from './LoadEstimates';
import { filmSide, limitsForPlacement, reviewItems } from './placement-view';
import '../three/measurements.css';
import './place.css';
export function PlaceSheet({ accepted }: { accepted: AcceptedSpine }) {
  const hand=useStore(handState);
  const loaded=useStore(loadState,(s)=>s.phase==='loaded');
  const { sheet, result } = useStore(placeState), panel = useRef<HTMLElement>(null), heading = useRef<HTMLHeadingElement>(null);
  useKey('sheet', ['Escape'], () => { placeState.setState({ sheet: false }); }, () => sheet);
  useKey('sheet', ['Tab'], (event) => {
    const stops = panel.current?.querySelectorAll<HTMLElement>('button, summary, [tabindex="0"]');
    const first = stops?.[0], last = stops?.[stops.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === heading.current)) last?.focus();
    else if (!event.shiftKey && document.activeElement === last) first?.focus();
    else return false;
    return true;
  }, () => sheet);
  useKey('sheet', ['m'], () => { placeState.setState({ sheet: !sheet }); }, () => !placeState.getState().arriving && (placeState.getState().result?.layouts.length ?? 0) > 0);
  useEffect(() => {
    if (!sheet) return;
    const close = (event: PointerEvent) => { if (event.target instanceof Element && (panel.current?.contains(event.target) || event.target.closest('[aria-controls="place-sheet"]') !== null)) return; placeState.setState({ sheet: false }); };
    document.addEventListener('pointerdown', close, true);
    return () => { document.removeEventListener('pointerdown', close, true); };
  }, [sheet]);
  useLayoutEffect(() => {
    if (!sheet) return;
    const origin = document.activeElement;
    heading.current?.focus(); const unstop = stopKeysAt('sheet');
    return () => { unstop(); if (origin instanceof HTMLElement) origin.focus(); };
  }, [sheet]);
  if (!sheet || result === null) return null;
  const empty = result.layouts.length === 0;
  const items = reviewItems(result, accepted.input.front.marks.namedFor);
  const reasons = [...new Set(result.cannotTell.map((r) => r.code))].map((code) => ({ code, levels: [...new Set(result.cannotTell.filter((r) => r.code === code).flatMap((r) => r.levels))] }));
  const diameters = result.layouts[0]?.anchors.flatMap((a) => a.diametersBeside.flatMap((d) => d.diameterMm === null ? [] : [W.otherDiameter(a.level, filmSide(a.side, accepted.input.front.marks.namedFor), d.diameterMm, d.widthRatio)])) ?? [];
  return <aside className="panel glass measurements sheet place-sheet" id="place-sheet" ref={panel} role="dialog" aria-modal="true" aria-labelledby="place-sheet-title">
    <h2 ref={heading} tabIndex={-1} id="place-sheet-title">{hand.why!==null?E.why:empty ? W.cannotTell : W.needsLook}</h2>
    <WhyHere/>
    {hand.why!==null&&<h2>{W.needsLook}</h2>}
    <ul className="place-sheet-list">{items.map((item, i) => <li key={item.id}><Button look="text" onPress={() => { placeState.setState({ sheet: false, review: i, pressed: null, hovered: null }); }}><span className="place-review-label"><LevelWords>{item.label}</LevelWords></span> <LevelWords>{item.sentences.join(' ')}</LevelWords></Button></li>)}</ul>
    <LoadEstimates accepted={accepted}/>
    {!empty && <h2>{W.cannotTell}</h2>}
    <ul className="place-sheet-list">{reasons.map((r) => <li key={r.code}><LevelWords>{CANNOT_TELL_WORDS[r.code]}</LevelWords>{(r.code === 'ends-unsettled' || r.code === 'twist-not-measured') && r.levels.length > 0 && <span className="num"> {r.levels.join(', ')}</span>}</li>)}</ul>
    <>{diameters.length > 0 && <><h2>{W.otherDiameters}</h2><ul className="place-sheet-list">{diameters.map((line) => <li key={line}><LevelWords>{line}</LevelWords></li>)}</ul></>}</>
    <h2>{W.about}</h2>
    <ul className="place-sheet-list">{limitsForPlacement(loaded,accepted.fit.sideFilm===null,accepted.input.front.geometry.mmPerPx===null).map((words) => <li key={words}><LevelWords>{words}</LevelWords></li>)}</ul>
    <div className="panel-note">{accepted.input.front.facingAssumed && <p>{W.facing}</p>}<p>{W.sizing}</p><p>{W.unsaved}</p></div>
    <div className="panel-acts">{hand.past.length>0&&<Button look="text" busy={hand.busy!==null} onPress={startFirstPlacement}>{E.startFirst}</Button>}<Button look="text" onPress={() => { placeState.setState({ sheet: false }); }}>{W.done}</Button></div>
  </aside>;
}

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react';
import { LevelWords } from '@/building-blocks/LevelWords';
import { Button } from '@/building-blocks/Button';
import { Segmented } from '@/building-blocks/Segmented';
import { LOAD_STOPS, SHOWN_LOAD_STOPS, PLACE_LOAD_WORDS, PLACE_WORDS } from '@/words/place';
import { THREE_WORDS } from '@/words/three';
import { SHELL_WORDS } from '@/words/shell';
import type { DockHelper, DockScene, DockSpec } from './dock-spec';
import { LitButton } from './LitButton';
import './Dock.css';

const SCENES: readonly DockScene[] = ['add', 'reading', 'ready', 'finding', 'found', 'review', 'failed', 'engine', 'building', 'build-failed', 'three', 'compare', 'place'];

/** Counts and progress change silently; the helper's words are spoken with the sentence. */
function helperWords(helper: DockHelper | undefined): string {
  if (helper === undefined) return '';
  if (helper.kind === 'words') return helper.words.map((part) => typeof part === 'string' ? part : part.link).join('');
  return helper.words ?? '';
}

/** A wait's meter that has not moved for this long shows that work goes on (Dock.css). */
export const STILL_AFTER_MS = 1500;

function Meter({ helper, sentence, on }: { helper: Extract<DockHelper, { kind: 'meter' | 'count' }>; sentence: string; on: boolean }) {
  const share = Math.max(0, Math.min(1, helper.share));
  const percent = Math.round(share * 100);
  // Only a wait is watched: a count stands still because nothing new has been counted.
  const watched = on && helper.kind === 'meter';
  const [still, setStill] = useState(false);
  useEffect(() => {
    if (!watched) return;
    const timer = setTimeout(() => { setStill(true); }, STILL_AFTER_MS);
    return () => { clearTimeout(timer); setStill(false); };
  }, [share, watched]);
  return (
    <span
      className={watched && still ? 'meter waiting' : 'meter'}
      role="progressbar"
      aria-label={sentence}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      aria-valuetext={helper.kind === 'count' ? helper.spoken : `${String(percent)}%`}
      style={{ '--p': share } as CSSProperties}
    ><i /></span>
  );
}

function Helper({ helper, sentence, on }: { helper: DockHelper; sentence: string; on: boolean }) {
  const words = helperWords(helper);
  return (
    <>
      {helper.kind === 'look' && <i className="dot" aria-hidden="true" />}
      {helper.kind === 'count' && <span className="num" aria-hidden="true">{helper.count}</span>}
      {(helper.kind === 'meter' || helper.kind === 'count') && <Meter key={helper.kind} helper={helper} sentence={sentence} on={on} />}
      {words !== '' && (
        <span key={words} data-fresh="">
          {helper.kind === 'words'
            ? helper.words.map((part, at) => typeof part === 'string' ? <LevelWords key={at}>{part}</LevelWords> : (
              <button key={at} type="button" className="link" onClick={on ? part.press : undefined}>{part.link}</button>
            ))
            : <LevelWords>{words}</LevelWords>}
        </span>
      )}
    </>
  );
}

function Scene({ name, spec }: { name: DockScene; spec: DockSpec | null }) {
  // Keep the words and buttons of a leaving scene until it is shown again.
  const [last, setLast] = useState(spec);
  if (spec !== null && last !== spec) setLast(spec);
  const shown = spec ?? last;
  const on = spec !== null, root=useRef<HTMLElement>(null), focus=shown?.focus;
  useLayoutEffect(()=>{
    if(!on||focus===undefined)return;
    root.current?.querySelector<HTMLElement>(focus==='chosen-stop'?'.load-stops [aria-current="true"]':focus==='primary'?'.primary':'.button')?.focus();
  },[on,focus,shown?.primary?.label,shown?.quiet?.label]);
  return (
    <section ref={root} className={on ? 'scene on' : 'scene'} data-scene={name} aria-hidden={on ? undefined : true} inert={!on}>
      {shown !== null && (
        <>
          <div className="say">
            <h2><span key={shown.sentence} data-fresh=""><LevelWords>{shown.sentence}</LevelWords></span></h2>
            {shown.helper !== undefined && <p><Helper helper={shown.helper} sentence={shown.sentence} on={on} /></p>}
            {shown.small !== undefined && <p className="dock-small">{shown.small}</p>}
            {shown.review !== undefined && <div className="place-review"><span className="num">{shown.review.count}</span><h3><LevelWords>{shown.review.label}</LevelWords></h3>{shown.review.sentences.map((sentence) => <p key={sentence}><LevelWords>{sentence}</LevelWords></p>)}{shown.review.details?.map((detail) => <details key={detail.summary}><summary>{detail.summary}</summary><ul>{detail.rows.map((row) => <li key={row}><LevelWords>{row}</LevelWords></li>)}</ul></details>)}{shown.review.choices!==undefined&&<div className="place-review-choices">{shown.review.choices.map((choice)=><div key={choice.label}><Button look="text" aria-pressed={choice.chosen} disabled={choice.reason!==null} busy={choice.busy} onPress={choice.press}><LevelWords>{choice.label}</LevelWords></Button>{choice.reason!==null&&<p><LevelWords>{choice.reason}</LevelWords></p>}</div>)}</div>}</div>}
          </div>
          {shown.more !== undefined && <Button look="text" aria-expanded={shown.more.expanded} aria-controls={shown.more.controls} {...(on ? { onPress: shown.more.press } : {})}>{shown.more.label}</Button>}
          {shown.quiet !== undefined && <Button look="text" aria-expanded={shown.quiet.expanded} aria-controls={shown.quiet.controls} {...(on ? { onPress: shown.quiet.press } : {})}>{shown.quiet.label}</Button>}
          {shown.secondary !== undefined && <Button look="standard" size="large" {...(on ? { onPress: shown.secondary.press } : {})}>{shown.secondary.label}</Button>}
          {shown.views !== undefined && <Segmented className="views" label={THREE_WORDS.view} chosen={shown.views.chosen} options={[
            { id: 'front', label: THREE_WORDS.frontView }, { id: 'side', label: THREE_WORDS.sideView }, { id: 'top', label: THREE_WORDS.topView }, ...(shown.views.back === true ? [{ id: 'back', label: PLACE_WORDS.back }] : []),
          ]} onChoose={(name) => { if (on && (name === 'front' || name === 'side' || name === 'top' || name === 'back')) shown.views?.choose(name); }} />}
          {shown.stops !== undefined && <Segmented className="load-stops" label={PLACE_LOAD_WORDS.stops} chosen={shown.stops.chosen} options={SHOWN_LOAD_STOPS.map((id)=>({id,label:LOAD_STOPS[id]}))} onChoose={(name)=>{if(on && SHOWN_LOAD_STOPS.some((stop)=>stop===name))shown.stops?.choose(name as keyof typeof LOAD_STOPS);}} />}
          {shown.primary !== undefined && (
            <LitButton label={shown.primary.label} disabled={shown.primary.disabled} asleep={!on} onPress={shown.primary.press} />
          )}
        </>
      )}
    </section>
  );
}

/** One dock, with the design system's scene transitions and one spoken sentence. */
export function Dock({ spec }: { spec: DockSpec | null }) {
  const helper = helperWords(spec?.helper);
  const spoken = spec === null ? '' : `${spec.sentence}${helper === '' ? '' : ` ${helper}`}`;
  return (
    <footer className="dock glass" aria-label={SHELL_WORDS.dockLabel}>
      <p className="spoken-only" role="status">{spoken}</p>
      {SCENES.map((name) => <Scene key={name} name={name} spec={spec?.scene === name ? spec : null} />)}
    </footer>
  );
}

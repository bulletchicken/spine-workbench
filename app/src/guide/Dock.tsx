import { useEffect, useState, type CSSProperties } from 'react';
import { Button } from '@/building-blocks/Button';
import { Segmented } from '@/building-blocks/Segmented';
import { PLACE_WORDS } from '@/words/place';
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
            ? helper.words.map((part, at) => typeof part === 'string' ? part : (
              <button key={at} type="button" className="link" onClick={on ? part.press : undefined}>{part.link}</button>
            ))
            : words}
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
  const on = spec !== null;
  return (
    <section className={on ? 'scene on' : 'scene'} data-scene={name} aria-hidden={on ? undefined : true} inert={!on}>
      {shown !== null && (
        <>
          <div className="say">
            <h2><span key={shown.sentence} data-fresh="">{shown.sentence}</span></h2>
            {shown.helper !== undefined && <p><Helper helper={shown.helper} sentence={shown.sentence} on={on} /></p>}
            {shown.small !== undefined && <p className="dock-small">{shown.small}</p>}
            {shown.review !== undefined && <div className="place-review glass"><span className="num">{shown.review.count}</span><h3>{shown.review.label}</h3>{shown.review.sentences.map((sentence) => <p key={sentence}>{sentence}</p>)}{shown.review.details?.map((detail) => <details key={detail.summary}><summary>{detail.summary}</summary><ul>{detail.rows.map((row) => <li key={row}>{row}</li>)}</ul></details>)}</div>}
          </div>
          {shown.more !== undefined && <Button look="text" aria-expanded={shown.more.expanded} aria-controls={shown.more.controls} {...(on ? { onPress: shown.more.press } : {})}>{shown.more.label}</Button>}
          {shown.quiet !== undefined && <Button look="text" aria-expanded={shown.quiet.expanded} aria-controls={shown.quiet.controls} {...(on ? { onPress: shown.quiet.press } : {})}>{shown.quiet.label}</Button>}
          {shown.secondary !== undefined && <Button look="standard" size="large" {...(on ? { onPress: shown.secondary.press } : {})}>{shown.secondary.label}</Button>}
          {shown.views !== undefined && <Segmented className="views" label={THREE_WORDS.view} chosen={shown.views.chosen} options={[
            { id: 'front', label: THREE_WORDS.frontView }, { id: 'side', label: THREE_WORDS.sideView }, { id: 'top', label: THREE_WORDS.topView }, ...(shown.views.back === true ? [{ id: 'back', label: PLACE_WORDS.back }] : []),
          ]} onChoose={(name) => { if (on && (name === 'front' || name === 'side' || name === 'top' || name === 'back')) shown.views?.choose(name); }} />}
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

import { Fragment, type ReactNode } from 'react';
import { placeStyle } from '@/choreography/stagger';
import { ChoiceCards } from './ChoiceCards';
import './Guide.css';
import type { GuideSpec, Said } from './guide-spec';
import { LitButton } from './LitButton';
import { QuietActions } from './QuietActions';

function SaidWords({ said }: { said: Said }) {
  return said.map((part, at) => (typeof part === 'string' ? part : <b key={at}>{part.strong}</b>));
}

/** What the spec says, as text: equal for two specs that read the same, whatever functions they carry. */
function wordsOf(spec: GuideSpec): string {
  return JSON.stringify([spec.eyebrow, spec.sentence, spec.body ?? null]);
}

/** What the spec offers, as text. Which choice is lit is left out: a pointer passing over a card changes nothing that is offered. */
function offerOf(spec: GuideSpec): string {
  return JSON.stringify([
    spec.lit === undefined ? null : [spec.lit.label, spec.lit.icon ?? null],
    spec.choices?.map((choice) => [choice.id, choice.keys, choice.title, choice.line, choice.tag ?? null]) ?? null,
    spec.quiet?.map((action) => action.label) ?? null,
  ]);
}

interface GuideProps {
  spec: GuideSpec;
  /** This state's cards, composed by the step. */
  children?: ReactNode;
}

/**
 * The one place that says what to do now: an eyebrow, one sentence, one
 * paragraph, then exactly one of a lit button and a pair of choices, at
 * most two quiet actions, and this state's cards.
 *
 * The words are a polite live region, read as one when they change. They
 * are drawn afresh only when they read differently, so a sentence is
 * spoken once: a render that changes nothing said, such as the pointer
 * crossing a choice, leaves them untouched. Lines that are new arrive
 * with `settle`, one after another, and what stays does not arrive again.
 * A card arrives when it is first drawn, in the turn of its place among
 * the cards; as with any element, it stays while the step keeps drawing
 * the same card in the same place, or under the same key.
 */
export function Guide({ spec, children }: GuideProps) {
  return (
    <div className="guide">
      <div className="guide-say">
        <div className="guide-words" aria-live="polite" aria-atomic="true">
          <Fragment key={wordsOf(spec)}>
            <p className="guide-eyebrow guide-line" style={placeStyle(0)}>
              <SaidWords said={spec.eyebrow} />
            </p>
            <h1 className="guide-sentence guide-line" style={placeStyle(1)}>
              {spec.sentence}
            </h1>
            {spec.body !== undefined && (
              <p className="guide-body guide-line" style={placeStyle(2)}>
                <SaidWords said={spec.body} />
              </p>
            )}
          </Fragment>
        </div>
        <Fragment key={offerOf(spec)}>
          {spec.choices !== undefined ? (
            <div className="guide-action guide-line" style={placeStyle(3)}>
              <ChoiceCards choices={spec.choices} hot={spec.hotChoice} onHot={spec.onHotChoice} />
            </div>
          ) : (
            spec.lit !== undefined && (
              <div className="guide-action guide-line" style={placeStyle(3)}>
                <LitButton label={spec.lit.label} icon={spec.lit.icon} onPress={spec.lit.press} />
              </div>
            )
          )}
          {spec.quiet !== undefined && spec.quiet.length > 0 && (
            <div className="guide-quiet guide-line" style={placeStyle(4)}>
              <QuietActions actions={spec.quiet} />
            </div>
          )}
        </Fragment>
      </div>
      <div className="guide-cards">{children}</div>
    </div>
  );
}

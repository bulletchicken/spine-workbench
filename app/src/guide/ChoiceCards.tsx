import { keyName } from '@/building-blocks/keyboard/keyboard';
import { useKey } from '@/building-blocks/keyboard/use-key';
import { Icon } from '@/look-and-feel/icons/Icon';
import type { IconName } from '@/look-and-feel/icons/icon-name';
import { GUIDE_WORDS } from '@/words/guide';
import './ChoiceCards.css';
import type { Choice } from './guide-spec';

/** An arrow key is drawn, since the app's typefaces have no left or right arrow. */
const ARROW_ICONS: Readonly<Record<string, IconName>> = {
  ArrowLeft: 'arrow-left',
  ArrowRight: 'arrow-right',
  ArrowUp: 'arrow-up',
  ArrowDown: 'arrow-down',
};

/** The key that answers a choice, as its card shows it: an arrow, or the letter in capitals. */
function KeyBadge({ keyboardKey }: { keyboardKey: string }) {
  const arrow = ARROW_ICONS[keyboardKey];
  return (
    <span className="choice-card-key" aria-hidden="true">
      {arrow === undefined ? keyboardKey.toUpperCase() : <Icon name={arrow} />}
    </span>
  );
}

interface ChoiceCardsProps {
  choices: readonly [Choice, Choice];
  /** The choice to light, because the pointer is on its twin on the film. */
  hot?: string | null | undefined;
  /** The pointer or the keyboard's focus came to a card, or left it. */
  onHot?: ((id: string | null) => void) | undefined;
}

/**
 * A question only a person may answer: two equal cards, ink-lined and
 * never solid, with nothing chosen for them. Each card's keys answer it
 * from anywhere, and this is the only place those keys are answered, so
 * an answer is given once; a key held down does not answer again. Enter
 * answers nothing unless a card has the keyboard's focus.
 */
export function ChoiceCards({ choices, hot = null, onHot }: ChoiceCardsProps) {
  useKey(
    'choice',
    choices.flatMap((choice) => choice.keys),
    (event) => {
      if (event.repeat) return;
      const pressed = keyName(event.key);
      choices.find((choice) => choice.keys.some((key) => keyName(key) === pressed))?.choose();
    },
  );

  return (
    <div className="choice-cards" role="group" aria-label={GUIDE_WORDS.chooseOne}>
      {choices.map((choice) => (
        <button
          key={choice.id}
          type="button"
          className="choice-card"
          data-hot={hot === choice.id || undefined}
          aria-keyshortcuts={choice.keys.join(' ')}
          onClick={choice.choose}
          onPointerEnter={() => onHot?.(choice.id)}
          onPointerLeave={() => onHot?.(null)}
          onFocus={() => onHot?.(choice.id)}
          onBlur={() => onHot?.(null)}
        >
          {choice.keys[0] !== undefined && <KeyBadge keyboardKey={choice.keys[0]} />}
          {choice.tag !== undefined && <span className="choice-card-tag">{choice.tag}</span>}
          {choice.figure}
          <b className="choice-card-title">{choice.title}</b>
          <span className="choice-card-line">{choice.line}</span>
        </button>
      ))}
    </div>
  );
}
